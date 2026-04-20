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
            AgentEffect::with_proposal(ProposedFact {
                key: ContextKey::Seeds,
                id: "zero-1".into(),
                content: "zero confidence".into(),
                confidence: 0.0,
                provenance: "zero-conf".into(),
            })
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
async fn confidence_slightly_above_one_rejected() {
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
            AgentEffect::with_proposal(ProposedFact {
                key: ContextKey::Seeds,
                id: "over-1".into(),
                content: "over confidence".into(),
                confidence: 1.0001,
                provenance: "over-conf".into(),
            })
        }
    }
    let mut engine = Engine::new();
    engine.register_suggestor(OverConfSuggestor);
    let result = engine
        .run(ContextState::new())
        .await
        .expect("converges with rejection");
    assert!(!result.context.has(ContextKey::Seeds));
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
            AgentEffect::with_proposal(ProposedFact {
                key: ContextKey::Seeds,
                id: String::new(),
                content: "has empty id".into(),
                confidence: 0.8,
                provenance: "empty-id".into(),
            })
        }
    }
    let mut engine = Engine::new();
    engine.register_suggestor(EmptyIdSuggestor);
    // Should still converge — empty ID is valid at the pack level
    let result = engine.run(ContextState::new()).await.expect("converges");
    assert!(result.converged);
}
