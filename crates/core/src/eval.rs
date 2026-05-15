// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Eval system for Converge.
//!
//! Evals are formal definitions of acceptable outcomes.
//! They are outcome-based, not path-based, and define what "good" means
//! independently of implementation.
//!
//! # Eval Philosophy
//!
//! In Converge, evals are not tests of behavior — they are formal definitions
//! of acceptable outcomes. This aligns with the principle that:
//!
//! - Evals test whether a convergence outcome satisfies intent-level properties
//! - Evals are reusable across models, agents, and time
//! - Evals are stored as traceable artifacts
//! - Evals can be used in invariant checks
//!
//! # Eval Layers
//!
//! 1. **Engine-level evals** (foundational, non-negotiable)
//!    - Axioms like no hidden state, deterministic convergence
//!    - These define whether Converge itself is correct
//!
//! 2. **Domain-level evals** (business semantics)
//!    - Intent-relative, outcome-oriented
//!    - Examples: "Did SDR funnel produce diverse, qualified leads?"
//!    - These are the competitive moat
//!
//! 3. **Suggestor-level evals** (quality hints)
//!    - Prompt quality, output format, hallucination containment
//!    - Scoped, local, non-authoritative
//!
//! # Example
//!
//! ```
//! use converge_core::eval::{Eval, EvalOutcome, EvalResult};
//! use converge_core::{Context, ContextKey};
//!
//! struct StrategyDiversityEval;
//!
//! impl Eval for StrategyDiversityEval {
//!     fn name(&self) -> &str { "strategy_diversity" }
//!     fn description(&self) -> &str { "Ensures at least 3 distinct strategies exist" }
//!
//!     fn evaluate(&self, ctx: &dyn Context) -> EvalResult {
//!         let strategies = ctx.get(ContextKey::Strategies);
//!         let distinct_count = strategies.len();
//!
//!         if distinct_count >= 3 {
//!             EvalResult {
//!                 eval_name: self.name().to_string(),
//!                 outcome: EvalOutcome::Pass,
//!                 score: 1.0,
//!                 rationale: format!("Found {} distinct strategies", distinct_count),
//!                 fact_ids: strategies.iter().map(|s| s.id().clone()).collect(),
//!                 metadata: None,
//!             }
//!         } else {
//!             EvalResult {
//!                 eval_name: self.name().to_string(),
//!                 outcome: EvalOutcome::Fail,
//!                 score: distinct_count as f64 / 3.0,
//!                 rationale: format!("Only {} strategies found, need 3", distinct_count),
//!                 fact_ids: strategies.iter().map(|s| s.id().clone()).collect(),
//!                 metadata: None,
//!             }
//!         }
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};

use crate::context::{ContextFact, ContextKey, FactId};

/// The outcome of an eval execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvalOutcome {
    /// Eval passed (outcome satisfies the definition).
    Pass,
    /// Eval failed (outcome does not satisfy the definition).
    Fail,
    /// Eval could not be determined (missing data, error, etc.).
    Indeterminate,
}

impl EvalOutcome {
    /// Returns true if the eval passed.
    #[must_use]
    pub fn is_pass(&self) -> bool {
        matches!(self, Self::Pass)
    }

    /// Returns true if the eval failed.
    #[must_use]
    pub fn is_fail(&self) -> bool {
        matches!(self, Self::Fail)
    }

    /// Returns true if the eval is indeterminate.
    #[must_use]
    pub fn is_indeterminate(&self) -> bool {
        matches!(self, Self::Indeterminate)
    }
}

/// The result of executing an eval.
///
/// Eval results are stored as facts in context for traceability
/// and can be used in invariant checks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvalResult {
    /// Name of the eval that produced this result.
    pub eval_name: String,
    /// The outcome (pass/fail/indeterminate).
    pub outcome: EvalOutcome,
    /// Numeric score (0.0 - 1.0) for quantitative evals.
    pub score: f64,
    /// Human-readable explanation of the result.
    pub rationale: String,
    /// IDs of facts that were evaluated (for traceability).
    pub fact_ids: Vec<FactId>,
    /// Optional: metadata about the eval execution.
    pub metadata: Option<String>,
}

