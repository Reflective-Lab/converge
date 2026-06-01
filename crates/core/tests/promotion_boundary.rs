// Promotion boundary tests.
//
// Prove staged context input is still governed by the engine:
// - valid staged input is promoted before downstream suggestors run
// - invalid staged input is rejected and never becomes visible as a Fact
// - these invariants hold across arbitrary valid and invalid seed inputs

use converge_core::{
    AgentEffect, Context, ContextKey, ContextState, Engine, ProposedFact, Suggestor, TextPayload,
};
use converge_pack::Provenance;
use proptest::prelude::*;

const SEED_DEPENDENCIES: &[ContextKey] = &[ContextKey::Seeds];

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().expect("runtime")
}

struct SeedObserver;

#[async_trait::async_trait]
impl Suggestor for SeedObserver {
    fn name(&self) -> &str {
        "seed-observer"
    }

    fn dependencies(&self) -> &[ContextKey] {
        SEED_DEPENDENCIES
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(ContextKey::Seeds) && !ctx.has(ContextKey::Hypotheses)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let seed = &ctx.get(ContextKey::Seeds)[0];
        AgentEffect::with_proposal(ProposedFact::new(
            ContextKey::Hypotheses,
            format!("observed-{}", seed.id()),
            TextPayload::new(format!("observed {}", seed.text().unwrap_or_default())),
            self.provenance(),
        ))
    }

    fn provenance(&self) -> Provenance {
        Provenance::new("test-suggestor")
    }
}

#[tokio::test]
async fn staged_input_is_promoted_before_downstream_suggestors_run() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedObserver);

    let mut context = ContextState::new();
    assert!(
        context
            .add_input_with_provenance(
                ContextKey::Seeds,
                "seed-1",
                "seed content",
                Provenance::new("external-request"),
            )
            .expect("staging should succeed")
    );

    let result = engine.run(context).await.expect("run should converge");

    assert!(result.converged);
    assert!(!result.context.has_pending_proposals());

    let seeds = result.context.get(ContextKey::Seeds);
    assert_eq!(seeds.len(), 1);
    assert_eq!(seeds[0].id(), "seed-1");
    assert_eq!(seeds[0].text(), Some("seed content"));

    let hypotheses = result.context.get(ContextKey::Hypotheses);
    assert_eq!(hypotheses.len(), 1);
    assert_eq!(hypotheses[0].id(), "observed-seed-1");
    assert_eq!(hypotheses[0].text(), Some("observed seed content"));
}

#[tokio::test]
async fn rejected_staged_input_never_becomes_visible_to_downstream_suggestors() {
    let mut engine = Engine::new();
    engine.register_suggestor(SeedObserver);

    let mut context = ContextState::new();
    assert!(
        context
            .add_input_with_provenance(
                ContextKey::Seeds,
                "seed-1",
                "   \t\n  ",
                Provenance::new("external")
            )
            .expect("staging should succeed")
    );

    let result = engine.run(context).await.expect("run should converge");

    assert!(result.converged);
    assert!(!result.context.has(ContextKey::Seeds));
    assert!(!result.context.has(ContextKey::Hypotheses));
    assert!(!result.context.has_pending_proposals());
}

proptest! {
    #[test]
    fn arbitrary_valid_staged_inputs_roundtrip_through_promotion(
        id in "[a-z0-9][a-z0-9-]{0,15}",
        content in "[A-Za-z0-9][A-Za-z0-9 _-]{0,31}",
        provenance in "[a-z][a-z0-9-]{2,15}",
    ) {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedObserver);

        let mut context = ContextState::new();
        context
            .add_input_with_provenance(
                ContextKey::Seeds,
                id.clone(),
                content.clone(),
                Provenance::new(provenance.clone()),
            )
            .expect("staging should succeed");

        let result = rt().block_on(engine.run(context)).expect("run should converge");
        let seeds = result.context.get(ContextKey::Seeds);
        let hypotheses = result.context.get(ContextKey::Hypotheses);

        prop_assert_eq!(seeds.len(), 1);
        prop_assert_eq!(seeds[0].id().as_str(), id.as_str());
        prop_assert_eq!(seeds[0].text(), Some(content.as_str()));
        prop_assert_eq!(hypotheses.len(), 1);
        let expected_hypothesis = format!("observed {content}");
        prop_assert_eq!(hypotheses[0].text(), Some(expected_hypothesis.as_str()));
        prop_assert!(!result.context.has_pending_proposals());
    }

    #[test]
    fn whitespace_only_staged_inputs_are_never_promoted(
        id in "[a-z0-9][a-z0-9-]{0,15}",
        content in "[ \\t\\n]{1,12}",
        provenance in "[a-z][a-z0-9-]{2,15}",
    ) {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedObserver);

        let mut context = ContextState::new();
        context
            .add_input_with_provenance(
                ContextKey::Seeds,
                id,
                content,
                Provenance::new(provenance.clone()),
            )
            .expect("staging should succeed");

        let result = rt().block_on(engine.run(context)).expect("run should converge");

        prop_assert!(!result.context.has(ContextKey::Seeds));
        prop_assert!(!result.context.has(ContextKey::Hypotheses));
        prop_assert!(!result.context.has_pending_proposals());
    }
}
