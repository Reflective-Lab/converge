//! Promotion gate for plan approval

use serde::{Deserialize, Serialize};

/// Promotion gate for plan approval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionGate {
    /// Invariants that must hold
    pub required_invariants: Vec<String>,
    /// Business truths that must be verified
    pub required_truths: Vec<String>,
    /// Who can promote this plan
    pub authority: AuthorityPolicy,
    /// The gate decision
    pub decision: GateDecision,
    /// Rationale for the decision
    pub rationale: String,
}

impl PromotionGate {
    /// Create a gate that promotes automatically
    pub fn auto_promote(rationale: impl Into<String>) -> Self {
        Self {
            required_invariants: Vec::new(),
            required_truths: Vec::new(),
            authority: AuthorityPolicy::Automatic,
            decision: GateDecision::Promote,
            rationale: rationale.into(),
        }
    }

    /// Create a gate that requires human review
    pub fn requires_review(required_truths: Vec<String>, rationale: impl Into<String>) -> Self {
        Self {
            required_invariants: Vec::new(),
            required_truths,
            authority: AuthorityPolicy::HumanRequired,
            decision: GateDecision::Escalate,
            rationale: rationale.into(),
        }
    }

    /// Create a rejection gate
    pub fn reject(rationale: impl Into<String>) -> Self {
        Self {
            required_invariants: Vec::new(),
            required_truths: Vec::new(),
            authority: AuthorityPolicy::Automatic,
            decision: GateDecision::Reject,
            rationale: rationale.into(),
        }
    }

    /// Create a gate with invariant requirements
    pub fn with_invariants(
        required_invariants: Vec<String>,
        decision: GateDecision,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            required_invariants,
            required_truths: Vec::new(),
            authority: AuthorityPolicy::Automatic,
            decision,
            rationale: rationale.into(),
        }
    }

    /// Check if this gate promotes the plan
    pub fn is_promoted(&self) -> bool {
        self.decision == GateDecision::Promote
    }

    /// Check if this gate rejects the plan
    pub fn is_rejected(&self) -> bool {
        self.decision == GateDecision::Reject
    }

    /// Check if this gate requires escalation
    pub fn requires_escalation(&self) -> bool {
        self.decision == GateDecision::Escalate
    }

    /// Check if human approval is required
    pub fn requires_human(&self) -> bool {
        matches!(
            self.authority,
            AuthorityPolicy::HumanRequired
                | AuthorityPolicy::RoleRequired { .. }
                | AuthorityPolicy::MultiApproval { .. }
        )
    }
}

/// Gate decision outcome
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateDecision {
    /// Plan is approved for execution
    Promote,
    /// Plan is rejected
    Reject,
    /// Plan requires escalation to human/higher authority
    Escalate,
}

impl GateDecision {
    /// Check if this is a terminal decision (Promote or Reject)
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Promote | Self::Reject)
    }
}

/// Authority policy for who can make decisions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AuthorityPolicy {
    /// System can decide automatically
    Automatic,
    /// Human approval required
    HumanRequired,
    /// Specific role required
    RoleRequired {
        /// Roles that can approve
        roles: Vec<String>,
    },
    /// Multiple approvers required
    MultiApproval {
        /// Number of approvals needed
        count: usize,
        /// Roles that can approve
        roles: Vec<String>,
    },
}

impl AuthorityPolicy {
    /// Create role-based authority
    pub fn role(role: impl Into<String>) -> Self {
        Self::RoleRequired {
            roles: vec![role.into()],
        }
    }

    /// Create multi-role authority
    pub fn roles(roles: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self::RoleRequired {
            roles: roles.into_iter().map(Into::into).collect(),
        }
    }

    /// Create multi-approval authority
    pub fn multi_approval(
        count: usize,
        roles: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::MultiApproval {
            count,
            roles: roles.into_iter().map(Into::into).collect(),
        }
    }

    /// Check if this policy allows automatic decision
    pub fn is_automatic(&self) -> bool {
        matches!(self, Self::Automatic)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_promote() {
        let gate = PromotionGate::auto_promote("all invariants passed");
        assert!(gate.is_promoted());
        assert!(!gate.requires_human());
        assert!(!gate.requires_escalation());
    }

    #[test]
    fn test_reject() {
        let gate = PromotionGate::reject("constraint violation");
        assert!(gate.is_rejected());
        assert!(!gate.is_promoted());
    }

    #[test]
    fn test_requires_review() {
        let gate = PromotionGate::requires_review(
            vec!["budget_approved".to_string()],
            "high value transaction",
        );
        assert!(gate.requires_escalation());
        assert!(gate.requires_human());
        assert!(!gate.is_promoted());
    }

    #[test]
    fn test_authority_policy() {
        let auto = AuthorityPolicy::Automatic;
        assert!(auto.is_automatic());

        let role = AuthorityPolicy::role("admin");
        assert!(!role.is_automatic());

        let multi = AuthorityPolicy::multi_approval(2, vec!["manager", "director"]);
        if let AuthorityPolicy::MultiApproval { count, roles } = multi {
            assert_eq!(count, 2);
            assert_eq!(roles.len(), 2);
        } else {
            panic!("expected MultiApproval");
        }
    }

    #[test]
    fn test_decision_terminal() {
        assert!(GateDecision::Promote.is_terminal());
        assert!(GateDecision::Reject.is_terminal());
        assert!(!GateDecision::Escalate.is_terminal());
    }

    #[test]
    fn test_serde_roundtrip() {
        let gate = PromotionGate::requires_review(vec!["truth1".to_string()], "needs review");
        let json = serde_json::to_string(&gate).unwrap();
        let restored: PromotionGate = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.required_truths, gate.required_truths);
    }
}
