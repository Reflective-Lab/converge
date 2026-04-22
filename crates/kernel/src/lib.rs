// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Converge Kernel
//!
//! This crate is the curated in-process execution API for Converge.
//! Consumers embed the kernel here; they author packs in `converge-pack`
//! and use `converge-model` for shared semantic types.

pub mod formation {
    //! Grouped offering API for self-assembling formations.
    //!
    //! The stable pattern is:
    //! - semantics in `converge-model`
    //! - authoring in `converge-pack`
    //! - runnable machinery in `converge-kernel`

    pub use converge_model::formation::{
        FormationPlan, FormationRequest, ProfileSnapshot, RoleAssignment, SuggestorCapability,
        SuggestorProfile, SuggestorRole,
    };
    pub use converge_optimization::suggestors::FormationAssemblySuggestor;
    pub use converge_provider::ProviderSelectionSuggestor;
    pub use converge_provider_api::{
        Capability, CapabilityAssignment, CostClass, LatencyClass, ProviderAssignment,
        ProviderRequest,
    };
}

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

#[cfg(test)]
mod tests {
    use super::{
        BudgetResource, StopReason,
        formation::{Capability, FormationRequest, ProviderRequest, SuggestorRole},
    };

    #[test]
    fn kernel_reexports_runtime_stop_and_budget_types() {
        let stop = StopReason::converged();
        assert!(matches!(stop, StopReason::Converged));
        assert!(matches!(BudgetResource::Tokens, BudgetResource::Tokens));
    }

    #[test]
    fn kernel_groups_formation_offering_api() {
        let formation = FormationRequest {
            id: "req-1".to_string(),
            required_roles: vec![SuggestorRole::Analysis],
            required_capabilities: vec![],
        };
        let provider = ProviderRequest {
            id: "provider-1".to_string(),
            required_capabilities: vec![Capability::Reasoning],
        };

        assert_eq!(formation.required_roles, vec![SuggestorRole::Analysis]);
        assert_eq!(provider.required_capabilities, vec![Capability::Reasoning]);
    }
}
