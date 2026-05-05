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
//! - [`Fact`] / [`ProposedFact`] for the current context boundary
//! - [`Pack`] for domain packs with gate infrastructure
//! - [`PackSuggestor`] for bridging Pack to Suggestor
//!
//! Provider selection and backend capability routing do not live here.
//! Those contracts belong to `converge-provider-api`.

mod agent;
pub mod context;
pub mod effect;
pub mod fact;
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
pub use effect::AgentEffect;
pub use fact::{
    CONFIDENCE_STEP_MAJOR, CONFIDENCE_STEP_MEDIUM, CONFIDENCE_STEP_MINOR, CONFIDENCE_STEP_PRIMARY,
    CONFIDENCE_STEP_TINY, Fact, FactActor, FactActorKind, FactEvidenceRef, FactLocalTrace,
    FactPromotionRecord, FactRemoteTrace, FactTraceLink, FactValidationSummary, ProposedFact,
    ValidationError,
};
pub use gate::{
    AuthorityPolicy, ConstraintHardness, ConstraintSpec, ConstraintType, DeterminismSpec,
    Diagnostic, DiagnosticKind, GateDecision, GateError, GateResult, KernelTraceLink,
    ObjectiveDirection, ObjectiveSpec, ProblemSpec, ProblemSpecBuilder, PromotionGate,
    ProposedPlan, ProvenanceEnvelope, ReplayEnvelope, SolveBudgets, SolverReport, StopReason,
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
pub use pack_suggestor::PackSuggestor;
pub use types::{
    ActorId, ApprovalId, ApprovalPointId, ArtifactId, BackendId, BasisPoints, ChainId,
    ConstraintName, ConstraintValue, ContentHash, CorrelationId, CriterionId, DisagreementId,
    DomainId, EventId, FactId, GateId, ObservationId, PackId, PolicyId, PolicyVersionId,
    PrincipalId, ProposalId, ResourceId, ResourceKind, SpanId, TenantId, Timestamp, TraceId,
    TraceLinkId, TraceReference, TraceSystemId, TruthId, UnitInterval, ValidationCheckId, VoteId,
    VoteTopicId,
};
