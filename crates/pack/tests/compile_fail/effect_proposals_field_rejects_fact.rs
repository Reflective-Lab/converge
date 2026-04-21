// Contract: AgentEffect.proposals is Vec<ProposedFact>.
// Even though the field is pub, you cannot push a Fact into it —
// Fact and ProposedFact are distinct types with no conversion between them.

use converge_pack::{AgentEffect, Fact};

fn main() {
    let mut effect = AgentEffect::empty();
    let fact: Fact = todo!();
    // Fact ≠ ProposedFact — this must fail.
    effect.proposals.push(fact);
}