impl EvalResult {
    /// Creates a new eval result.
    #[must_use]
    pub fn new(
        eval_name: impl Into<String>,
        outcome: EvalOutcome,
        score: f64,
        rationale: impl Into<String>,
    ) -> Self {
        Self {
            eval_name: eval_name.into(),
            outcome,
            score: score.clamp(0.0, 1.0),
            rationale: rationale.into(),
            fact_ids: Vec::new(),
            metadata: None,
        }
    }

    /// Creates an eval result with associated fact IDs.
    #[must_use]
    pub fn with_facts(
        eval_name: impl Into<String>,
        outcome: EvalOutcome,
        score: f64,
        rationale: impl Into<String>,
        fact_ids: Vec<FactId>,
    ) -> Self {
        Self {
            eval_name: eval_name.into(),
            outcome,
            score: score.clamp(0.0, 1.0),
            rationale: rationale.into(),
            fact_ids,
            metadata: None,
        }
    }

    /// Converts an eval result to a fact for storage in context.
    ///
    /// The fact is stored in `ContextKey::Evaluations` with an ID
    /// that includes the eval name for traceability.
    #[must_use]
    pub fn to_fact(&self, eval_id: Option<&str>) -> ContextFact {
        let id = if let Some(eid) = eval_id {
            format!("eval:{}:{}", self.eval_name, eid)
        } else {
            format!("eval:{}", self.eval_name)
        };

        let content = format!(
            "Outcome: {:?} | Score: {:.2} | {}",
            self.outcome, self.score, self.rationale
        );

        crate::context::new_fact(ContextKey::Evaluations, id, content)
    }
}

/// A runtime eval that defines acceptable outcomes.
///
/// Evals are evaluative (measure quality/acceptability), not prescriptive
/// (must hold). They can be run by agents or as part of convergence.
///
/// Unlike invariants, evals:
/// - Return results, not violations
/// - Can be run on-demand
/// - Store results as facts
/// - Can be used in invariant checks
pub trait Eval: Send + Sync {
    /// Human-readable name for tracing and storage.
    fn name(&self) -> &str;

    /// Human-readable description of what this eval measures.
    fn description(&self) -> &str;

    /// Evaluates the context and returns a result.
    ///
    /// This is pure and side-effect free. The result is stored
    /// as a fact in context by the caller.
    fn evaluate(&self, ctx: &dyn crate::Context) -> EvalResult;

    /// Optional: Context keys this eval depends on.
    ///
    /// Used for efficient scheduling (only run when dependencies change).
    /// If not specified, the eval may run on every cycle.
    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }
}

/// Unique identifier for a registered eval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvalId(pub(crate) u32);

impl std::fmt::Display for EvalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Eval({})", self.0)
    }
}

/// Registry of evals, organized for efficient execution.
#[derive(Default)]
pub struct EvalRegistry {
    evals: Vec<Box<dyn Eval>>,
    by_dependency: std::collections::HashMap<ContextKey, Vec<EvalId>>,
    next_id: u32,
}

