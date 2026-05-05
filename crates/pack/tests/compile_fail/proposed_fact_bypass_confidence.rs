// Contract: ProposedFact cannot self-promote to a ContextFact.
// Even with confidence set to any value, there is no path from ProposedFact → ContextFact
// without kernel authority. The type system makes bypass impossible.

use converge_pack::{ContextFact, ContextKey, ProposedFact};

fn main() {
    let proposal = ProposedFact::new(
        ContextKey::Seeds,
        "bypass-id",
        "bypass-content",
        "attacker",
    );

    // Attempt to convert ProposedFact into ContextFact — no such conversion exists.
    let _fact: ContextFact = proposal.into();
}
