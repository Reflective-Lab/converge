// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Validator Capability Boundary Trait
//!
//! This module defines the capability boundary trait for proposal validation.
//! Validators examine `Proposal<Draft>` and produce `ValidationReport` evidence
//! that validation occurred.
//!
//! ## Design Philosophy
//!
//! - **Type-state enforcement:** Works with `Proposal<Draft>` from the type-state
//!   pattern established in Phase 4. Validators only accept draft proposals.
//!
//! - **Proof production:** Validators produce `ValidationReport` which serves as
//!   cryptographic proof that validation occurred. Reports cannot be forged.
//!
//! - **GAT async pattern:** Uses generic associated types for zero-cost async
//!   without proc macros or `async_trait`. Keeps core dependency-free.
//!
//! - **Split from promotion:** Validation and promotion are separate capabilities.
//!   A validator validates; a promoter promotes. This allows different authorization
//!   boundaries and audit trails.
//!
//! ## Integration with Gate Pattern
//!
//! The `Validator` trait abstracts the validation capability that `PromotionGate`
//! uses internally. This allows:
//! - Swapping validation implementations (rule-based, ML-based, hybrid)
//! - Testing with mock validators
//! - Distributed validation across services
//!
//! ## Error Handling
//!
//! [`ValidatorError`] implements [`CapabilityError`](super::error::CapabilityError)
//! for uniform error classification, enabling generic retry/circuit breaker logic.

use super::error::{CapabilityError, ErrorCategory};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use crate::gates::validation::{ValidationPolicy, ValidationReport};
use crate::types::{Draft, Proposal};

/// Boxed future type for dyn-safe trait variant.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// ============================================================================
// Error Type
// ============================================================================

/// Error type for validation operations.
///
/// Implements [`CapabilityError`] for uniform error classification.
#[derive(Debug, Clone)]
pub enum ValidatorError {
    /// Validation check failed.
    CheckFailed {
        /// Name of the failed check.
        check_name: String,
        /// Reason for failure.
        reason: String,
    },
    /// Policy violation detected.
    PolicyViolation {
        /// Policy that was violated.
        policy: String,
        /// Description of violation.
        message: String,
    },
    /// Required evidence missing.
    MissingEvidence {
        /// What evidence was expected.
        expected: String,
    },
    /// Validator service unavailable.
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
    /// Internal validator error.
    Internal {
        /// Error message.
        message: String,
    },
}

impl std::fmt::Display for ValidatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CheckFailed { check_name, reason } => {
                write!(f, "validation check '{}' failed: {}", check_name, reason)
            }
            Self::PolicyViolation { policy, message } => {
                write!(f, "policy '{}' violated: {}", policy, message)
            }
            Self::MissingEvidence { expected } => {
                write!(f, "missing required evidence: {}", expected)
            }
            Self::Unavailable { message } => write!(f, "validator unavailable: {}", message),
            Self::Timeout { elapsed, deadline } => {
                write!(
                    f,
                    "validation timeout after {:?} (deadline: {:?})",
                    elapsed, deadline
                )
            }
            Self::Internal { message } => write!(f, "internal validator error: {}", message),
        }
    }
}

impl std::error::Error for ValidatorError {}

impl CapabilityError for ValidatorError {
    fn category(&self) -> ErrorCategory {
        match self {
            Self::CheckFailed { .. } => ErrorCategory::InvalidInput,
            Self::PolicyViolation { .. } => ErrorCategory::InvalidInput,
            Self::MissingEvidence { .. } => ErrorCategory::InvalidInput,
            Self::Unavailable { .. } => ErrorCategory::Unavailable,
            Self::Timeout { .. } => ErrorCategory::Timeout,
            Self::Internal { .. } => ErrorCategory::Internal,
        }
    }

    fn is_transient(&self) -> bool {
        matches!(self, Self::Unavailable { .. } | Self::Timeout { .. })
    }

