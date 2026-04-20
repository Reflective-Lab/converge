//! Formation: Mixed Domain
//!
//! Demonstrates how upper layers (Organism, Helms) assemble formations from
//! heterogeneous Suggestors across multiple Converge domains:
//!
//! - Optimization solver (budget allocation)
//! - Policy gate (spending limits)
//! - Custom LLM-style reasoning agent (stub)
//!
//! All converge in ONE Engine run. Same contract, same governance.

use converge_kernel::{
    AgentEffect, Budget, Context, ContextKey, ContextState, Engine, ProposedFact, Suggestor,
};
use converge_optimization::packs::budget_allocation::BudgetAllocationPack;
use converge_optimization::suggestor::SolverSuggestor;
use converge_policy::{engine::PolicyEngine, suggestor::PolicyGateSuggestor};
use std::sync::Arc;

// ── Seed Agent ────────────────────────────────────────────────────────
// In real usage, Organism seeds the context from the IntentPacket.

struct IntentSeeder;

#[async_trait::async_trait]
impl Suggestor for IntentSeeder {
    fn name(&self) -> &str {
        "intent-seeder"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }
    fn accepts(&self, ctx: &dyn Context) -> bool {
        !ctx.has(ContextKey::Seeds)
    }
    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        // Seed: "Allocate $1M across 4 departments"
        let problem = serde_json::json!({
            "total_budget": 1_000_000,
            "categories": [
                {"name": "Engineering", "min": 200_000, "max": 500_000, "priority": 0.9},
                {"name": "Marketing", "min": 100_000, "max": 300_000, "priority": 0.7},
                {"name": "Sales", "min": 150_000, "max": 350_000, "priority": 0.8},
                {"name": "Operations", "min": 50_000, "max": 200_000, "priority": 0.5}
            ]
        });
        AgentEffect::with_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "budget-intent",
            problem.to_string(),
            "organism",
        ))
    }
}

// ── LLM Reasoning Agent (Stub) ───────────────────────────────────────
// In real usage, this calls Claude/GPT to reason about the allocation.

struct ReasoningAgent;

#[async_trait::async_trait]
impl Suggestor for ReasoningAgent {
    fn name(&self) -> &str {
        "llm-reasoning"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }
    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }
    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        // In production: send strategies to LLM for evaluation
        // Here: approve if any strategy exists
        if let Some(strategy) = strategies.first() {
            let evaluation = serde_json::json!({
                "strategy_id": strategy.id,
                "assessment": "allocation meets priority ordering",
                "confidence": 0.85,
                "recommendation": "proceed"
            });
            AgentEffect::with_proposal(ProposedFact::new(
                ContextKey::Evaluations,
                "llm-eval-1",
                evaluation.to_string(),
                "llm-reasoning",
            ))
        } else {
            AgentEffect::empty()
        }
    }
}

#[tokio::main]
async fn main() {
    println!("=== Formation: Mixed Domain ===\n");
    println!("Agents: Solver + Policy Gate + LLM Reasoning\n");

    let mut engine = Engine::with_budget(Budget {
        max_cycles: 10,
        max_facts: 1000,
    });

    // 1. Seed agent (would be Organism in production)
    engine.register_suggestor(IntentSeeder);

    // 2. Optimization solver — finds the allocation
    engine.register_suggestor(SolverSuggestor::new(
        BudgetAllocationPack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    ));

    // 3. Policy gate — enforces spending limits
    let policy = PolicyEngine::from_policy_str(
        r#"permit(principal, action == Action::"allocate", resource)
           when { resource.amount <= 500000 };"#,
    )
    .expect("policy should parse");
    engine.register_suggestor(PolicyGateSuggestor::with_keys(
        Arc::new(policy),
        ContextKey::Strategies,
        ContextKey::Constraints,
    ));

    // 4. LLM reasoning — evaluates the allocation
    engine.register_suggestor(ReasoningAgent);

    // Run convergence
    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge");

    println!(
        "Converged: {} (cycles: {})",
        result.converged, result.cycles
    );
    println!("Stop:      {:?}\n", result.stop_reason);

    // Show results
    println!("Seeds:");
    for fact in result.context.get(ContextKey::Seeds) {
        println!(
            "  {} ({})",
            fact.id,
            fact.content.chars().take(60).collect::<String>()
        );
    }

    println!("\nStrategies (solver output):");
    for fact in result.context.get(ContextKey::Strategies) {
        println!(
            "  {} ({})",
            fact.id,
            fact.content.chars().take(80).collect::<String>()
        );
    }

    println!("\nEvaluations (LLM output):");
    for fact in result.context.get(ContextKey::Evaluations) {
        println!(
            "  {} ({})",
            fact.id,
            fact.content.chars().take(80).collect::<String>()
        );
    }

    println!("\nConstraints (policy violations):");
    let constraints = result.context.get(ContextKey::Constraints);
    if constraints.is_empty() {
        println!("  (none — all policies passed)");
    } else {
        for fact in constraints {
            println!("  {} ({})", fact.id, fact.content);
        }
    }

    println!("\n=== Done ===");
}
