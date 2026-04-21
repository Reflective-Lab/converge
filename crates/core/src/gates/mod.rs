// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Gate Pattern - Enforcing "agents suggest, engine decides".
//!
//! This module implements the Gate Pattern for Converge:
//!
//! - **ProposalLifecycle**: Generic trait defining validation and promotion
//! - **PromotionGate**: Concrete gate requiring ValidationReport for promotion
//! - **ValidationReport**: Proof object that validation occurred
//! - **Budget types**: Resource limits for guaranteed termination
//! - **StopReason**: Exhaustive termination reasons
//! - **Boundary types**: Constitutional interfaces for kernel-platform contract
//!
//! # Design Axiom
//!
//! **"Agents suggest, engine decides"** - the PromotionGate is the ONLY path
//! to create Facts. It requires a ValidationReport, which can only be
//! created by the gate's validation process.
//!
//! # Key Types
//!
//! | Type | Purpose |
//! |------|---------|
//! | `ProposalLifecycle<I, P, V, F>` | Generic trait for validation/promotion |
//! | `PromotionGate` | Concrete gate implementation |
//! | `ValidatedProposal` | Proof bundle (validated proposal + report) |
//! | `ValidationReport` | Proof of validation (pub(crate) constructor) |
//! | `ValidationPolicy` | Policy defining required checks |
//! | `CycleBudget`, `FactBudget`, `TokenBudget` | Resource limits |
//! | `ExecutionBudget` | Combined budget tracking |
//! | `StopReason` | Why execution stopped |
//! | `AuthorityGrant` | Explicit permission for promotion |
//!
//! # Invariants
//!
//! 1. **No bypass path**: Facts can only be created through PromotionGate
//! 2. **No forgery**: ValidationReport has a private token field
//! 3. **Complete audit**: Every Fact has a PromotionRecord
//! 4. **Type-state enforcement**: `Proposal<Draft>` -> `Proposal<Validated>` -> `Fact`
//! 5. **Guaranteed termination**: Budget exhaustion returns StopReason
//! 6. **Explicit authority**: AuthorityGrant required (no defaults)
//!
//! # Design Tenets Alignment
//!
//! This module is the **central enforcement point** for these tenets from [`crate`]:
//!
//! | Tenet | How This Module Enforces It |
//! |-------|----------------------------|
//! | **Agents Suggest, Engine Decides** | PromotionGate is the ONLY path; agents cannot create Facts |
//! | **Explicit Authority** | AuthorityGrant has `pub(crate)` constructors; no external forgery |
//! | **Safety by Construction** | Type-state pattern makes unvalidated promotion impossible |
//! | **Convergence Over Control Flow** | StopReason exhaustively enumerates why execution halted |
//! | **No Hidden Work** | Budget types make resource consumption visible and bounded |
//!
//! # Cross-Module References
//!
//! - **Types**: [`crate::types::Proposal`] and [`crate::types::Fact`] flow through this gate
//! - **Traits**: [`crate::traits::Validator`] and [`crate::traits::Promoter`] implement the lifecycle
//!
//! # Module Structure
//!
//! - `lifecycle.rs`: ProposalLifecycle trait
//! - `promotion.rs`: PromotionGate, ValidatedProposal, SimpleIntent
//! - `validation.rs`: ValidationReport, ValidationPolicy, ValidationContext, CheckResult
//! - `budget.rs`: CycleBudget, FactBudget, TokenBudget, ExecutionBudget
//! - `stop.rs`: StopReason, ErrorCategory
//! - `boundary.rs`: constitutional module, AuthorityGrant, AuthorityScope

pub mod authorization;
pub mod boundary;
pub mod budget;
pub mod hitl;
pub mod lifecycle;
pub mod promotion;
pub mod stop;
pub mod validation;

// Core gate types
pub use lifecycle::ProposalLifecycle;
pub use promotion::{PromotionGate, SimpleIntent, ValidatedProposal};
pub use validation::{
    CheckResult, ValidationContext, ValidationError, ValidationPolicy, ValidationReport,
};

// Budget types
pub use budget::{CycleBudget, ExecutionBudget, FactBudget, TokenBudget};

// Stop reasons
pub use stop::{ErrorCategory, StopReason};

// Boundary types
pub use boundary::{AuthorityGrant, AuthorityGrantor, AuthorityScope, constitutional};

// Flow gate authorization contract
pub use authorization::{
    AllowAllFlowGateAuthorizer, AuthorityLevel, FlowAction, FlowGateAuthorizer, FlowGateContext,
    FlowGateDecision, FlowGateError, FlowGateInput, FlowGateOutcome, FlowGatePrincipal,
    FlowGateResource, FlowPhase, RejectAllFlowGateAuthorizer,
};

// HITL gate types
pub use hitl::{
    ContextItem, GateDecision, GateEvent, GateEventKind, GateRequest, GateVerdict, HitlPolicy,
    TimeoutAction, TimeoutPolicy,
};