impl EvalRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an eval and returns its ID.
    pub fn register(&mut self, eval: impl Eval + 'static) -> EvalId {
        let id = EvalId(self.next_id);
        self.next_id += 1;

        let deps = eval.dependencies();
        for &key in deps {
            self.by_dependency.entry(key).or_default().push(id);
        }

        self.evals.push(Box::new(eval));
        id
    }

    /// Returns the number of registered evals.
    #[must_use]
    pub fn count(&self) -> usize {
        self.evals.len()
    }

    /// Executes all registered evals and returns their results.
    ///
    /// Results are returned in registration order for determinism.
    #[must_use]
    pub fn evaluate_all(&self, ctx: &dyn crate::Context) -> Vec<EvalResult> {
        self.evals.iter().map(|eval| eval.evaluate(ctx)).collect()
    }

    /// Executes evals that depend on the given context keys.
    ///
    /// Used for efficient scheduling (only run when dependencies change).
    #[must_use]
    pub fn evaluate_dependent(
        &self,
        ctx: &dyn crate::Context,
        dirty_keys: &[ContextKey],
    ) -> Vec<EvalResult> {
        let mut eval_ids: std::collections::HashSet<EvalId> = std::collections::HashSet::new();

        // Find evals that depend on dirty keys
        for key in dirty_keys {
            if let Some(ids) = self.by_dependency.get(key) {
                eval_ids.extend(ids);
            }
        }

        // Execute evals in registration order
        self.evals
            .iter()
            .enumerate()
            .filter_map(|(idx, eval)| {
                let id = EvalId(idx as u32);
                if eval_ids.contains(&id) || eval.dependencies().is_empty() {
                    Some(eval.evaluate(ctx))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Executes a specific eval by ID.
    ///
    /// # Panics
    ///
    /// Panics if the eval ID is out of bounds.
    #[must_use]
    pub fn evaluate_by_id(&self, id: EvalId, ctx: &dyn crate::Context) -> EvalResult {
        self.evals[id.0 as usize].evaluate(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextState;

    /// Eval that checks if at least one seed exists.
    struct RequireSeedsEval;

    impl Eval for RequireSeedsEval {
        fn name(&self) -> &'static str {
            "require_seeds"
        }

        fn description(&self) -> &'static str {
            "Checks if at least one seed exists in context"
        }

        fn evaluate(&self, ctx: &dyn crate::Context) -> EvalResult {
            let seeds = ctx.get(ContextKey::Seeds);
            let count = seeds.len();

            if count > 0 {
                EvalResult::new(
                    self.name(),
                    EvalOutcome::Pass,
                    1.0,
                    format!("Found {} seeds", count),
                )
            } else {
                EvalResult::new(self.name(), EvalOutcome::Fail, 0.0, "No seeds found")
            }
        }

        fn dependencies(&self) -> &[ContextKey] {
            &[ContextKey::Seeds]
        }
    }

    #[test]
    fn registry_registers_evals() {
        let mut registry = EvalRegistry::new();
        let id1 = registry.register(RequireSeedsEval);
        let id2 = registry.register(RequireSeedsEval);

        assert_eq!(registry.count(), 2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn eval_passes_when_seeds_exist() {
        let mut registry = EvalRegistry::new();
        let id = registry.register(RequireSeedsEval);

        let mut ctx = ContextState::new();
        let _ = ctx.add_fact(crate::context::new_fact(ContextKey::Seeds, "s1", "value"));

        let result = registry.evaluate_by_id(id, &ctx);
        assert_eq!(result.outcome, EvalOutcome::Pass);
        assert!((result.score - 1.0_f64).abs() < f64::EPSILON);
    }

    #[test]
    fn eval_fails_when_no_seeds() {
        let mut registry = EvalRegistry::new();
        let id = registry.register(RequireSeedsEval);

        let ctx = ContextState::new();
        let result = registry.evaluate_by_id(id, &ctx);

        assert_eq!(result.outcome, EvalOutcome::Fail);
        assert!((result.score - 0.0_f64).abs() < f64::EPSILON);
    }

    #[test]
    fn eval_result_converts_to_fact() {
        let result = EvalResult::new("test_eval", EvalOutcome::Pass, 0.85, "Test passed");

        let fact = result.to_fact(None);
        assert_eq!(fact.key(), ContextKey::Evaluations);
        assert!(fact.id().starts_with("eval:test_eval"));
        let text = fact.text().unwrap_or_default();
        assert!(text.contains("Pass"));
        assert!(text.contains("0.85"));
    }

    #[test]
    fn eval_result_score_is_clamped() {
        let result = EvalResult::new(
            "test",
            EvalOutcome::Pass,
            1.5, // > 1.0
            "test",
        );

        assert!((result.score - 1.0_f64).abs() < f64::EPSILON);

        let result = EvalResult::new(
            "test",
            EvalOutcome::Pass,
            -0.5, // < 0.0
            "test",
        );

        assert!((result.score - 0.0_f64).abs() < f64::EPSILON);
    }
}
