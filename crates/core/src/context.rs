// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Context model for Converge.
//!
//! Context is the shared, typed, evolving representation of a job.
//! Types are defined in `converge-traits`; this module provides the
//! concrete `Context` struct that the engine uses.

use crate::error::ConvergeError;
use std::collections::HashMap;

// Re-export canonical types from converge-pack
pub use converge_pack::{
    ContextKey, Fact, FactId, ProposalId, ProposedFact, Timestamp, ValidationError,
};

pub(crate) fn new_fact(key: ContextKey, id: impl Into<FactId>, content: impl Into<String>) -> Fact {
    converge_pack::fact::kernel_authority::new_fact(key, id, content)
}

pub(crate) fn new_fact_with_promotion(
    key: ContextKey,
    id: impl Into<FactId>,
    content: impl Into<String>,
    promotion_record: converge_pack::FactPromotionRecord,
    created_at: impl Into<Timestamp>,
) -> Fact {
    converge_pack::fact::kernel_authority::new_fact_with_promotion(
        key,
        id,
        content,
        promotion_record,
        created_at,
    )
}

/// The shared context for a Converge job.
///
/// Agents receive `&dyn converge_pack::Context` (immutable) during execution.
/// Only the engine holds `&mut Context` during the merge phase.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct ContextState {
    /// Facts stored by their key category.
    facts: HashMap<ContextKey, Vec<Fact>>,
    /// Pending proposals staged for engine validation/promotion.
    proposals: HashMap<ContextKey, Vec<ProposedFact>>,
    /// Tracks which keys changed in the last merge cycle.
    dirty_keys: Vec<ContextKey>,
    /// Monotonic version counter for convergence detection.
    version: u64,
}

/// Implement the converge-pack Context trait for the concrete Context struct.
/// This allows agents to use `&dyn converge_pack::Context`.
impl converge_pack::Context for ContextState {
    fn has(&self, key: ContextKey) -> bool {
        self.facts.get(&key).is_some_and(|v| !v.is_empty())
    }

    fn get(&self, key: ContextKey) -> &[Fact] {
        self.facts.get(&key).map_or(&[], Vec::as_slice)
    }

    fn get_proposals(&self, key: ContextKey) -> &[ProposedFact] {
        self.proposals.get(&key).map_or(&[], Vec::as_slice)
    }
}

impl ContextState {
    /// Creates a new empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns all facts for a given key.
    #[must_use]
    pub fn get(&self, key: ContextKey) -> &[Fact] {
        self.facts.get(&key).map_or(&[], Vec::as_slice)
    }

    /// Returns true if there are any facts for the given key.
    #[must_use]
    pub fn has(&self, key: ContextKey) -> bool {
        self.facts.get(&key).is_some_and(|v| !v.is_empty())
    }

    /// Returns the current version (for convergence detection).
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Returns keys that changed in the last merge cycle.
    #[must_use]
    pub fn dirty_keys(&self) -> &[ContextKey] {
        &self.dirty_keys
    }

    /// Returns all keys that currently have facts in the context.
    #[must_use]
    pub fn all_keys(&self) -> Vec<ContextKey> {
        self.facts.keys().copied().collect()
    }

    /// Returns true if any staged proposals are pending promotion.
    #[must_use]
    pub fn has_pending_proposals(&self) -> bool {
        self.proposals.values().any(|items| !items.is_empty())
    }

    /// Clears the dirty key tracker (called at start of each cycle).
    pub fn clear_dirty(&mut self) {
        self.dirty_keys.clear();
    }

    /// Stages a proposal for engine validation/promotion.
    ///
    /// Returns `Ok(true)` if the proposal was new.
    /// Returns `Ok(false)` if an identical proposal is already pending.
    pub fn add_proposal(&mut self, proposal: ProposedFact) -> Result<bool, ConvergeError> {
        let key = proposal.key;
        let proposals = self.proposals.entry(key).or_default();

        if let Some(existing) = proposals.iter().find(|p| p.id == proposal.id) {
            if existing.content == proposal.content
                && existing.confidence == proposal.confidence
                && existing.provenance == proposal.provenance
            {
                return Ok(false);
            }
            return Err(ConvergeError::Conflict {
                id: proposal.id.to_string(),
                existing: existing.content.clone(),
                new: proposal.content,
                context: Box::new(self.clone()),
            });
        }

        proposals.push(proposal);
        Ok(true)
    }

    /// Stages external input as a proposal to be governed by the engine.
    pub fn add_input(
        &mut self,
        key: ContextKey,
        id: impl Into<ProposalId>,
        content: impl Into<String>,
    ) -> Result<bool, ConvergeError> {
        self.add_input_with_provenance(key, id, content, "context-input")
    }

    /// Stages external input with explicit provenance.
    pub fn add_input_with_provenance(
        &mut self,
        key: ContextKey,
        id: impl Into<ProposalId>,
        content: impl Into<String>,
        provenance: impl Into<String>,
    ) -> Result<bool, ConvergeError> {
        self.add_proposal(ProposedFact::new(key, id, content, provenance))
    }

    /// Drains all pending proposals from the context.
    pub(crate) fn drain_proposals(&mut self) -> Vec<ProposedFact> {
        let mut drained = Vec::new();
        for proposals in self.proposals.values_mut() {
            drained.append(proposals);
        }
        self.proposals.retain(|_, proposals| !proposals.is_empty());
        drained
    }

