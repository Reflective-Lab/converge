// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Converge Core
//!
//! A correctness-first, context-driven multi-suggestor runtime.
//!
//! Converge is a Suggestor OS where:
//! - Context is the API
//! - Suggestors collaborate through data, not calls
//! - Execution proceeds until a fixed point
//! - Convergence is explicit and observable
//!
//! ## Quick Start
//!
//! ```ignore
//! use converge_core::{ContextState, ContextKey, Engine};
//! use converge_core::suggestors::{ReactOnceSuggestor, SeedSuggestor};
//!
//! // Create engine and register suggestors
//! let mut engine = Engine::new();
//! engine.register_suggestor(SeedSuggestor::new("seed-1", "initial data"));
//! engine.register_suggestor(ReactOnceSuggestor::new("hyp-1", "derived insight"));
//!
//! // Run until convergence (async)
//! let result = engine.run(ContextState::new()).await.expect("should converge");
//!
//! // Inspect results
//! assert!(result.converged);
//! assert!(result.context.has(ContextKey::Seeds));
//! assert!(result.context.has(ContextKey::Hypotheses));
//! println!("Converged in {} cycles", result.cycles);
//! ```
//!
//! ## Core Concepts
//!
//! - [`Context`]: The shared, typed, evolving state of a job
//! - [`Suggestor`]: A capability that reads context and emits effects
//! - [`AgentEffect`]: Buffered proposal output from a suggestor
//! - [`Engine`]: The convergence loop that coordinates suggestors
//!
//! ## Guarantees
//!
//! - **Determinism**: Same input → same output
//! - **Termination**: Budgets prevent infinite loops
//! - **Isolation**: Suggestors never call each other
//! - **Auditability**: All changes are traceable
//!
//! # Design Tenets
//!
//! These are the nine non-negotiable axioms that `converge-core` exists to encode,
//! enforce, and protect. Every type, trait, and pattern in this crate serves one
//! or more of these tenets.
//!
//! ## 1. Explicit Authority
//!
//! **Axiom**: No defaults that grant authority. Authority is always explicit, typed, and traceable.
//!
//! **Why**: Implicit permissions lead to security drift and unauditable systems.
//!
//! **In code**: [`AuthorityGrant`] and [`AuthorityScope`] require explicit construction.
//! The `pub(crate)` constructors on `AuthorityGrant` prevent external code from forging
//! authority. See also [`PromotionRecord`] which traces approvers.
//!
//! ## 2. Convergence Over Control Flow
//!
//! **Axiom**: We converge on outcomes via governed proposals, not ad-hoc loops or hidden heuristics.
//!
//! **Why**: Control flow hides decisions; convergence makes them observable.
//!
//! **In code**: The [`Engine`] runs suggestors repeatedly until a fixed point is reached.
//! [`StopReason`] exhaustively enumerates why execution halted. No hidden loops.
//!
//! ## 3. Append-Only Truth
//!
//! **Axiom**: Facts are never mutated. Corrections are new facts.
//!
//! **Why**: Mutable state hides history and prevents audit replay.
//!
//! **In code**: [`TypesFact`] has private fields with no `&mut` methods.
//! [`CorrectionEvent`] creates new correction facts rather than
//! mutating existing ones. The [`Context`] accumulates facts without overwriting.
//!
//! ## 4. Suggestors Suggest, Engine Decides
//!
//! **Axiom**: Suggestors emit proposals; promotion requires validation gates (and sometimes humans).
//!
//! **Why**: Separates suggestion from decision, enabling governance and audit.
//!
//! **In code**: [`PromotionGate`] is the ONLY path to create Facts. Suggestors produce
//! [`Proposal`] in the `Draft` state which must go through [`ValidationReport`]
//! before becoming `Validated` and finally [`TypesFact`].
//!
//! ## 5. Safety by Construction
//!
//! **Axiom**: Make invalid states unrepresentable. Prefer types over conventions.
//!
//! **Why**: Runtime checks can be bypassed; type-level guarantees cannot.
//!
//! **In code**: The type-state pattern on [`Proposal`] (`Draft` vs `Validated`)
//! makes it impossible to promote an unvalidated proposal. Newtype IDs like [`FactId`],
//! [`ProposalId`], and [`ObservationId`] prevent mixing.
//!
//! ## 6. Transparent Determinism
//!
//! **Axiom**: The system tells the truth about replayability and determinism.
//!
//! **Why**: Hidden non-determinism corrupts audit trails and reproducibility.
//!
//! **In code**: [`TypesTraceLink`] distinguishes [`LocalTrace`]
//! (replay-eligible) from [`RemoteRef`] (audit-only). [`Replayability`]
//! explicitly marks whether operations can be replayed deterministically.
//!
//! ## 7. Human Authority First-Class
//!
//! **Axiom**: Explicit pause/approve gates for consequential actions.
//!
//! **Why**: AI systems must preserve human oversight for high-stakes decisions.
//!
//! **In code**: [`Actor`] and [`ActorKind`] distinguish
//! human from automated approvers. [`PromotionRecord`] records
//! who approved each fact. The [`ValidationPolicy`] can require human approval.
//!
//! ## 8. No Hidden Work
//!
//! **Axiom**: No silent background effects, retries, implicit state changes, or shadow decisioning.
//!
//! **Why**: Hidden work makes systems unpredictable and unauditable.
//!
//! **In code**: [`AgentEffect`] explicitly captures all suggestor output. The [`Engine`]
//! budget system ([`CycleBudget`], [`FactBudget`], [`TokenBudget`]) makes resource
//! consumption visible. [`StopReason`] explains exactly why execution ended.
//!
//! ## 9. Scale by Intent Replication
//!
//! **Axiom**: Scale by replicating intent and invariants across domains.
//!
//! **Why**: Scaling should preserve governance, not bypass it.
//!
//! **In code**: [`RootIntent`] and [`Frame`] capture intent as data.
//! [`Invariant`] enforces governance rules. These types can be serialized and
//! replicated across distributed systems while preserving their constraints.
//!
//! # Purity Declaration
//!
//! `converge-core` is the constitutional foundation for Converge. It must remain
//! **pure** in the architectural sense: no I/O, no persistence, no hidden
//! background work, and no runtime ownership. Core coordination logic such as
//! the engine and promotion gates belongs here; adapters and transport/runtime
//! wiring do not.
//!
//! ## Allowed Dependencies
//!
//! | Crate | Purpose | Rationale |
//! |-------|---------|-----------|
//! | `thiserror` | Error derives | Pure derives, no runtime |
//! | `tracing` | Log macros | Compile-time only when used |
//! | `serde` | Serialization derives | Pure derives, no I/O |
//! | `serde_json` | JSON encoding | Value-only, no I/O |
//! | `typed-builder` | Builder derives | Pure derives |
//! | `hex` | Hex encoding | Pure transforms |
//! | Small pure libs | Hashing, encoding | No I/O, no runtime ownership |
//!
//! ## Forbidden Dependencies
//!
//! | Crate | Category | Why Forbidden |
//! |-------|----------|---------------|
//! | `tokio` | Async runtime | Implies execution |
//! | `reqwest` | HTTP client | Network I/O |
//! | `axum` | HTTP server | Network I/O |
//! | `tonic` | gRPC | Network I/O |
//! | `prost` | Protobuf | gRPC dependency |
//! | `burn` | ML runtime | Heavy computation |
//! | `llama-burn` | LLM inference | Model execution |
//! | `fastembed` | Embeddings | Model execution |
//! | `polars` | DataFrames | Heavy computation |
//! | `arrow` | Columnar data | Analytics dependency |
//! | `manifold` | Database/object-store adapters | Persistence |
//! | `postgres` | Database | Persistence |
//! | `rand` | Randomness | Non-determinism |
//! | `rayon` | Parallelism | Execution strategy |
//!
//! ## The Purity Rule
//!
//! > If a module implies executor ownership, I/O, network, model inference, or
//! > persistence, it does not belong in `converge-core`.
//!
//! Capability crates (e.g., `converge-runtime`, `converge-llm`, `converge-provider`)
//! implement the traits defined here using the forbidden dependencies.
//!
//! See `deny.toml` at the crate root for CI enforcement of these rules.

