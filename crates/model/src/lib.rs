// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Converge Model
//!
//! This crate is the curated semantic model surface for Converge.
//! It intentionally excludes the execution engine and runtime adapters.

pub mod formation;

pub use converge_core::{
    Actor, ActorKind, AdmissionActor, AdmissionActorKind, AdmissionContent, AdmissionError,
    AdmissionReceipt, AdmissionRequest, AdmissionSource, ApprovalId, ApprovalPointId, ArtifactId,
    BackendId, CaptureContext, ChainId, ChosenSide, ConflictType, ConstraintKind, ConstraintName,
    ConstraintValue, Criterion, CriterionId, EventId, EvidenceRef, FactContent, FactContentKind,
    FactId, Frame, FrameConstraint, FrameId, GateId, Hypothesis, LocalTrace, Observation,
    ObservationError, ObservationId, ObservationKind, ObservationProvenance, PackId, PolicyId,
    PromotionError, PromotionRecord, Proposal, ProposalId, RiskPosture, StopReason, TenantId,
    Tension, TensionId, TensionResolution, TensionSide, Timestamp, TraceLinkId, TruthCatalog,
    TruthDefinition, TruthId, TruthKind, TypeError, TypesBudgets, TypesConstraintSeverity,
    TypesContextKey, TypesContextSnapshot, TypesFact, TypesIntentConstraint, TypesIntentId,
    TypesIntentKind, TypesObjective, TypesProposedContent, TypesRootIntent, TypesTraceLink,
    TypesValidationError, Validated, ValidationSummary,
};
pub use converge_pack::{ContextKey, ProposedFact, ValidationError};
pub use formation::{
    DeliberatedFormationTemplate, FormationCatalog, FormationKind, FormationPlan, FormationRequest,
    FormationTemplate, FormationTemplateMetadata, FormationTemplateQuery,
    OpenClawFormationTemplate, ProfileSnapshot, RoleAssignment, ScoredFormationTemplate,
    ScoringWeights, StaticFormationTemplate, SuggestorCapability, SuggestorProfile, SuggestorRole,
};
