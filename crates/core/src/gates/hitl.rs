// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Human-in-the-Loop (HITL) gate for convergence pause/resume.
//!
//! When a proposal matches a HITL policy, the engine pauses convergence
//! and emits a [`GateRequest`]. The hosting application notifies a human
//! (Slack, email, UI — not our concern). The human approves or rejects.
//! The application calls [`Engine::resume`] with a [`GateDecision`].
//!
//! # Separation of Concerns
//!
//! | Layer | Responsibility |
//! |-------|---------------|
//! | **converge-core** (this module) | Gate types, pause/resume, audit, policy |
//! | **Application layer** | Webhook dispatch, REST endpoints, signed tokens, timeouts |
//!
//! converge-core does NOT know about Slack, email, or any notification channel.
//!
//! # Design Tenets
//!
//! - **Human Authority First-Class**: HITL gates make human approval an explicit,
//!   typed step in the convergence lifecycle.
//! - **Explicit Authority**: `GateDecision::Approve` creates an `AuthorityGrant::human()`.
//!   No auto-approval unless the policy explicitly allows timeout escalation.
//! - **No Hidden Work**: Every gate event (pause, approve, reject, timeout)
//!   is recorded in the audit trail via [`GateEvent`].

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::types::id::{GateId, ProposalId, Timestamp};
use crate::types::proposal::{Draft, Proposal, ProposedContentKind};

// ============================================================================
// HitlPolicy — when does human approval apply?
// ============================================================================

/// Policy controlling when HITL approval is required.
///
/// Can be configured per-workspace or per-agent. A proposal triggers
/// HITL if ANY of the configured conditions match.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitlPolicy {
    /// Which proposal content kinds require HITL approval.
    /// Empty means no kind-based gating.
    pub gated_kinds: Vec<ProposedContentKind>,

    /// Confidence threshold: proposals below this trigger HITL.
    /// `None` means no confidence-based gating.
    pub confidence_threshold: Option<f32>,

    /// Suggestor IDs whose proposals always require HITL.
    /// Empty means no agent-based gating.
    pub gated_agent_ids: Vec<String>,

    /// Timeout behavior when human doesn't respond.
    pub timeout: TimeoutPolicy,
}

impl HitlPolicy {
    /// Create a policy that gates all proposals (strictest).
    pub fn gate_all() -> Self {
        Self {
            gated_kinds: Vec::new(),
            confidence_threshold: Some(1.0), // everything below 1.0 = everything
            gated_agent_ids: Vec::new(),
            timeout: TimeoutPolicy::default(),
        }
    }

    /// Create a policy that gates specific content kinds.
    pub fn for_kinds(kinds: Vec<ProposedContentKind>) -> Self {
        Self {
            gated_kinds: kinds,
            confidence_threshold: None,
            gated_agent_ids: Vec::new(),
            timeout: TimeoutPolicy::default(),
        }
    }

    /// Check if a proposal requires HITL approval under this policy.
    pub fn requires_approval(&self, proposal: &Proposal<Draft>, agent_id: &str) -> bool {
        // Kind-based gating
        if !self.gated_kinds.is_empty() && self.gated_kinds.contains(&proposal.content().kind) {
            return true;
        }

        // Confidence-based gating
        if let Some(threshold) = self.confidence_threshold {
            if let Some(confidence) = proposal.content().confidence() {
                if confidence < threshold {
                    return true;
                }
            } else {
                // No confidence score + threshold configured = gate it
                return true;
            }
        }

        // Suggestor-based gating
        if self.gated_agent_ids.contains(&agent_id.to_string()) {
            return true;
        }

        false
    }

    /// Set custom timeout policy.
    pub fn with_timeout(mut self, timeout: TimeoutPolicy) -> Self {
        self.timeout = timeout;
        self
    }
}

// ============================================================================
// TimeoutPolicy — what happens when human doesn't respond?
// ============================================================================

/// What happens when the human doesn't respond in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutPolicy {
    /// How long to wait before timeout action (in seconds).
    pub timeout_secs: u64,

    /// What to do on timeout.
    pub action: TimeoutAction,
}

impl TimeoutPolicy {
    /// Get timeout as Duration.
    pub fn duration(&self) -> Duration {
        Duration::from_secs(self.timeout_secs)
    }
}

impl Default for TimeoutPolicy {
    fn default() -> Self {
        Self {
            timeout_secs: 30 * 60, // 30 minutes
            action: TimeoutAction::Reject,
        }
    }
}