pub mod admission;
mod agent;
pub mod backend;
pub mod capability;
mod context;
mod effect;
mod engine;
mod error;
pub mod eval;
pub mod experience_store;
pub mod formation;
pub mod gates;
pub mod governed_artifact;
pub mod integrity;
pub mod invariant;
pub mod kernel_boundary;
pub mod model_selection;
pub mod prompt;
pub mod recall;
pub mod root_intent;
#[path = "agents.rs"]
pub mod suggestors;
pub mod traits;
pub mod truth;
pub mod types;
pub mod validation;

pub use admission::{
    AdmissionActor, AdmissionActorKind, AdmissionContent, AdmissionError, AdmissionReceipt,
    AdmissionRequest, AdmissionSource,
};
pub use agent::{Suggestor, SuggestorId};
pub use context::{
    ContextFact, ContextKey, ContextSnapshot, ContextState, FactPayload, ProposedFact, Provenance,
    ProvenanceSource, TextPayload, ValidationError, WireContextSnapshot,
};
pub use effect::AgentEffect;
pub use formation::{
    DeliberatedFormation, Formation, FormationDecision, FormationKind, FormationOutcome,
    OpenClawFormation, ScoredFormation, ScoringWeights, StaticFormation,
};

/// Re-export canonical pack value types used by core public structs.
pub use converge_pack::{Context, UnitInterval};
pub use engine::{
    Budget, ConvergeResult, Engine, EngineHitlPolicy, ExperienceEventObserver, HitlPause,
    RunResult, StreamingCallback, TypesRunHooks,
};
pub use error::ConvergeError;
pub use eval::{Eval, EvalId, EvalOutcome, EvalRegistry, EvalResult};
pub use experience_store::{
    ArtifactKind, ArtifactQuery, BoundaryKind, BoundaryTarget, BudgetResource,
    ContractResultSnapshot, CorrectionTarget, EventQuery, ExperienceEvent, ExperienceEventEnvelope,
    ExperienceEventKind, ExperienceRecord, ExperienceStore, ExperienceStoreError,
    ExperienceStoreResult, HypothesisOutcome, OverrideTarget, PolicySnapshot, TimeRange,
    UserExperienceEvent, UserExperienceEventEnvelope,
};
pub use integrity::{IntegrityProof, LamportClock, MerkleRoot, TrackedContext};
pub use invariant::{Invariant, InvariantClass, InvariantError, InvariantResult, Violation};
pub use model_selection::{
    AgentRequirements, ComplianceLevel, CostClass, CostTier, DataSovereignty, Jurisdiction,
    LatencyClass, ModelSelectorTrait, RequiredCapabilities, SelectionCriteria, TaskComplexity,
};
pub use prompt::{
    AgentPrompt, AgentRole, Constraint, OutputContract, OutputFormat, OutputKind, PromptContext,
    PromptFormat,
};
pub use root_intent::{
    Budgets, ConstraintSeverity, IntentConstraint, IntentKind, IntentValidationError, Objective,
    RootIntent, Scope, ScopeConstraint, SuccessCriteria, SuccessCriterion,
};
// IntentId canonical definition is in types::frame, re-exported here
pub use truth::{
    CriterionEvaluator, CriterionOutcome, CriterionResult, TruthCatalog, TruthDefinition, TruthKind,
};
pub use types::{
    ApprovalPointId, BackendId, ChainId, ConstraintName, ConstraintValue, CorrelationId,
    CriterionId, EventId, IntentId, PackId, PolicyId, TenantId, TraceLinkId, TruthId,
};

