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
        DeliberatedFormationTemplate, FormationCatalog, FormationKind, FormationPlan,
        FormationRequest, FormationTemplate, FormationTemplateMetadata, FormationTemplateQuery,
        OpenClawFormationTemplate, ProfileSnapshot, RoleAssignment, ScoredFormationTemplate,
        ScoringWeights, StaticFormationTemplate, SuggestorCapability, SuggestorProfile,
        SuggestorRole,
    };
    pub use converge_optimization::suggestors::FormationAssemblySuggestor;
    pub use converge_provider::ProviderSelectionSuggestor;
    pub use converge_provider_api::{
        Capability, CapabilityAssignment, CostClass, LatencyClass, ProviderAssignment,
        ProviderRequest,
    };
}

pub mod admission {
    //! External observation admission API.
    //!
    //! Admission stages observations as proposals. It does not construct
    //! authoritative facts; promotion remains engine-owned.

    pub use converge_model::{
        AdmissionActor, AdmissionActorKind, AdmissionContent, AdmissionError, AdmissionReceipt,
        AdmissionRequest, AdmissionSource,
    };

    use crate::{ContextState, ConvergeError};

    /// Stages an external observation for governed promotion.
    pub fn admit_observation(
        context: &mut ContextState,
        request: AdmissionRequest,
    ) -> Result<AdmissionReceipt, ConvergeError> {
        context.submit_observation(request)
    }
}

pub use converge_core::gates::hitl::{
    ContextItem, GateDecision, GateEvent, GateEventKind, GateRequest, GateVerdict, HitlPolicy,
    TimeoutAction, TimeoutPolicy,
};
pub use converge_core::gates::{
    AuthorityLevel, FlowAction, FlowGateAuthorizer, FlowGateContext, FlowGateInput,
    FlowGateOutcome, FlowGatePrincipal, FlowGateResource, FlowPhase, StopReason,
};
pub use converge_core::recall::{
    CandidateProvenance, CandidateSourceType, RecallCandidate, RecallPolicy, RecallQuery,
    RecallUse, RelevanceLevel, recall_from_store,
};
pub use converge_core::{
    AdmissionActor, AdmissionActorKind, AdmissionContent, AdmissionError, AdmissionReceipt,
    AdmissionRequest, AdmissionSource, ApprovalPointId, BackendId, Budget, BudgetResource, ChainId,
    ConstraintName, ConstraintValue, ContextSnapshot, ContextState, ConvergeError, ConvergeResult,
    CorrelationId, Criterion, CriterionEvaluator, CriterionId, CriterionOutcome, CriterionResult,
    DecisionStep, Engine, EngineHitlPolicy, EventId, EventQuery, ExperienceEvent,
    ExperienceEventEnvelope, ExperienceEventKind, ExperienceEventObserver, ExperienceRecord,
    ExperienceStore, ExperienceStoreError, ExperienceStoreResult, HitlPause, IntegrityProof,
    Invariant, InvariantClass, InvariantResult, LamportClock, MerkleRoot, OverrideTarget, PackId,
    RunResult, StreamingCallback, SuggestorId, TenantId, TraceLinkId, TruthId, TypesBudgets,
    TypesIntentId, TypesIntentKind, TypesRootIntent, TypesRunHooks, UnitInterval,
    UserExperienceEvent, UserExperienceEventEnvelope,
};
pub use converge_pack::{
    AgentEffect, ArtifactId, Context, ContextFact, ContextKey, ProposedFact, Suggestor,
    ValidationError,
};

// Experience-side surface (artifact lifecycle, replay traces).
// These let downstream consumers (Organism learning, Helms, etc.) reference
// governed-artifact and replay vocabulary without importing converge_core.
pub use converge_core::experience_store::ArtifactKind;
pub use converge_core::governed_artifact::LifecycleEvent;
pub use converge_core::kernel_boundary::{LocalReplayTrace, RemoteReplayTrace, ReplayTrace};

#[cfg(test)]
mod tests {
    use super::{
        BudgetResource, ContextKey, ContextState, StopReason,
        admission::{
            AdmissionActor, AdmissionActorKind, AdmissionContent, AdmissionRequest,
            AdmissionSource, admit_observation,
        },
        formation::{
            Capability, FormationCatalog, FormationRequest, FormationTemplate,
            FormationTemplateMetadata, ProviderRequest, StaticFormationTemplate, SuggestorRole,
        },
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
            backend_requirements: None,
        };

        assert_eq!(formation.required_roles, vec![SuggestorRole::Analysis]);
        assert_eq!(provider.required_capabilities, vec![Capability::Reasoning]);
    }

    #[test]
    fn kernel_reexports_template_catalog_surface() {
        let catalog = FormationCatalog::new().with_template(FormationTemplate::static_template(
            StaticFormationTemplate::new(FormationTemplateMetadata::new(
                "analysis-only",
                "Single-role analysis formation",
                [SuggestorRole::Analysis],
            )),
        ));

        assert_eq!(catalog.len(), 1);
        assert_eq!(
            catalog.get("analysis-only").map(FormationTemplate::id),
            Some("analysis-only")
        );
    }

    #[test]
    fn kernel_admission_stages_observation() {
        let mut context = ContextState::new();
        let request = AdmissionRequest::new(
            AdmissionActor::new("organism-runtime", AdmissionActorKind::System).unwrap(),
            AdmissionSource::new("truth-document").unwrap(),
            ContextKey::Seeds,
            "truth-doc-1",
            AdmissionContent::new(r#"{"claim":"approved source"}"#).unwrap(),
        )
        .unwrap();

        let receipt = admit_observation(&mut context, request).unwrap();

        assert!(receipt.staged());
        assert_eq!(receipt.proposal_id().as_str(), "truth-doc-1");
    }
}