/// Action taken when HITL gate times out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeoutAction {
    /// Auto-reject the proposal (safe default).
    Reject,
    /// Auto-approve the proposal (use with caution).
    Approve,
    /// Escalate to a different approver.
    Escalate,
}

// ============================================================================
// GateRequest — the payload emitted when engine pauses
// ============================================================================

/// Request for human approval, emitted when convergence pauses at a HITL gate.
///
/// The hosting application receives this and is responsible for notifying
/// the human via whatever channel is configured (Slack, email, UI, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateRequest {
    /// Unique ID for this gate request (used to resume).
    pub gate_id: GateId,

    /// The proposal awaiting approval.
    pub proposal_id: ProposalId,

    /// Human-readable summary of what the agent proposed.
    pub summary: String,

    /// Which agent made the proposal.
    pub agent_id: String,

    /// Suggestor's stated reason for the proposal.
    pub rationale: Option<String>,

    /// Key data the agent used to make this proposal (for human context).
    pub context_data: Vec<ContextItem>,

    /// The convergence cycle that was interrupted.
    pub cycle: u32,

    /// When the gate was triggered.
    pub requested_at: Timestamp,

    /// Timeout policy in effect.
    pub timeout: TimeoutPolicy,
}

/// A key-value pair of context data for human review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    /// Label for this data point.
    pub label: String,
    /// Value (plain text).
    pub value: String,
}

impl ContextItem {
    /// Create a new context item.
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }
}

impl GateRequest {
    /// Create a new gate request from a proposal.
    #[allow(dead_code)]
    pub(crate) fn new(
        proposal: &Proposal<Draft>,
        agent_id: impl Into<String>,
        cycle: u32,
        timeout: TimeoutPolicy,
    ) -> Self {
        Self {
            gate_id: GateId::new(format!("hitl-{}", pseudo_uuid())),
            proposal_id: proposal.id().clone(),
            summary: proposal.content().content.clone(),
            agent_id: agent_id.into(),
            rationale: None,
            context_data: Vec::new(),
            cycle,
            requested_at: Timestamp::now(),
            timeout,
        }
    }

    /// Add rationale for the proposal.
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    /// Add context data for human review.
    pub fn with_context(mut self, items: Vec<ContextItem>) -> Self {
        self.context_data = items;
        self
    }
}

// ============================================================================
// GateDecision — the human's response
// ============================================================================

/// Human's decision on a HITL gate request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateDecision {
    /// The gate ID this decision responds to.
    pub gate_id: GateId,

    /// The decision.
    pub verdict: GateVerdict,

    /// Who made the decision.
    pub decided_by: String,

    /// When the decision was made.
    pub decided_at: Timestamp,
}

/// The actual approve/reject verdict.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateVerdict {
    /// Human approved the proposal.
    Approve,
    /// Human rejected the proposal, with optional reason.
    Reject {
        /// Why the proposal was rejected (free text).
        reason: Option<String>,
    },
}

impl GateDecision {
    /// Create an approval decision.
    pub fn approve(gate_id: GateId, decided_by: impl Into<String>) -> Self {
        Self {
            gate_id,
            verdict: GateVerdict::Approve,
            decided_by: decided_by.into(),
            decided_at: Timestamp::now(),
        }
    }

    /// Create a rejection decision.
    pub fn reject(gate_id: GateId, decided_by: impl Into<String>, reason: Option<String>) -> Self {
        Self {
            gate_id,
            verdict: GateVerdict::Reject { reason },
            decided_by: decided_by.into(),
            decided_at: Timestamp::now(),
        }
    }

    /// Is this an approval?
    pub fn is_approved(&self) -> bool {
        matches!(self.verdict, GateVerdict::Approve)
    }
}

// ============================================================================
// GateEvent — audit trail entry
// ============================================================================

/// Audit trail entry for HITL gate events.
///
/// Every gate interaction is recorded for compliance and debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GateEvent {
    /// The gate this event belongs to.
    pub gate_id: GateId,
    /// What happened.
    pub kind: GateEventKind,
    /// When it happened.
    pub timestamp: Timestamp,
}

/// Kind of gate event for audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GateEventKind {
    /// Gate was triggered, convergence paused.
    Requested {
        /// ID of the proposal awaiting approval.
        proposal_id: ProposalId,
        /// ID of the agent that created the proposal.
        agent_id: String,
    },
    /// Human approved the proposal.
    Approved {
        /// Who approved.
        decided_by: String,
    },
    /// Human rejected the proposal.
    Rejected {
        /// Who rejected.
        decided_by: String,
        /// Reason for rejection.
        reason: Option<String>,
    },
    /// Gate timed out.
    TimedOut {
        /// What automatic action was taken.
        action_taken: TimeoutAction,
    },
}

