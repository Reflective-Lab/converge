// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Promoter Capability Boundary Trait
//!
//! This module defines the capability boundary trait for promoting validated
//! proposals to Facts. Promoters take `Proposal<Validated>` (which requires proof
//! of validation) and produce `Fact` with complete audit trail.
//!
//! ## Design Philosophy
//!
//! - **Type-state enforcement:** Works with `Proposal<Validated>` which can only
//!   be created after validation. This ensures "no bypass path" at the type level.
//!
//! - **Fact immutability:** Once a `Fact` is created, it's immutable. Corrections
//!   are new events, not mutations (append-only principle).
//!
//! - **GAT async pattern:** Uses generic associated types for zero-cost async
//!   without proc macros or `async_trait`. Keeps core dependency-free.
//!
//! - **Split from validation:** Promotion is a separate capability from validation.
//!   Different authorization boundaries and audit trails.
//!
//! ## Integration with Gate Pattern
//!
//! The `Promoter` trait abstracts the promotion capability that `PromotionGate`
//! uses internally. This allows:
//! - Swapping promotion implementations (immediate, queued, consensus-based)
//! - Testing with mock promoters
//! - Distributed promotion across services
//!
//! ## Error Handling
//!
//! [`PromoterError`] implements [`CapabilityError`](super::error::CapabilityError)
//! for uniform error classification, enabling generic retry/circuit breaker logic.

use super::error::{CapabilityError, ErrorCategory};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::gates::validation::ValidationReport;
use crate::types::{Actor, EvidenceRef, Fact, Proposal, TraceLink, Validated};

/// Boxed future type for dyn-safe trait variant.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// ============================================================================
// Promotion Context
// ============================================================================

/// Context for promotion operations.
///
/// Contains the metadata and evidence required for creating the promotion record.
#[derive(Debug, Clone)]
pub struct PromotionContext {
    /// Actor approving the promotion.
    pub approver: Actor,
    /// Evidence references supporting the promotion.
    pub evidence: Vec<EvidenceRef>,
    /// Trace link for audit/replay.
    pub trace: TraceLink,
}

impl PromotionContext {
    /// Create a new promotion context.
    pub fn new(approver: Actor, trace: TraceLink) -> Self {
        Self {
            approver,
            evidence: Vec::new(),
            trace,
        }
    }

    /// Add evidence to the context.
    pub fn with_evidence(mut self, evidence: Vec<EvidenceRef>) -> Self {
        self.evidence = evidence;
        self
    }

    /// Add a single evidence reference.
    pub fn with_evidence_ref(mut self, evidence: EvidenceRef) -> Self {
        self.evidence.push(evidence);
        self
    }
}

// ============================================================================
// Error Type
// ============================================================================

/// Error type for promotion operations.
///
/// Implements [`CapabilityError`] for uniform error classification.
#[derive(Debug, Clone)]
pub enum PromoterError {
    /// Validation report doesn't match proposal.
    ReportMismatch {
        /// Expected proposal ID from report.
        expected: String,
        /// Actual proposal ID.
        actual: String,
    },
    /// Actor not authorized to promote.
    Unauthorized {
        /// Actor that attempted promotion.
        actor: String,
        /// Reason for denial.
        reason: String,
    },
    /// Proposal already promoted.
    AlreadyPromoted {
        /// Proposal ID.
        proposal_id: String,
        /// Existing fact ID.
        fact_id: String,
    },
    /// Promoter service unavailable.
    Unavailable {
        /// Error message.
        message: String,
    },
    /// Operation timed out.
    Timeout {
        /// Time elapsed before timeout.
        elapsed: Duration,
        /// Configured deadline.
        deadline: Duration,
    },
    /// Storage error during fact creation.
    StorageError {
        /// Error message.
        message: String,
    },
    /// Internal promoter error.
    Internal {
        /// Error message.
        message: String,
    },
}

