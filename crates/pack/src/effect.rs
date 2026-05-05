// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Suggestor effects — what suggestors produce, the engine merges.
//!
//! Effects are proposal-only. Suggestors suggest; the engine validates and promotes.

use crate::context::ContextKey;
use crate::fact::ProposedFact;

/// The output of a suggestor's `execute()` call.
///
/// An effect describes what a suggestor wants to suggest to the context.
/// The engine collects effects from all eligible suggestors, validates them,
/// and promotes them serially in deterministic order.
#[derive(Debug, Default)]
pub struct AgentEffect {
    /// New proposals to be validated by the engine.
    proposals: Vec<ProposedFact>,
}

/// Construction helper for incrementally assembling an [`AgentEffect`].
///
/// This keeps mutation in the authoring phase while preserving [`AgentEffect`]
/// as the finished proposal output value returned by a suggestor.
#[derive(Debug, Default)]
pub struct AgentEffectBuilder {
    proposals: Vec<ProposedFact>,
}

impl AgentEffectBuilder {
    /// Creates an empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds one proposal and returns the builder for fluent construction.
    #[must_use]
    pub fn proposal(mut self, proposal: ProposedFact) -> Self {
        self.proposals.push(proposal);
        self
    }

    /// Adds many proposals and returns the builder for fluent construction.
    #[must_use]
    pub fn proposals(mut self, proposals: impl IntoIterator<Item = ProposedFact>) -> Self {
        self.proposals.extend(proposals);
        self
    }

    /// Appends one proposal to an existing mutable builder.
    pub fn push(&mut self, proposal: ProposedFact) {
        self.proposals.push(proposal);
    }

    /// Appends many proposals to an existing mutable builder.
    pub fn extend(&mut self, proposals: impl IntoIterator<Item = ProposedFact>) {
        self.proposals.extend(proposals);
    }

    /// Returns true if the builder has no proposals.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.proposals.is_empty()
    }

    /// Finalizes the builder into a suggestor effect.
    #[must_use]
    pub fn build(self) -> AgentEffect {
        AgentEffect::with_proposals(self.proposals)
    }
}

impl AgentEffect {
    /// Starts building an effect incrementally.
    #[must_use]
    pub fn builder() -> AgentEffectBuilder {
        AgentEffectBuilder::new()
    }

    /// Creates an empty effect (no contributions).
    #[must_use]
    pub fn empty() -> Self {
        Self::default()
    }

    /// Creates an effect with a single proposal.
    #[must_use]
    pub fn with_proposal(proposal: ProposedFact) -> Self {
        Self {
            proposals: vec![proposal],
        }
    }

    /// Creates an effect with multiple proposals.
    #[must_use]
    pub fn with_proposals(proposals: Vec<ProposedFact>) -> Self {
        Self { proposals }
    }

    /// Borrows the proposals carried by this effect.
    #[must_use]
    pub fn proposals(&self) -> &[ProposedFact] {
        &self.proposals
    }

    /// Consumes the effect and returns its proposals.
    #[must_use]
    pub fn into_proposals(self) -> Vec<ProposedFact> {
        self.proposals
    }

