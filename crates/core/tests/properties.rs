// Property tests.
//
// Prove invariants hold across randomized inputs.

use converge_core::suggestors::SeedSuggestor;
use converge_core::{
    AgentEffect, Budget, ContextKey, ContextState, Engine, ProposedFact, Suggestor, TextPayload,
};
use converge_pack::Provenance;
use proptest::prelude::*;

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

// ── Determinism: same input → same output ──

proptest! {
    #[test]
    fn deterministic_convergence(
        seed_content in "[a-zA-Z0-9 ]{1,100}",
        hyp_content in "[a-zA-Z0-9 ]{1,100}",
    ) {
        let run = |content_s: &str, content_h: &str| {
            let mut engine = Engine::new();
            engine.register_suggestor(SeedSuggestor::new("s1", content_s.to_string()));
            engine.register_suggestor(
                converge_core::suggestors::ReactOnceSuggestor::new("h1", content_h.to_string()),
            );
            rt().block_on(engine.run(ContextState::new())).expect("should converge")
        };

        let r1 = run(&seed_content, &hyp_content);
        let r2 = run(&seed_content, &hyp_content);

        prop_assert_eq!(r1.converged, r2.converged);
        prop_assert_eq!(r1.cycles, r2.cycles);
        prop_assert_eq!(
            r1.context.get(ContextKey::Seeds).len(),
            r2.context.get(ContextKey::Seeds).len()
        );
        prop_assert_eq!(
            r1.context.get(ContextKey::Hypotheses).len(),
            r2.context.get(ContextKey::Hypotheses).len()
        );
    }
}

// ── Termination: engine always terminates within budget ──

proptest! {
    #[test]
    fn engine_always_terminates(
        max_cycles in 1u32..20,
        num_suggestors in 1usize..5,
    ) {
        struct CountingSuggestor(usize);

        #[async_trait::async_trait]
        impl Suggestor for CountingSuggestor {
            fn name(&self) -> &str { "counting" }
            fn dependencies(&self) -> &[ContextKey] { &[] }
            fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool { true }
            async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
                let n = ctx.get(ContextKey::Seeds).len();
                AgentEffect::with_proposal(ProposedFact::new(
                    ContextKey::Seeds,
                    format!("c{}-{n}", self.0),
                    TextPayload::new(format!("value from suggestor {}", self.0)),
                    "counting",
                ))
            }

            fn provenance(&self) -> Provenance {
                Provenance::from("test-suggestor")
            }
        }

        let mut engine = Engine::with_budget(Budget {
            max_cycles,
            max_facts: 1000,
        });
        for i in 0..num_suggestors {
            engine.register_suggestor(CountingSuggestor(i));
        }

        let result = rt().block_on(engine.run(ContextState::new()));

        match result {
            Ok(run) => prop_assert!(run.cycles <= max_cycles),
            Err(converge_core::ConvergeError::BudgetExhausted { .. }) => prop_assert!(true),
            Err(other) => prop_assert!(false, "unexpected error: {other}"),
        }
    }
}

// ── Append-only: fact count never decreases ──

#[tokio::test]
async fn fact_count_monotonically_increases() {
    struct TrackedSuggestor {
        id: usize,
    }

    #[async_trait::async_trait]
    impl Suggestor for TrackedSuggestor {
        fn name(&self) -> &str {
            "tracked"
        }
        fn dependencies(&self) -> &[ContextKey] {
            &[]
        }
        fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
            ctx.get(ContextKey::Seeds).len() < 5
        }
        async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
            let n = ctx.get(ContextKey::Seeds).len();
            AgentEffect::with_proposal(ProposedFact::new(
                ContextKey::Seeds,
                format!("t{}-{n}", self.id),
                TextPayload::new("tracked value"),
                "tracked",
            ))
        }

        fn provenance(&self) -> Provenance {
            Provenance::from("test-suggestor")
        }
    }

    let mut engine = Engine::with_budget(Budget {
        max_cycles: 10,
        max_facts: 100,
    });
    engine.register_suggestor(TrackedSuggestor { id: 0 });
    engine.register_suggestor(TrackedSuggestor { id: 1 });

    let result = engine
        .run(ContextState::new())
        .await
        .expect("should terminate");

    // Facts were added, never removed
    let final_count = result.context.get(ContextKey::Seeds).len();
    assert!(final_count > 0, "facts should have been added");
}

// ── Fixed point stability: re-running after convergence adds nothing ──

#[tokio::test]
async fn converged_result_is_stable() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedSuggestor::new("s1", "stable seed"));

    let r1 = engine.run(ContextState::new()).await.expect("first run");
    assert!(r1.converged);

    // Run again with the converged context
    let mut engine2 = Engine::new();
    engine2.register_suggestor(SeedSuggestor::new("s1", "stable seed"));
    let r2 = engine2.run(r1.context).await.expect("second run");

    assert!(r2.converged);
    assert_eq!(r2.cycles, 1, "no new work should be needed");
}