    fn is_retryable(&self) -> bool {
        // Transient errors are retryable
        // Internal errors may also be retryable (temporary service issues)
        self.is_transient() || matches!(self, Self::Internal { .. })
    }

    fn retry_after(&self) -> Option<Duration> {
        // No specific retry-after for validation errors
        None
    }
}

// ============================================================================
// Static Dispatch Trait (GAT Async Pattern)
// ============================================================================

/// Proposal validation capability.
///
/// Validates `Proposal<Draft>` and produces `ValidationReport` as proof.
/// This trait uses the GAT async pattern for zero-cost static dispatch.
///
/// # Type-State Integration
///
/// Works with the type-state pattern established in Phase 4:
/// - Input: `Proposal<Draft>` - publicly constructible
/// - Output: `ValidationReport` - proof that validation occurred
///
/// The report can then be used by a `Promoter` to create `Proposal<Validated>`.
///
/// # Example Implementation
///
/// ```ignore
/// struct RuleBasedValidator {
///     rules: Vec<ValidationRule>,
/// }
///
/// impl Validator for RuleBasedValidator {
///     type ValidateFut<'a> = impl Future<Output = Result<ValidationReport, ValidatorError>> + Send + 'a
///     where
///         Self: 'a;
///
///     fn validate<'a>(
///         &'a self,
///         proposal: &'a Proposal<Draft>,
///         policy: &'a ValidationPolicy,
///     ) -> Self::ValidateFut<'a> {
///         async move {
///             // Run rules against proposal...
///             Ok(report)
///         }
///     }
/// }
/// ```
pub trait Validator: Send + Sync {
    /// Associated future type for validation.
    ///
    /// Must be `Send` to work with multi-threaded runtimes.
    type ValidateFut<'a>: Future<Output = Result<ValidationReport, ValidatorError>> + Send + 'a
    where
        Self: 'a;

    /// Validate a draft proposal against the given policy.
    ///
    /// # Arguments
    ///
    /// * `proposal` - The draft proposal to validate.
    /// * `policy` - The validation policy to apply.
    ///
    /// # Returns
    ///
    /// A future that resolves to the validation report or an error.
    /// The report serves as proof that validation occurred.
    fn validate<'a>(
        &'a self,
        proposal: &'a Proposal<Draft>,
        policy: &'a ValidationPolicy,
    ) -> Self::ValidateFut<'a>;
}

// ============================================================================
// Dyn-Safe Wrapper (Runtime Polymorphism)
// ============================================================================

/// Dyn-safe validator for runtime polymorphism.
///
/// Use this trait when you need `dyn Trait` compatibility, such as:
/// - Storing multiple validator types in a collection
/// - Runtime routing between different validation strategies
/// - Plugin systems with dynamic loading
///
/// For static dispatch (better performance, no allocation), use [`Validator`].
///
/// # Blanket Implementation
///
/// Any type implementing [`Validator`] automatically implements [`DynValidator`]
/// via a blanket impl that boxes the future.
pub trait DynValidator: Send + Sync {
    /// Validate a draft proposal against the given policy.
    ///
    /// Returns a boxed future for dyn-safety.
    fn validate<'a>(
        &'a self,
        proposal: &'a Proposal<Draft>,
        policy: &'a ValidationPolicy,
    ) -> BoxFuture<'a, Result<ValidationReport, ValidatorError>>;
}

