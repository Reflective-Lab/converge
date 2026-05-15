// Truth pipeline integration tests.
//
// Prove: observation → proposal → promotion → fact is the only path.
// Prove: the engine promotes through the gate, not directly.

use converge_core::suggestors::{ReactOnceSuggestor, SeedSuggestor};
use converge_core::{
    AgentEffect, ContextKey, ContextState, Engine, ProposedFact, Suggestor, TextPayload,
};

// ── End-to-end: suggestor → proposal → engine promotion → fact ──

#[tokio::test]
async fn single_suggestor_proposal_promoted_to_fact() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("seed-1", "initial observation"));

    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge");

    assert!(result.converged);
    let facts = result.context.get(ContextKey::Seeds);
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].id(), "seed-1");
    assert_eq!(facts[0].text(), Some("initial observation"));
}

#[tokio::test]
async fn multi_suggestor_cascade_converges() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("s1", "seed value"));
    engine.register_suggestor(ReactOnceSuggestor::new("h1", "derived hypothesis"));

    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge");

    assert!(result.converged);
    assert!(result.context.has(ContextKey::Seeds));
    assert!(result.context.has(ContextKey::Hypotheses));
    assert_eq!(result.context.get(ContextKey::Seeds).len(), 1);
    assert_eq!(result.context.get(ContextKey::Hypotheses).len(), 1);
}

#[tokio::test]
async fn three_stage_pipeline_converges() {
    struct StrategyFromHypothesis;

    #[async_trait::async_trait]
    impl Suggestor for StrategyFromHypothesis {
        fn name(&self) -> &str {
            "StrategyFromHypothesis"
        }
        fn dependencies(&self) -> &[ContextKey] {
            &[ContextKey::Hypotheses]
        }
        fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
            ctx.has(ContextKey::Hypotheses) && !ctx.has(ContextKey::Strategies)
        }
        async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
            AgentEffect::with_proposal(ProposedFact::new(
                ContextKey::Strategies,
                "strategy-1",
                TextPayload::new("recommended action"),
                self.name().to_string(),
            ))
        }

        fn provenance(&self) -> &'static str {
            "test-suggestor"
        }
    }

    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("s1", "evidence"));
    engine.register_suggestor(ReactOnceSuggestor::new("h1", "hypothesis"));
    engine.register_suggestor(StrategyFromHypothesis);

    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge");

    assert!(result.converged);
    assert!(result.context.has(ContextKey::Seeds));
    assert!(result.context.has(ContextKey::Hypotheses));
    assert!(result.context.has(ContextKey::Strategies));
}

// ── Seed input path: context.add_input() → engine promotes ──

#[tokio::test]
async fn seed_input_promoted_through_gate() {
    let mut engine = Engine::new();
    // No suggestors — just seed input
    let mut ctx = ContextState::new();
    ctx.add_input_with_provenance(
        ContextKey::Seeds,
        "external-1",
        "external observation",
        "test-harness",
    )
    .expect("add_input should succeed");

    let result = engine.run(ctx).await.expect("should converge");

    assert!(result.converged);
    let facts = result.context.get(ContextKey::Seeds);
    assert_eq!(facts.len(), 1);
    assert_eq!(facts[0].id(), "external-1");
}

// ── Idempotency: same suggestor does not re-propose ──

#[tokio::test]
async fn suggestor_does_not_re_propose_existing_fact() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("s1", "value"));

    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge");

    assert!(result.converged);
    // Should have exactly 1 fact, not 2
    assert_eq!(result.context.get(ContextKey::Seeds).len(), 1);
}
