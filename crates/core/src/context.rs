// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Context model for Converge.
//!
//! Context is the shared, typed, evolving representation of a job.
//! Types are defined in `converge-traits`; this module provides the
//! concrete `Context` struct that the engine uses.

use crate::error::ConvergeError;
use crate::{AdmissionReceipt, AdmissionRequest};
use std::collections::{BTreeMap, BTreeSet, HashMap};

// Re-export canonical types from converge-pack
pub use converge_pack::{
    ContextFact, ContextKey, FactId, FactPayload, PayloadError, PayloadRegistry, ProposalId,
    ProposedFact, Provenance, ProvenanceSource, TextPayload, Timestamp, ValidationError,
    WireContextFact, WireProposedFact,
};

/// Canonical provenance marker for caller-supplied context input.
#[derive(Copy, Clone, Debug)]
pub struct ContextInput;

impl ProvenanceSource for ContextInput {
    fn as_str(&self) -> &'static str {
        "context-input"
    }
}

/// Canonical provenance constant for [`ContextState::add_input`].
pub const CONTEXT_INPUT_PROVENANCE: ContextInput = ContextInput;

/// Durable, verified context snapshot for storage adapters.
///
/// This is the supported rehydration boundary for embedders such as Helms.
/// Storage persists this value and later calls [`ContextState::from_snapshot`].
/// It must not reconstruct facts through promotion constructors.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextSnapshot {
    version: u64,
    merkle_root: crate::integrity::MerkleRoot,
    facts: BTreeMap<ContextKey, Vec<ContextFact>>,
    proposals: BTreeMap<ContextKey, Vec<ProposedFact>>,
}

/// Stable serialized form of a [`ContextSnapshot`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireContextSnapshot {
    version: u64,
    merkle_root: crate::integrity::MerkleRoot,
    facts: BTreeMap<ContextKey, Vec<WireContextFact>>,
    proposals: BTreeMap<ContextKey, Vec<WireProposedFact>>,
}

impl ContextSnapshot {
    /// Build a storage snapshot from a live context.
    #[must_use]
    pub fn from_context(context: &ContextState) -> Self {
        let facts = context
            .facts
            .iter()
            .map(|(key, facts)| (*key, facts.clone()))
            .collect();
        let proposals = context
            .proposals
            .iter()
            .map(|(key, proposals)| (*key, proposals.clone()))
            .collect();

        Self {
            version: context.version,
            merkle_root: crate::integrity::MerkleRoot::from_context(context),
            facts,
            proposals,
        }
    }

    /// Returns the context version captured by the snapshot.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Returns the snapshot Merkle root.
    #[must_use]
    pub fn merkle_root(&self) -> &crate::integrity::MerkleRoot {
        &self.merkle_root
    }

    /// Returns fact projections grouped by context key.
    #[must_use]
    pub fn facts(&self) -> &BTreeMap<ContextKey, Vec<ContextFact>> {
        &self.facts
    }

    /// Returns staged proposals grouped by context key.
    #[must_use]
    pub fn proposals(&self) -> &BTreeMap<ContextKey, Vec<ProposedFact>> {
        &self.proposals
    }

