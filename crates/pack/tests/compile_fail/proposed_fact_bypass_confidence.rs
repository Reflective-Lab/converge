// Contract: ProposedFact cannot self-promote to a Fact.
// Even with confidence set to any value, there is no path from ProposedFact → Fact
// without kernel authority. The type system makes bypass impossible.

use converge_pack::{ContextKey, Fact, ProposedFact};

fn main() {
    let proposal = ProposedFact::new(
        ContextKey::Seeds,
        "bypass-id",
        "bypass-content",
        "attacker",
    );

    // Attempt to convert ProposedFact into Fact — no such conversion exists.
    let _fact: Fact = proposal.into();
}
