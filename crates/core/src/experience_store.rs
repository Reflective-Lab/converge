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
    GateId, PolicyId, ProposalId, TenantId, TensionId, Timestamp, TraceLinkId,
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

    /// Append a single user-side experience event.
    ///
    /// Default implementation returns `Unsupported`. In-process backends
    /// override this to record the event in the same ledger as engine events.
    fn append_user_event(&self, _event: UserExperienceEventEnvelope) -> ExperienceStoreResult<()> {
        Err(ExperienceStoreError::StorageError {
            message: "user-side events are not supported by this backend".to_string(),
        })
    }

    /// Query both engine-side and user-side records, ordered by occurrence.
    ///
    /// Default implementation lifts engine events through the
    /// [`ExperienceRecord::Engine`] variant. Backends that store user events
    /// override this to interleave both record kinds.
    fn query_records(&self, query: &EventQuery) -> ExperienceStoreResult<Vec<ExperienceRecord>> {
        Ok(self
            .query_events(query)?
            .into_iter()
            .map(ExperienceRecord::Engine)
            .collect())
    }
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

// ============================================================================
// User-side Experience Events (sibling type to ExperienceEvent)
// ============================================================================

/// What a user override applies to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id")]
pub enum OverrideTarget {
    Fact(FactId),
    Proposal(ProposalId),
    Constraint(String),
}

/// User-side experience event.
///
/// This is the trust-transfer counterpart to [`ExperienceEvent`]: every variant
/// records a deliberate human act that adjusts engine state — approval,
/// rejection, override, correction, or boundary change. Operator surfaces such
/// as Helms emit these events; planning consumes them through recall to weight
/// future priors.
///
/// Kept as a sibling enum (not a variant of `ExperienceEvent`) so additions on
/// either side stay non-breaking for downstream crates.io consumers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum UserExperienceEvent {
    /// A human approved a paused gate request.
    UserApprovalGranted {
        gate_request_id: GateId,
        actor: ActorId,
        policy_snapshot_hash: Option<ContentHash>,
        reason: Option<String>,
    },
    /// A human issued an override against a fact, proposal, or constraint.
    UserOverrideIssued {
        target: OverrideTarget,
        actor: ActorId,
        policy_snapshot_hash: Option<ContentHash>,
        reason: String,
    },
}

/// Envelope for a [`UserExperienceEvent`] — mirrors [`ExperienceEventEnvelope`]
/// for the user-side ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserExperienceEventEnvelope {
    pub event_id: EventId,
    pub occurred_at: Timestamp,
    pub tenant_id: Option<TenantId>,
    pub correlation_id: Option<CorrelationId>,
    pub event: UserExperienceEvent,
}

impl UserExperienceEventEnvelope {
    #[must_use]
    pub fn new(event_id: impl Into<EventId>, event: UserExperienceEvent) -> Self {
        Self {
            event_id: event_id.into(),
            occurred_at: Timestamp::epoch(),
            tenant_id: None,
            correlation_id: None,
            event,
        }
    }

    #[must_use]
    pub fn with_tenant(mut self, tenant_id: impl Into<TenantId>) -> Self {
        self.tenant_id = Some(tenant_id.into());
        self
    }

    #[must_use]
    pub fn with_correlation(mut self, correlation_id: impl Into<CorrelationId>) -> Self {
        self.correlation_id = Some(correlation_id.into());
        self
    }

    #[must_use]
    pub fn with_timestamp(mut self, occurred_at: impl Into<Timestamp>) -> Self {
        self.occurred_at = occurred_at.into();
        self
    }
}

/// Unified query result spanning both ledger sides.
///
/// Recall and audit consumers iterate `ExperienceRecord` rather than the two
/// envelope types directly, so a UserOverrideIssued and an OutcomeRecorded can
/// both feed the same prior calibration without the consumer needing to call
/// two stores.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum ExperienceRecord {
    Engine(ExperienceEventEnvelope),
    User(UserExperienceEventEnvelope),
}

impl ExperienceRecord {
    #[must_use]
    pub fn correlation_id(&self) -> Option<&CorrelationId> {
        match self {
            Self::Engine(env) => env.correlation_id.as_ref(),
            Self::User(env) => env.correlation_id.as_ref(),
        }
    }