    /// Converts the snapshot into its stable wire envelope.
    pub fn to_wire(&self) -> Result<WireContextSnapshot, PayloadError> {
        let facts = self
            .facts
            .iter()
            .map(|(key, facts)| {
                facts
                    .iter()
                    .map(ContextFact::to_wire)
                    .collect::<Result<Vec<_>, _>>()
                    .map(|facts| (*key, facts))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let proposals = self
            .proposals
            .iter()
            .map(|(key, proposals)| {
                proposals
                    .iter()
                    .map(ProposedFact::to_wire)
                    .collect::<Result<Vec<_>, _>>()
                    .map(|proposals| (*key, proposals))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        Ok(WireContextSnapshot {
            version: self.version,
            merkle_root: self.merkle_root.clone(),
            facts,
            proposals,
        })
    }

    /// Rehydrates a snapshot from its stable wire envelope.
    pub fn from_wire(
        wire: WireContextSnapshot,
        registry: &PayloadRegistry,
    ) -> Result<Self, PayloadError> {
        let facts = wire
            .facts
            .into_iter()
            .map(|(key, facts)| {
                facts
                    .into_iter()
                    .map(|fact| ContextFact::from_wire(fact, registry))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|facts| (key, facts))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let proposals = wire
            .proposals
            .into_iter()
            .map(|(key, proposals)| {
                proposals
                    .into_iter()
                    .map(|proposal| ProposedFact::from_wire(proposal, registry))
                    .collect::<Result<Vec<_>, _>>()
                    .map(|proposals| (key, proposals))
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;

        Ok(Self {
            version: wire.version,
            merkle_root: wire.merkle_root,
            facts,
            proposals,
        })
    }

    fn validate(&self) -> Result<(), ConvergeError> {
        for (key, facts) in &self.facts {
            let mut seen = BTreeSet::new();
            for fact in facts {
                if fact.key() != *key {
                    return Err(ConvergeError::InvalidSnapshot {
                        reason: format!(
                            "fact '{}' stored under {:?} but declares {:?}",
                            fact.id(),
                            key,
                            fact.key()
                        ),
                    });
                }
                if !seen.insert(fact.id().clone()) {
                    return Err(ConvergeError::InvalidSnapshot {
                        reason: format!("duplicate fact '{}' under {:?}", fact.id(), key),
                    });
                }
            }
        }

        for (key, proposals) in &self.proposals {
            let mut seen = BTreeSet::new();
            for proposal in proposals {
                if proposal.key() != *key {
                    return Err(ConvergeError::InvalidSnapshot {
                        reason: format!(
                            "proposal '{}' stored under {:?} but declares {:?}",
                            proposal.id(),
                            key,
                            proposal.key()
                        ),
                    });
                }
                if !seen.insert(proposal.id().clone()) {
                    return Err(ConvergeError::InvalidSnapshot {
                        reason: format!("duplicate proposal '{}' under {:?}", proposal.id(), key),
                    });
                }
            }
        }

        let context = ContextState {
            facts: self
                .facts
                .iter()
                .map(|(key, facts)| (*key, facts.clone()))
                .collect(),
            proposals: self
                .proposals
                .iter()
                .map(|(key, proposals)| (*key, proposals.clone()))
                .collect(),
            dirty_keys: Vec::new(),
            version: self.version,
        };
        let computed_root = crate::integrity::MerkleRoot::from_context(&context);
        if computed_root != self.merkle_root {
            return Err(ConvergeError::InvalidSnapshot {
                reason: "snapshot merkle root does not match restored facts".to_string(),
            });
        }

        Ok(())
    }
}

pub(crate) fn new_fact(
    key: ContextKey,
    id: impl Into<FactId>,
    content: impl Into<String>,
) -> ContextFact {
    new_fact_with_promotion(
        key,
        id,
        TextPayload::new(content),
        converge_pack::FactPromotionRecord::new_projection(
            "engine-projection",
            converge_pack::ContentHash::zero(),
            converge_pack::FactActor::new_projection(
                "converge-engine",
                converge_pack::FactActorKind::System,
            ),
            converge_pack::FactValidationSummary::default(),
            Vec::new(),
            converge_pack::FactTraceLink::Local(converge_pack::FactLocalTrace::new_projection(
                "engine-projection",
                "seed",
                None,
                true,
            )),
            Timestamp::epoch(),
        ),
        Timestamp::epoch(),
    )
}

pub(crate) fn new_fact_with_promotion(
    key: ContextKey,
    id: impl Into<FactId>,
    payload: impl FactPayload + PartialEq,
    promotion_record: converge_pack::FactPromotionRecord,
    created_at: impl Into<Timestamp>,
) -> ContextFact {
    ContextFact::new_projection(key, id, payload, promotion_record, created_at)
}

/// The shared context for a Converge job.
///
/// Agents receive `&dyn converge_pack::Context` (immutable) during execution.
/// Only the engine holds `&mut Context` during the merge phase.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct ContextState {
    /// Facts stored by their key category.
    facts: HashMap<ContextKey, Vec<ContextFact>>,
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

    fn get(&self, key: ContextKey) -> &[ContextFact] {
        self.facts.get(&key).map_or(&[], Vec::as_slice)
    }

    fn get_proposals(&self, key: ContextKey) -> &[ProposedFact] {
        self.proposals.get(&key).map_or(&[], Vec::as_slice)
    }

    fn version(&self) -> u64 {
        self.version
    }
}

impl ContextState {
    /// Creates a new empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Captures a durable storage snapshot for later rehydration.
    #[must_use]
    pub fn snapshot(&self) -> ContextSnapshot {
        ContextSnapshot::from_context(self)
    }

    /// Rehydrates a context from a verified storage snapshot.
    ///
    /// This restores previously promoted context state. It is not a promotion
    /// API: malformed snapshots, key mismatches, duplicate IDs, and Merkle
    /// mismatches are rejected before the context is returned.
    pub fn from_snapshot(snapshot: ContextSnapshot) -> Result<Self, ConvergeError> {
        snapshot.validate()?;
        Ok(Self {
            facts: snapshot.facts.into_iter().collect(),
            proposals: snapshot.proposals.into_iter().collect(),
            dirty_keys: Vec::new(),
            version: snapshot.version,
        })
    }

    /// Returns all facts for a given key.
    #[must_use]
    pub fn get(&self, key: ContextKey) -> &[ContextFact] {
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
        let key = proposal.key();
        let proposals = self.proposals.entry(key).or_default();

        if let Some(existing) = proposals.iter().find(|p| p.id() == proposal.id()) {
            if existing == &proposal {
                return Ok(false);
            }
            return Err(ConvergeError::Conflict {
                id: proposal.id().to_string(),
                existing: format!("{existing:?}"),
                new: format!("{proposal:?}"),
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
        self.add_input_with_provenance(key, id, content, CONTEXT_INPUT_PROVENANCE.provenance())
    }

    /// Stages external input with explicit provenance.
    pub fn add_input_with_provenance(
        &mut self,
        key: ContextKey,
        id: impl Into<ProposalId>,
        content: impl Into<String>,
        provenance: impl Into<Provenance>,
    ) -> Result<bool, ConvergeError> {
        self.add_proposal(ProposedFact::new(
            key,
            id,
            TextPayload::new(content),
            provenance.into(),
        ))
    }

    /// Stages a typed external observation as a proposal.
    ///
    /// This is the preferred boundary for systems such as Organism. It records
    /// actor and source provenance, but it does not create authoritative facts.
    pub fn submit_observation(
        &mut self,
        request: AdmissionRequest,
    ) -> Result<AdmissionReceipt, ConvergeError> {
        let staged = self.add_proposal(request.clone().into_proposal())?;
        Ok(AdmissionReceipt::new(&request, staged))
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
    pub(crate) fn add_fact(&mut self, fact: ContextFact) -> Result<bool, ConvergeError> {
        let key = fact.key();
        let facts = self.facts.entry(key).or_default();

        if let Some(existing) = facts.iter().find(|f| f.id() == fact.id()) {
            if existing == &fact {
                return Ok(false);
            }
            return Err(ConvergeError::Conflict {
                id: fact.id().to_string(),
                existing: format!("{existing:?}"),
                new: format!("{fact:?}"),
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
                assert!(existing.contains("ContextFact"));
                assert!(new.contains("ContextFact"));
            }
            _ => panic!("Expected Conflict error, got {result:?}"),
        }
    }

    #[test]
    fn adding_proposal_tracks_pending_state() {
        let mut ctx = ContextState::new();
        let proposal = ProposedFact::new(
            ContextKey::Hypotheses,
            "hyp-1",
            TextPayload::new("market is growing"),
            CONTEXT_INPUT_PROVENANCE.provenance(),
        );

        assert!(ctx.add_proposal(proposal).unwrap());
        assert!(ctx.has_pending_proposals());
        assert_eq!(ctx.get_proposals(ContextKey::Hypotheses).len(), 1);
    }

    #[test]
    fn conflicting_staged_inputs_are_rejected_before_promotion() {
        let mut ctx = ContextState::new();

        assert!(
            ctx.add_input_with_provenance(
                ContextKey::Seeds,
                "seed-1",
                "version A",
                CONTEXT_INPUT_PROVENANCE.provenance(),
            )
            .unwrap()
        );

        let result = ctx.add_input_with_provenance(
            ContextKey::Seeds,
            "seed-1",
            "version B",
            CONTEXT_INPUT_PROVENANCE.provenance(),
        );

        match result {
            Err(ConvergeError::Conflict {
                id, existing, new, ..
            }) => {
                assert_eq!(id, "seed-1");
                assert!(existing.contains("ProposedFact"));
                assert!(new.contains("ProposedFact"));
            }
            _ => panic!("Expected Conflict error, got {result:?}"),
        }

        assert!(ctx.has_pending_proposals());
        assert_eq!(ctx.get_proposals(ContextKey::Seeds).len(), 1);
    }

    #[test]
    fn snapshot_round_trips_facts_and_proposals() {
        let mut ctx = ContextState::new();
        ctx.add_fact(crate::context::new_fact(
            ContextKey::Seeds,
            "seed-1",
            "persisted seed",
        ))
        .unwrap();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Hypotheses,
            "hyp-1",
            TextPayload::new("staged hypothesis"),
            CONTEXT_INPUT_PROVENANCE.provenance(),
        ))
        .unwrap();

        let restored = ContextState::from_snapshot(ctx.snapshot()).unwrap();

        assert_eq!(restored.version(), 1);
        assert!(restored.dirty_keys().is_empty());
        assert_eq!(restored.get(ContextKey::Seeds)[0].id(), "seed-1");
        assert_eq!(
            restored.get(ContextKey::Seeds)[0].text(),
            Some("persisted seed")
        );
        assert_eq!(
            restored.get_proposals(ContextKey::Hypotheses)[0].id(),
            "hyp-1"
        );
    }

    #[test]
    fn wire_snapshot_round_trips_through_payload_registry() {
        let mut ctx = ContextState::new();
        ctx.add_fact(crate::context::new_fact(
            ContextKey::Seeds,
            "seed-1",
            "persisted seed",
        ))
        .unwrap();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Hypotheses,
            "hyp-1",
            TextPayload::new("staged hypothesis"),
            CONTEXT_INPUT_PROVENANCE.provenance(),
        ))
        .unwrap();

        let registry = PayloadRegistry::with_pack_payloads();
        let wire = ctx.snapshot().to_wire().unwrap();
        let snapshot = ContextSnapshot::from_wire(wire, &registry).unwrap();
        let restored = ContextState::from_snapshot(snapshot).unwrap();

        assert_eq!(
            restored.get(ContextKey::Seeds)[0].text(),
            Some("persisted seed")
        );
        assert_eq!(
            restored.get_proposals(ContextKey::Hypotheses)[0].text(),
            Some("staged hypothesis")
        );
    }

    #[test]
    fn snapshot_rejects_fact_key_mismatch() {
        let mut ctx = ContextState::new();
        ctx.add_fact(crate::context::new_fact(
            ContextKey::Seeds,
            "seed-1",
            "value",
        ))
        .unwrap();

        let mut snapshot = ctx.snapshot();
        let fact = snapshot
            .facts
            .get_mut(&ContextKey::Seeds)
            .unwrap()
            .pop()
            .unwrap();
        snapshot
            .facts
            .entry(ContextKey::Signals)
            .or_default()
            .push(fact);

        let err = ContextState::from_snapshot(snapshot).unwrap_err();
        assert!(matches!(err, ConvergeError::InvalidSnapshot { .. }));
        assert!(err.to_string().contains("stored under Signals"));
    }

    #[test]
    fn snapshot_rejects_merkle_mismatch() {
        let mut ctx = ContextState::new();
        ctx.add_fact(crate::context::new_fact(
            ContextKey::Seeds,
            "seed-1",
            "value",
        ))
        .unwrap();

        let mut snapshot = ctx.snapshot();
        snapshot.merkle_root =
            crate::integrity::MerkleRoot(crate::integrity::ContentHash::compute("tampered"));

        let err = ContextState::from_snapshot(snapshot).unwrap_err();
        assert!(matches!(err, ConvergeError::InvalidSnapshot { .. }));
        assert!(err.to_string().contains("merkle root"));
    }

    #[test]
    fn snapshot_rejects_duplicate_fact_ids() {
        let mut ctx = ContextState::new();
        ctx.add_fact(crate::context::new_fact(
            ContextKey::Seeds,
            "seed-1",
            "value",
        ))
        .unwrap();

        let mut snapshot = ctx.snapshot();
        let duplicate = snapshot.facts.get(&ContextKey::Seeds).unwrap()[0].clone();
        snapshot
            .facts
            .get_mut(&ContextKey::Seeds)
            .unwrap()
            .push(duplicate);

        let err = ContextState::from_snapshot(snapshot).unwrap_err();
        assert!(matches!(err, ConvergeError::InvalidSnapshot { .. }));
        assert!(err.to_string().contains("duplicate fact"));
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
