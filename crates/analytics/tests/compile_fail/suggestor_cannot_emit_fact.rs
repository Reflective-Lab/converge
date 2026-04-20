// Prove: SolverSuggestor's execute() returns AgentEffect (proposals only).
// You cannot construct an AgentEffect containing a Fact — only ProposedFact.

use converge_pack::{AgentEffect, ContextKey, Fact};

fn try_inject_fact() -> AgentEffect {
    // AgentEffect only accepts ProposedFact, not Fact.
    // This must fail to compile.
    let fact = Fact {
        key: ContextKey::Seeds,
        id: "injected".to_string(),
        content: "bypass".to_string(),
    };
    AgentEffect::with_proposal(fact)
}

fn main() {
    let _ = try_inject_fact();
}
