// Additional happy-path tests for the core engine.

use converge_core::suggestors::{ReactOnceSuggestor, SeedSuggestor};
use converge_core::{
    AgentEffect, Context, ContextKey, ContextState, Engine, ProposedFact, Suggestor,
};

#[tokio::test]
async fn five_seeds_all_converge() {
    let mut engine = Engine::new();
    for i in 0..5 {
        engine.register_suggestor(SeedSuggestor::new(format!("s{i}"), format!("value-{i}")));
    }
    let result = engine.run(ContextState::new()).await.expect("converges");
    assert!(result.converged);
    assert_eq!(result.context.get(ContextKey::Seeds).len(), 5);
}

#[tokio::test]
async fn seed_with_high_confidence_promoted() {
    struct HighConfidenceSuggestor;
    #[async_trait::async_trait]
    impl Suggestor for HighConfidenceSuggestor {
        fn name(&self) -> &str {
            "high-conf"
        }
        fn dependencies(&self) -> &[ContextKey] {
            &[]
        }
        fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
            !ctx.has(ContextKey::Seeds)
        }
        async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
            AgentEffect::with_proposal(ProposedFact {
                key: ContextKey::Seeds,
                id: "high-conf-1".into(),
                content: "high confidence fact".into(),
                confidence: 1.0,
                provenance: "high-conf-suggestor".into(),
            })
        }
    }
    let mut engine = Engine::new();
    engine.register_suggestor(HighConfidenceSuggestor);
    let result = engine.run(ContextState::new()).await.expect("converges");
    assert!(result.converged);
    assert_eq!(
        result.context.get(ContextKey::Seeds)[0].content,
        "high confidence fact"
    );
}

#[tokio::test]
async fn multiple_context_keys_populated() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("s1", "seed"));
    engine.register_suggestor(ReactOnceSuggestor::new("h1", "hypothesis"));

    struct StrategySuggestor;
    #[async_trait::async_trait]
    impl Suggestor for StrategySuggestor {
        fn name(&self) -> &str {
            "strategy"
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
                "strat-1",
                "go forward",
                self.name(),
            ))
        }
    }

    engine.register_suggestor(StrategySuggestor);
    let result = engine.run(ContextState::new()).await.expect("converges");
    assert!(result.converged);
    assert!(result.context.has(ContextKey::Seeds));
    assert!(result.context.has(ContextKey::Hypotheses));
    assert!(result.context.has(ContextKey::Strategies));
}

#[test]
fn empty_context_version_is_zero() {
    let ctx = ContextState::new();
    assert_eq!(ctx.version(), 0);
}

#[tokio::test]
async fn converged_result_reports_correct_cycle_count() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("s1", "v"));
    engine.register_suggestor(ReactOnceSuggestor::new("h1", "v"));
    let result = engine.run(ContextState::new()).await.expect("converges");
    assert!(result.converged);
    // Seed on cycle 1, react on cycle 2, convergence on cycle 3
    assert!(result.cycles >= 2);
}
