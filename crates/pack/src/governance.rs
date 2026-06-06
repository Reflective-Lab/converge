// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Governance primitives shared across packs.
//!
//! These are pure data types: a rule for tallying votes, a vote payload, a
//! disagreement payload, and the deterministic outcome of evaluating votes
//! against a rule. They carry no scheduling, round, or pipeline semantics —
//! consumers (huddles, approval flows, multi-agent panels) compose them.

use serde::de;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;

use crate::fact::FactPayload;
use crate::types::{ActorId, VoteTopicId};

/// Error returned when governance payloads violate their typed invariants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernanceError {
    /// A consensus rule needs at least one eligible voter.
    ZeroEligibleVoters,
    /// Tallied votes cannot exceed the eligible voter count.
    TalliesExceedEligibleVoters {
        tallied_votes: usize,
        eligible_voters: usize,
    },
    /// A serialized outcome carried a `passes` flag that does not match the rule.
    PassFlagMismatch { expected: bool, actual: bool },
}

impl std::fmt::Display for GovernanceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroEligibleVoters => f.write_str("eligible voters must be greater than zero"),
            Self::TalliesExceedEligibleVoters {
                tallied_votes,
                eligible_voters,
            } => write!(
                f,
                "tallied votes ({tallied_votes}) exceed eligible voters ({eligible_voters})"
            ),
            Self::PassFlagMismatch { expected, actual } => write!(
                f,
                "serialized consensus outcome pass flag mismatch: expected {expected}, got {actual}"
            ),
        }
    }
}

impl std::error::Error for GovernanceError {}

/// Number of actors eligible to vote on a topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct EligibleVoters(NonZeroUsize);

impl EligibleVoters {
    /// Create a non-zero eligible voter count.
    pub fn new(value: usize) -> Result<Self, GovernanceError> {
        NonZeroUsize::new(value)
            .map(Self)
            .ok_or(GovernanceError::ZeroEligibleVoters)
    }

    /// Return the eligible voter count.
    #[must_use]
    pub fn get(self) -> usize {
        self.0.get()
    }
}

impl TryFrom<usize> for EligibleVoters {
    type Error = GovernanceError;

    fn try_from(value: usize) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<EligibleVoters> for usize {
    fn from(value: EligibleVoters) -> Self {
        value.get()
    }
}

impl<'de> Deserialize<'de> for EligibleVoters {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = usize::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Tally of latest votes for a topic.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoteTally {
    yes_votes: usize,
    no_votes: usize,
    abstain_votes: usize,
}

impl VoteTally {
    /// Create a tally from counted decisions.
    #[must_use]
    pub const fn new(yes_votes: usize, no_votes: usize, abstain_votes: usize) -> Self {
        Self {
            yes_votes,
            no_votes,
            abstain_votes,
        }
    }

    #[must_use]
    pub const fn yes_votes(self) -> usize {
        self.yes_votes
    }

    #[must_use]
    pub const fn no_votes(self) -> usize {
        self.no_votes
    }

    #[must_use]
    pub const fn abstain_votes(self) -> usize {
        self.abstain_votes
    }

    #[must_use]
    pub const fn total_cast(self) -> usize {
        self.yes_votes
            .saturating_add(self.no_votes)
            .saturating_add(self.abstain_votes)
    }
}

/// Decision rule used to tally votes on a topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusRule {
    /// Strict majority: yes votes must exceed half of total voters.
    Majority,
    /// Two-thirds threshold (yes * 3 >= total * 2).
    Supermajority,
    /// Every voter must vote yes.
    Unanimous,
    /// A single yes from any voter is sufficient.
    LeadDecides,
    /// Votes are recorded but never block.
    AdvisoryOnly,
}

impl ConsensusRule {
    /// Whether the given tally satisfies the rule.
    #[must_use]
    pub fn passes(self, tally: VoteTally, total_voters: EligibleVoters) -> bool {
        if tally.total_cast() > total_voters.get() {
            return false;
        }
        let yes_votes = tally.yes_votes() as u128;
        let total_voters = total_voters.get() as u128;
        match self {
            Self::Majority => yes_votes * 2 > total_voters,
            Self::Supermajority => yes_votes * 3 >= total_voters * 2,
            Self::Unanimous => yes_votes == total_voters,
            Self::LeadDecides => yes_votes >= 1,
            Self::AdvisoryOnly => true,
        }
    }

