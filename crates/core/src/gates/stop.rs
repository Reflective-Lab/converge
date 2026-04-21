// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Stop reasons for engine termination.
//!
//! StopReason is an exhaustive enumeration of why execution stopped.
//! This provides complete audit trails and enables proper handling
//! of different termination conditions.
//!
//! # Categories
//!
//! - **Successful**: Converged, CriteriaMet, UserCancelled
//! - **Human intervention**: HumanInterventionRequired, HitlGatePending
//! - **Budget exhaustion**: CycleBudgetExhausted, FactBudgetExhausted, TokenBudgetExhausted
//! - **Validation failures**: InvariantViolated, PromotionRejected
//! - **System errors**: Error, AgentRefused

use serde::{Deserialize, Serialize};

use crate::invariant::InvariantClass;
use crate::{ApprovalPointId, CriterionId, GateId, ProposalId};

/// Why execution stopped. Exhaustive enumeration for audit trails.
///
/// Every engine run terminates with a StopReason. This enables:
/// - Audit: Know exactly why execution ended
/// - Recovery: Different reasons may have different retry strategies
/// - Monitoring: Track termination patterns
///
/// # Non-Exhaustive
///
/// Marked `#[non_exhaustive]` to allow adding new reasons without
/// breaking existing match statements.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum StopReason {
    // ========================================================================
    // Successful termination
    // ========================================================================
    /// Convergence reached - context stabilized (fixed point).
    /// This is the ideal outcome.
    Converged,

    /// Intent criteria satisfied before convergence.
    /// All success conditions met, no need to continue.
    CriteriaMet {
        /// Which criteria were satisfied
        criteria: Vec<CriterionId>,
    },

    /// User requested stop via cancellation.
    /// Graceful termination, not an error.
    UserCancelled,

    /// Agents converged, but completion is blocked on human intervention.
    /// Unlike `HitlGatePending`, the engine did not pause mid-merge; the
    /// application-level criteria evaluation determined that a human must act
    /// before the truth can be considered complete.
    HumanInterventionRequired {
        /// Which required criteria are blocked.
        criteria: Vec<CriterionId>,
        /// Optional approval/workflow references surfaced by the evaluator.
        approval_refs: Vec<ApprovalPointId>,
    },

    // ========================================================================
    // Budget exhaustion
    // ========================================================================
    /// Maximum execution cycles exceeded.
    /// May indicate non-converging problem or need for larger budget.
    CycleBudgetExhausted {
        /// How many cycles were executed
        cycles_executed: u32,
        /// What the limit was
        limit: u32,
    },

    /// Maximum facts in context exceeded.
    /// Prevents unbounded context growth.
    FactBudgetExhausted {
        /// How many facts were in context
        facts_count: u32,
        /// What the limit was
        limit: u32,
    },

    /// Maximum LLM tokens exceeded.
    /// Cost control for LLM-based operations.
    TokenBudgetExhausted {
        /// How many tokens were consumed
        tokens_consumed: u64,
        /// What the limit was
        limit: u64,
    },

    /// Maximum wall-clock time exceeded.
    /// Prevents indefinite execution.
    TimeBudgetExhausted {
        /// How long execution ran (milliseconds)
        duration_ms: u64,
        /// What the limit was (milliseconds)
        limit_ms: u64,
    },

    // ========================================================================
    // Validation failures
    // ========================================================================
    /// An invariant was violated.
    /// Includes the class (Structural/Semantic/Acceptance) and invariant name.
    InvariantViolated {
        /// Which class of invariant
        class: InvariantClass,
        /// Name of the invariant
        name: String,
        /// Description of the violation
        reason: String,
    },

    /// Promotion gate rejected a proposal.
    /// Proposal failed validation and could not be promoted.
    PromotionRejected {
        /// ID of the rejected proposal
        proposal_id: ProposalId,
        /// Why it was rejected
        reason: String,
    },

    // ========================================================================
    // System errors
    // ========================================================================
    /// Unrecoverable error during execution.
    /// Something went wrong that couldn't be handled.
    Error {
        /// Error message
        message: String,
        /// Error category for programmatic handling
        category: ErrorCategory,
    },

    /// An agent refused to continue.
    /// Suggestor explicitly declined to produce output.
    AgentRefused {
        /// ID of the refusing agent
        agent_id: String,
        /// Why it refused
        reason: String,
    },

    // ========================================================================
    // HITL gate pause
    // ========================================================================
    /// Convergence paused at a human-in-the-loop gate.
    /// A proposal requires human approval before convergence can continue.
    /// The hosting application should notify the human and call
    /// `Engine::resume()` with the decision.
    HitlGatePending {
        /// Unique ID for this gate (used to resume)
        gate_id: GateId,
        /// ID of the proposal awaiting approval
        proposal_id: ProposalId,
        /// Human-readable summary of the proposal
        summary: String,
        /// Which agent made the proposal
        agent_id: String,
        /// Cycle at which convergence was paused
        cycle: u32,
    },
}

