// Additional negative tests for edge cases.

use converge_core::{AgentEffect, ContextKey, ContextState, Engine, ProposedFact, Suggestor};

#[tokio::test]
async fn confidence_exactly_zero_accepted() {
    struct ZeroConfSuggestor;
    #[async_trait::async_trait]
    impl Suggestor for ZeroConfSuggestor {
        fn name(&self) -> &str {
            "zero-conf"
        }
        fn dependencies(&self) -> &[ContextKey] {
            &[]
        }
        fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
            !ctx.has(ContextKey::Seeds)
        }
        async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
            AgentEffect::with_proposal(
                ProposedFact::new(ContextKey::Seeds, "zero-1", "zero confidence", "zero-conf")
                    .with_confidence(0.0),
            )
        }
    }
    let mut engine = Engine::new();
    engine.register_suggestor(ZeroConfSuggestor);
    let result = engine.run(ContextState::new()).await.expect("converges");
    assert!(result.converged);
    assert!(result.context.has(ContextKey::Seeds));
}

#[tokio::test]
async fn confidence_exactly_one_accepted() {
    let mut engine = Engine::new();
    engine.register_suggestor(converge_core::suggestors::SeedSuggestor::new("s1", "v"));
    let result = engine.run(ContextState::new()).await.expect("converges");
    assert!(result.converged);
}

#[tokio::test]
async fn confidence_slightly_above_one_clamped_and_accepted() {
    // with_confidence clamps 1.0001 → 1.0 at construction time, so the
    // proposal is valid and should be promoted.
    struct OverConfSuggestor;
    #[async_trait::async_trait]
    impl Suggestor for OverConfSuggestor {
        fn name(&self) -> &str {
            "over-conf"
        }
        fn dependencies(&self) -> &[ContextKey] {
            &[]
        }
        fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
            true
        }
        async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
            AgentEffect::with_proposal(
                ProposedFact::new(ContextKey::Seeds, "over-1", "over confidence", "over-conf")
                    .with_confidence(1.0001),
            )
        }
    }
    let mut engine = Engine::new();
    engine.register_suggestor(OverConfSuggestor);
    let result = engine.run(ContextState::new()).await.expect("converges");
    assert!(result.context.has(ContextKey::Seeds));
}

#[tokio::test]
async fn suggestor_that_never_accepts_produces_no_facts() {
    struct NeverAccepts;
    #[async_trait::async_trait]
    impl Suggestor for NeverAccepts {
        fn name(&self) -> &str {
            "never"
        }
        fn dependencies(&self) -> &[ContextKey] {
            &[]
        }
        fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
            false
        }
        async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
            panic!("should never be called")
        }
    }
    let mut engine = Engine::new();
    engine.register_suggestor(NeverAccepts);
    let result = engine.run(ContextState::new()).await.expect("converges");
    assert!(result.converged);
}

#[tokio::test]
async fn empty_proposal_id_still_works() {
    struct EmptyIdSuggestor;
    #[async_trait::async_trait]
    impl Suggestor for EmptyIdSuggestor {
        fn name(&self) -> &str {
            "empty-id"
        }
        fn dependencies(&self) -> &[ContextKey] {
            &[]
        }
        fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
            !ctx.has(ContextKey::Seeds)
        }
        async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
            AgentEffect::with_proposal(
                ProposedFact::new(ContextKey::Seeds, "", "has empty id", "empty-id")
                    .with_confidence(0.8),
            )
        }
    }
    let mut engine = Engine::new();
    engine.register_suggestor(EmptyIdSuggestor);
    // Should still converge — empty ID is valid at the pack level
    let result = engine.run(ContextState::new()).await.expect("converges");
    assert!(result.converged);
}
