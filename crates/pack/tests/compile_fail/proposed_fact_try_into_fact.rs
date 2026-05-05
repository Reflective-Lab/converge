// Contract: There is no TryFrom or From impl from ProposedFact to ContextFact.
// A proposal can never self-promote. The only path to an authoritative ContextFact
// is through the PromotionGate inside converge-core.

use converge_pack::{ContextFact, ContextKey, ProposedFact};

fn main() {
    let proposal = ProposedFact::new(
        ContextKey::Seeds,
        "bypass-id",
        "bypass-content",
        "attacker",
    );
    let _fact: ContextFact = ContextFact::try_from(proposal).unwrap();
}
