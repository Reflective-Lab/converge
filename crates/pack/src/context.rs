// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Context keys and the shared context contract.
//!
//! Context is the API. Suggestors don't call each other — they read from and
//! write to shared context through typed keys.

use serde::{Deserialize, Serialize};

use crate::fact::{ContextFact, ProposedFact};
use crate::formation::FormationKind;

/// Typed keys for the shared context namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "strum", derive(strum::EnumIter))]
pub enum ContextKey {
    /// Initial inputs from the root intent. Set once at initialization.
    Seeds,
    /// Proposed ideas and hypotheses from analysis suggestors.
    Hypotheses,
    /// Action plans and strategic recommendations.
    Strategies,
    /// Limitations, rules, and boundary conditions.
    Constraints,
    /// Observations, market data, and signals from the environment.
    Signals,
    /// Competitive intelligence and comparisons.
    Competitors,
    /// Assessments, ratings, and evaluations of other facts.
    Evaluations,
    /// LLM-generated suggestions awaiting validation.
    Proposals,
    /// Error and debugging information. Never blocks convergence.
    Diagnostic,
    /// Votes cast on topics — payload is `governance::Vote`.
    Votes,
    /// Substantive concerns recorded by participants — payload is
    /// `governance::Disagreement`.
    Disagreements,
    /// Deterministic outcomes of evaluating votes against a `ConsensusRule` —
    /// payload is `governance::ConsensusOutcome`.
    ConsensusOutcomes,
}

/// Read-only view of the shared context.
///
/// Suggestors receive `&dyn Context` during `accepts()` and `execute()`.
/// They cannot mutate it directly — mutations happen through `AgentEffect`
/// after the engine collects all effects and merges them deterministically.
pub trait Context: Send + Sync {
    /// Check whether any facts exist under this key.
    fn has(&self, key: ContextKey) -> bool;

    /// Get all read-only context fact projections under this key.
    fn get(&self, key: ContextKey) -> &[ContextFact];

    /// Get all proposed facts (unvalidated).
    fn get_proposals(&self, key: ContextKey) -> &[ProposedFact] {
        let _ = key;
        &[]
    }

    /// Monotonic context version, when the backing implementation tracks one.
    ///
    /// Stateless test contexts and simple external implementations can keep the
    /// default `0`.
    fn version(&self) -> u64 {
        0
    }

    /// Count of facts under a key.
    fn count(&self, key: ContextKey) -> usize {
        self.get(key).len()
    }

    /// The kind of formation orchestrating this suggestor's current
    /// execution, if any. `None` means the suggestor is running outside
    /// a formation harness (e.g., the engine's default registration
    /// path); fall back to standalone behavior.
    ///
    /// Formation harnesses that orchestrate inner suggestors override
    /// this on the context they pass down. Suggestors that don't care
    /// about formation context can ignore this method entirely.
    fn formation_kind(&self) -> Option<FormationKind> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockContext {
        facts: std::collections::HashMap<ContextKey, Vec<ContextFact>>,
    }

    impl MockContext {
        fn empty() -> Self {
            Self {
                facts: std::collections::HashMap::new(),
            }
        }
    }

    impl Context for MockContext {
        fn has(&self, key: ContextKey) -> bool {
            self.facts.get(&key).is_some_and(|v| !v.is_empty())
        }

        fn get(&self, key: ContextKey) -> &[ContextFact] {
            self.facts.get(&key).map_or(&[], Vec::as_slice)
        }
    }

    #[test]
    fn get_proposals_default_returns_empty() {
        let ctx = MockContext::empty();
        assert!(ctx.get_proposals(ContextKey::Seeds).is_empty());
        assert!(ctx.get_proposals(ContextKey::Hypotheses).is_empty());
    }

    #[test]
    fn count_default_delegates_to_get() {
        let ctx = MockContext::empty();
        assert_eq!(ctx.count(ContextKey::Seeds), 0);
    }

    #[test]
    fn has_returns_false_for_empty() {
        let ctx = MockContext::empty();
        assert!(!ctx.has(ContextKey::Seeds));
    }

    #[test]
    fn count_reflects_facts() {
        use crate::fact::{
            FactActor, FactActorKind, FactLocalTrace, FactPromotionRecord, FactTraceLink,
            FactValidationSummary, TextPayload,
        };
        use crate::types::{ContentHash, Timestamp};

        let mut ctx = MockContext::empty();
        let record = FactPromotionRecord::new_projection(
            "projection-test",
            ContentHash::zero(),
            FactActor::new_projection("test", FactActorKind::System),
            FactValidationSummary::default(),
            Vec::new(),
            FactTraceLink::Local(FactLocalTrace::new_projection("trace", "span", None, true)),
            Timestamp::epoch(),
        );
        ctx.facts.insert(
            ContextKey::Seeds,
            vec![ContextFact::new_projection(
                ContextKey::Seeds,
                "f1",
                TextPayload::new("a"),
                record,
                Timestamp::epoch(),
            )],
        );
        assert_eq!(ctx.count(ContextKey::Seeds), 1);
        assert!(ctx.has(ContextKey::Seeds));
        assert!(!ctx.has(ContextKey::Hypotheses));
    }
}