// Re-export core capability types for convenience
pub use capability::{
    CapabilityError, CapabilityErrorKind, CapabilityKind, CapabilityMetadata, EmbedInput,
    EmbedRequest, EmbedResponse, Embedding, GraphEdge, GraphNode, GraphQuery, GraphRecall,
    GraphResult, Modality, RankedItem, RerankRequest, RerankResponse, Reranking, VectorMatch,
    VectorQuery, VectorRecall, VectorRecord,
};

// Re-export capability boundary traits (interfaces for external implementations)
pub use traits::{ContextStore, Executor, Fingerprint, FingerprintError, Randomness};

// Re-export kernel boundary types (constitutional types for all kernels)
pub use kernel_boundary::{
    AdapterTrace,
    ContentKind,
    ContextFact as KernelContextFact,
    // Contract types
    ContractResult,
    DataClassification,
    // Decision step (reasoning phases)
    DecisionStep,
    ExecutionEnv,
    KernelContext,
    // Kernel input types (platform-to-kernel contract)
    KernelIntent,
    KernelPolicy,
    // Proposal types (kernel output boundary)
    KernelProposal,
    LocalReplayTrace,
    ProposalKind,
    ProposedContent,
    RecallTrace,
    RemoteReplayTrace,
    ReplayTrace,
    Replayability,
    // Routing policy types
    RiskTier,
    RoutingPolicy,
    SamplerParams,
};

