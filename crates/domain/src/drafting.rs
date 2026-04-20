// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Deterministic drafting flow (short path).
//!
//! Seeds -> Signals (research notes) -> Strategies (draft output)

use converge_core::{AgentEffect, ContextKey, Suggestor};

const DRAFT_RESEARCH_PREFIX: &str = "drafting_research:";
const DRAFT_OUTPUT_PREFIX: &str = "drafting_output:";

/// Drafting research agent (deterministic fallback).
pub struct DraftingResearchAgent;

#[async_trait::async_trait]
impl Suggestor for DraftingResearchAgent {
    fn name(&self) -> &str {
        "DraftingResearchAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.has(ContextKey::Seeds)
            && !ctx
                .get(ContextKey::Signals)
                .iter()
                .any(|fact| fact.id.starts_with(DRAFT_RESEARCH_PREFIX))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let summary = ctx
            .get(ContextKey::Seeds)
            .iter()
            .map(|seed| seed.content.clone())
            .collect::<Vec<_>>()
            .join(" | ");

        AgentEffect::with_proposal(crate::proposal(
            self.name(),
            ContextKey::Signals,
            format!("{DRAFT_RESEARCH_PREFIX}notes"),
            format!("Drafting research notes: {summary}"),
        ))
    }
}

/// Drafting composer agent (deterministic fallback).
pub struct DraftingComposerAgent;

#[async_trait::async_trait]
impl Suggestor for DraftingComposerAgent {
    fn name(&self) -> &str {
        "DraftingComposerAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|fact| fact.id.starts_with(DRAFT_RESEARCH_PREFIX))
            && !ctx
                .get(ContextKey::Strategies)
                .iter()
                .any(|fact| fact.id.starts_with(DRAFT_OUTPUT_PREFIX))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let notes = ctx
            .get(ContextKey::Signals)
            .iter()
            .filter(|fact| fact.id.starts_with(DRAFT_RESEARCH_PREFIX))
            .map(|fact| fact.content.clone())
            .collect::<Vec<_>>()
            .join("\n");

        AgentEffect::with_proposal(crate::proposal(
            self.name(),
            ContextKey::Strategies,
            format!("{DRAFT_OUTPUT_PREFIX}v0"),
            format!("Draft output (deterministic):\n{notes}"),
        ))
    }
}
