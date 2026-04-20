// Contract: A Suggestor's execute() returns AgentEffect, which only holds ProposedFact.
// Returning a Fact directly from execute() is a type error.

use converge_pack::{AgentEffect, Context, ContextKey, Fact, Suggestor};

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

    // Wrong return type: Fact instead of AgentEffect.
    async fn execute(&self, _ctx: &dyn Context) -> Fact {
        todo!()
    }
}

fn main() {}
