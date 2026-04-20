// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Suggestor trait and types for Converge.
//!
//! The `Suggestor` trait is defined in `converge-pack` and re-exported here.
//! `SuggestorId` is a core-internal type for deterministic ordering.

// Re-export the canonical Suggestor trait
pub use converge_pack::Suggestor;

/// Unique identifier for a registered suggestor.
///
/// Assigned monotonically at registration time.
/// Used for deterministic effect merge ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SuggestorId(pub(crate) u32);

impl SuggestorId {
    /// Returns the raw numeric ID.
    #[must_use]
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for SuggestorId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Suggestor({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextKey;
    use crate::effect::AgentEffect;

    /// A minimal test suggestor that emits one proposal then stops.
    struct TestSuggestor {
        fact_id: String,
    }

    #[async_trait::async_trait]
    impl Suggestor for TestSuggestor {
        fn name(&self) -> &str {
            "TestSuggestor"
        }

        fn dependencies(&self) -> &[ContextKey] {
            &[ContextKey::Seeds]
        }

        fn accepts(&self, ctx: &dyn crate::Context) -> bool {
            !ctx.get(ContextKey::Seeds)
                .iter()
                .any(|f| f.id == self.fact_id)
        }

        async fn execute(&self, _ctx: &dyn crate::Context) -> AgentEffect {
            AgentEffect::with_proposal(crate::ProposedFact::new(
                ContextKey::Seeds,
                self.fact_id.clone(),
                "test content",
                self.name(),
            ))
        }
    }

    #[test]
    fn suggestor_accepts_when_fact_missing() {
        let suggestor = TestSuggestor {
            fact_id: "test-1".into(),
        };
        let ctx = crate::context::ContextState::new();
        assert!(suggestor.accepts(&ctx));
    }

    #[test]
    fn suggestor_rejects_when_fact_present() {
        let suggestor = TestSuggestor {
            fact_id: "test-1".into(),
        };
        let mut ctx = crate::context::ContextState::new();
        let fact = converge_pack::fact::kernel_authority::new_fact(
            ContextKey::Seeds,
            "test-1",
            "already here",
        );
        let _ = ctx.add_fact(fact);
        assert!(!suggestor.accepts(&ctx));
    }

    #[tokio::test]
    async fn suggestor_produces_effect() {
        let suggestor = TestSuggestor {
            fact_id: "test-1".into(),
        };
        let ctx = crate::context::ContextState::new();
        let effect = suggestor.execute(&ctx).await;
        assert_eq!(effect.proposals.len(), 1);
        assert_eq!(effect.proposals[0].id, "test-1");
    }

    #[test]
    fn suggestor_id_ordering() {
        let a = SuggestorId(1);
        let b = SuggestorId(2);
        let c = SuggestorId(1);
        assert!(a < b);
        assert_eq!(a, c);
    }
}