    /// Stable wire-format label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Majority => "majority",
            Self::Supermajority => "supermajority",
            Self::Unanimous => "unanimous",
            Self::LeadDecides => "lead_decides",
            Self::AdvisoryOnly => "advisory_only",
        }
    }
}

/// How a single voter answered on a topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoteDecision {
    Yes,
    No,
    Abstain,
}

impl VoteDecision {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Yes => "yes",
            Self::No => "no",
            Self::Abstain => "abstain",
        }
    }
}

/// A single vote cast by an actor on a topic.
///
/// Identity, timestamps, and provenance live on the wrapping `Fact`; this
/// payload only carries the semantic content of the vote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Vote {
    pub topic: VoteTopicId,
    pub voter: ActorId,
    pub decision: VoteDecision,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
}

impl FactPayload for Vote {
    const FAMILY: &'static str = "converge.governance.vote";
    const VERSION: u16 = 1;
}

/// A substantive concern recorded by an actor against a topic.
///
/// Independent of vote direction — an actor can vote `Yes` on proceeding
/// while still registering a disagreement on a sub-point.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Disagreement {
    pub topic: VoteTopicId,
    pub dissenter: ActorId,
    pub reason: String,
}

impl FactPayload for Disagreement {
    const FAMILY: &'static str = "converge.governance.disagreement";
    const VERSION: u16 = 1;
}

/// Deterministic result of evaluating votes against a rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsensusOutcome {
    topic: VoteTopicId,
    rule: ConsensusRule,
    tally: VoteTally,
    total_voters: EligibleVoters,
    passes: bool,
}

impl ConsensusOutcome {
    /// Tally `votes` against `rule` for `total_voters` eligible participants.
    ///
    /// Votes whose `topic` does not match are ignored. Each `voter` is counted
    /// at most once: when an actor appears multiple times, the latest entry
    /// wins (callers control ordering).
    pub fn evaluate(
        topic: VoteTopicId,
        rule: ConsensusRule,
        votes: &[Vote],
        total_voters: EligibleVoters,
    ) -> Result<Self, GovernanceError> {
        let mut latest: Vec<(&ActorId, VoteDecision)> = Vec::new();
        for vote in votes.iter().filter(|v| v.topic == topic) {
            if let Some(slot) = latest.iter_mut().find(|(voter, _)| *voter == &vote.voter) {
                slot.1 = vote.decision;
            } else {
                latest.push((&vote.voter, vote.decision));
            }
        }

        let mut yes_votes = 0usize;
        let mut no_votes = 0usize;
        let mut abstain_votes = 0usize;
        for (_, decision) in &latest {
            match decision {
                VoteDecision::Yes => yes_votes += 1,
                VoteDecision::No => no_votes += 1,
                VoteDecision::Abstain => abstain_votes += 1,
            }
        }

        Self::from_tally(
            topic,
            rule,
            VoteTally::new(yes_votes, no_votes, abstain_votes),
            total_voters,
        )
    }

    /// Build an outcome from an already-counted tally.
    pub fn from_tally(
        topic: VoteTopicId,
        rule: ConsensusRule,
        tally: VoteTally,
        total_voters: EligibleVoters,
    ) -> Result<Self, GovernanceError> {
        if tally.total_cast() > total_voters.get() {
            return Err(GovernanceError::TalliesExceedEligibleVoters {
                tallied_votes: tally.total_cast(),
                eligible_voters: total_voters.get(),
            });
        }
        Ok(Self {
            topic,
            rule,
            tally,
            total_voters,
            passes: rule.passes(tally, total_voters),
        })
    }

    #[must_use]
    pub fn topic(&self) -> &VoteTopicId {
        &self.topic
    }

