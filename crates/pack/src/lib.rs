// Copyright 2024-2026 Reflective Labs

// SPDX-License-Identifier: MIT

//! # Converge Pack
//!
//! This crate is the strict Rust authoring contract for Converge packs.
//! External modules implement these traits to participate in convergence:
//!
//! - [`Suggestor`] for pure suggestors
//! - [`Context`] for read-only context access
//! - [`AgentEffect`] for buffered proposal output
//! - [`ContextFact`] / [`ProposedFact`] for the current context boundary
//! - [`Pack`] for domain packs with gate infrastructure
//! - [`PackSuggestor`] for bridging Pack to Suggestor
//!
//! Provider selection and backend capability routing do not live here.
//! Those contracts belong to `converge-provider`.

mod agent;
pub mod context;
pub mod effect;
pub mod fact;
pub mod formation;
pub mod gate;
pub mod governance;
pub mod pack;
pub mod pack_suggestor;
pub mod types;
pub mod suggestor {
    pub use super::agent::Suggestor;
}

pub use agent::Suggestor;
pub use context::{Context, ContextKey};
pub use effect::{AgentEffect, AgentEffectBuilder};
pub use fact::{
    CONFIDENCE_STEP_MAJOR, CONFIDENCE_STEP_MEDIUM, CONFIDENCE_STEP_MINOR, CONFIDENCE_STEP_PRIMARY,
    CONFIDENCE_STEP_TINY, ContextFact, DiagnosticPayload, ExecutionIdentity,
    ExecutionIdentityEvidence, ExecutionProducerIdentity, FactActor, FactActorKind,
    FactEvidenceRef, FactFamilyId, FactLocalTrace, FactPayload, FactPromotionRecord,
    FactRemoteTrace, FactTraceLink, FactValidationSummary, NativeExecutionIdentity, PayloadError,
    PayloadRegistry, PayloadVersion, ProposedFact, Provenance, ProvenanceSource, TextPayload,
    ValidationError, WireContextFact, WireFactPayload, WireProposedFact,
};
pub use formation::FormationKind;
#[allow(deprecated)]
pub use gate::ProvenanceEnvelope;
pub use gate::{
    AuditEnvelope, AuthorityPolicy, ConstraintHardness, ConstraintSpec, ConstraintType,
    DeterminismSpec, Diagnostic, DiagnosticKind, GateDecision, GateError, GateResult,
    KernelTraceLink, ObjectiveDirection, ObjectiveSpec, ProblemSpec, ProblemSpecBuilder,
    PromotionGate, ProposedPlan, ReplayEnvelope, SolveBudgets, SolverReport, StopReason,
    TieBreakStrategy, TraceMode, Violation,
};
pub use governance::{
    ConsensusOutcome, ConsensusRule, Disagreement, EligibleVoters, GovernanceError, Vote,
    VoteDecision, VoteTally,
};
pub use pack::{
    InvariantDef, InvariantResult, Pack, PackSchema, PackSolveResult, PackSolver,
    default_gate_evaluation,
};
pub use pack_suggestor::{PackInputPayload, PackPlanPayload, PackSuggestor};
pub use types::{
    ActorId, ApprovalId, ApprovalPointId, ArtifactId, BackendId, BasisPoints, ChainId,
    ConstraintName, ConstraintValue, ContentHash, CorrelationId, CriterionId, DisagreementId,
    DomainId, EventId, FactId, GateId, ObservationId, PackId, PolicyId, PolicyVersionId,
    PrincipalId, ProposalId, ResourceId, ResourceKind, SpanId, TenantId, Timestamp, TraceId,
    TraceLinkId, TraceReference, TraceSystemId, TruthId, UnitInterval, ValidationCheckId, VoteId,
    VoteTopicId,
};