// Blanket implementation: Validator -> DynValidator
impl<T: Validator> DynValidator for T {
    fn validate<'a>(
        &'a self,
        proposal: &'a Proposal<Draft>,
        policy: &'a ValidationPolicy,
    ) -> BoxFuture<'a, Result<ValidationReport, ValidatorError>> {
        Box::pin(Validator::validate(self, proposal, policy))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::error::{CapabilityError, ErrorCategory};

    // ── ValidatorError Display ────────────────────────────────────────────────

    #[test]
    fn display_check_failed() {
        let e = ValidatorError::CheckFailed {
            check_name: "schema".into(),
            reason: "missing required field".into(),
        };
        let s = e.to_string();
        assert!(s.contains("schema"));
        assert!(s.contains("missing required field"));
    }

    #[test]
    fn display_policy_violation() {
        let e = ValidatorError::PolicyViolation {
            policy: "no-pii".into(),
            message: "SSN detected".into(),
        };
        let s = e.to_string();
        assert!(s.contains("no-pii"));
        assert!(s.contains("SSN detected"));
    }

    #[test]
    fn display_missing_evidence() {
        let e = ValidatorError::MissingEvidence {
            expected: "receipt attachment".into(),
        };
        assert!(e.to_string().contains("receipt attachment"));
    }

    #[test]
    fn display_unavailable() {
        let e = ValidatorError::Unavailable {
            message: "connection refused".into(),
        };
        assert!(e.to_string().contains("connection refused"));
    }

    #[test]
    fn display_timeout() {
        let e = ValidatorError::Timeout {
            elapsed: Duration::from_secs(5),
            deadline: Duration::from_secs(3),
        };
        let s = e.to_string();
        assert!(s.contains("5s"));
        assert!(s.contains("3s"));
    }

    #[test]
    fn display_internal() {
        let e = ValidatorError::Internal {
            message: "null pointer".into(),
        };
        assert!(e.to_string().contains("null pointer"));
    }

    // ── CapabilityError classification ───────────────────────────────────────

    #[test]
    fn category_check_failed_is_invalid_input() {
        let e = ValidatorError::CheckFailed {
            check_name: "x".into(),
            reason: "y".into(),
        };
        assert_eq!(e.category(), ErrorCategory::InvalidInput);
        assert!(!e.is_transient());
        assert!(!e.is_retryable());
    }

    #[test]
    fn category_policy_violation_is_invalid_input() {
        let e = ValidatorError::PolicyViolation {
            policy: "x".into(),
            message: "y".into(),
        };
        assert_eq!(e.category(), ErrorCategory::InvalidInput);
        assert!(!e.is_transient());
    }

    #[test]
    fn category_missing_evidence_is_invalid_input() {
        let e = ValidatorError::MissingEvidence {
            expected: "x".into(),
        };
        assert_eq!(e.category(), ErrorCategory::InvalidInput);
    }

    #[test]
    fn category_unavailable_is_transient_and_retryable() {
        let e = ValidatorError::Unavailable {
            message: "down".into(),
        };
        assert_eq!(e.category(), ErrorCategory::Unavailable);
        assert!(e.is_transient());
        assert!(e.is_retryable());
    }

    #[test]
    fn category_timeout_is_transient_and_retryable() {
        let e = ValidatorError::Timeout {
            elapsed: Duration::from_secs(1),
            deadline: Duration::from_secs(1),
        };
        assert_eq!(e.category(), ErrorCategory::Timeout);
        assert!(e.is_transient());
        assert!(e.is_retryable());
    }

    #[test]
    fn category_internal_is_retryable_but_not_transient() {
        let e = ValidatorError::Internal {
            message: "oom".into(),
        };
        assert_eq!(e.category(), ErrorCategory::Internal);
        assert!(!e.is_transient());
        assert!(e.is_retryable());
    }

    #[test]
    fn retry_after_always_none() {
        let errors: Vec<ValidatorError> = vec![
            ValidatorError::CheckFailed {
                check_name: "x".into(),
                reason: "y".into(),
            },
            ValidatorError::Unavailable {
                message: "x".into(),
            },
            ValidatorError::Timeout {
                elapsed: Duration::from_secs(1),
                deadline: Duration::from_secs(1),
            },
        ];
        for e in &errors {
            assert!(e.retry_after().is_none());
        }
    }

    // ── std::error::Error ────────────────────────────────────────────────────

    #[test]
    fn validator_error_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(ValidatorError::Internal {
            message: "test".into(),
        });
        assert!(e.to_string().contains("test"));
    }
}