// Re-export governed artifact types (lifecycle governance for any artifact that changes outcomes)
pub use governed_artifact::{
    // Lifecycle state machine
    GovernedArtifactState,
    InvalidStateTransition,
    // Audit trail
    LifecycleEvent,
    // Replay integrity
    ReplayIntegrityViolation,
    RollbackImpact,
    RollbackRecord,
    // Rollback types
    RollbackSeverity,
    validate_transition,
};

// Re-export backend types (unified LLM interface for local and remote)
pub use backend::{
    BackendAdapterPolicy,
    BackendBudgets,
    // Capabilities
    BackendCapability,
    BackendContractResult,
    // Error types
    BackendError,
    BackendPrompt,
    BackendRecallPolicy,
    // Request types
    BackendRequest,
    // Response types
    BackendResponse,
    BackendResult,
    BackendUsage,
    BackoffStrategy,
    CircuitBreakerConfig,
    CircuitState,
    ContractReport,
    ContractSpec,
    Message,
    MessageRole,
    // Retry and circuit breaker types (production error handling)
    RetryPolicy,
};

// Re-export new types module (3-tier hierarchy: Observation -> Proposal -> Fact)
// Note: types::Fact is the governed semantic fact with PromotionRecord.
// context::ContextFact is the stable read-only runtime/context surface.
pub use types::{
    Actor,
    ActorKind,
    ApprovalId,
    ArtifactId,
    CaptureContext,
    ChosenSide,
    ConflictType,
    ConstraintKind,
    ConstraintSeverity as TypesConstraintSeverity,
    ContentHash,
    ContextBuilder,
    CorrectionError,
    // Correction types (append-only corrections)
    CorrectionEvent,
    CorrectionReason,
    CorrectionScope,
    Criterion,
    Draft,
    EvidenceRef,
    // Fact types (third tier - promoted, governed)
    // Note: types::Fact as TypesFact to avoid collision with context::Fact
    Fact as TypesFact,
    FactContent,
    FactContentKind,
    // ID types (newtypes for type safety)
    FactId,
    Frame,
    FrameConstraint,
    // Frame types (six-phase flow - framing)
    FrameId,
    GateId,
    Hypothesis,
    IntentId as TypesIntentId,
    LocalTrace,
    // Observation types (first tier - raw provider output)
    Observation,
    ObservationError,
    ObservationId,
    ObservationKind,
    ObservationProvenance,
    PromotionError,
    // Provenance types (audit/replay support)
    PromotionRecord,
    // Proposal types (second tier - type-state pattern)
    Proposal,
    ProposalId,
    ProposedContent as TypesProposedContent,
    ProposedContentKind,
    ProviderIdentity,
    RemoteRef,
    RiskPosture,
    Tension,
    // Tension types (six-phase flow - tension/convergence)
    TensionId,
    TensionResolution,
    TensionSide,
    Timestamp,
    TraceLink as TypesTraceLink,
    // Error types (thiserror)
    TypeError,
    TypesBudgets,
    // Context types (builder pattern)
    TypesContextKey,
    TypesContextSnapshot,
    TypesIntentConstraint,
    // Intent types (builder pattern)
    TypesIntentKind,
    TypesObjective,
    TypesRootIntent,
    TypesValidationError,
    Validated,
    ValidationSummary,
};

// Gate Pattern (suggestors suggest, engine decides)
pub use gates::{
    AllowAllFlowGateAuthorizer,
    // Boundary types (constitutional kernel-platform contract)
    AuthorityGrant,
    AuthorityGrantor,
    AuthorityLevel,
    AuthorityScope,
    // Validation types
    CheckResult,
    // Budget types (guaranteed termination)
    CycleBudget,
    ErrorCategory,
    ExecutionBudget,
    FactBudget,
    FlowAction,
    FlowGateAuthorizer,
    FlowGateContext,
    FlowGateDecision,
    FlowGateError,
    FlowGateInput,
    FlowGateOutcome,
    FlowGatePrincipal,
    FlowGateResource,
    FlowPhase,
    // Gate implementation
    PromotionGate,
    // Trait
    ProposalLifecycle,
    RejectAllFlowGateAuthorizer,
    SimpleIntent,
    // Stop reasons (exhaustive termination enumeration)
    StopReason,
    TokenBudget,
    ValidatedProposal,
    ValidationContext,
    ValidationError as GatesValidationError,
    ValidationPolicy,
    ValidationReport,
};

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        // Placeholder: proves the crate structure is valid
    }
}