impl std::fmt::Display for PromoterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ReportMismatch { expected, actual } => {
                write!(
                    f,
                    "validation report mismatch: expected proposal '{}', got '{}'",
                    expected, actual
                )
            }
            Self::Unauthorized { actor, reason } => {
                write!(f, "actor '{}' not authorized: {}", actor, reason)
            }
            Self::AlreadyPromoted {
                proposal_id,
                fact_id,
            } => {
                write!(
                    f,
                    "proposal '{}' already promoted to fact '{}'",
                    proposal_id, fact_id
                )
            }
            Self::Unavailable { message } => write!(f, "promoter unavailable: {}", message),
            Self::Timeout { elapsed, deadline } => {
                write!(
                    f,
                    "promotion timeout after {:?} (deadline: {:?})",
                    elapsed, deadline
                )
            }
            Self::StorageError { message } => write!(f, "storage error: {}", message),
            Self::Internal { message } => write!(f, "internal promoter error: {}", message),
        }
    }
}

impl std::error::Error for PromoterError {}

impl CapabilityError for PromoterError {
    fn category(&self) -> ErrorCategory {
        match self {
            Self::ReportMismatch { .. } => ErrorCategory::InvalidInput,
            Self::Unauthorized { .. } => ErrorCategory::Auth,
            Self::AlreadyPromoted { .. } => ErrorCategory::Conflict,
            Self::Unavailable { .. } => ErrorCategory::Unavailable,
            Self::Timeout { .. } => ErrorCategory::Timeout,
            Self::StorageError { .. } => ErrorCategory::Internal,
            Self::Internal { .. } => ErrorCategory::Internal,
        }
    }

    fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Unavailable { .. } | Self::Timeout { .. } | Self::StorageError { .. }
        )
    }

    fn is_retryable(&self) -> bool {
        // Transient errors are retryable
        // Internal errors may also be retryable (temporary service issues)
        // AlreadyPromoted is NOT retryable (idempotency check)
        self.is_transient() || matches!(self, Self::Internal { .. })
    }

    fn retry_after(&self) -> Option<Duration> {
        // No specific retry-after for promotion errors
        None
    }
}

// ============================================================================
// Static Dispatch Trait (GAT Async Pattern)
// ============================================================================

/// Proposal promotion capability.
///
/// Promotes `Proposal<Validated>` to `Fact` with complete audit trail.
/// This trait uses the GAT async pattern for zero-cost static dispatch.
///
/// # Type-State Integration
///
/// Works with the type-state pattern established in Phase 4:
/// - Input: `Proposal<Validated>` - can only be created after validation
/// - Input: `ValidationReport` - proof that validation occurred
/// - Output: `Fact` - immutable record with promotion provenance
///
/// The type system ensures no bypass path: you cannot call `promote` without
/// first having a `Proposal<Validated>` (which requires validation).
///
/// # Example Implementation
///
/// ```ignore
/// struct ImmediatePromoter {
///     store: Arc<dyn FactStore>,
/// }
///
/// impl Promoter for ImmediatePromoter {
///     type PromoteFut<'a> = impl Future<Output = Result<Fact, PromoterError>> + Send + 'a
///     where
///         Self: 'a;
///
///     fn promote<'a>(
///         &'a self,
///         proposal: Proposal<Validated>,
///         report: &'a ValidationReport,
///         context: &'a PromotionContext,
///     ) -> Self::PromoteFut<'a> {
///         async move {
///             // Create fact and store...
///             Ok(fact)
///         }
///     }
/// }
/// ```
pub trait Promoter: Send + Sync {
    /// Associated future type for promotion.
    ///
    /// Must be `Send` to work with multi-threaded runtimes.
    type PromoteFut<'a>: Future<Output = Result<Fact, PromoterError>> + Send + 'a
    where
        Self: 'a;

