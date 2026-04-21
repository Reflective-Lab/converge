// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Kernel domain agents and use cases for Converge.
//!
//! This crate contains the kernel packs and pure constraint-satisfaction
//! use cases that form the foundation of Converge.
//!
//! # Kernel Packs
//!
//! - [`packs::money`]: Financial transaction substrate
//! - [`packs::trust`]: Audit/access/provenance substrate
//! - [`packs::delivery`]: Promise fulfillment protocol
//! - [`packs::data_metrics`]: Instrumentation substrate
//!
//! # Use Cases
//!
//! - [`ask_converge`]: Query interface
//! - [`meeting_scheduler`]: Pure constraint satisfaction (kernel)
//! - [`resource_routing`]: Pure constraint satisfaction (kernel)
//! - [`drafting`]: Content drafting (kernel utility)
//! - [`form_filler`]: Form filling agents (kernel utility)

use converge_core::{ContextKey, ProposalId, ProposedFact};

pub mod ask_converge;
pub mod domain_invariants;
pub mod drafting;
pub mod drafting_llm;
pub mod eval_agent;
pub mod evals;
mod flow_governance;
pub mod form_filler;
pub mod meeting_scheduler;
pub mod packs;
pub mod resource_routing;

pub mod llm_utils;
pub mod mock;
pub mod retrieval;

// LLM-enabled versions of use cases
pub mod meeting_scheduler_llm;

pub use ask_converge::{AskConvergeAgent, GroundedAnswerInvariant, RecallNotEvidenceInvariant};
pub use drafting::{DraftingComposerAgent, DraftingResearchAgent};

pub use form_filler::{
    CompletenessAgent, FieldMappingAgent, FillPlanAgent, FormSchemaAgent, NormalizationAgent,
    ProposalEmitterAgent, RiskClassifierAgent,
};

pub use meeting_scheduler::{
    // Agents
    AvailabilityRetrievalAgent,
    ConflictDetectionAgent,
    // Invariants
    RequireParticipantAvailability,
    RequirePositiveDuration,
    RequireValidSlot,
    SlotOptimizationAgent,
    TimeZoneNormalizationAgent,
    WorkingHoursConstraintAgent,
};

pub use resource_routing::{
    // Agents
    ConstraintValidationAgent,
    FeasibilityAgent,
    // Invariants
    RequireAllTasksAssigned,
    RequireCapacityRespected,
    RequireValidDefinitions,
    ResourceRetrievalAgent,
    SolverAgent,
    TaskRetrievalAgent,
};

pub use domain_invariants::{AuditTrailRequired, AuthorityRequired};

// Pack-specific evals
pub use evals::{
    // Trust Pack
    AccessComplianceEval,
    AuditCoverageEval,
    // Data Metrics Pack
    DashboardSourceEval,
    // Money Pack
    InvoiceAccuracyEval,
    // General kernel evals
    MeetingScheduleFeasibilityEval,
    MetricDefinitionQualityEval,
    PaymentReconciliationEval,
    // Delivery Pack
    PromiseFulfillmentEval,
    RbacEnforcementEval,
    ScopeCreepDetectionEval,
};

pub(crate) fn proposal(
    provenance: impl Into<String>,
    key: ContextKey,
    id: impl Into<String>,
    content: impl Into<String>,
) -> ProposedFact {
    ProposedFact::new(key, ProposalId::new(id.into()), content, provenance)
}