impl GateEvent {
    /// Create a "requested" event.
    pub fn requested(gate_id: GateId, proposal_id: ProposalId, agent_id: String) -> Self {
        Self {
            gate_id,
            kind: GateEventKind::Requested {
                proposal_id,
                agent_id,
            },
            timestamp: Timestamp::now(),
        }
    }

    /// Create an event from a decision.
    pub fn from_decision(decision: &GateDecision) -> Self {
        let kind = match &decision.verdict {
            GateVerdict::Approve => GateEventKind::Approved {
                decided_by: decision.decided_by.clone(),
            },
            GateVerdict::Reject { reason } => GateEventKind::Rejected {
                decided_by: decision.decided_by.clone(),
                reason: reason.clone(),
            },
        };
        Self {
            gate_id: decision.gate_id.clone(),
            kind,
            timestamp: decision.decided_at.clone(),
        }
    }

    /// Create a "timed out" event.
    pub fn timed_out(gate_id: GateId, action_taken: TimeoutAction) -> Self {
        Self {
            gate_id,
            kind: GateEventKind::TimedOut { action_taken },
            timestamp: Timestamp::now(),
        }
    }
}

// ============================================================================
// PendingGate — engine-internal state for a paused HITL gate
// ============================================================================

/// Engine-internal state tracking a pending HITL gate.
///
/// Stored in the engine when convergence pauses. Contains everything
/// needed to resume after a human decision.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct PendingGate {
    /// The gate request that was emitted.
    pub request: GateRequest,
    /// The draft proposal awaiting approval.
    pub proposal: Proposal<Draft>,
    /// The agent that produced this proposal.
    pub agent_id: String,
    /// Cycle at which convergence was paused.
    pub paused_at_cycle: u32,
}

// ============================================================================
// Helpers
// ============================================================================

