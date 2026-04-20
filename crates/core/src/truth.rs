// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Truth catalog primitives.
//!
//! Truths describe jobs, policies, and invariants above domain packs.
//! Applications provide the catalog content; the runtime consumes a common
//! shape for intent construction, guardrails, and pack participation.

use serde::{Deserialize, Serialize};

use crate::{Context, Criterion, FactId, TypesIntentConstraint};

/// What class of truth is being described.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TruthKind {
    /// A job-to-be-done spanning multiple packs.
    Job,
    /// A cross-cutting policy or guardrail.
    Policy,
    /// A module-local or pack-local invariant.
    Invariant,
}

/// Portable truth definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TruthDefinition {
    /// Stable truth identifier.
    pub key: String,
    /// Truth class.
    pub kind: TruthKind,
    /// Human-readable summary.
    pub summary: String,
    /// Required or optional success criteria.
    pub success_criteria: Vec<Criterion>,
    /// Hard and soft constraints derived from the truth.
    pub constraints: Vec<TypesIntentConstraint>,
    /// Human approval points that the runtime must respect.
    pub approval_points: Vec<String>,
    /// Which packs should participate when this truth is active.
    pub participating_packs: Vec<String>,
}

/// Machine-evaluable outcome for a single criterion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CriterionResult {
    /// The criterion was satisfied, with fact IDs that justify the result.
    Met { evidence: Vec<FactId> },
    /// The criterion is currently blocked on human intervention.
    Blocked {
        /// Why the criterion is blocked.
        reason: String,
        /// Optional approval or workflow reference the host can surface.
        approval_ref: Option<String>,
    },
    /// The criterion was evaluated and is not satisfied.
    Unmet { reason: String },
    /// The runtime could not determine whether the criterion was satisfied.
    Indeterminate,
}

/// Evaluated outcome for a specific criterion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionOutcome {
    /// The criterion that was evaluated.
    pub criterion: Criterion,
    /// The result of the evaluation.
    pub result: CriterionResult,
}

/// Application-provided boundary for evaluating success criteria.
pub trait CriterionEvaluator: Send + Sync {
    /// Evaluate a criterion against the converged context.
    fn evaluate(&self, criterion: &Criterion, context: &dyn Context) -> CriterionResult;
}

/// Application-provided truth catalog boundary.
pub trait TruthCatalog: Send + Sync {
    /// List all truths known to the application.
    fn list_truths(&self) -> Vec<TruthDefinition>;

    /// Resolve a truth by key.
    fn find_truth(&self, key: &str) -> Option<TruthDefinition> {
        self.list_truths()
            .into_iter()
            .find(|truth| truth.key == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truth_kind_equality() {
        assert_eq!(TruthKind::Job, TruthKind::Job);
        assert_ne!(TruthKind::Job, TruthKind::Policy);
        assert_ne!(TruthKind::Policy, TruthKind::Invariant);
    }

    #[test]
    fn truth_kind_serde_roundtrip() {
        for kind in [TruthKind::Job, TruthKind::Policy, TruthKind::Invariant] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: TruthKind = serde_json::from_str(&json).unwrap();
            assert_eq!(kind, back);
        }
    }

    #[test]
    fn criterion_result_met_with_evidence() {
        let result = CriterionResult::Met {
            evidence: vec!["fact-1".into(), "fact-2".into()],
        };
        assert!(matches!(result, CriterionResult::Met { evidence } if evidence.len() == 2));
    }

    #[test]
    fn criterion_result_blocked() {
        let result = CriterionResult::Blocked {
            reason: "needs approval".into(),
            approval_ref: Some("approval:top-up".into()),
        };
        assert!(matches!(result, CriterionResult::Blocked { .. }));
    }

    #[test]
    fn criterion_result_unmet() {
        let result = CriterionResult::Unmet {
            reason: "insufficient funds".into(),
        };
        assert!(matches!(result, CriterionResult::Unmet { reason } if reason.contains("funds")));
    }

    #[test]
    fn criterion_result_indeterminate() {
        let result = CriterionResult::Indeterminate;
        assert!(matches!(result, CriterionResult::Indeterminate));
    }

    #[test]
    fn criterion_result_serde_roundtrip() {
        let variants = vec![
            CriterionResult::Met {
                evidence: vec!["e1".into()],
            },
            CriterionResult::Blocked {
                reason: "wait".into(),
                approval_ref: None,
            },
            CriterionResult::Unmet {
                reason: "fail".into(),
            },
            CriterionResult::Indeterminate,
        ];
        for variant in variants {
            let json = serde_json::to_string(&variant).unwrap();
            let back: CriterionResult = serde_json::from_str(&json).unwrap();
            assert_eq!(variant, back);
        }
    }

    #[test]
    fn truth_catalog_find_truth_default_impl() {
        struct TestCatalog;
        impl TruthCatalog for TestCatalog {
            fn list_truths(&self) -> Vec<TruthDefinition> {
                vec![
                    TruthDefinition {
                        key: "job:onboard".into(),
                        kind: TruthKind::Job,
                        summary: "Onboard a new employee".into(),
                        success_criteria: vec![],
                        constraints: vec![],
                        approval_points: vec![],
                        participating_packs: vec!["hr".into()],
                    },
                    TruthDefinition {
                        key: "policy:expense".into(),
                        kind: TruthKind::Policy,
                        summary: "Expense policy".into(),
                        success_criteria: vec![],
                        constraints: vec![],
                        approval_points: vec![],
                        participating_packs: vec![],
                    },
                ]
            }
        }

        let catalog = TestCatalog;
        let found = catalog.find_truth("job:onboard");
        assert!(found.is_some());
        assert_eq!(found.unwrap().kind, TruthKind::Job);

        let not_found = catalog.find_truth("nonexistent");
        assert!(not_found.is_none());
    }
}
