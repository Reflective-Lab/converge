// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Experience Store Types — Append-only ledger boundary
//!
//! This module defines the **portable contract** for Converge's experience-store
//! subsystem. It captures append-only events, provenance, and lifecycle
//! transitions without binding to any storage backend.
//!
//! ## Axioms
//!
//! - **Append-only**: Corrections are new events, not mutations
//! - **Audit-first**: Every promotion and policy snapshot is explicit
//! - **Replay clarity**: Replayability downgrades are explicit
//!
//! ## What lives here (converge-core)
//!
//! - `ExperienceEvent` + `ExperienceEventEnvelope`
//! - `ExperienceStore` trait (boundary only)
//! - Query types for events and artifacts
//!
//! ## What stays out
//!
//! - Storage implementation (SurrealDB, SQLite, etc.)
//! - Index definitions and migrations

use serde::{Deserialize, Serialize};

use crate::governed_artifact::{GovernedArtifactState, LifecycleEvent, RollbackRecord};
use crate::kernel_boundary::{
    DecisionStep, KernelPolicy, KernelProposal, ReplayTrace, Replayability,
    ReplayabilityDowngradeReason, RoutingPolicy,
};
use crate::recall::{RecallPolicy, RecallProvenanceEnvelope, RecallQuery};

// ============================================================================
// Event Envelope
// ============================================================================

/// Append-only event envelope.
///
/// The envelope carries stable metadata (ids, timestamps, correlation) and a
/// typed event payload. Implementations store and index envelopes, not raw
/// payloads, to keep provenance queryable without decoding payload JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperienceEventEnvelope {
    /// Unique event identifier (ULID/UUID)
    pub event_id: String,
    /// ISO 8601 timestamp of occurrence
    pub occurred_at: String,
    /// Optional tenant scope
    pub tenant_id: Option<String>,
    /// Correlation ID for chain/run grouping
    pub correlation_id: Option<String>,
    /// Typed event payload
    pub event: ExperienceEvent,
}

impl ExperienceEventEnvelope {
    /// Create a new envelope with a placeholder timestamp.
    ///
    /// Production systems should call `with_timestamp()` to set a trusted time.
    #[must_use]
    pub fn new(event_id: impl Into<String>, event: ExperienceEvent) -> Self {
        Self {
            event_id: event_id.into(),
            occurred_at: Self::now_iso8601(),
            tenant_id: None,
            correlation_id: None,
            event,
        }
    }

