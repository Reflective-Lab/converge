// Contract: AgentEffect is a finalized output value, not the accumulator.
// Incremental construction belongs on AgentEffectBuilder.

use converge_pack::{AgentEffect, ContextKey, ProposedFact, TextPayload};

fn main() {
    let mut effect = AgentEffect::empty();
    effect.proposals.push(ProposedFact::new(
        ContextKey::Seeds,
        "seed-1",
        TextPayload::new("content"),
        "test",
    ));
}
