// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Converge Kernel
//!
//! This crate is the curated in-process execution API for Converge.
//! Consumers embed the kernel here; they author packs in `converge-pack`
//! and use `converge-model` for shared semantic types.

pub use converge_core::gates::hitl::{GateDecision, TimeoutAction, TimeoutPolicy};
pub use converge_core::gates::{
    AuthorityLevel, FlowAction, FlowGateAuthorizer, FlowGateContext, FlowGateInput,
    FlowGateOutcome, FlowGatePrincipal, FlowGateResource, FlowPhase, StopReason,
};
pub use converge_core::{
    ApprovalPointId, BackendId, Budget, BudgetResource, ChainId, ConstraintName, ConstraintValue,
    ContextState, ConvergeError, ConvergeResult, CorrelationId, Criterion, CriterionEvaluator,
    CriterionId, CriterionOutcome, CriterionResult, DecisionStep, Engine, EngineHitlPolicy,
    EventId, EventQuery, ExperienceEvent, ExperienceEventEnvelope, ExperienceEventKind,
    ExperienceEventObserver, ExperienceStore, HitlPause, IntegrityProof, Invariant, InvariantClass,
    InvariantResult, LamportClock, MerkleRoot, PackId, RunResult, StreamingCallback, SuggestorId,
    TenantId, TraceLinkId, TruthId, TypesBudgets, TypesIntentId, TypesIntentKind, TypesRootIntent,
    TypesRunHooks,
};
pub use converge_pack::{
    AgentEffect, Context, ContextKey, Fact, ProposedFact, Suggestor, ValidationError,
};
