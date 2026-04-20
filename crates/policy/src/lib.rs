//! Converge Policy — Cedar-based Policy Decision Point for the Converge gate model
//!
//! Evaluates agent authority, commitment constraints, and phase gate requirements
//! as deterministic, auditable policy decisions.
//!
//! Two decision paths:
//! - **Policy mode**: Cedar policy evaluation against principal/resource/context
//! - **Delegation mode**: Ed25519-signed, time-scoped authority tokens

pub mod decision;
pub mod delegation;
pub mod engine;
pub mod flow;
pub mod suggestor;
pub mod types;

pub const EXPENSE_APPROVAL_POLICY: &str = include_str!("../policies/expense_approval.cedar");
pub const FLOW_GOVERNANCE_POLICY: &str = include_str!("../policies/flow_governance.cedar");
pub const VENDOR_SELECTION_POLICY: &str = include_str!("../policies/vendor_selection.cedar");

pub use converge_core::{
    FlowAction, FlowGateAuthorizer, FlowGateDecision, FlowGateError, FlowGateInput, FlowGateOutcome,
};
pub use decision::{PolicyDecision, PolicyOutcome};
pub use delegation::Delegation;
pub use engine::PolicyEngine;
pub use flow::{FlowGateContext, FlowGatePrincipal, FlowGateResource};
pub use types::{ContextIn, DecideRequest, PrincipalIn, ResourceIn};