/// Generate a deterministic pseudo-UUID from a process-local sequence.
/// Good enough for gate IDs; not cryptographic.
#[allow(dead_code)]
fn pseudo_uuid() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!(
        "00000000-0000-4000-8000-{:012x}",
        id & 0x0000_ffff_ffff_ffff,
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::id::{ContentHash, ObservationId};
    use crate::types::observation::CaptureContext;
    use crate::types::proposal::{ObservationProvenance, ProposedContent};

    fn make_provenance() -> ObservationProvenance {
        ObservationProvenance::new(
            ObservationId::new("obs-test"),
            ContentHash::zero(),
            CaptureContext::default(),
        )
    }

    fn make_draft(kind: ProposedContentKind, confidence: Option<f32>) -> Proposal<Draft> {
        let mut content = ProposedContent::new(kind, "Test proposal content");
        if let Some(c) = confidence {
            content = content.with_confidence(c);
        }
        Proposal::new(ProposalId::new("test-proposal"), content, make_provenance())
    }

    #[test]
    fn policy_gates_by_kind() {
        let policy = HitlPolicy::for_kinds(vec![ProposedContentKind::Plan]);
        let plan = make_draft(ProposedContentKind::Plan, Some(0.95));
        let claim = make_draft(ProposedContentKind::Claim, Some(0.95));

        assert!(policy.requires_approval(&plan, "agent-1"));
        assert!(!policy.requires_approval(&claim, "agent-1"));
    }

    #[test]
    fn policy_gates_by_confidence() {
        let policy = HitlPolicy {
            gated_kinds: Vec::new(),
            confidence_threshold: Some(0.8),
            gated_agent_ids: Vec::new(),
            timeout: TimeoutPolicy::default(),
        };
        let low = make_draft(ProposedContentKind::Claim, Some(0.5));
        let high = make_draft(ProposedContentKind::Claim, Some(0.9));
        let none = make_draft(ProposedContentKind::Claim, None);

        assert!(policy.requires_approval(&low, "agent-1"));
        assert!(!policy.requires_approval(&high, "agent-1"));
        assert!(policy.requires_approval(&none, "agent-1")); // no confidence = gated
    }

    #[test]
    fn policy_gates_by_agent() {
        let policy = HitlPolicy {
            gated_kinds: Vec::new(),
            confidence_threshold: None,
            gated_agent_ids: vec!["risky-agent".to_string()],
            timeout: TimeoutPolicy::default(),
        };
        let proposal = make_draft(ProposedContentKind::Claim, Some(0.99));

        assert!(policy.requires_approval(&proposal, "risky-agent"));
        assert!(!policy.requires_approval(&proposal, "safe-agent"));
    }

    #[test]
    fn gate_all_catches_everything() {
        let policy = HitlPolicy::gate_all();
        let proposal = make_draft(ProposedContentKind::Claim, Some(0.99));
        // confidence 0.99 < threshold 1.0 = gated
        assert!(policy.requires_approval(&proposal, "any-agent"));
    }

    #[test]
    fn no_conditions_means_no_gating() {
        let policy = HitlPolicy {
            gated_kinds: Vec::new(),
            confidence_threshold: None,
            gated_agent_ids: Vec::new(),
            timeout: TimeoutPolicy::default(),
        };
        let proposal = make_draft(ProposedContentKind::Claim, Some(0.5));
        assert!(!policy.requires_approval(&proposal, "agent-1"));
    }

    #[test]
    fn gate_decision_approve() {
        let decision = GateDecision::approve(GateId::new("hitl-123"), "user@example.com");
        assert!(decision.is_approved());
    }

    #[test]
    fn gate_decision_reject_with_reason() {
        let decision = GateDecision::reject(
            GateId::new("hitl-123"),
            "user@example.com",
            Some("Proposal is too aggressive".to_string()),
        );
        assert!(!decision.is_approved());
        if let GateVerdict::Reject { reason } = &decision.verdict {
            assert_eq!(reason.as_deref(), Some("Proposal is too aggressive"));
        } else {
            panic!("Expected Reject verdict");
        }
    }

    #[test]
    fn gate_event_from_approval() {
        let decision = GateDecision::approve(GateId::new("hitl-123"), "admin");
        let event = GateEvent::from_decision(&decision);
        assert!(matches!(event.kind, GateEventKind::Approved { .. }));
    }

    #[test]
    fn gate_event_from_rejection() {
        let decision = GateDecision::reject(GateId::new("hitl-123"), "admin", None);
        let event = GateEvent::from_decision(&decision);
        assert!(matches!(event.kind, GateEventKind::Rejected { .. }));
    }

    #[test]
    fn gate_event_timed_out() {
        let event = GateEvent::timed_out(GateId::new("hitl-123"), TimeoutAction::Reject);
        assert!(matches!(
            event.kind,
            GateEventKind::TimedOut {
                action_taken: TimeoutAction::Reject
            }
        ));
    }

    #[test]
    fn timeout_policy_default() {
        let policy = TimeoutPolicy::default();
        assert_eq!(policy.timeout_secs, 30 * 60);
        assert_eq!(policy.duration(), Duration::from_secs(1800));
        assert_eq!(policy.action, TimeoutAction::Reject);
    }

    #[test]
    fn context_item_creation() {
        let item = ContextItem::new("Revenue Impact", "$50,000 pipeline value");
        assert_eq!(item.label, "Revenue Impact");
        assert_eq!(item.value, "$50,000 pipeline value");
    }

    #[test]
    fn gate_request_serde_roundtrip() {
        let request = GateRequest {
            gate_id: GateId::new("hitl-test"),
            proposal_id: ProposalId::new("prop-1"),
            summary: "Recommend premium tier for Acme Corp".to_string(),
            agent_id: "pricing-agent".to_string(),
            rationale: Some("High engagement signals".to_string()),
            context_data: vec![ContextItem::new("ARR", "$120k")],
            cycle: 3,
            requested_at: Timestamp::now(),
            timeout: TimeoutPolicy::default(),
        };

        let json = serde_json::to_string(&request).expect("serialize");
        let back: GateRequest = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.gate_id.as_str(), "hitl-test");
        assert_eq!(back.agent_id, "pricing-agent");
        assert_eq!(back.cycle, 3);
    }

    #[test]
    fn gate_decision_serde_roundtrip() {
        let decisions = vec![
            GateDecision::approve(GateId::new("hitl-1"), "user"),
            GateDecision::reject(GateId::new("hitl-2"), "admin", Some("too risky".into())),
            GateDecision::reject(GateId::new("hitl-3"), "admin", None),
        ];

        for decision in decisions {
            let json = serde_json::to_string(&decision).expect("serialize");
            let back: GateDecision = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(back.gate_id.as_str(), decision.gate_id.as_str());
            assert_eq!(back.is_approved(), decision.is_approved());
        }
    }
}