    /// Promote a validated proposal to a Fact.
    ///
    /// # Arguments
    ///
    /// * `proposal` - The validated proposal to promote (consumed).
    /// * `report` - The validation report (proof of validation).
    /// * `context` - Promotion context (approver, evidence, trace).
    ///
    /// # Returns
    ///
    /// A future that resolves to the created Fact or an error.
    /// The Fact includes complete promotion provenance.
    fn promote<'a>(
        &'a self,
        proposal: Proposal<Validated>,
        report: &'a ValidationReport,
        context: &'a PromotionContext,
    ) -> Self::PromoteFut<'a>;
}

// ============================================================================
// Dyn-Safe Wrapper (Runtime Polymorphism)
// ============================================================================

/// Dyn-safe promoter for runtime polymorphism.
///
/// Use this trait when you need `dyn Trait` compatibility, such as:
/// - Storing multiple promoter types in a collection
/// - Runtime routing between different promotion strategies
/// - Plugin systems with dynamic loading
///
/// For static dispatch (better performance, no allocation), use [`Promoter`].
///
/// # Blanket Implementation
///
/// Any type implementing [`Promoter`] automatically implements [`DynPromoter`]
/// via a blanket impl that boxes the future.
pub trait DynPromoter: Send + Sync {
    /// Promote a validated proposal to a Fact.
    ///
    /// Returns a boxed future for dyn-safety.
    fn promote<'a>(
        &'a self,
        proposal: Proposal<Validated>,
        report: &'a ValidationReport,
        context: &'a PromotionContext,
    ) -> BoxFuture<'a, Result<Fact, PromoterError>>;
}

