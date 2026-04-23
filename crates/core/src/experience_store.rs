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

use crate::StopReason as EngineStopReason;
use crate::gates::hitl::{GateDecision, GateRequest};
use crate::governed_artifact::{GovernedArtifactState, LifecycleEvent, RollbackRecord};
use crate::kernel_boundary::{
    DecisionStep, KernelPolicy, KernelProposal, ReplayTrace, Replayability,
    ReplayabilityDowngradeReason, RoutingPolicy,
};
use crate::recall::{RecallPolicy, RecallProvenanceEnvelope, RecallQuery};
use crate::types::{
    ActorId, ArtifactId, BackendId, ChainId, ContentHash, CorrelationId, DomainId, EventId, FactId,
    PolicyId, ProposalId, TenantId, TensionId, Timestamp, TraceLinkId,
};

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
    pub event_id: EventId,
    /// ISO 8601 timestamp of occurrence
    pub occurred_at: Timestamp,
    /// Optional tenant scope
    pub tenant_id: Option<TenantId>,
    /// Correlation ID for chain/run grouping
    pub correlation_id: Option<CorrelationId>,
    /// Typed event payload
    pub event: ExperienceEvent,
}

impl ExperienceEventEnvelope {
    /// Create a new envelope with a placeholder timestamp.
    ///
    /// Production systems should call `with_timestamp()` to set a trusted time.
    #[must_use]
    pub fn new(event_id: impl Into<EventId>, event: ExperienceEvent) -> Self {
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
    pub fn with_tenant(mut self, tenant_id: impl Into<TenantId>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    /// Add a correlation ID.
    #[must_use]
    pub fn with_correlation(mut self, correlation_id: impl Into<CorrelationId>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    /// Set explicit timestamp (for replay/testing).
    #[must_use]
    pub fn with_timestamp(mut self, occurred_at: impl Into<Timestamp>) -> Self {
        self.occurred_at = occurred_at.into();
        self
    }

    /// Generate ISO 8601 timestamp.
    ///
    /// Note: This returns a placeholder. Production systems should use
    /// `with_timestamp()` to inject a timestamp from a trusted source.
    fn now_iso8601() -> Timestamp {
        Timestamp::epoch()
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
    GateDecisionRecorded,
}

/// Append-only experience event payloads.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ExperienceEvent {
    /// Kernel proposal was created.
    ProposalCreated {
        proposal: KernelProposal,
        chain_id: ChainId,
        step: DecisionStep,
        policy_snapshot_hash: Option<ContentHash>,
    },
    /// Proposal was validated (contracts/truths evaluated).
    ProposalValidated {
        proposal_id: ProposalId,
        chain_id: ChainId,
        step: DecisionStep,
        contract_results: Vec<ContractResultSnapshot>,
        all_passed: bool,
        validator: ActorId,
    },
    /// Proposal was promoted into a fact.
    FactPromoted {
        proposal_id: ProposalId,
        fact_id: FactId,
        promoted_by: ActorId,
        reason: String,
        requires_human: bool,
    },
    /// Recall operation executed with full provenance.
    RecallExecuted {
        query: RecallQuery,
        provenance: RecallProvenanceEnvelope,
        trace_link_id: Option<TraceLinkId>,
    },
    /// Trace link recorded as a first-class object.
    ReplayTraceRecorded {
        trace_link_id: TraceLinkId,
        trace_link: ReplayTrace,
    },
    /// Replayability downgraded for a trace.
    ReplayabilityDowngraded {
        trace_link_id: TraceLinkId,
        from: Replayability,
        to: Replayability,
        reason: ReplayabilityDowngradeReason,
    },
    /// Governed artifact state transition recorded.
    ArtifactStateTransitioned {
        artifact_id: ArtifactId,
        artifact_kind: ArtifactKind,
        event: LifecycleEvent,
    },
    /// Governed artifact rollback recorded.
    ArtifactRollbackRecorded { rollback: RollbackRecord },
    /// Backend invocation occurred (useful for audit/latency analysis).
    BackendInvoked {
        backend_name: BackendId,
        adapter_id: Option<BackendId>,
        trace_link_id: TraceLinkId,
        step: DecisionStep,
        policy_snapshot_hash: Option<ContentHash>,
    },
    /// Outcome recorded for a chain step.
    OutcomeRecorded {
        chain_id: ChainId,
        step: DecisionStep,
        passed: bool,
        stop_reason: Option<EngineStopReason>,
        latency_ms: Option<u64>,
        tokens: Option<u64>,
        cost_microdollars: Option<u64>,
        backend: Option<BackendId>,
        /// Provider/gateway metadata (Kong headers, OpenRouter cost, etc.).
        #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
        metadata: std::collections::HashMap<String, String>,
    },
    /// Budget exceeded event for a chain/run.
    BudgetExceeded {
        chain_id: ChainId,
        resource: BudgetResource,
        limit: String,
        observed: Option<String>,
    },
    /// Policy snapshot captured for provenance.
    PolicySnapshotCaptured {
        policy_id: PolicyId,
        policy: PolicySnapshot,
        snapshot_hash: ContentHash,
        captured_by: ActorId,
    },
    /// A tracked hypothesis reached a terminal outcome.
    HypothesisResolved {
        chain_id: ChainId,
        fact_id: FactId,
        domain: DomainId,
        claim: String,
        confidence: f64,
        outcome: HypothesisOutcome,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        contradiction_id: Option<TensionId>,
        formed_cycle: u32,
        resolved_cycle: u32,
    },
    /// Human decision on a HITL gate recorded for audit and later policy mining.
    GateDecisionRecorded {
        request: GateRequest,
        decision: GateDecision,
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
            Self::GateDecisionRecorded { .. } => ExperienceEventKind::GateDecisionRecorded,
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

/// Budget dimension that was exhausted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BudgetResource {
    EngineBudget,
    Tokens,
    Facts,
    Cycles,
    Time,
    Cost,
    Other(String),
}

/// Terminal outcome for a tracked hypothesis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HypothesisOutcome {
    Confirmed,
    Falsified,
    Superseded,
    Unresolved,
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
    pub tenant_id: Option<TenantId>,
    pub time_range: Option<TimeRange>,
    pub kinds: Vec<ExperienceEventKind>,
    pub correlation_id: Option<CorrelationId>,
    pub chain_id: Option<ChainId>,
    pub limit: Option<usize>,
}

/// Query for governed artifacts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ArtifactQuery {
    pub tenant_id: Option<TenantId>,
    pub artifact_id: Option<ArtifactId>,
    pub kind: Option<ArtifactKind>,
    pub state: Option<GovernedArtifactState>,
    pub limit: Option<usize>,
}

/// Inclusive time range filter (ISO 8601 strings).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: Option<Timestamp>,
    pub end: Option<Timestamp>,
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
        artifact_id: &ArtifactId,
        artifact_kind: ArtifactKind,
        event: LifecycleEvent,
    ) -> ExperienceStoreResult<()>;

