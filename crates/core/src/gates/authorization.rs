// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Neutral flow-gate authorization contract.
//!
//! Converging flows should project their current state into [`FlowGateInput`]
//! and ask a [`FlowGateAuthorizer`] for a deterministic decision. Concrete
//! implementations may use Cedar, fixed test doubles, or another governed
//! evaluator, but the flow runtime stays decoupled from those details.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use converge_pack::{
    DomainId, FactPayload, GateId, PolicyVersionId, PrincipalId, ResourceId, ResourceKind,
};

/// Action being attempted against a converging flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowAction {
    Propose,
    Validate,
    Promote,
    Commit,
    AdvancePhase,
}

impl FlowAction {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Propose => "propose",
            Self::Validate => "validate",
            Self::Promote => "promote",
            Self::Commit => "commit",
            Self::AdvancePhase => "advance_phase",
        }
    }
}

/// Authority level granted to a flow-gate principal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityLevel {
    Advisory,
    Supervisory,
    Participatory,
    Sovereign,
}

impl AuthorityLevel {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Advisory => "advisory",
            Self::Supervisory => "supervisory",
            Self::Participatory => "participatory",
            Self::Sovereign => "sovereign",
        }
    }
}

/// Current phase of a converging flow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowPhase {
    Intent,
    Framing,
    Exploration,
    Tension,
    Convergence,
    Commitment,
}

impl FlowPhase {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Intent => "intent",
            Self::Framing => "framing",
            Self::Exploration => "exploration",
            Self::Tension => "tension",
            Self::Convergence => "convergence",
            Self::Commitment => "commitment",
        }
    }
}

/// Principal facts projected from the flow host or application runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowGatePrincipal {
    pub id: PrincipalId,
    pub authority: AuthorityLevel,
    pub domains: Vec<DomainId>,
    pub policy_version: Option<PolicyVersionId>,
}

/// Resource facts projected from the current flow state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowGateResource {
    pub id: ResourceId,
    pub kind: ResourceKind,
    pub phase: FlowPhase,
    pub gates_passed: Vec<GateId>,
}

/// Decision-relevant facts projected from the flow state.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowGateContext {
    pub commitment_type: Option<String>,
    pub amount: Option<i64>,
    pub human_approval_present: Option<bool>,
    pub required_gates_met: Option<bool>,
}

/// Canonical input to an authorization decision for a flow gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowGateInput {
    pub principal: FlowGatePrincipal,
    pub resource: FlowGateResource,
    pub action: FlowAction,
    pub context: FlowGateContext,
}

impl FactPayload for FlowGateInput {
    const FAMILY: &'static str = "converge.flow_gate.input";
    const VERSION: u16 = 1;
}

/// Neutral outcome of a flow gate authorization decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowGateOutcome {
    Promote,
    Reject,
    Escalate,
}

impl FlowGateOutcome {
    #[must_use]
    pub const fn is_allowed(self) -> bool {
        matches!(self, Self::Promote)
    }
}

/// Full gate decision with rationale and source attribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowGateDecision {
    pub outcome: FlowGateOutcome,
    pub reason: Option<String>,
    pub source: Option<String>,
}

impl FactPayload for FlowGateDecision {
    const FAMILY: &'static str = "converge.flow_gate.decision";
    const VERSION: u16 = 1;
}

impl FlowGateDecision {
    #[must_use]
    pub fn promote(reason: Option<String>, source: Option<String>) -> Self {
        Self {
            outcome: FlowGateOutcome::Promote,
            reason,
            source,
        }
    }

    #[must_use]
    pub fn reject(reason: Option<String>, source: Option<String>) -> Self {
        Self {
            outcome: FlowGateOutcome::Reject,
            reason,
            source,
        }
    }

    #[must_use]
    pub fn escalate(reason: Option<String>, source: Option<String>) -> Self {
        Self {
            outcome: FlowGateOutcome::Escalate,
            reason,
            source,
        }
    }
}

/// Pure error surface for flow gate authorization.
#[derive(Debug, Error)]
pub enum FlowGateError {
    #[error("authorizer failed: {0}")]
    Authorizer(String),
    #[error("invalid flow gate input: {0}")]
    InvalidInput(String),
}