impl StopReason {
    // ========================================================================
    // Constructor helpers
    // ========================================================================

    /// Create a Converged stop reason.
    pub fn converged() -> Self {
        Self::Converged
    }

    /// Create a CriteriaMet stop reason.
    pub fn criteria_met(criteria: Vec<CriterionId>) -> Self {
        Self::CriteriaMet { criteria }
    }

    /// Create a UserCancelled stop reason.
    pub fn user_cancelled() -> Self {
        Self::UserCancelled
    }

    /// Create a HumanInterventionRequired stop reason.
    pub fn human_intervention_required(
        criteria: Vec<CriterionId>,
        approval_refs: Vec<ApprovalPointId>,
    ) -> Self {
        Self::HumanInterventionRequired {
            criteria,
            approval_refs,
        }
    }

    /// Create a CycleBudgetExhausted stop reason.
    pub fn cycle_budget_exhausted(cycles_executed: u32, limit: u32) -> Self {
        Self::CycleBudgetExhausted {
            cycles_executed,
            limit,
        }
    }

    /// Create a FactBudgetExhausted stop reason.
    pub fn fact_budget_exhausted(facts_count: u32, limit: u32) -> Self {
        Self::FactBudgetExhausted { facts_count, limit }
    }

    /// Create a TokenBudgetExhausted stop reason.
    pub fn token_budget_exhausted(tokens_consumed: u64, limit: u64) -> Self {
        Self::TokenBudgetExhausted {
            tokens_consumed,
            limit,
        }
    }

    /// Create a TimeBudgetExhausted stop reason.
    pub fn time_budget_exhausted(duration_ms: u64, limit_ms: u64) -> Self {
        Self::TimeBudgetExhausted {
            duration_ms,
            limit_ms,
        }
    }

    /// Create an InvariantViolated stop reason.
    pub fn invariant_violated(
        class: InvariantClass,
        name: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvariantViolated {
            class,
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Create a PromotionRejected stop reason.
    pub fn promotion_rejected(
        proposal_id: impl Into<ProposalId>,
        reason: impl Into<String>,
    ) -> Self {
        Self::PromotionRejected {
            proposal_id: proposal_id.into(),
            reason: reason.into(),
        }
    }

    /// Create an Error stop reason.
    pub fn error(message: impl Into<String>, category: ErrorCategory) -> Self {
        Self::Error {
            message: message.into(),
            category,
        }
    }

    /// Create an AgentRefused stop reason.
    pub fn agent_refused(agent_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::AgentRefused {
            agent_id: agent_id.into(),
            reason: reason.into(),
        }
    }

    /// Create a HitlGatePending stop reason.
    pub fn hitl_gate_pending(
        gate_id: impl Into<GateId>,
        proposal_id: impl Into<ProposalId>,
        summary: impl Into<String>,
        agent_id: impl Into<String>,
        cycle: u32,
    ) -> Self {
        Self::HitlGatePending {
            gate_id: gate_id.into(),
            proposal_id: proposal_id.into(),
            summary: summary.into(),
            agent_id: agent_id.into(),
            cycle,
        }
    }

    // ========================================================================
    // Query methods
    // ========================================================================

    /// Returns true if this is a successful termination.
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            Self::Converged | Self::CriteriaMet { .. } | Self::UserCancelled
        )
    }

    /// Returns true if this is a budget exhaustion.
    pub fn is_budget_exhausted(&self) -> bool {
        matches!(
            self,
            Self::CycleBudgetExhausted { .. }
                | Self::FactBudgetExhausted { .. }
                | Self::TokenBudgetExhausted { .. }
                | Self::TimeBudgetExhausted { .. }
        )
    }

    /// Returns true if this is a validation failure.
    pub fn is_validation_failure(&self) -> bool {
        matches!(
            self,
            Self::InvariantViolated { .. } | Self::PromotionRejected { .. }
        )
    }