    #[must_use]
    pub const fn rule(&self) -> ConsensusRule {
        self.rule
    }

    #[must_use]
    pub const fn tally(&self) -> VoteTally {
        self.tally
    }

    #[must_use]
    pub const fn total_voters(&self) -> EligibleVoters {
        self.total_voters
    }

    #[must_use]
    pub const fn passes(&self) -> bool {
        self.passes
    }
}

impl Serialize for ConsensusOutcome {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire<'a> {
            topic: &'a VoteTopicId,
            rule: ConsensusRule,
            yes_votes: usize,
            no_votes: usize,
            abstain_votes: usize,
            total_voters: usize,
            passes: bool,
        }

        Wire {
            topic: &self.topic,
            rule: self.rule,
            yes_votes: self.tally.yes_votes(),
            no_votes: self.tally.no_votes(),
            abstain_votes: self.tally.abstain_votes(),
            total_voters: self.total_voters.get(),
            passes: self.passes,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ConsensusOutcome {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire {
            topic: VoteTopicId,
            rule: ConsensusRule,
            yes_votes: usize,
            no_votes: usize,
            abstain_votes: usize,
            total_voters: EligibleVoters,
            passes: bool,
        }

        let wire = Wire::deserialize(deserializer)?;
        let outcome = Self::from_tally(
            wire.topic,
            wire.rule,
            VoteTally::new(wire.yes_votes, wire.no_votes, wire.abstain_votes),
            wire.total_voters,
        )
        .map_err(de::Error::custom)?;
        if outcome.passes != wire.passes {
            return Err(de::Error::custom(GovernanceError::PassFlagMismatch {
                expected: outcome.passes,
                actual: wire.passes,
            }));
        }
        Ok(outcome)
    }
}

impl FactPayload for ConsensusOutcome {
    const FAMILY: &'static str = "converge.governance.consensus_outcome";
    const VERSION: u16 = 1;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn topic(s: &str) -> VoteTopicId {
        VoteTopicId::new(s)
    }

    fn voter(s: &str) -> ActorId {
        ActorId::new(s)
    }

    fn eligible(n: usize) -> EligibleVoters {
        EligibleVoters::new(n).unwrap()
    }

    fn vote(t: &str, v: &str, d: VoteDecision) -> Vote {
        Vote {
            topic: topic(t),
            voter: voter(v),
            decision: d,
            reason: None,
        }
    }

    #[test]
    fn rule_passes_thresholds_match_organism_baseline() {
        assert!(ConsensusRule::Majority.passes(VoteTally::new(3, 1, 0), eligible(4)));
        assert!(!ConsensusRule::Majority.passes(VoteTally::new(2, 2, 0), eligible(4)));
        assert!(ConsensusRule::Supermajority.passes(VoteTally::new(2, 1, 0), eligible(3)));
        assert!(!ConsensusRule::Supermajority.passes(VoteTally::new(1, 2, 0), eligible(3)));
        assert!(ConsensusRule::Unanimous.passes(VoteTally::new(5, 0, 0), eligible(5)));
        assert!(!ConsensusRule::Unanimous.passes(VoteTally::new(4, 1, 0), eligible(5)));
        assert!(ConsensusRule::LeadDecides.passes(VoteTally::new(1, 0, 0), eligible(9)));
        assert!(ConsensusRule::AdvisoryOnly.passes(VoteTally::new(0, 0, 0), eligible(10)));
    }

    #[test]
    fn rule_label_is_stable_snake_case() {
        assert_eq!(ConsensusRule::Majority.label(), "majority");
        assert_eq!(ConsensusRule::Supermajority.label(), "supermajority");
        assert_eq!(ConsensusRule::Unanimous.label(), "unanimous");
        assert_eq!(ConsensusRule::LeadDecides.label(), "lead_decides");
        assert_eq!(ConsensusRule::AdvisoryOnly.label(), "advisory_only");
    }

