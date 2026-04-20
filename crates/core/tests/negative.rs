// Negative and edge-case tests.
//
// Prove: the system rejects invalid input and handles edge cases gracefully.

use converge_core::{
    AgentEffect, Budget, Context, ContextKey, ContextState, Engine, ProposedFact, Suggestor,
};

// ── Invalid proposals: rejected by promotion gate ──

struct BadProposalAgent {
    name: &'static str,
    key: ContextKey,
    id: &'static str,
    content: &'static str,
    confidence: f64,
}

#[async_trait::async_trait]
impl Suggestor for BadProposalAgent {
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
        AgentEffect::with_proposal(ProposedFact {
            key: self.key,
            id: self.id.to_string(),
            content: self.content.to_string(),
            confidence: self.confidence,
            provenance: "test-agent".to_string(),
        })
    }
}

#[tokio::test]
async fn nan_confidence_rejected() {
    let mut engine = Engine::new();
    engine.register_suggestor(BadProposalAgent {
        name: "nan-agent",
        key: ContextKey::Seeds,
        id: "bad-1",
        content: "valid content",
        confidence: f64::NAN,
    });

    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge (proposal rejected, no facts)");

    assert!(result.converged);
    assert!(!result.context.has(ContextKey::Seeds));
}

#[tokio::test]
async fn infinite_confidence_rejected() {
    let mut engine = Engine::new();
    engine.register_suggestor(BadProposalAgent {
        name: "inf-agent",
        key: ContextKey::Seeds,
        id: "bad-2",
        content: "valid content",
        confidence: f64::INFINITY,
    });

    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge (proposal rejected)");

    assert!(result.converged);
    assert!(!result.context.has(ContextKey::Seeds));
}

#[tokio::test]
async fn negative_confidence_rejected() {
    let mut engine = Engine::new();
    engine.register_suggestor(BadProposalAgent {
        name: "neg-agent",
        key: ContextKey::Seeds,
        id: "bad-3",
        content: "valid content",
        confidence: -0.1,
    });

    let result = engine
        .run(ContextState::new())
        .await
        .expect("should converge (proposal rejected)");

    assert!(result.converged);
    assert!(!result.context.has(ContextKey::Seeds));
}

#[tokio::test]
async fn empty_content_rejected() {
    let mut engine = Engine::new();
    engine.register_suggestor(BadProposalAgent {
        name: "empty-agent",
        key: ContextKey::Seeds,
        id: "bad-4",
        content: "",
        confidence: 0.9,
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
    engine.register_suggestor(BadProposalAgent {
        name: "ws-agent",
        key: ContextKey::Seeds,
        id: "bad-5",
        content: "   \t\n  ",
        confidence: 0.9,
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
                format!("fact number {count}"),
                self.name(),
            ))
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
                        format!("content {i}"),
                        "flood-agent",
                    )
                })
                .collect();
            AgentEffect::with_proposals(proposals)
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