    /// Returns true if this is an error condition.
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error { .. } | Self::AgentRefused { .. })
    }

    /// Returns true if convergence is paused at a HITL gate.
    pub fn is_hitl_pending(&self) -> bool {
        matches!(self, Self::HitlGatePending { .. })
    }

    /// Returns true if completion is blocked on human intervention.
    pub fn is_human_intervention_required(&self) -> bool {
        matches!(self, Self::HumanInterventionRequired { .. })
    }
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Converged => write!(f, "Converged"),
            Self::CriteriaMet { criteria } => {
                write!(
                    f,
                    "Criteria met: {}",
                    criteria
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Self::UserCancelled => write!(f, "User cancelled"),
            Self::HumanInterventionRequired {
                criteria,
                approval_refs,
            } => {
                if approval_refs.is_empty() {
                    write!(
                        f,
                        "Human intervention required for: {}",
                        criteria
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                } else {
                    write!(
                        f,
                        "Human intervention required for: {} (refs: {})",
                        criteria
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", "),
                        approval_refs
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                }
            }
            Self::CycleBudgetExhausted {
                cycles_executed,
                limit,
            } => {
                write!(f, "Cycle budget exhausted: {}/{}", cycles_executed, limit)
            }
            Self::FactBudgetExhausted { facts_count, limit } => {
                write!(f, "Fact budget exhausted: {}/{}", facts_count, limit)
            }
            Self::TokenBudgetExhausted {
                tokens_consumed,
                limit,
            } => {
                write!(f, "Token budget exhausted: {}/{}", tokens_consumed, limit)
            }
            Self::TimeBudgetExhausted {
                duration_ms,
                limit_ms,
            } => {
                write!(f, "Time budget exhausted: {}ms/{}ms", duration_ms, limit_ms)
            }
            Self::InvariantViolated {
                class,
                name,
                reason,
            } => {
                write!(f, "{:?} invariant '{}' violated: {}", class, name, reason)
            }
            Self::PromotionRejected {
                proposal_id,
                reason,
            } => {
                write!(f, "Promotion rejected for '{}': {}", proposal_id, reason)
            }
            Self::Error { message, category } => {
                write!(f, "Error ({:?}): {}", category, message)
            }
            Self::AgentRefused { agent_id, reason } => {
                write!(f, "Suggestor '{}' refused: {}", agent_id, reason)
            }
            Self::HitlGatePending {
                gate_id,
                agent_id,
                cycle,
                ..
            } => {
                write!(
                    f,
                    "HITL gate pending: {} (agent: {}, cycle: {})",
                    gate_id, agent_id, cycle
                )
            }
        }
    }
}

/// Category of error for programmatic handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Internal error in the engine
    Internal,
    /// Configuration error
    Configuration,
    /// External service error (LLM, database, etc.)
    External,
    /// Resource error (memory, disk, etc.)
    Resource,
    /// Unknown/uncategorized error
    Unknown,
}