    #[test]
    fn outcome_tallies_only_matching_topic() {
        let votes = vec![
            vote("t1", "alice", VoteDecision::Yes),
            vote("t1", "bob", VoteDecision::No),
            vote("t2", "carol", VoteDecision::Yes),
        ];
        let outcome =
            ConsensusOutcome::evaluate(topic("t1"), ConsensusRule::Majority, &votes, eligible(2))
                .unwrap();
        assert_eq!(outcome.tally().yes_votes(), 1);
        assert_eq!(outcome.tally().no_votes(), 1);
        assert_eq!(outcome.total_voters().get(), 2);
        assert!(!outcome.passes());
    }

    #[test]
    fn outcome_collapses_repeat_votes_per_actor_to_latest() {
        let votes = vec![
            vote("t1", "alice", VoteDecision::No),
            vote("t1", "alice", VoteDecision::Yes),
            vote("t1", "bob", VoteDecision::Yes),
        ];
        let outcome =
            ConsensusOutcome::evaluate(topic("t1"), ConsensusRule::Unanimous, &votes, eligible(2))
                .unwrap();
        assert_eq!(outcome.tally().yes_votes(), 2);
        assert_eq!(outcome.tally().no_votes(), 0);
        assert!(outcome.passes());
    }

    #[test]
    fn outcome_counts_abstentions_separately() {
        let votes = vec![
            vote("t", "a", VoteDecision::Yes),
            vote("t", "b", VoteDecision::Abstain),
            vote("t", "c", VoteDecision::Yes),
        ];
        let outcome =
            ConsensusOutcome::evaluate(topic("t"), ConsensusRule::Majority, &votes, eligible(3))
                .unwrap();
        assert_eq!(outcome.tally().yes_votes(), 2);
        assert_eq!(outcome.tally().abstain_votes(), 1);
        assert!(outcome.passes());
    }

    #[test]
    fn eligible_voters_rejects_zero() {
        assert_eq!(
            EligibleVoters::new(0).unwrap_err(),
            GovernanceError::ZeroEligibleVoters
        );
    }

    #[test]
    fn outcome_rejects_more_votes_than_eligible_voters() {
        let result = ConsensusOutcome::from_tally(
            topic("t"),
            ConsensusRule::Majority,
            VoteTally::new(2, 1, 0),
            eligible(2),
        );
        assert!(matches!(
            result,
            Err(GovernanceError::TalliesExceedEligibleVoters { .. })
        ));
    }

    #[test]
    fn outcome_deserialization_rejects_forged_pass_flag() {
        let json = r#"{
            "topic":"t",
            "rule":"majority",
            "yesVotes":1,
            "noVotes":1,
            "abstainVotes":0,
            "totalVoters":2,
            "passes":true
        }"#;
        let result = serde_json::from_str::<ConsensusOutcome>(json);
        assert!(result.is_err());
    }

    #[test]
    fn outcome_serializes_flat_public_shape() {
        let outcome = ConsensusOutcome::from_tally(
            topic("t"),
            ConsensusRule::Majority,
            VoteTally::new(2, 1, 0),
            eligible(3),
        )
        .unwrap();
        let json = serde_json::to_string(&outcome).unwrap();
        assert_eq!(
            json,
            r#"{"topic":"t","rule":"majority","yesVotes":2,"noVotes":1,"abstainVotes":0,"totalVoters":3,"passes":true}"#
        );
    }

    #[test]
    fn vote_serializes_camel_case_and_skips_empty_reason() {
        let v = Vote {
            topic: topic("done"),
            voter: voter("alice"),
            decision: VoteDecision::Yes,
            reason: None,
        };
        let json = serde_json::to_string(&v).expect("vote should serialize");
        assert_eq!(json, r#"{"topic":"done","voter":"alice","decision":"yes"}"#);
    }

    #[test]
    fn disagreement_roundtrips_through_json() {
        let d = Disagreement {
            topic: topic("plan"),
            dissenter: voter("bob"),
            reason: "scope is too broad".into(),
        };
        let json = serde_json::to_string(&d).unwrap();
        let parsed: Disagreement = serde_json::from_str(&json).unwrap();
        assert_eq!(d, parsed);
    }
}
