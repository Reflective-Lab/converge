// Contract: ConsensusOutcome cannot be forged by a downstream pack.
// Callers must evaluate votes through ConsensusOutcome::evaluate() or
// ConsensusOutcome::from_tally(), which recompute the pass flag from the rule.

use converge_pack::{ConsensusOutcome, ConsensusRule, EligibleVoters, VoteTally, VoteTopicId};

fn main() {
    let _outcome = ConsensusOutcome {
        topic: VoteTopicId::new("admission"),
        rule: ConsensusRule::Majority,
        tally: VoteTally::new(0, 1, 0),
        total_voters: EligibleVoters::new(1).unwrap(),
        passes: true,
    };
}