    /// Fetch a trace link by id.
    fn get_trace_link(
        &self,
        trace_link_id: &TraceLinkId,
    ) -> ExperienceStoreResult<Option<ReplayTrace>>;
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
            chain_id: "chain-1".into(),
            resource: BudgetResource::Tokens,
            limit: "1024".to_string(),
            observed: Some("2048".to_string()),
        };
        assert_eq!(event.kind(), ExperienceEventKind::BudgetExceeded);
    }

    #[test]
    fn envelope_builder_sets_fields() {
        let event = ExperienceEvent::OutcomeRecorded {
            chain_id: "chain-1".into(),
            step: DecisionStep::Planning,
            passed: true,
            stop_reason: None,
            latency_ms: Some(12),
            tokens: Some(42),
            cost_microdollars: None,
            backend: Some("local".into()),
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

    // ── ExperienceEvent::kind() exhaustive coverage ──────────────────────────

    #[test]
    fn event_kind_proposal_created() {
        let event = ExperienceEvent::ProposalCreated {
            proposal: crate::kernel_boundary::KernelProposal {
                id: "p-1".into(),
                kind: crate::kernel_boundary::ProposalKind::Claims,
                payload: "test".into(),
                structured_payload: None,
                trace_link: crate::kernel_boundary::ReplayTrace::Local(
                    crate::kernel_boundary::LocalReplayTrace {
                        base_model_hash: "abc".into(),
                        adapter: None,
                        tokenizer_hash: "tok".into(),
                        seed: 42,
                        sampler: crate::kernel_boundary::SamplerParams::default(),
                        prompt_version: "v1".into(),
                        recall: None,
                        weights_mutated: false,
                        execution_env: crate::kernel_boundary::ExecutionEnv::default(),
                    },
                ),
                contract_results: vec![crate::kernel_boundary::ContractResult::passed(
                    "grounded-answering",
                )],
                requires_human: false,
                confidence: Some(0.9),
            },
            chain_id: "c-1".into(),
            step: DecisionStep::Planning,
            policy_snapshot_hash: None,
        };
        assert_eq!(event.kind(), ExperienceEventKind::ProposalCreated);
    }

    #[test]
    fn event_kind_fact_promoted() {
        let event = ExperienceEvent::FactPromoted {
            proposal_id: "p-1".into(),
            fact_id: "f-1".into(),
            promoted_by: "engine".into(),
            reason: "validated".into(),
            requires_human: false,
        };
        assert_eq!(event.kind(), ExperienceEventKind::FactPromoted);
    }

    #[test]
    fn event_kind_hypothesis_resolved() {
        let event = ExperienceEvent::HypothesisResolved {
            chain_id: "c-1".into(),
            fact_id: "f-1".into(),
            domain: "market".into(),
            claim: "price will increase".into(),
            confidence: 0.85,
            outcome: HypothesisOutcome::Confirmed,
            contradiction_id: None,
            formed_cycle: 1,
            resolved_cycle: 3,
        };
        assert_eq!(event.kind(), ExperienceEventKind::HypothesisResolved);
    }

    #[test]
    fn event_kind_policy_snapshot_captured() {
        let event = ExperienceEvent::PolicySnapshotCaptured {
            policy_id: "pol-1".into(),
            policy: PolicySnapshot::Routing(crate::kernel_boundary::RoutingPolicy::default()),
            snapshot_hash: ContentHash::zero(),
            captured_by: "engine".into(),
        };
        assert_eq!(event.kind(), ExperienceEventKind::PolicySnapshotCaptured);
    }

    // ── ExperienceStoreError ─────────────────────────────────────────────────

    #[test]
    fn store_error_display_storage() {
        let e = ExperienceStoreError::StorageError {
            message: "disk full".into(),
        };
        assert!(e.to_string().contains("disk full"));
    }

    #[test]
    fn store_error_display_invalid_query() {
        let e = ExperienceStoreError::InvalidQuery {
            message: "bad filter".into(),
        };
        assert!(e.to_string().contains("bad filter"));
    }

    #[test]
    fn store_error_display_not_found() {
        let e = ExperienceStoreError::NotFound {
            message: "trace-99".into(),
        };
        assert!(e.to_string().contains("trace-99"));
    }

    #[test]
    fn store_error_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(ExperienceStoreError::StorageError {
            message: "test".into(),
        });
        assert!(!e.to_string().is_empty());
    }

    // ── ArtifactKind equality ────────────────────────────────────────────────

    #[test]
    fn artifact_kind_equality_named() {
        assert_eq!(ArtifactKind::Adapter, ArtifactKind::Adapter);
        assert_ne!(ArtifactKind::Pack, ArtifactKind::Policy);
    }

    #[test]
    fn artifact_kind_other_variant() {
        let a = ArtifactKind::Other("custom".into());
        let b = ArtifactKind::Other("custom".into());
        assert_eq!(a, b);
        assert_ne!(
            ArtifactKind::Other("x".into()),
            ArtifactKind::Other("y".into())
        );
    }

    // ── ContractResultSnapshot From conversion ───────────────────────────────

    #[test]
    fn contract_result_snapshot_from_contract_result() {
        let cr = crate::kernel_boundary::ContractResult {
            name: "schema-check".into(),
            passed: false,
            failure_reason: Some("missing field".into()),
        };
        let snap: ContractResultSnapshot = cr.into();
        assert_eq!(snap.name, "schema-check");
        assert!(!snap.passed);
        assert_eq!(snap.failure_reason.as_deref(), Some("missing field"));
    }

    // ── EventQuery defaults ──────────────────────────────────────────────────

    #[test]
    fn event_query_default_is_empty() {
        let q = EventQuery::default();
        assert!(q.tenant_id.is_none());
        assert!(q.kinds.is_empty());
        assert!(q.correlation_id.is_none());
        assert!(q.chain_id.is_none());
        assert!(q.limit.is_none());
    }

    // ── Envelope without optional fields ─────────────────────────────────────

    #[test]
    fn envelope_minimal_no_optional_fields() {
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "c".into(),
            resource: BudgetResource::Cycles,
            limit: "10".into(),
            observed: None,
        };
        let env = ExperienceEventEnvelope::new("e-1", event);
        assert!(env.tenant_id.is_none());
        assert!(env.correlation_id.is_none());
        assert_eq!(env.occurred_at, "1970-01-01T00:00:00Z");
    }

    // ── Serde roundtrips ─────────────────────────────────────────────────────

    #[test]
    fn experience_event_kind_serde_roundtrip() {
        let kinds = [
            ExperienceEventKind::ProposalCreated,
            ExperienceEventKind::ProposalValidated,
            ExperienceEventKind::FactPromoted,
            ExperienceEventKind::RecallExecuted,
            ExperienceEventKind::ReplayTraceRecorded,
            ExperienceEventKind::ReplayabilityDowngraded,
            ExperienceEventKind::ArtifactStateTransitioned,
            ExperienceEventKind::ArtifactRollbackRecorded,
            ExperienceEventKind::BackendInvoked,
            ExperienceEventKind::OutcomeRecorded,
            ExperienceEventKind::BudgetExceeded,
            ExperienceEventKind::PolicySnapshotCaptured,
            ExperienceEventKind::HypothesisResolved,
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let back: ExperienceEventKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, kind);
        }
    }

    #[test]
    fn artifact_kind_serde_roundtrip() {
        let kinds = [
            ArtifactKind::Adapter,
            ArtifactKind::Pack,
            ArtifactKind::Policy,
            ArtifactKind::TruthFile,
            ArtifactKind::EvalSuite,
            ArtifactKind::Other("custom".into()),
        ];
        for kind in kinds {
            let json = serde_json::to_string(&kind).unwrap();
            let back: ArtifactKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, kind);
        }
    }
}