    #[must_use]
    pub fn tenant_id(&self) -> Option<&TenantId> {
        match self {
            Self::Engine(env) => env.tenant_id.as_ref(),
            Self::User(env) => env.tenant_id.as_ref(),
        }
    }

    #[must_use]
    pub fn occurred_at(&self) -> &Timestamp {
        match self {
            Self::Engine(env) => &env.occurred_at,
            Self::User(env) => &env.occurred_at,
        }
    }
}

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

    // ── Envelope serialization (pack) ────────────────────────────────────────

    #[test]
    fn envelope_pack_with_all_fields() {
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "c-1".into(),
            resource: BudgetResource::Tokens,
            limit: "1024".to_string(),
            observed: Some("2048".to_string()),
        };
        let env = ExperienceEventEnvelope::new("evt-abc123", event)
            .with_tenant("tenant-prod")
            .with_correlation("corr-xyz789")
            .with_timestamp("2026-04-28T15:30:45Z");

        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("evt-abc123"));
        assert!(json.contains("tenant-prod"));
        assert!(json.contains("corr-xyz789"));
        assert!(json.contains("2026-04-28T15:30:45Z"));
    }

    #[test]
    fn envelope_pack_minimal() {
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "c".into(),
            resource: BudgetResource::Cycles,
            limit: "5".to_string(),
            observed: None,
        };
        let env = ExperienceEventEnvelope::new("e1", event);

        let json = serde_json::to_string(&env).unwrap();
        assert!(json.contains("e1"));
        assert!(json.contains("1970-01-01T00:00:00Z"));
        // Optional fields should still serialize as null
        assert!(json.contains("tenant_id"));
        assert!(json.contains("correlation_id"));
    }

    // ── Envelope deserialization (unpack) ────────────────────────────────────

    #[test]
    fn envelope_unpack_with_all_fields() {
        let json = r#"{
            "event_id": "evt-1",
            "occurred_at": "2026-04-28T12:00:00Z",
            "tenant_id": "tenant-x",
            "correlation_id": "corr-1",
            "event": {
                "type": "BudgetExceeded",
                "data": {
                    "chain_id": "c-1",
                    "resource": "Tokens",
                    "limit": "999",
                    "observed": "500"
                }
            }
        }"#;

        let env: ExperienceEventEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(env.event_id, "evt-1");
        assert_eq!(env.occurred_at, "2026-04-28T12:00:00Z");
        assert_eq!(env.tenant_id.as_deref(), Some("tenant-x"));
        assert_eq!(env.correlation_id.as_deref(), Some("corr-1"));
    }

    #[test]
    fn envelope_unpack_missing_optional_fields() {
        let json = r#"{
            "event_id": "evt-minimal",
            "occurred_at": "2026-01-01T00:00:00Z",
            "tenant_id": null,
            "correlation_id": null,
            "event": {
                "type": "BudgetExceeded",
                "data": {
                    "chain_id": "c",
                    "resource": "Cycles",
                    "limit": "1",
                    "observed": null
                }
            }
        }"#;

        let env: ExperienceEventEnvelope = serde_json::from_str(json).unwrap();
        assert!(env.tenant_id.is_none());
        assert!(env.correlation_id.is_none());
    }

    #[test]
    fn envelope_unpack_missing_optional_keys_entirely() {
        let json = r#"{
            "event_id": "evt-sparse",
            "occurred_at": "2026-01-01T00:00:00Z",
            "event": {
                "type": "BudgetExceeded",
                "data": {
                    "chain_id": "c",
                    "resource": "Facts",
                    "limit": "10",
                    "observed": null
                }
            }
        }"#;

        let env: ExperienceEventEnvelope = serde_json::from_str(json).unwrap();
        assert_eq!(env.event_id, "evt-sparse");
        assert!(env.tenant_id.is_none());
        assert!(env.correlation_id.is_none());
    }

    // ── Roundtrip: pack then unpack ──────────────────────────────────────────

    #[test]
    fn envelope_roundtrip_complete() {
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "chain-rt".into(),
            resource: BudgetResource::Tokens,
            limit: "777".to_string(),
            observed: Some("333".to_string()),
        };
        let original = ExperienceEventEnvelope::new("evt-rt", event)
            .with_tenant("tenant-rt")
            .with_correlation("corr-rt")
            .with_timestamp("2026-04-28T10:15:30Z");

        let json = serde_json::to_string(&original).unwrap();
        let restored: ExperienceEventEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.event_id, original.event_id);
        assert_eq!(restored.occurred_at, original.occurred_at);
        assert_eq!(restored.tenant_id, original.tenant_id);
        assert_eq!(restored.correlation_id, original.correlation_id);
    }

    #[test]
    fn envelope_roundtrip_minimal() {
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "c".into(),
            resource: BudgetResource::Cycles,
            limit: "2".to_string(),
            observed: None,
        };
        let original = ExperienceEventEnvelope::new("evt-min", event);

        let json = serde_json::to_string(&original).unwrap();
        let restored: ExperienceEventEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.event_id, original.event_id);
        assert!(restored.tenant_id.is_none());
        assert!(restored.correlation_id.is_none());
    }

    // ── Edge cases: empty/special/long strings ───────────────────────────────

    #[test]
    fn envelope_edge_case_empty_event_id() {
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "c".into(),
            resource: BudgetResource::Cycles,
            limit: "1".to_string(),
            observed: None,
        };
        let env = ExperienceEventEnvelope::new("", event);
        assert_eq!(env.event_id, "");

        let json = serde_json::to_string(&env).unwrap();
        let restored: ExperienceEventEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.event_id, "");
    }

    #[test]
    fn envelope_edge_case_special_chars_in_ids() {
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "c".into(),
            resource: BudgetResource::Cycles,
            limit: "1".to_string(),
            observed: None,
        };
        let special_id = "evt-🚀-/\\\"'";
        let env = ExperienceEventEnvelope::new(special_id, event)
            .with_tenant("tenant-@#$%^&*()")
            .with_correlation("corr-\n\t\r");

        let json = serde_json::to_string(&env).unwrap();
        let restored: ExperienceEventEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.event_id, special_id);
        assert_eq!(restored.tenant_id.as_deref(), Some("tenant-@#$%^&*()"));
        assert_eq!(restored.correlation_id.as_deref(), Some("corr-\n\t\r"));
    }

    #[test]
    fn envelope_edge_case_very_long_strings() {
        let long_id = "x".repeat(10_000);
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "c".into(),
            resource: BudgetResource::Cycles,
            limit: "1".to_string(),
            observed: None,
        };
        let env = ExperienceEventEnvelope::new(long_id.clone(), event)
            .with_tenant("y".repeat(5_000))
            .with_correlation("z".repeat(3_000));

        let json = serde_json::to_string(&env).unwrap();
        let restored: ExperienceEventEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.event_id, long_id);
        assert_eq!(restored.tenant_id.as_ref().map(|s| s.len()), Some(5_000));
        assert_eq!(
            restored.correlation_id.as_ref().map(|s| s.len()),
            Some(3_000)
        );
    }

    #[test]
    fn envelope_edge_case_unicode_in_ids() {
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "c".into(),
            resource: BudgetResource::Cycles,
            limit: "1".to_string(),
            observed: None,
        };
        let env = ExperienceEventEnvelope::new("evt-中文-العربية-русский", event)
            .with_tenant("テナント-यन्त्र")
            .with_correlation("相関-συσχέτιση");

        let json = serde_json::to_string(&env).unwrap();
        let restored: ExperienceEventEnvelope = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.event_id, "evt-中文-العربية-русский");
        assert_eq!(restored.tenant_id.as_deref(), Some("テナント-यन्त्र"));
        assert_eq!(restored.correlation_id.as_deref(), Some("相関-συσχέτιση"));
    }

    // ── Malformed JSON / deserialization errors ──────────────────────────────

    #[test]
    fn envelope_unpack_missing_required_event_id() {
        let json = r#"{
            "occurred_at": "2026-01-01T00:00:00Z",
            "tenant_id": null,
            "correlation_id": null,
            "event": {"type": "BudgetExceeded", "data": {"chain_id": "c", "resource": "Cycles", "limit": "1", "observed": null}}
        }"#;

        let result: Result<ExperienceEventEnvelope, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn envelope_unpack_missing_required_occurred_at() {
        let json = r#"{
            "event_id": "evt-1",
            "tenant_id": null,
            "correlation_id": null,
            "event": {"type": "BudgetExceeded", "data": {"chain_id": "c", "resource": "Cycles", "limit": "1", "observed": null}}
        }"#;

        let result: Result<ExperienceEventEnvelope, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn envelope_unpack_missing_required_event() {
        let json = r#"{
            "event_id": "evt-1",
            "occurred_at": "2026-01-01T00:00:00Z",
            "tenant_id": null,
            "correlation_id": null
        }"#;

        let result: Result<ExperienceEventEnvelope, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn envelope_unpack_wrong_type_for_field() {
        let json = r#"{
            "event_id": 12345,
            "occurred_at": "2026-01-01T00:00:00Z",
            "tenant_id": null,
            "correlation_id": null,
            "event": {"type": "BudgetExceeded", "data": {"chain_id": "c", "resource": "Cycles", "limit": "1", "observed": null}}
        }"#;

        let result: Result<ExperienceEventEnvelope, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn envelope_unpack_invalid_json_syntax() {
        let invalid = r#"{"event_id": "evt-1", "occurred_at": "2026-01-01"#;
        let result: Result<ExperienceEventEnvelope, _> = serde_json::from_str(invalid);
        assert!(result.is_err());
    }

    #[test]
    fn envelope_unpack_extra_unknown_fields_ignored() {
        let json = r#"{
            "event_id": "evt-1",
            "occurred_at": "2026-01-01T00:00:00Z",
            "tenant_id": null,
            "correlation_id": null,
            "event": {"type": "BudgetExceeded", "data": {"chain_id": "c", "resource": "Cycles", "limit": "1", "observed": null}},
            "unknown_field": "ignored",
            "another_extra": 42
        }"#;

        let result: Result<ExperienceEventEnvelope, _> = serde_json::from_str(json);
        // Extra fields are ignored by default serde behavior
        assert!(result.is_ok());
        assert_eq!(result.unwrap().event_id, "evt-1");
    }

    #[test]
    fn envelope_unpack_null_for_event_id_invalid() {
        let json = r#"{
            "event_id": null,
            "occurred_at": "2026-01-01T00:00:00Z",
            "tenant_id": null,
            "correlation_id": null,
            "event": {"type": "BudgetExceeded", "data": {"chain_id": "c", "resource": "Cycles", "limit": "1", "observed": null}}
        }"#;

        let result: Result<ExperienceEventEnvelope, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // ── Builder chaining and override behavior ────────────────────────────────

    #[test]
    fn envelope_builder_chaining_consistency() {
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "c".into(),
            resource: BudgetResource::Cycles,
            limit: "1".to_string(),
            observed: None,
        };
        let env = ExperienceEventEnvelope::new("evt-1", event)
            .with_tenant("t1")
            .with_tenant("t2")
            .with_correlation("corr1")
            .with_correlation("corr2")
            .with_timestamp("2026-01-01T00:00:00Z")
            .with_timestamp("2026-12-31T23:59:59Z");

        assert_eq!(env.tenant_id.as_deref(), Some("t2"));
        assert_eq!(env.correlation_id.as_deref(), Some("corr2"));
        assert_eq!(env.occurred_at, "2026-12-31T23:59:59Z");
    }

    #[test]
    fn envelope_builder_override_to_last_value() {
        let event = ExperienceEvent::BudgetExceeded {
            chain_id: "c".into(),
            resource: BudgetResource::Cycles,
            limit: "1".to_string(),
            observed: None,
        };
        let final_tenant = "final-tenant";
        let final_corr = "final-corr";
        let final_ts = "2099-01-01T00:00:00Z";

        let env = ExperienceEventEnvelope::new("evt", event)
            .with_tenant("ignored1")
            .with_tenant("ignored2")
            .with_tenant(final_tenant)
            .with_correlation("ignored1")
            .with_correlation(final_corr)
            .with_timestamp("ignored1")
            .with_timestamp(final_ts);

        assert_eq!(env.tenant_id.as_deref(), Some(final_tenant));
        assert_eq!(env.correlation_id.as_deref(), Some(final_corr));
        assert_eq!(env.occurred_at, final_ts);
    }
}
