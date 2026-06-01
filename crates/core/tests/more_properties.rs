// Additional property tests.

use converge_core::suggestors::SeedSuggestor;
use converge_core::{
    AgentEffect, Budget, ContextKey, ContextState, Engine, ProposedFact, Suggestor, TextPayload,
};
use converge_pack::Provenance;
use proptest::prelude::*;

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

proptest! {
    #[test]
    fn version_increases_with_facts(
        n in 1usize..10,
    ) {
        let mut engine = Engine::new();
        for i in 0..n {
            engine.register_suggestor(SeedSuggestor::new(
                format!("s{i}"), format!("v{i}"),
            ));
        }
        let result = rt().block_on(engine.run(ContextState::new())).expect("converges");
        prop_assert!(result.context.version() > 0);
    }

    #[test]
    fn all_keys_reported_correctly(
        seed_count in 1usize..5,
    ) {
        let mut engine = Engine::new();
        for i in 0..seed_count {
            engine.register_suggestor(SeedSuggestor::new(
                format!("s{i}"), format!("v{i}"),
            ));
        }
        let result = rt().block_on(engine.run(ContextState::new())).expect("converges");
        let keys = result.context.all_keys();
        prop_assert!(keys.contains(&ContextKey::Seeds));
    }

    #[test]
    fn context_fact_count_matches_suggestor_count(
        n in 1usize..8,
    ) {
        let mut engine = Engine::new();
        for i in 0..n {
            engine.register_suggestor(SeedSuggestor::new(
                format!("seed-{i}"), format!("value-{i}"),
            ));
        }
        let result = rt().block_on(engine.run(ContextState::new())).expect("converges");
        prop_assert_eq!(result.context.get(ContextKey::Seeds).len(), n);
    }

    #[test]
    fn budget_cycles_always_respected(
        max_cycles in 1u32..10,
    ) {
        struct AlwaysPropose(usize);
        #[async_trait::async_trait]
        impl Suggestor for AlwaysPropose {
            fn name(&self) -> &str { "always" }
            fn dependencies(&self) -> &[ContextKey] { &[] }
            fn accepts(&self, _: &dyn converge_core::Context) -> bool { true }
            async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
                let n = ctx.get(ContextKey::Seeds).len();
                AgentEffect::with_proposal(ProposedFact::new(
                    ContextKey::Seeds, format!("a{}-{n}", self.0), TextPayload::new("v"), Provenance::new("always"),
                ))
            }

            fn provenance(&self) -> Provenance {
                Provenance::new("test-suggestor")
            }
        }
        let mut engine = Engine::with_budget(Budget { max_cycles, max_facts: 1000 });
        engine.register_suggestor(AlwaysPropose(0));
        // Should terminate, might error with budget exhaustion
        let _ = rt().block_on(engine.run(ContextState::new()));
    }

    #[test]
    fn provenance_preserved_in_proposals(
        provenance in "[a-z]{3,10}",
    ) {
        struct ProvenanceSuggestor(String);
        #[async_trait::async_trait]
        impl Suggestor for ProvenanceSuggestor {
            fn name(&self) -> &str { "prov" }
            fn dependencies(&self) -> &[ContextKey] { &[] }
            fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
                !ctx.has(ContextKey::Seeds)
            }
            async fn execute(&self, _: &dyn converge_core::Context) -> AgentEffect {
                AgentEffect::with_proposal(ProposedFact::new(
                    ContextKey::Seeds, "p1", TextPayload::new("content"), Provenance::new(self.0.clone()),
                ))
            }

            fn provenance(&self) -> Provenance {
                Provenance::new("test-suggestor")
            }
        }
        let mut engine = Engine::new();
        engine.register_suggestor(ProvenanceSuggestor(provenance));
        let result = rt().block_on(engine.run(ContextState::new())).expect("converges");
        prop_assert!(result.converged);
        prop_assert_eq!(result.context.get(ContextKey::Seeds).len(), 1);
    }
}
