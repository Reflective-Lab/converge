// Negative and edge-case tests.
//
// Prove: the system rejects invalid input and handles edge cases gracefully.

use converge_core::{
    AgentEffect, Budget, ContextKey, ContextState, Engine, ProposedFact, Suggestor, TextPayload,
};

// ── Invalid proposals: rejected by promotion gate ──

struct BadContentAgent {
    name: &'static str,
    key: ContextKey,
    id: &'static str,
    content: &'static str,
}

#[async_trait::async_trait]
impl Suggestor for BadContentAgent {
    fn name(&self) -> &str {
        self.name
    }
    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }
    fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
        true
    }
    async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
        AgentEffect::with_proposal(ProposedFact::new(
            self.key,
            self.id,
            TextPayload::new(self.content),
            "test-agent",
        ))
    }

    fn provenance(&self) -> &'static str {
        "test-suggestor"
    }
}

#[tokio::test]
async fn empty_content_rejected() {
    let mut engine = Engine::new();
    engine.register_suggestor(BadContentAgent {
        name: "empty-agent",
        key: ContextKey::Seeds,
        id: "bad-1",
        content: "",
    });

    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge (proposal rejected)");

    assert!(result.converged);
    assert!(!result.context.has(ContextKey::Seeds));
}

#[tokio::test]
async fn whitespace_only_content_rejected() {
    let mut engine = Engine::new();
    engine.register_suggestor(BadContentAgent {
        name: "ws-agent",
        key: ContextKey::Seeds,
        id: "bad-2",
        content: "   \t\n  ",
    });

    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge (proposal rejected)");

    assert!(result.converged);
    assert!(!result.context.has(ContextKey::Seeds));
}

// ── Edge cases: zero suggestors, budget limits ──

#[tokio::test]
async fn zero_suggestors_converges_immediately() {
    let mut engine = Engine::new();
    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge");

    assert!(result.converged);
    assert_eq!(result.cycles, 1);
}

#[tokio::test]
async fn budget_exhaustion_terminates() {
    struct InfiniteProposer;

    #[async_trait::async_trait]
    impl Suggestor for InfiniteProposer {
        fn name(&self) -> &str {
            "infinite-proposer"
        }
        fn dependencies(&self) -> &[ContextKey] {
            &[]
        }
        fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
            true
        }
        async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
            let count = ctx.get(ContextKey::Seeds).len();
            AgentEffect::with_proposal(ProposedFact::new(
                ContextKey::Seeds,
                format!("infinite-{count}"),
                TextPayload::new(format!("fact number {count}")),
                self.name().to_string(),
            ))
        }

        fn provenance(&self) -> &'static str {
            "test-suggestor"
        }
    }

    let mut engine = Engine::with_budget(Budget {
        max_cycles: 3,
        max_facts: 100,
    });
    engine.register_suggestor(InfiniteProposer);

    let result = engine.run(ContextState::new()).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        converge_core::ConvergeError::BudgetExhausted { .. }
    ));
}

#[tokio::test]
async fn max_facts_budget_terminates() {
    struct FloodAgent;

    #[async_trait::async_trait]
    impl Suggestor for FloodAgent {
        fn name(&self) -> &str {
            "flood-agent"
        }
        fn dependencies(&self) -> &[ContextKey] {
            &[]
        }
        fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
            true
        }
        async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
            let n = ctx.get(ContextKey::Seeds).len();
            let proposals: Vec<ProposedFact> = (0..10)
                .map(|i| {
                    ProposedFact::new(
                        ContextKey::Seeds,
                        format!("flood-{}-{i}", n),
                        TextPayload::new(format!("content {i}")),
                        "flood-agent",
                    )
                })
                .collect();
            AgentEffect::with_proposals(proposals)
        }

        fn provenance(&self) -> &'static str {
            "test-suggestor"
        }
    }

    let mut engine = Engine::with_budget(Budget {
        max_cycles: 100,
        max_facts: 5,
    });
    engine.register_suggestor(FloodAgent);

    let result = engine.run(ContextState::new()).await;
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        converge_core::ConvergeError::BudgetExhausted { .. }
    ));
}
