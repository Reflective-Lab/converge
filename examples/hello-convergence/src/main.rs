// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Hello Convergence — minimal example of the convergence engine.
//!
//! Shows: Engine, agents, context, facts, and the convergence loop.

use converge_kernel::{
    AgentEffect, Context, ContextKey, ContextState, Engine, ProposedFact, Suggestor,
};

const NO_DEPENDENCIES: [ContextKey; 0] = [];
const SEED_DEPENDENCIES: [ContextKey; 1] = [ContextKey::Seeds];

struct SeedOnceSuggestor {
    name: &'static str,
    id: &'static str,
    content: &'static str,
}

impl SeedOnceSuggestor {
    fn new(name: &'static str, id: &'static str, content: &'static str) -> Self {
        Self { name, id, content }
    }
}

#[async_trait::async_trait]
impl Suggestor for SeedOnceSuggestor {
    fn name(&self) -> &str {
        self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &NO_DEPENDENCIES
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        !ctx.has(ContextKey::Seeds)
    }

    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        AgentEffect::with_proposal(
            ProposedFact::new(ContextKey::Seeds, self.id, self.content, self.name)
                .with_confidence(1.0),
        )
    }
}

struct ReactOnceSuggestor {
    name: &'static str,
    id: &'static str,
    content: &'static str,
}

impl ReactOnceSuggestor {
    fn new(name: &'static str, id: &'static str, content: &'static str) -> Self {
        Self { name, id, content }
    }
}

#[async_trait::async_trait]
impl Suggestor for ReactOnceSuggestor {
    fn name(&self) -> &str {
        self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &SEED_DEPENDENCIES
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Hypotheses)
    }

    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        AgentEffect::with_proposal(
            ProposedFact::new(ContextKey::Hypotheses, self.id, self.content, self.name)
                .with_confidence(0.95),
        )
    }
}

#[tokio::main]
async fn main() {
    println!("=== Hello Convergence ===\n");

    // 1. Create an engine
    let mut engine = Engine::new();

    // 2. Register agents
    //    SeedSuggestor:      writes a fact once, then goes idle
    //    ReactOnceSuggestor: waits for Seeds, then writes Hypotheses once
    engine.register_suggestor(SeedOnceSuggestor::new(
        "seed-suggestor",
        "seed-1",
        "Monthly active users grew 15%",
    ));
    engine.register_suggestor(ReactOnceSuggestor::new(
        "react-once-suggestor",
        "hypothesis-1",
        "Growth driven by new onboarding flow",
    ));

    // 3. Run until convergence (fixed point)
    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge");

    // 4. Inspect the outcome
    println!("Converged: {}", result.converged);
    println!("Cycles:    {}", result.cycles);
    println!("Stop:      {:?}", result.stop_reason);
    println!(
        "Integrity: {} facts, clock={}, merkle={}...\n",
        result.integrity.fact_count,
        result.integrity.clock_time,
        &result.integrity.merkle_root.to_hex()[..16]
    );

    println!("Seeds:");
    for fact in result.context.get(ContextKey::Seeds) {
        println!("  [{:?}] {}: {}", fact.key(), fact.id, fact.content);
    }

    println!("\nHypotheses:");
    for fact in result.context.get(ContextKey::Hypotheses) {
        println!("  [{:?}] {}: {}", fact.key(), fact.id, fact.content);
    }

    println!("\n=== Done ===");
}
