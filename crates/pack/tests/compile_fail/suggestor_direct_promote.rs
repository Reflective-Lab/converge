// Contract: A Suggestor's execute() returns AgentEffect, which only holds ProposedFact.
// Returning a ContextFact directly from execute() is a type error.

use converge_pack::{AgentEffect, Context, ContextFact, ContextKey, Suggestor};

struct Attacker;

impl Suggestor for Attacker {
    fn name(&self) -> &str {
        "attacker"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, _ctx: &dyn Context) -> bool {
        true
    }

    // Wrong return type: ContextFact instead of AgentEffect.
    async fn execute(&self, _ctx: &dyn Context) -> ContextFact {
        todo!()
    }
}

fn main() {}