impl Default for ErrorCategory {
    fn default() -> Self {
        Self::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_converged_constructor() {
        let reason = StopReason::converged();
        assert!(matches!(reason, StopReason::Converged));
        assert!(reason.is_success());
        assert!(!reason.is_budget_exhausted());
        assert!(!reason.is_validation_failure());
        assert!(!reason.is_error());
    }

    #[test]
    fn test_criteria_met_constructor() {
        let reason = StopReason::criteria_met(vec!["goal1".into(), "goal2".into()]);
        if let StopReason::CriteriaMet { criteria } = &reason {
            assert_eq!(criteria.len(), 2);
            assert_eq!(criteria[0], "goal1");
            assert_eq!(criteria[1], "goal2");
        } else {
            panic!("Expected CriteriaMet");
        }
        assert!(reason.is_success());
    }

    #[test]
    fn test_user_cancelled_constructor() {
        let reason = StopReason::user_cancelled();
        assert!(matches!(reason, StopReason::UserCancelled));
        assert!(reason.is_success());
    }

    #[test]
    fn test_human_intervention_required_constructor() {
        let reason = StopReason::human_intervention_required(
            vec!["payment.confirmed".into()],
            vec!["approval:top-up".into()],
        );
        if let StopReason::HumanInterventionRequired {
            criteria,
            approval_refs,
        } = &reason
        {
            assert_eq!(criteria, &vec!["payment.confirmed".to_string()]);
            assert_eq!(approval_refs, &vec!["approval:top-up".to_string()]);
        } else {
            panic!("Expected HumanInterventionRequired");
        }
        assert!(!reason.is_success());
        assert!(reason.is_human_intervention_required());
    }

    #[test]
    fn test_cycle_budget_exhausted_constructor() {
        let reason = StopReason::cycle_budget_exhausted(100, 100);
        if let StopReason::CycleBudgetExhausted {
            cycles_executed,
            limit,
        } = &reason
        {
            assert_eq!(*cycles_executed, 100);
            assert_eq!(*limit, 100);
        } else {
            panic!("Expected CycleBudgetExhausted");
        }
        assert!(!reason.is_success());
        assert!(reason.is_budget_exhausted());
    }

    #[test]
    fn test_fact_budget_exhausted_constructor() {
        let reason = StopReason::fact_budget_exhausted(10000, 10000);
        assert!(reason.is_budget_exhausted());
    }

    #[test]
    fn test_token_budget_exhausted_constructor() {
        let reason = StopReason::token_budget_exhausted(1_000_000, 1_000_000);
        assert!(reason.is_budget_exhausted());
    }

    #[test]
    fn test_time_budget_exhausted_constructor() {
        let reason = StopReason::time_budget_exhausted(60000, 60000);
        assert!(reason.is_budget_exhausted());
    }

    #[test]
    fn test_invariant_violated_constructor() {
        let reason = StopReason::invariant_violated(
            InvariantClass::Structural,
            "no_empty_facts",
            "Found empty fact content",
        );
        if let StopReason::InvariantViolated {
            class,
            name,
            reason: r,
        } = &reason
        {
            assert_eq!(*class, InvariantClass::Structural);
            assert_eq!(name, "no_empty_facts");
            assert_eq!(r, "Found empty fact content");
        } else {
            panic!("Expected InvariantViolated");
        }
        assert!(reason.is_validation_failure());
    }

    #[test]
    fn test_promotion_rejected_constructor() {
        let reason = StopReason::promotion_rejected("proposal-123", "schema validation failed");
        assert!(reason.is_validation_failure());
    }

    #[test]
    fn test_error_constructor() {
        let reason = StopReason::error("connection refused", ErrorCategory::External);
        if let StopReason::Error { message, category } = &reason {
            assert_eq!(message, "connection refused");
            assert_eq!(*category, ErrorCategory::External);
        } else {
            panic!("Expected Error");
        }
        assert!(reason.is_error());
    }

    #[test]
    fn test_agent_refused_constructor() {
        let reason = StopReason::agent_refused("agent-1", "cannot process unsafe content");
        assert!(reason.is_error());
    }

    #[test]
    fn test_display_converged() {
        let reason = StopReason::converged();
        assert_eq!(reason.to_string(), "Converged");
    }

    #[test]
    fn test_display_criteria_met() {
        let reason = StopReason::criteria_met(vec!["g1".into(), "g2".into()]);
        assert_eq!(reason.to_string(), "Criteria met: g1, g2");
    }

    #[test]
    fn test_display_human_intervention_required() {
        let reason = StopReason::human_intervention_required(
            vec!["payment.confirmed".into()],
            vec!["approval:top-up".into()],
        );
        assert_eq!(
            reason.to_string(),
            "Human intervention required for: payment.confirmed (refs: approval:top-up)"
        );
    }

    #[test]
    fn test_display_cycle_budget_exhausted() {
        let reason = StopReason::cycle_budget_exhausted(50, 100);
        assert_eq!(reason.to_string(), "Cycle budget exhausted: 50/100");
    }

    #[test]
    fn test_display_invariant_violated() {
        let reason =
            StopReason::invariant_violated(InvariantClass::Semantic, "test_inv", "test reason");
        assert_eq!(
            reason.to_string(),
            "Semantic invariant 'test_inv' violated: test reason"
        );
    }

    #[test]
    fn test_display_error() {
        let reason = StopReason::error("oops", ErrorCategory::Internal);
        assert_eq!(reason.to_string(), "Error (Internal): oops");
    }

    #[test]
    fn test_serde_roundtrip() {
        let reasons = vec![
            StopReason::converged(),
            StopReason::criteria_met(vec!["done".into()]),
            StopReason::human_intervention_required(
                vec!["approval".into()],
                vec!["workflow:1".into()],
            ),
            StopReason::cycle_budget_exhausted(10, 10),
            StopReason::invariant_violated(InvariantClass::Acceptance, "test", "reason"),
            StopReason::error("msg", ErrorCategory::Configuration),
        ];

        for reason in reasons {
            let json = serde_json::to_string(&reason).expect("serialize");
            let back: StopReason = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(reason, back);
        }
    }

    #[test]
    fn test_error_category_default() {
        let category = ErrorCategory::default();
        assert_eq!(category, ErrorCategory::Unknown);
    }
}