// Blanket implementation: Promoter -> DynPromoter
impl<T: Promoter> DynPromoter for T {
    fn promote<'a>(
        &'a self,
        proposal: Proposal<Validated>,
        report: &'a ValidationReport,
        context: &'a PromotionContext,
    ) -> BoxFuture<'a, Result<Fact, PromoterError>> {
        Box::pin(Promoter::promote(self, proposal, report, context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::error::{CapabilityError, ErrorCategory};

    // ── PromoterError Display ────────────────────────────────────────────────

    #[test]
    fn display_report_mismatch() {
        let e = PromoterError::ReportMismatch {
            expected: "p-1".into(),
            actual: "p-2".into(),
        };
        let s = e.to_string();
        assert!(s.contains("p-1"));
        assert!(s.contains("p-2"));
    }

    #[test]
    fn display_unauthorized() {
        let e = PromoterError::Unauthorized {
            actor: "bot-7".into(),
            reason: "no promote scope".into(),
        };
        let s = e.to_string();
        assert!(s.contains("bot-7"));
        assert!(s.contains("no promote scope"));
    }

    #[test]
    fn display_already_promoted() {
        let e = PromoterError::AlreadyPromoted {
            proposal_id: "p-1".into(),
            fact_id: "f-1".into(),
        };
        let s = e.to_string();
        assert!(s.contains("p-1"));
        assert!(s.contains("f-1"));
    }

    #[test]
    fn display_unavailable() {
        let e = PromoterError::Unavailable {
            message: "service down".into(),
        };
        assert!(e.to_string().contains("service down"));
    }

    #[test]
    fn display_timeout() {
        let e = PromoterError::Timeout {
            elapsed: Duration::from_millis(500),
            deadline: Duration::from_millis(200),
        };
        let s = e.to_string();
        assert!(s.contains("500ms"));
        assert!(s.contains("200ms"));
    }

    #[test]
    fn display_storage_error() {
        let e = PromoterError::StorageError {
            message: "write failed".into(),
        };
        assert!(e.to_string().contains("write failed"));
    }

    #[test]
    fn display_internal() {
        let e = PromoterError::Internal {
            message: "bug".into(),
        };
        assert!(e.to_string().contains("bug"));
    }

    // ── CapabilityError classification ───────────────────────────────────────

    #[test]
    fn category_report_mismatch_is_invalid_input() {
        let e = PromoterError::ReportMismatch {
            expected: "x".into(),
            actual: "y".into(),
        };
        assert_eq!(e.category(), ErrorCategory::InvalidInput);
        assert!(!e.is_transient());
        assert!(!e.is_retryable());
    }

    #[test]
    fn category_unauthorized_is_auth() {
        let e = PromoterError::Unauthorized {
            actor: "x".into(),
            reason: "y".into(),
        };
        assert_eq!(e.category(), ErrorCategory::Auth);
        assert!(!e.is_transient());
        assert!(!e.is_retryable());
    }

    #[test]
    fn category_already_promoted_is_conflict() {
        let e = PromoterError::AlreadyPromoted {
            proposal_id: "p".into(),
            fact_id: "f".into(),
        };
        assert_eq!(e.category(), ErrorCategory::Conflict);
        assert!(!e.is_transient());
        assert!(!e.is_retryable());
    }

    #[test]
    fn category_unavailable_is_transient_and_retryable() {
        let e = PromoterError::Unavailable {
            message: "x".into(),
        };
        assert_eq!(e.category(), ErrorCategory::Unavailable);
        assert!(e.is_transient());
        assert!(e.is_retryable());
    }

    #[test]
    fn category_timeout_is_transient_and_retryable() {
        let e = PromoterError::Timeout {
            elapsed: Duration::from_secs(1),
            deadline: Duration::from_secs(1),
        };
        assert_eq!(e.category(), ErrorCategory::Timeout);
        assert!(e.is_transient());
        assert!(e.is_retryable());
    }

    #[test]
    fn category_storage_error_is_internal_transient_retryable() {
        let e = PromoterError::StorageError {
            message: "x".into(),
        };
        assert_eq!(e.category(), ErrorCategory::Internal);
        assert!(e.is_transient());
        assert!(e.is_retryable());
    }

    #[test]
    fn category_internal_is_retryable_not_transient() {
        let e = PromoterError::Internal {
            message: "x".into(),
        };
        assert_eq!(e.category(), ErrorCategory::Internal);
        assert!(!e.is_transient());
        assert!(e.is_retryable());
    }

    #[test]
    fn retry_after_always_none() {
        let errors: Vec<PromoterError> = vec![
            PromoterError::Unavailable {
                message: "x".into(),
            },
            PromoterError::Timeout {
                elapsed: Duration::from_secs(1),
                deadline: Duration::from_secs(1),
            },
            PromoterError::Internal {
                message: "x".into(),
            },
        ];
        for e in &errors {
            assert!(e.retry_after().is_none());
        }
    }

    // ── PromotionContext builder ─────────────────────────────────────────────

    fn sample_actor() -> Actor {
        Actor::human("karl")
    }

    fn sample_trace() -> TraceLink {
        TraceLink::local(crate::types::LocalTrace::new("trace-1", "span-1"))
    }

    #[test]
    fn promotion_context_new_has_no_evidence() {
        let ctx = PromotionContext::new(sample_actor(), sample_trace());
        assert!(ctx.evidence.is_empty());
    }

    #[test]
    fn promotion_context_with_evidence() {
        let evidence = vec![
            EvidenceRef::observation("obs-1".into()),
            EvidenceRef::human_approval("approval-1".into()),
        ];
        let ctx = PromotionContext::new(sample_actor(), sample_trace()).with_evidence(evidence);
        assert_eq!(ctx.evidence.len(), 2);
    }

    #[test]
    fn promotion_context_with_evidence_ref() {
        let ctx = PromotionContext::new(sample_actor(), sample_trace())
            .with_evidence_ref(EvidenceRef::observation("obs-1".into()))
            .with_evidence_ref(EvidenceRef::derived("art-1".into()));
        assert_eq!(ctx.evidence.len(), 2);
    }

    // ── std::error::Error ────────────────────────────────────────────────────

    #[test]
    fn promoter_error_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(PromoterError::Internal {
            message: "test".into(),
        });
        assert!(e.to_string().contains("test"));
    }
}
