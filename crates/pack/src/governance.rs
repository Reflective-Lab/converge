// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Governance primitives shared across packs.
//!
//! These are pure data types: a rule for tallying votes, a vote payload, a
//! disagreement payload, and the deterministic outcome of evaluating votes
//! against a rule. They carry no scheduling, round, or pipeline semantics —
//! consumers (huddles, approval flows, multi-agent panels) compose them.

use serde::{Deserialize, Serialize};

use crate::types::{ActorId, VoteTopicId};

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
    pub fn passes(self, yes_votes: usize, total_voters: usize) -> bool {
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

/// Deterministic result of evaluating votes against a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsensusOutcome {
    pub topic: VoteTopicId,
    pub rule: ConsensusRule,
    pub yes_votes: usize,
    pub no_votes: usize,
    pub abstain_votes: usize,
    pub total_voters: usize,
    pub passes: bool,
}

impl ConsensusOutcome {
    /// Tally `votes` against `rule` for `total_voters` eligible participants.
    ///
    /// Votes whose `topic` does not match are ignored. Each `voter` is counted
    /// at most once: when an actor appears multiple times, the latest entry
    /// wins (callers control ordering).
    #[must_use]
    pub fn evaluate(
        topic: VoteTopicId,
        rule: ConsensusRule,
        votes: &[Vote],
        total_voters: usize,
    ) -> Self {
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

        Self {
            topic,
            rule,
            yes_votes,
            no_votes,
            abstain_votes,
            total_voters,
            passes: rule.passes(yes_votes, total_voters),
        }
    }
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
        assert!(ConsensusRule::Majority.passes(3, 4));
        assert!(!ConsensusRule::Majority.passes(2, 4));
        assert!(ConsensusRule::Supermajority.passes(2, 3));
        assert!(!ConsensusRule::Supermajority.passes(1, 3));
        assert!(ConsensusRule::Unanimous.passes(5, 5));
        assert!(!ConsensusRule::Unanimous.passes(4, 5));
        assert!(ConsensusRule::LeadDecides.passes(1, 9));
        assert!(ConsensusRule::AdvisoryOnly.passes(0, 10));
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
        let outcome = ConsensusOutcome::evaluate(topic("t1"), ConsensusRule::Majority, &votes, 2);
        assert_eq!(outcome.yes_votes, 1);
        assert_eq!(outcome.no_votes, 1);
        assert_eq!(outcome.total_voters, 2);
        assert!(!outcome.passes);
    }

    #[test]
    fn outcome_collapses_repeat_votes_per_actor_to_latest() {
        let votes = vec![
            vote("t1", "alice", VoteDecision::No),
            vote("t1", "alice", VoteDecision::Yes),
            vote("t1", "bob", VoteDecision::Yes),
        ];
        let outcome = ConsensusOutcome::evaluate(topic("t1"), ConsensusRule::Unanimous, &votes, 2);
        assert_eq!(outcome.yes_votes, 2);
        assert_eq!(outcome.no_votes, 0);
        assert!(outcome.passes);
    }

    #[test]
    fn outcome_counts_abstentions_separately() {
        let votes = vec![
            vote("t", "a", VoteDecision::Yes),
            vote("t", "b", VoteDecision::Abstain),
            vote("t", "c", VoteDecision::Yes),
        ];
        let outcome = ConsensusOutcome::evaluate(topic("t"), ConsensusRule::Majority, &votes, 3);
        assert_eq!(outcome.yes_votes, 2);
        assert_eq!(outcome.abstain_votes, 1);
        assert!(outcome.passes);
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