    /// Removes a specific pending proposal if it exists.
    pub(crate) fn remove_proposal(&mut self, key: ContextKey, id: &ProposalId) {
        if let Some(proposals) = self.proposals.get_mut(&key) {
            proposals.retain(|proposal| proposal.id != id);
            if proposals.is_empty() {
                self.proposals.remove(&key);
            }
        }
    }

    /// Adds a fact to the context (engine-only, during merge phase).
    ///
    /// Returns `Ok(true)` if the fact was new (context changed).
    /// Returns `Ok(false)` if the fact was already present and identical.
    pub(crate) fn add_fact(&mut self, fact: Fact) -> Result<bool, ConvergeError> {
        let key = fact.key();
        let facts = self.facts.entry(key).or_default();

        if let Some(existing) = facts.iter().find(|f| f.id == fact.id) {
            if existing.content == fact.content {
                return Ok(false);
            }
            return Err(ConvergeError::Conflict {
                id: fact.id.to_string(),
                existing: existing.content.clone(),
                new: fact.content,
                context: Box::new(self.clone()),
            });
        }

        facts.push(fact);
        self.proposals.remove(&key);
        self.dirty_keys.push(key);

        self.version += 1;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::Context as _;

    #[test]
    fn empty_context_has_no_facts() {
        let ctx = ContextState::new();
        assert!(!ctx.has(ContextKey::Seeds));
        assert_eq!(ctx.version(), 0);
    }

    #[test]
    fn adding_fact_increments_version() {
        let mut ctx = ContextState::new();
        let fact = crate::context::new_fact(ContextKey::Seeds, "seed-1", "initial value");

        let changed = ctx.add_fact(fact).expect("should add");
        assert!(changed);
        assert_eq!(ctx.version(), 1);
        assert!(ctx.has(ContextKey::Seeds));
    }

    #[test]
    fn duplicate_fact_does_not_change_context() {
        let mut ctx = ContextState::new();
        let fact = crate::context::new_fact(ContextKey::Seeds, "seed-1", "initial");

        ctx.add_fact(fact.clone()).expect("should add first");
        let changed = ctx.add_fact(fact).expect("should not error on duplicate");
        assert!(!changed);
        assert_eq!(ctx.version(), 1);
    }

    #[test]
    fn dirty_keys_track_new_facts_and_clear() {
        let mut ctx = ContextState::new();
        let fact = crate::context::new_fact(ContextKey::Hypotheses, "hyp-1", "value");

        ctx.add_fact(fact).expect("should add");
        assert_eq!(ctx.dirty_keys(), &[ContextKey::Hypotheses]);

        ctx.clear_dirty();
        assert!(ctx.dirty_keys().is_empty());
    }

    #[test]
    fn detects_conflict() {
        let mut ctx = ContextState::new();
        ctx.add_fact(crate::context::new_fact(
            ContextKey::Seeds,
            "fact-1",
            "version A",
        ))
        .unwrap();

        let result = ctx.add_fact(crate::context::new_fact(
            ContextKey::Seeds,
            "fact-1",
            "version B",
        ));

        match result {
            Err(ConvergeError::Conflict {
                id, existing, new, ..
            }) => {
                assert_eq!(id, "fact-1");
                assert_eq!(existing, "version A");
                assert_eq!(new, "version B");
            }
            _ => panic!("Expected Conflict error, got {result:?}"),
        }
    }

    #[test]
    fn adding_proposal_tracks_pending_state() {
        let mut ctx = ContextState::new();
        let proposal =
            ProposedFact::new(ContextKey::Hypotheses, "hyp-1", "market is growing", "test");

        assert!(ctx.add_proposal(proposal).unwrap());
        assert!(ctx.has_pending_proposals());
        assert_eq!(ctx.get_proposals(ContextKey::Hypotheses).len(), 1);
    }

    #[test]
    fn conflicting_staged_inputs_are_rejected_before_promotion() {
        let mut ctx = ContextState::new();

        assert!(
            ctx.add_input_with_provenance(ContextKey::Seeds, "seed-1", "version A", "user")
                .unwrap()
        );

        let result =
            ctx.add_input_with_provenance(ContextKey::Seeds, "seed-1", "version B", "user");

        match result {
            Err(ConvergeError::Conflict {
                id, existing, new, ..
            }) => {
                assert_eq!(id, "seed-1");
                assert_eq!(existing, "version A");
                assert_eq!(new, "version B");
            }
            _ => panic!("Expected Conflict error, got {result:?}"),
        }

        assert!(ctx.has_pending_proposals());
        assert_eq!(ctx.get_proposals(ContextKey::Seeds).len(), 1);
    }

    /// Test that Context implements the converge_pack::Context trait.
    #[test]
    fn context_implements_trait() {
        let mut ctx = ContextState::new();
        ctx.add_fact(crate::context::new_fact(ContextKey::Seeds, "s1", "hello"))
            .unwrap();

        // Use via trait object
        let dyn_ctx: &dyn converge_pack::Context = &ctx;
        assert!(dyn_ctx.has(ContextKey::Seeds));
        assert_eq!(dyn_ctx.get(ContextKey::Seeds).len(), 1);
    }
}