/// Deterministic decision provider for consequential flow actions.
pub trait FlowGateAuthorizer: Send + Sync {
    /// Decide whether the attempted flow action should promote, reject, or escalate.
    fn decide(&self, input: &FlowGateInput) -> Result<FlowGateDecision, FlowGateError>;
}

/// Test double: always promote.
#[derive(Debug, Default, Clone, Copy)]
pub struct AllowAllFlowGateAuthorizer;

impl FlowGateAuthorizer for AllowAllFlowGateAuthorizer {
    fn decide(&self, _input: &FlowGateInput) -> Result<FlowGateDecision, FlowGateError> {
        Ok(FlowGateDecision::promote(
            Some("allow_all test authorizer".into()),
            Some("allow_all".into()),
        ))
    }
}

/// Test double: always reject.
#[derive(Debug, Default, Clone)]
pub struct RejectAllFlowGateAuthorizer {
    reason: Option<String>,
}

impl RejectAllFlowGateAuthorizer {
    #[must_use]
    pub fn with_reason(reason: impl Into<String>) -> Self {
        Self {
            reason: Some(reason.into()),
        }
    }
}

impl FlowGateAuthorizer for RejectAllFlowGateAuthorizer {
    fn decide(&self, _input: &FlowGateInput) -> Result<FlowGateDecision, FlowGateError> {
        Ok(FlowGateDecision::reject(
            self.reason
                .clone()
                .or_else(|| Some("reject_all test authorizer".into())),
            Some("reject_all".into()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input() -> FlowGateInput {
        FlowGateInput {
            principal: FlowGatePrincipal {
                id: "agent:test".into(),
                authority: AuthorityLevel::Supervisory,
                domains: vec!["finance".into()],
                policy_version: Some("v1".into()),
            },
            resource: FlowGateResource {
                id: "expense:1".into(),
                kind: "expense".into(),
                phase: FlowPhase::Commitment,
                gates_passed: vec!["receipt".into()],
            },
            action: FlowAction::Validate,
            context: FlowGateContext {
                commitment_type: Some("expense".into()),
                amount: Some(100),
                human_approval_present: Some(false),
                required_gates_met: Some(true),
            },
        }
    }

    #[test]
    fn allow_all_authorizer_promotes() {
        let decision = AllowAllFlowGateAuthorizer
            .decide(&sample_input())
            .expect("allow_all should succeed");
        assert_eq!(decision.outcome, FlowGateOutcome::Promote);
    }

    #[test]
    fn reject_all_authorizer_rejects() {
        let decision = RejectAllFlowGateAuthorizer::with_reason("blocked")
            .decide(&sample_input())
            .expect("reject_all should succeed");
        assert_eq!(decision.outcome, FlowGateOutcome::Reject);
        assert_eq!(decision.reason.as_deref(), Some("blocked"));
    }

    // ── FlowAction enum ──────────────────────────────────────────────────────

    #[test]
    fn flow_action_as_str_all_variants() {
        assert_eq!(FlowAction::Propose.as_str(), "propose");
        assert_eq!(FlowAction::Validate.as_str(), "validate");
        assert_eq!(FlowAction::Promote.as_str(), "promote");
        assert_eq!(FlowAction::Commit.as_str(), "commit");
        assert_eq!(FlowAction::AdvancePhase.as_str(), "advance_phase");
    }

    #[test]
    fn flow_action_serde_roundtrip() {
        let actions = [
            FlowAction::Propose,
            FlowAction::Validate,
            FlowAction::Promote,
            FlowAction::Commit,
            FlowAction::AdvancePhase,
        ];
        for action in actions {
            let json = serde_json::to_string(&action).unwrap();
            let back: FlowAction = serde_json::from_str(&json).unwrap();
            assert_eq!(back, action);
            assert_eq!(json, format!("\"{}\"", action.as_str()));
        }
    }

    // ── AuthorityLevel enum ──────────────────────────────────────────────────

    #[test]
    fn authority_level_as_str_all_variants() {
        assert_eq!(AuthorityLevel::Advisory.as_str(), "advisory");
        assert_eq!(AuthorityLevel::Supervisory.as_str(), "supervisory");
        assert_eq!(AuthorityLevel::Participatory.as_str(), "participatory");
        assert_eq!(AuthorityLevel::Sovereign.as_str(), "sovereign");
    }

    #[test]
    fn authority_level_serde_roundtrip() {
        let levels = [
            AuthorityLevel::Advisory,
            AuthorityLevel::Supervisory,
            AuthorityLevel::Participatory,
            AuthorityLevel::Sovereign,
        ];
        for level in levels {
            let json = serde_json::to_string(&level).unwrap();
            let back: AuthorityLevel = serde_json::from_str(&json).unwrap();
            assert_eq!(back, level);
        }
    }

    // ── FlowPhase enum ───────────────────────────────────────────────────────

    #[test]
    fn flow_phase_as_str_all_variants() {
        assert_eq!(FlowPhase::Intent.as_str(), "intent");
        assert_eq!(FlowPhase::Framing.as_str(), "framing");
        assert_eq!(FlowPhase::Exploration.as_str(), "exploration");
        assert_eq!(FlowPhase::Tension.as_str(), "tension");
        assert_eq!(FlowPhase::Convergence.as_str(), "convergence");
        assert_eq!(FlowPhase::Commitment.as_str(), "commitment");
    }

    #[test]
    fn flow_phase_serde_roundtrip() {
        let phases = [
            FlowPhase::Intent,
            FlowPhase::Framing,
            FlowPhase::Exploration,
            FlowPhase::Tension,
            FlowPhase::Convergence,
            FlowPhase::Commitment,
        ];
        for phase in phases {
            let json = serde_json::to_string(&phase).unwrap();
            let back: FlowPhase = serde_json::from_str(&json).unwrap();
            assert_eq!(back, phase);
        }
    }

    // ── FlowGateOutcome ──────────────────────────────────────────────────────

    #[test]
    fn outcome_is_allowed() {
        assert!(FlowGateOutcome::Promote.is_allowed());
        assert!(!FlowGateOutcome::Reject.is_allowed());
        assert!(!FlowGateOutcome::Escalate.is_allowed());
    }

    // ── FlowGateDecision factories ───────────────────────────────────────────

    #[test]
    fn decision_promote_factory() {
        let d = FlowGateDecision::promote(Some("approved".into()), Some("policy:1".into()));
        assert_eq!(d.outcome, FlowGateOutcome::Promote);
        assert_eq!(d.reason.as_deref(), Some("approved"));
        assert_eq!(d.source.as_deref(), Some("policy:1"));
    }

    #[test]
    fn decision_reject_factory() {
        let d = FlowGateDecision::reject(None, None);
        assert_eq!(d.outcome, FlowGateOutcome::Reject);
        assert!(d.reason.is_none());
        assert!(d.source.is_none());
    }

    #[test]
    fn decision_escalate_factory() {
        let d = FlowGateDecision::escalate(Some("needs human".into()), Some("hitl".into()));
        assert_eq!(d.outcome, FlowGateOutcome::Escalate);
        assert_eq!(d.reason.as_deref(), Some("needs human"));
    }

    // ── FlowGateError ────────────────────────────────────────────────────────

    #[test]
    fn flow_gate_error_display() {
        let e = FlowGateError::Authorizer("connection reset".into());
        assert!(e.to_string().contains("connection reset"));

        let e = FlowGateError::InvalidInput("missing principal".into());
        assert!(e.to_string().contains("missing principal"));
    }

    #[test]
    fn flow_gate_error_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(FlowGateError::Authorizer("test".into()));
        assert!(e.to_string().contains("test"));
    }

    // ── FlowGateInput serde ──────────────────────────────────────────────────

    #[test]
    fn flow_gate_input_serde_roundtrip() {
        let input = sample_input();
        let json = serde_json::to_string(&input).unwrap();
        let back: FlowGateInput = serde_json::from_str(&json).unwrap();
        assert_eq!(back.action, FlowAction::Validate);
        assert_eq!(back.principal.authority, AuthorityLevel::Supervisory);
        assert_eq!(back.resource.phase, FlowPhase::Commitment);
        assert_eq!(back.context.amount, Some(100));
    }

    // ── RejectAllFlowGateAuthorizer default reason ───────────────────────────

    #[test]
    fn reject_all_default_reason() {
        let authorizer = RejectAllFlowGateAuthorizer::default();
        let decision = authorizer.decide(&sample_input()).unwrap();
        assert_eq!(decision.outcome, FlowGateOutcome::Reject);
        assert!(decision.reason.as_deref().unwrap().contains("reject_all"));
    }
}