    /// Add a tenant scope.
    #[must_use]
    pub fn with_tenant(mut self, tenant_id: impl Into<String>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    /// Add a correlation ID.
    #[must_use]
    pub fn with_correlation(mut self, correlation_id: impl Into<String>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    /// Set explicit timestamp (for replay/testing).
    #[must_use]
    pub fn with_timestamp(mut self, occurred_at: impl Into<String>) -> Self {
        self.occurred_at = occurred_at.into();
        self
    }

    /// Generate ISO 8601 timestamp.
    ///
    /// Note: This returns a placeholder. Production systems should use
    /// `with_timestamp()` to inject a timestamp from a trusted source.
    fn now_iso8601() -> String {
        "1970-01-01T00:00:00Z".to_string()
    }
}

// ============================================================================
// Experience Events
// ============================================================================

/// High-level event kinds for query filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ExperienceEventKind {
    ProposalCreated,
    ProposalValidated,
    FactPromoted,
    RecallExecuted,
    ReplayTraceRecorded,
    ReplayabilityDowngraded,
    ArtifactStateTransitioned,
    ArtifactRollbackRecorded,
    BackendInvoked,
    OutcomeRecorded,
    BudgetExceeded,
    PolicySnapshotCaptured,
    HypothesisResolved,
}

/// Append-only experience event payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ExperienceEvent {
    /// Kernel proposal was created.
    ProposalCreated {
        proposal: KernelProposal,
        chain_id: String,
        step: DecisionStep,
        policy_snapshot_hash: Option<String>,
    },
    /// Proposal was validated (contracts/truths evaluated).
    ProposalValidated {
        proposal_id: String,
        chain_id: String,
        step: DecisionStep,
        contract_results: Vec<ContractResultSnapshot>,
        all_passed: bool,
        validator: String,
    },
    /// Proposal was promoted into a fact.
    FactPromoted {
        proposal_id: String,
        fact_id: String,
        promoted_by: String,
        reason: String,
        requires_human: bool,
    },
    /// Recall operation executed with full provenance.
    RecallExecuted {
        query: RecallQuery,
        provenance: RecallProvenanceEnvelope,
        trace_link_id: Option<String>,
    },
    /// Trace link recorded as a first-class object.
    ReplayTraceRecorded {
        trace_link_id: String,
        trace_link: ReplayTrace,
    },
    /// Replayability downgraded for a trace.
    ReplayabilityDowngraded {
        trace_link_id: String,
        from: Replayability,
        to: Replayability,
        reason: ReplayabilityDowngradeReason,
    },
    /// Governed artifact state transition recorded.
    ArtifactStateTransitioned {
        artifact_id: String,
        artifact_kind: ArtifactKind,
        event: LifecycleEvent,
    },
    /// Governed artifact rollback recorded.
    ArtifactRollbackRecorded { rollback: RollbackRecord },
    /// Backend invocation occurred (useful for audit/latency analysis).
    BackendInvoked {
        backend_name: String,
        adapter_id: Option<String>,
        trace_link_id: String,
        step: DecisionStep,
        policy_snapshot_hash: Option<String>,
    },
    /// Outcome recorded for a chain step.
    OutcomeRecorded {
        chain_id: String,
        step: DecisionStep,
        passed: bool,
        stop_reason: Option<String>,
        latency_ms: Option<u64>,
        tokens: Option<u64>,
        cost_microdollars: Option<u64>,
        backend: Option<String>,
        /// Provider/gateway metadata (Kong headers, OpenRouter cost, etc.).
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        metadata: std::collections::HashMap<String, String>,
    },
    /// Budget exceeded event for a chain/run.
    BudgetExceeded {
        chain_id: String,
        resource: String,
        limit: String,
        observed: Option<String>,
    },
    /// Policy snapshot captured for provenance.
    PolicySnapshotCaptured {
        policy_id: String,
        policy: PolicySnapshot,
        snapshot_hash: String,
        captured_by: String,
    },
    /// A tracked hypothesis reached a terminal outcome.
    HypothesisResolved {
        chain_id: String,
        fact_id: String,
        domain: String,
        claim: String,
        confidence: f64,
        /// "confirmed" | "falsified" | "superseded" | "unresolved"
        outcome: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        contradiction_id: Option<String>,
        formed_cycle: u32,
        resolved_cycle: u32,
    },
}

impl ExperienceEvent {
    /// Get the event kind for filtering.
    #[must_use]
    pub fn kind(&self) -> ExperienceEventKind {
        match self {
            Self::ProposalCreated { .. } => ExperienceEventKind::ProposalCreated,
            Self::ProposalValidated { .. } => ExperienceEventKind::ProposalValidated,
            Self::FactPromoted { .. } => ExperienceEventKind::FactPromoted,
            Self::RecallExecuted { .. } => ExperienceEventKind::RecallExecuted,
            Self::ReplayTraceRecorded { .. } => ExperienceEventKind::ReplayTraceRecorded,
            Self::ReplayabilityDowngraded { .. } => ExperienceEventKind::ReplayabilityDowngraded,
            Self::ArtifactStateTransitioned { .. } => {
                ExperienceEventKind::ArtifactStateTransitioned
            }
            Self::ArtifactRollbackRecorded { .. } => ExperienceEventKind::ArtifactRollbackRecorded,
            Self::BackendInvoked { .. } => ExperienceEventKind::BackendInvoked,
            Self::OutcomeRecorded { .. } => ExperienceEventKind::OutcomeRecorded,
            Self::BudgetExceeded { .. } => ExperienceEventKind::BudgetExceeded,
            Self::PolicySnapshotCaptured { .. } => ExperienceEventKind::PolicySnapshotCaptured,
            Self::HypothesisResolved { .. } => ExperienceEventKind::HypothesisResolved,
        }
    }
}

// ============================================================================
// Supporting Types
// ============================================================================

/// Snapshot of a contract result for validation events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractResultSnapshot {
    pub name: String,
    pub passed: bool,
    pub failure_reason: Option<String>,
}

impl From<crate::kernel_boundary::ContractResult> for ContractResultSnapshot {
    fn from(result: crate::kernel_boundary::ContractResult) -> Self {
        Self {
            name: result.name,
            passed: result.passed,
            failure_reason: result.failure_reason,
        }
    }
}