    /// Returns true if this effect contributes nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.proposals.is_empty()
    }

    /// Returns the context keys affected by this effect.
    #[must_use]
    pub fn affected_keys(&self) -> Vec<ContextKey> {
        let mut keys: Vec<ContextKey> = self.proposals.iter().map(|p| p.key).collect();
        keys.sort();
        keys.dedup();
        keys
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proposal(key: ContextKey, id: &str) -> ProposedFact {
        ProposedFact::new(key, id, "content", "test")
    }

    #[test]
    fn empty_effect_is_empty() {
        let e = AgentEffect::empty();
        assert!(e.is_empty());
        assert!(e.proposals().is_empty());
    }

    #[test]
    fn with_proposal_single() {
        let e = AgentEffect::with_proposal(proposal(ContextKey::Seeds, "p1"));
        assert!(!e.is_empty());
        assert_eq!(e.proposals().len(), 1);
        assert_eq!(e.proposals()[0].id, "p1");
    }

    #[test]
    fn with_proposals_multiple() {
        let e = AgentEffect::with_proposals(vec![
            proposal(ContextKey::Seeds, "p1"),
            proposal(ContextKey::Hypotheses, "p2"),
        ]);
        assert_eq!(e.proposals().len(), 2);
    }

    #[test]
    fn builder_supports_fluent_proposal_construction() {
        let e = AgentEffect::builder()
            .proposal(proposal(ContextKey::Seeds, "p1"))
            .proposal(proposal(ContextKey::Hypotheses, "p2"))
            .build();

        assert_eq!(e.proposals().len(), 2);
        assert_eq!(e.proposals()[0].id, "p1");
        assert_eq!(e.proposals()[1].id, "p2");
    }

    #[test]
    fn builder_supports_mutable_incremental_construction() {
        let mut builder = AgentEffect::builder();
        assert!(builder.is_empty());

        builder.push(proposal(ContextKey::Seeds, "p1"));
        builder.extend([proposal(ContextKey::Hypotheses, "p2")]);

        let e = builder.build();
        assert_eq!(e.proposals().len(), 2);
        assert_eq!(e.affected_keys().len(), 2);
    }

    #[test]
    fn builder_supports_iterator_construction() {
        let proposals = [
            proposal(ContextKey::Seeds, "p1"),
            proposal(ContextKey::Hypotheses, "p2"),
        ];

        let e = AgentEffect::builder().proposals(proposals).build();

        assert_eq!(e.proposals().len(), 2);
    }

    #[test]
    fn is_empty_false_for_nonempty() {
        let e = AgentEffect::with_proposal(proposal(ContextKey::Signals, "x"));
        assert!(!e.is_empty());
    }

    #[test]
    fn affected_keys_deduplicates_and_sorts() {
        let e = AgentEffect::with_proposals(vec![
            proposal(ContextKey::Signals, "a"),
            proposal(ContextKey::Seeds, "b"),
            proposal(ContextKey::Signals, "c"),
            proposal(ContextKey::Seeds, "d"),
            proposal(ContextKey::Hypotheses, "e"),
        ]);
        let keys = e.affected_keys();
        assert_eq!(keys.len(), 3);
        // Sorted by Ord impl
        for window in keys.windows(2) {
            assert!(window[0] <= window[1]);
        }
        // No duplicates
        let mut deduped = keys.clone();
        deduped.dedup();
        assert_eq!(keys, deduped);
    }

    #[test]
    fn affected_keys_empty_for_empty_effect() {
        let e = AgentEffect::empty();
        assert!(e.affected_keys().is_empty());
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        fn arb_context_key() -> impl Strategy<Value = ContextKey> {
            prop_oneof![
                Just(ContextKey::Seeds),
                Just(ContextKey::Hypotheses),
                Just(ContextKey::Strategies),
                Just(ContextKey::Constraints),
                Just(ContextKey::Signals),
                Just(ContextKey::Competitors),
                Just(ContextKey::Evaluations),
                Just(ContextKey::Proposals),
                Just(ContextKey::Diagnostic),
                Just(ContextKey::Votes),
                Just(ContextKey::Disagreements),
                Just(ContextKey::ConsensusOutcomes),
            ]
        }

        proptest! {
            #[test]
            fn affected_keys_never_has_duplicates(
                keys in proptest::collection::vec(arb_context_key(), 0..50),
            ) {
                let proposals: Vec<ProposedFact> = keys
                    .iter()
                    .enumerate()
                    .map(|(i, &k)| ProposedFact::new(k, format!("p{i}"), "c", "prov"))
                    .collect();
                let effect = AgentEffect::with_proposals(proposals);
                let result = effect.affected_keys();
                let mut deduped = result.clone();
                deduped.dedup();
                prop_assert_eq!(result, deduped);
            }
        }
    }
}
