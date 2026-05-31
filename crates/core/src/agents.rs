// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Example suggestors for testing and demonstration.
//!
// Suggestor trait returns &str, but we return literals. This is fine.
#![allow(clippy::unnecessary_literal_bound)]
//!
//! These suggestors prove the core convergence properties:
//! - `SeedSuggestor`: Emits initial facts, stops when done
//! - `ReactOnceSuggestor`: Reacts to changes, stops after one contribution
//!
//! # Example
//!
//! ```ignore
//! use converge_core::{Engine, Context, ContextKey};
//! use converge_core::suggestors::{SeedSuggestor, ReactOnceSuggestor};
//!
//! let mut engine = Engine::new();
//! engine.register_suggestor(SeedSuggestor::new("seed-1", "initial value"));
//! engine.register_suggestor(ReactOnceSuggestor::new("hyp-1", "derived insight"));
//!
//! let result = engine.run(ContextState::new()).await.expect("converges");
//! assert!(result.converged);
//! assert!(result.context.has(ContextKey::Seeds));
//! assert!(result.context.has(ContextKey::Hypotheses));
//! ```

use crate::agent::Suggestor;
use crate::context::{ContextKey, ProposedFact, TextPayload};
use crate::effect::AgentEffect;
use converge_pack::{Provenance, ProvenanceSource};

/// Canonical provenance marker for reference suggestors shipped by
/// `converge-core` itself (e.g., [`SeedSuggestor`], [`ReactOnceSuggestor`]).
/// External crates declare their own marker; this one exists so the core's
/// own demonstration suggestors satisfy the empty-provenance contract.
#[derive(Copy, Clone, Debug)]
pub struct ConvergeCore;

impl ProvenanceSource for ConvergeCore {
    fn as_str(&self) -> &'static str {
        "converge-core"
    }
}

/// Canonical provenance const for [`ConvergeCore`].
pub const CONVERGE_CORE_PROVENANCE: ConvergeCore = ConvergeCore;

/// A suggestor that emits an initial seed proposal once.
///
/// Demonstrates:
/// - Suggestor with no dependencies (runs first)
/// - Self-terminating behavior (checks if already contributed)
/// - Monotonic context evolution
pub struct SeedSuggestor {
    fact_id: String,
    content: String,
}

impl SeedSuggestor {
    /// Creates a new seed suggestor.
    #[must_use]
    pub fn new(fact_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            fact_id: fact_id.into(),
            content: content.into(),
        }
    }
}

#[async_trait::async_trait]
impl Suggestor for SeedSuggestor {
    fn name(&self) -> &str {
        "SeedSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[] // No dependencies = eligible on first cycle
    }

    fn accepts(&self, ctx: &dyn crate::Context) -> bool {
        // Only run if we haven't contributed yet
        !ctx.get(ContextKey::Seeds)
            .iter()
            .any(|f| f.id().as_str() == self.fact_id)
    }

    async fn execute(&self, _ctx: &dyn crate::Context) -> AgentEffect {
        AgentEffect::with_proposal(ProposedFact::new(
            ContextKey::Seeds,
            self.fact_id.clone(),
            TextPayload::new(self.content.clone()),
            self.name().to_string(),
        ))
    }

    fn provenance(&self) -> Provenance {
        Provenance::from(CONVERGE_CORE_PROVENANCE.as_str())
    }
}

/// A suggestor that reacts to seeds by emitting a hypothesis once.
///
/// Demonstrates:
/// - Dependency-driven activation (only runs when Seeds change)
/// - Data-driven behavior (reads context to decide)
/// - Self-terminating (checks if already contributed)
pub struct ReactOnceSuggestor {
    fact_id: String,
    content: String,
}

impl ReactOnceSuggestor {
    /// Creates a new reactive suggestor.
    #[must_use]
    pub fn new(fact_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            fact_id: fact_id.into(),
            content: content.into(),
        }
    }
}

#[async_trait::async_trait]
impl Suggestor for ReactOnceSuggestor {
    fn name(&self) -> &str {
        "ReactOnceSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds] // Only wake when Seeds change
    }

    fn accepts(&self, ctx: &dyn crate::Context) -> bool {
        // Run if: seeds exist AND we haven't contributed
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Hypotheses)
                .iter()
                .any(|f| f.id().as_str() == self.fact_id)
    }

    fn provenance(&self) -> Provenance {
        Provenance::from(CONVERGE_CORE_PROVENANCE.as_str())
    }

    async fn execute(&self, _ctx: &dyn crate::Context) -> AgentEffect {
        AgentEffect::with_proposal(ProposedFact::new(
            ContextKey::Hypotheses,
            self.fact_id.clone(),
            TextPayload::new(self.content.clone()),
            self.name().to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextState;
    use crate::engine::Engine;

    #[tokio::test]
    async fn seed_agent_emits_once() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("s1", "value"));

        let result = engine.run(ContextState::new()).await.expect("converges");

        assert!(result.converged);
        assert_eq!(result.context.get(ContextKey::Seeds).len(), 1);
    }

    #[tokio::test]
    async fn react_once_agent_chains_from_seed() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("s1", "seed"));
        engine.register_suggestor(ReactOnceSuggestor::new("h1", "hypothesis"));

        let result = engine.run(ContextState::new()).await.expect("converges");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Seeds));
        assert!(result.context.has(ContextKey::Hypotheses));
    }

    #[tokio::test]
    async fn multiple_seeds_all_converge() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("s1", "first"));
        engine.register_suggestor(SeedSuggestor::new("s2", "second"));
        engine.register_suggestor(SeedSuggestor::new("s3", "third"));

        let result = engine.run(ContextState::new()).await.expect("converges");

        assert!(result.converged);
        assert_eq!(result.context.get(ContextKey::Seeds).len(), 3);
    }

    #[tokio::test]
    async fn chain_of_three_converges() {
        /// Third suggestor in the chain.
        struct StrategyAgent;

        #[async_trait::async_trait]
        impl Suggestor for StrategyAgent {
            fn name(&self) -> &str {
                "StrategyAgent"
            }

            fn dependencies(&self) -> &[ContextKey] {
                &[ContextKey::Hypotheses]
            }

            fn accepts(&self, ctx: &dyn crate::Context) -> bool {
                ctx.has(ContextKey::Hypotheses) && !ctx.has(ContextKey::Strategies)
            }

            async fn execute(&self, _ctx: &dyn crate::Context) -> AgentEffect {
                AgentEffect::with_proposal(ProposedFact::new(
                    ContextKey::Strategies,
                    "strat-1",
                    TextPayload::new("derived strategy"),
                    self.name().to_string(),
                ))
            }

            fn provenance(&self) -> Provenance {
                Provenance::from(CONVERGE_CORE_PROVENANCE.as_str())
            }
        }

        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("s1", "seed"));
        engine.register_suggestor(ReactOnceSuggestor::new("h1", "hypothesis"));
        engine.register_suggestor(StrategyAgent);

        let result = engine.run(ContextState::new()).await.expect("converges");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Seeds));
        assert!(result.context.has(ContextKey::Hypotheses));
        assert!(result.context.has(ContextKey::Strategies));
    }
}