/// Kind of governed artifact.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArtifactKind {
    Adapter,
    Pack,
    Policy,
    TruthFile,
    EvalSuite,
    Other(String),
}

/// Policy snapshot payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "policy")]
pub enum PolicySnapshot {
    Kernel(KernelPolicy),
    Routing(RoutingPolicy),
    Recall(RecallPolicy),
}

/// Query for experience events.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventQuery {
    pub tenant_id: Option<String>,
    pub time_range: Option<TimeRange>,
    pub kinds: Vec<ExperienceEventKind>,
    pub correlation_id: Option<String>,
    pub chain_id: Option<String>,
    pub limit: Option<usize>,
}

/// Query for governed artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactQuery {
    pub tenant_id: Option<String>,
    pub artifact_id: Option<String>,
    pub kind: Option<ArtifactKind>,
    pub state: Option<GovernedArtifactState>,
    pub limit: Option<usize>,
}

/// Inclusive time range filter (ISO 8601 strings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<String>,
    pub end: Option<String>,
}

// ============================================================================
// Experience Store Trait
// ============================================================================

/// Experience store trait (append-only ledger boundary).
///
/// This is the canonical audit trail interface. Implementations provide
/// append-only event storage and query access for governance, debugging,
/// and downstream analytics.
///
/// See [`converge_experience`] for concrete implementations:
/// `InMemoryExperienceStore`, `SurrealDbExperienceStore`, `LanceDbExperienceStore`.
pub trait ExperienceStore: Send + Sync {
    /// Append a single event.
    fn append_event(&self, event: ExperienceEventEnvelope) -> ExperienceStoreResult<()>;

    /// Append multiple events (best-effort atomicity per implementation).
    fn append_events(&self, events: &[ExperienceEventEnvelope]) -> ExperienceStoreResult<()> {
        for event in events {
            self.append_event(event.clone())?;
        }
        Ok(())
    }

    /// Query events by tenant/time/kind/etc.
    fn query_events(
        &self,
        query: &EventQuery,
    ) -> ExperienceStoreResult<Vec<ExperienceEventEnvelope>>;

    /// Write an artifact lifecycle transition event.
    fn write_artifact_state_transition(
        &self,
        artifact_id: &str,
        artifact_kind: ArtifactKind,
        event: LifecycleEvent,
    ) -> ExperienceStoreResult<()>;

    /// Fetch a trace link by id.
    fn get_trace_link(&self, trace_link_id: &str) -> ExperienceStoreResult<Option<ReplayTrace>>;
}

/// Experience store error type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExperienceStoreError {
    /// Storage layer error with message
    StorageError { message: String },
    /// Query was invalid or unsupported
    InvalidQuery { message: String },
    /// Record not found
    NotFound { message: String },
}

impl std::fmt::Display for ExperienceStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StorageError { message } => write!(f, "Storage error: {}", message),
            Self::InvalidQuery { message } => write!(f, "Invalid query: {}", message),
            Self::NotFound { message } => write!(f, "Not found: {}", message),
        }
    }
}

impl std::error::Error for ExperienceStoreError {}

/// Result type for experience store operations.
pub type ExperienceStoreResult<T> = Result<T, ExperienceStoreError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_kind_mapping() {
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "chain-1".to_string(),
            resource: "tokens".to_string(),
            limit: "1024".to_string(),
            observed: Some("2048".to_string()),
        };
        assert_eq!(event.kind(), ExperienceEventKind::BudgetExceeded);
    }

    #[test]
    fn envelope_builder_sets_fields() {
        let event = ExperienceEvent::OutcomeRecorded {
            chain_id: "chain-1".to_string(),
            step: DecisionStep::Planning,
            passed: true,
            stop_reason: None,
            latency_ms: Some(12),
            tokens: Some(42),
            cost_microdollars: None,
            backend: Some("local".to_string()),
            metadata: Default::default(),
        };
        let envelope = ExperienceEventEnvelope::new("evt-1", event)
            .with_tenant("tenant-a")
            .with_correlation("corr-1")
            .with_timestamp("2026-01-21T12:00:00Z");

        assert_eq!(envelope.event_id, "evt-1");
        assert_eq!(envelope.tenant_id.as_deref(), Some("tenant-a"));
        assert_eq!(envelope.correlation_id.as_deref(), Some("corr-1"));
        assert_eq!(envelope.occurred_at, "2026-01-21T12:00:00Z");
    }
}
