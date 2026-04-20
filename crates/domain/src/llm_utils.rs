// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Utilities for creating LLM-enabled suggestors backed by a chat backend.
//!
//! This module provides helpers for setting up suggestors that delegate
//! to a `ChatBackend` for LLM completions.

use crate::mock::{MockChatBackend, MockResponse};
use converge_core::{
    AgentEffect, Context, ContextKey, ProposedFact,
    model_selection::{AgentRequirements, CostClass},
    prompt::PromptFormat,
    traits::{ChatMessage, ChatRequest, ChatRole, DynChatBackend, ResponseFormat},
};
use std::sync::Arc;

/// A `Suggestor` that delegates to a `DynChatBackend` for LLM completions.
///
/// Uses the canonical async `ChatBackend::chat()` boundary directly from the
/// async `Suggestor::execute()` contract.
pub struct ChatAgentSuggestor {
    name: String,
    system_prompt: String,
    prompt_template: String,
    #[allow(dead_code)]
    prompt_format: PromptFormat,
    target_key: ContextKey,
    deps: Vec<ContextKey>,
    default_confidence: f64,
    max_tokens: u32,
    temperature: f32,
    backend: Arc<dyn DynChatBackend>,
}

#[async_trait::async_trait]
impl converge_core::Suggestor for ChatAgentSuggestor {
    fn name(&self) -> &str {
        &self.name
    }

    fn dependencies(&self) -> &[ContextKey] {
        &self.deps
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        let result_id = format!("{}-result", self.name);
        !ctx.get(self.target_key).iter().any(|f| f.id == result_id)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let context_str = ctx
            .get(self.target_key)
            .iter()
            .map(|f| f.content.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let user_content = self.prompt_template.replace("{context}", &context_str);

        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: user_content,
                tool_calls: Vec::new(),
                tool_call_id: None,
            }],
            system: Some(self.system_prompt.clone()),
            model: None,
            temperature: Some(self.temperature),
            max_tokens: Some(self.max_tokens),
            stop_sequences: Vec::new(),
            tools: Vec::new(),
            response_format: ResponseFormat::Text,
        };

        let result = self.backend.chat(request).await;

        match result {
            Ok(response) => {
                let proposal = ProposedFact::new(
                    self.target_key,
                    format!("{}-result", self.name),
                    response.content,
                    &self.name,
                )
                .with_confidence(self.default_confidence);
                AgentEffect::with_proposal(proposal)
            }
            Err(e) => {
                tracing::warn!(agent = %self.name, error = %e, "ChatAgentSuggestor failed");
                AgentEffect::default()
            }
        }
    }
}

/// Creates a `ChatAgentSuggestor` backed by a `MockChatBackend` (for testing).
///
/// Returns both the suggestor and the mock backend so callers can inspect call counts.
#[must_use]
pub fn create_mock_llm_agent(
    name: impl Into<String>,
    system_prompt: impl Into<String>,
    prompt_template: impl Into<String>,
    target_key: ContextKey,
    dependencies: Vec<ContextKey>,
    _requirements: AgentRequirements,
    mock_responses: Vec<MockResponse>,
) -> (ChatAgentSuggestor, Arc<MockChatBackend>) {
    let mock_backend = Arc::new(MockChatBackend::new(mock_responses));

    let suggestor = ChatAgentSuggestor {
        name: name.into(),
        system_prompt: system_prompt.into(),
        prompt_template: prompt_template.into(),
        prompt_format: PromptFormat::Edn,
        target_key,
        deps: dependencies,
        default_confidence: 0.7,
        max_tokens: 1024,
        temperature: 0.7_f32,
        backend: mock_backend.clone(),
    };

    (suggestor, mock_backend)
}

/// Common requirement presets for different agent types.
pub mod requirements {
    use super::{AgentRequirements, CostClass};

    /// Requirements for fast, high-volume agents (e.g., data extraction).
    #[must_use]
    pub fn fast_extraction() -> AgentRequirements {
        AgentRequirements::fast_cheap()
    }

    /// Requirements for analysis agents (e.g., market analysis, strategy synthesis).
    #[must_use]
    pub fn analysis() -> AgentRequirements {
        AgentRequirements::balanced().with_min_quality(0.75)
    }

    /// Requirements for deep research agents (e.g., competitor analysis, risk assessment).
    #[must_use]
    pub fn deep_research() -> AgentRequirements {
        AgentRequirements::deep_research()
    }

    /// Requirements for synthesis agents (e.g., strategy synthesis, consolidation).
    #[must_use]
    pub fn synthesis() -> AgentRequirements {
        AgentRequirements::new(CostClass::Medium, 10000, true).with_min_quality(0.8)
    }

    /// Requirements for validation agents (e.g., compliance checking, quality gates).
    #[must_use]
    pub fn validation() -> AgentRequirements {
        AgentRequirements::balanced().with_min_quality(0.85)
    }

    /// Requirements for categorization agents (e.g., category inference, classification).
    #[must_use]
    pub fn categorization() -> AgentRequirements {
        AgentRequirements::fast_cheap().with_min_quality(0.7)
    }
}
