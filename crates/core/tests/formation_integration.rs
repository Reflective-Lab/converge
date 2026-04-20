//! Integration tests for the formation pattern.
//!
//! Prove that heterogeneous Suggestors from different domains converge
//! correctly in a single Engine run — the pattern used by Organism/Helms.

use async_trait::async_trait;
use converge_core::suggestors::{ReactOnceSuggestor, SeedSuggestor};
use converge_core::{AgentEffect, Budget, ContextState, Engine, ProposedFact};
use converge_pack::{Context, ContextKey, Suggestor};

// ── Helpers ───────────────────────────────────────────────────────────

/// A policy-like agent that blocks strategies with confidence > threshold.
struct ThresholdPolicyAgent {
    max_confidence: f64,
}

#[async_trait]
impl Suggestor for ThresholdPolicyAgent {
    fn name(&self) -> &str {
        "threshold-policy"
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies]
    }
    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Constraints)
    }
    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let mut proposals = Vec::new();
        for fact in strategies {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&fact.content) {
                if let Some(conf) = v.get("confidence").and_then(|c| c.as_f64()) {
                    if conf > self.max_confidence {
                        proposals.push(ProposedFact::new(
                            ContextKey::Constraints,
                            format!("block-{}", fact.id),
                            format!("confidence {conf} exceeds max {}", self.max_confidence),
                            "threshold-policy",
                        ));
                    }
                }
            }
        }
        AgentEffect::with_proposals(proposals)
    }
}

/// A strategy-proposing agent (simulates what an optimizer or LLM would do).
struct StrategyAgent {
    name: &'static str,
    fact_id: &'static str,
    content: &'static str,
}

#[async_trait]
impl Suggestor for StrategyAgent {
    fn name(&self) -> &str {
        self.name
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }
    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Strategies)
                .iter()
                .any(|f| f.id == self.fact_id)
    }
    async fn execute(&self, _ctx: &dyn Context) -> AgentEffect {
        AgentEffect::with_proposal(ProposedFact::new(
            ContextKey::Strategies,
            self.fact_id,
            self.content,
            self.name,
        ))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[tokio::test]
async fn formation_seed_and_react_converges() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("seed-1", "initial data"));
    engine.register_suggestor(ReactOnceSuggestor::new("hyp-1", "derived hypothesis"));

    let result = engine.run(ContextState::new()).await.unwrap();
    assert!(result.converged);
    assert!(result.context.has(ContextKey::Seeds));
    assert!(result.context.has(ContextKey::Hypotheses));
}

#[tokio::test]
async fn formation_policy_blocks_high_confidence() {
    let mut engine = Engine::new();

    engine.register_suggestor(SeedSuggestor::new("intent-1", "plan the budget"));
    engine.register_suggestor(StrategyAgent {
        name: "aggressive-planner",
        fact_id: "strategy-1",
        content: r#"{"confidence": 0.99, "plan": "spend everything"}"#,
    });
    engine.register_suggestor(ThresholdPolicyAgent {
        max_confidence: 0.9,
    });

    let result = engine.run(ContextState::new()).await.unwrap();
    assert!(result.converged);
    // Policy writes a constraint blocking the overconfident strategy
    assert!(result.context.has(ContextKey::Constraints));
    let constraints = result.context.get(ContextKey::Constraints);
    assert!(constraints[0].content.contains("0.99"));
}

#[tokio::test]
async fn formation_policy_allows_reasonable_confidence() {
    let mut engine = Engine::new();

    engine.register_suggestor(SeedSuggestor::new("intent-1", "plan the budget"));
    engine.register_suggestor(StrategyAgent {
        name: "conservative-planner",
        fact_id: "strategy-1",
        content: r#"{"confidence": 0.75, "plan": "balanced allocation"}"#,
    });
    engine.register_suggestor(ThresholdPolicyAgent {
        max_confidence: 0.9,
    });

    let result = engine.run(ContextState::new()).await.unwrap();
    assert!(result.converged);
    // No constraints — confidence is within threshold
    assert!(!result.context.has(ContextKey::Constraints));
    // Strategy was proposed and promoted
    assert!(result.context.has(ContextKey::Strategies));
}

#[tokio::test]
async fn formation_respects_budget() {
    let mut engine = Engine::with_budget(Budget {
        max_cycles: 2,
        max_facts: 100,
    });
    engine.register_suggestor(SeedSuggestor::new("s1", "data"));

    let result = engine.run(ContextState::new()).await.unwrap();
    assert!(result.converged);
    assert!(result.cycles <= 2);
}

#[tokio::test]
async fn formation_multiple_strategies_partial_block() {
    let mut engine = Engine::new();

    engine.register_suggestor(SeedSuggestor::new("intent", "multi-strategy"));

    // Two planners with different confidence
    engine.register_suggestor(StrategyAgent {
        name: "planner-aggressive",
        fact_id: "aggressive",
        content: r#"{"confidence": 0.95, "plan": "aggressive growth"}"#,
    });
    engine.register_suggestor(StrategyAgent {
        name: "planner-conservative",
        fact_id: "conservative",
        content: r#"{"confidence": 0.7, "plan": "steady growth"}"#,
    });

    engine.register_suggestor(ThresholdPolicyAgent {
        max_confidence: 0.9,
    });

    let result = engine.run(ContextState::new()).await.unwrap();
    assert!(result.converged);
    // Both strategies written
    assert_eq!(result.context.get(ContextKey::Strategies).len(), 2);
    // Only the aggressive one is blocked
    let constraints = result.context.get(ContextKey::Constraints);
    assert_eq!(constraints.len(), 1);
    assert!(constraints[0].id.contains("aggressive"));
}
