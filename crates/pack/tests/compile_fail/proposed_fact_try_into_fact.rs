// Contract: There is no TryFrom or From impl from ProposedFact to Fact.
// A proposal can never self-promote. The only path to an authoritative Fact
// is through the PromotionGate inside converge-core.

use converge_pack::{ContextKey, Fact, ProposedFact};

fn main() {
    let proposal = ProposedFact::new(
        ContextKey::Seeds,
        "bypass-id",
        "bypass-content",
        "attacker",
    );
    let _fact: Fact = Fact::try_from(proposal).unwrap();
}
