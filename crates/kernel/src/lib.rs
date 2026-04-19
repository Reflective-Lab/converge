// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Converge Kernel
//!
//! This crate is the curated in-process execution API for Converge.
//! Consumers embed the kernel here; they author packs in `converge-pack`
//! and use `converge-model` for shared semantic types.

pub use converge_core::gates::hitl::{GateDecision, TimeoutAction, TimeoutPolicy};
pub use converge_core::gates::{
    FlowAction, FlowGateAuthorizer, FlowGateContext, FlowGateInput, FlowGateOutcome,
    FlowGatePrincipal, FlowGateResource,
};
pub use converge_core::{
    Budget, Context, ConvergeError, ConvergeResult, Criterion, CriterionEvaluator,
    CriterionOutcome, CriterionResult, DecisionStep, Engine, EngineHitlPolicy, EventQuery,
    ExperienceEvent, ExperienceEventEnvelope, ExperienceEventKind, ExperienceEventObserver,
    ExperienceStore, HitlPause, Invariant, InvariantClass, InvariantResult, RunResult,
    StreamingCallback, SuggestorId, TypesBudgets, TypesIntentId, TypesIntentKind, TypesRootIntent,
    TypesRunHooks,
};
pub use converge_pack::{
    AgentEffect, Context as ContextView, ContextKey, Fact, ProposedFact, Suggestor, ValidationError,
};
