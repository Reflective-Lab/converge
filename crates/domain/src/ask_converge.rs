// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Ask Converge domain pack (grounded Q&A).
//!
//! This pack enforces grounded answering with explicit recall-only sources.

use converge_core::invariant::{Invariant, InvariantClass, InvariantResult, Violation};
use converge_core::{AgentEffect, ContextKey, Suggestor};
use serde::Deserialize;

const QUESTION_SEED_ID: &str = "ask:question";
const SOURCE_SEED_PREFIX: &str = "ask:source:";
const ANSWER_ID: &str = "ask:answer";

#[derive(Debug, Clone, Deserialize)]
struct AskSourcePayload {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    url: Option<String>,
    content: String,
}

#[derive(Debug, Clone)]
struct AskSource {
    id: String,
    title: Option<String>,
    url: Option<String>,
    content: String,
}

fn parse_question(ctx: &dyn converge_core::Context) -> Option<String> {
    ctx.get(ContextKey::Seeds)
        .iter()
        .find(|seed| seed.id == QUESTION_SEED_ID)
        .map(|seed| seed.content.clone())
}

fn parse_sources(ctx: &dyn converge_core::Context) -> Vec<AskSource> {
    ctx.get(ContextKey::Seeds)
        .iter()
        .filter(|seed| seed.id.starts_with(SOURCE_SEED_PREFIX))
        .map(|seed| {
            let payload: Option<AskSourcePayload> = serde_json::from_str(&seed.content).ok();
            if let Some(payload) = payload {
                AskSource {
                    id: payload.id.unwrap_or_else(|| seed.id.clone()),
                    title: payload.title,
                    url: payload.url,
                    content: payload.content,
                }
            } else {
                AskSource {
                    id: seed.id.clone(),
                    title: None,
                    url: None,
                    content: seed.content.clone(),
                }
            }
        })
        .collect()
}

fn build_answer(question: &str, sources: &[AskSource]) -> serde_json::Value {
    let source_ids: Vec<&str> = sources.iter().map(|source| source.id.as_str()).collect();
    let answer_text = format!(
        "Grounded response based on sources: {}.",
        source_ids.join(", ")
    );

    serde_json::json!({
        "question": question,
        "answer": answer_text,
        "grounded": true,
        "recall_only": true,
        "sources": sources.iter().map(|source| {
            serde_json::json!({
                "id": source.id,
                "title": source.title,
                "url": source.url,
                "content": source.content,
            })
        }).collect::<Vec<_>>(),
    })
}

/// Suggestor that produces a grounded answer based on provided sources.
#[derive(Debug, Clone, Default)]
pub struct AskConvergeAgent;

#[async_trait::async_trait]
impl Suggestor for AskConvergeAgent {
    fn name(&self) -> &str {
        "ask_converge"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        let has_question = parse_question(ctx).is_some();
        let has_answer = ctx
            .get(ContextKey::Strategies)
            .iter()
            .any(|fact| fact.id == ANSWER_ID);
        has_question && !has_answer
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let question = match parse_question(ctx) {
            Some(question) => question,
            None => return AgentEffect::empty(),
        };
        let sources = parse_sources(ctx);

        if sources.is_empty() {
            return AgentEffect::empty();
        }

        let answer = build_answer(&question, &sources);
        let fact = crate::proposal(
            self.name(),
            ContextKey::Strategies,
            ANSWER_ID,
            answer.to_string(),
        );

        AgentEffect::with_proposal(fact)
    }
}

/// Enforces grounded answering (answers must include sources).
pub struct GroundedAnswerInvariant;

impl Invariant for GroundedAnswerInvariant {
    fn name(&self) -> &str {
        "grounded_answer_required"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        for fact in ctx.get(ContextKey::Strategies) {
            if fact.id != ANSWER_ID {
                continue;
            }

            let payload: serde_json::Value = match serde_json::from_str(&fact.content) {
                Ok(payload) => payload,
                Err(_) => {
                    return InvariantResult::Violated(Violation::new(
                        "Ask answer must be valid JSON payload".to_string(),
                    ));
                }
            };

            let grounded = payload.get("grounded").and_then(|v| v.as_bool());
            let sources = payload.get("sources").and_then(|v| v.as_array());

            if grounded != Some(true) || sources.map_or(true, |s| s.is_empty()) {
                return InvariantResult::Violated(Violation::new(
                    "Ask answer must be grounded with at least one source".to_string(),
                ));
            }
        }

        InvariantResult::Ok
    }
}

/// Enforces recall-only usage (sources are recall, not evidence).
pub struct RecallNotEvidenceInvariant;

impl Invariant for RecallNotEvidenceInvariant {
    fn name(&self) -> &str {
        "recall_not_evidence"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        for fact in ctx.get(ContextKey::Strategies) {
            if fact.id != ANSWER_ID {
                continue;
            }

            let payload: serde_json::Value = match serde_json::from_str(&fact.content) {
                Ok(payload) => payload,
                Err(_) => {
                    return InvariantResult::Violated(Violation::new(
                        "Ask answer must be valid JSON payload".to_string(),
                    ));
                }
            };

            let recall_only = payload.get("recall_only").and_then(|v| v.as_bool());
            if recall_only != Some(true) {
                return InvariantResult::Violated(Violation::new(
                    "Ask answer must be marked recall_only".to_string(),
                ));
            }
        }

        InvariantResult::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{ContextState, Engine};

    fn promoted_context(entries: &[(ContextKey, &str, &str)]) -> ContextState {
        let mut ctx = ContextState::new();
        for (key, id, content) in entries {
            ctx.add_input(*key, *id, *content).unwrap();
        }
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Engine::new().run(ctx))
            .unwrap()
            .context
    }

    #[test]
    fn ask_agent_emits_answer_with_sources() {
        let source = serde_json::json!({
            "id": "source-1",
            "content": "Converge is a semantic governance runtime."
        })
        .to_string();
        let ctx = promoted_context(&[
            (ContextKey::Seeds, QUESTION_SEED_ID, "What is Converge?"),
            (ContextKey::Seeds, "ask:source:1", source.as_str()),
        ]);

        let agent = AskConvergeAgent::default();
        let effect = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(agent.execute(&ctx));
        assert!(!effect.is_empty());
        assert_eq!(effect.proposals.len(), 1);
    }

    #[test]
    fn invariants_accept_grounded_answer() {
        let payload = build_answer(
            "What is Converge?",
            &[AskSource {
                id: "source-1".to_string(),
                title: None,
                url: None,
                content: "Converge is a semantic governance runtime.".to_string(),
            }],
        );
        let payload = payload.to_string();
        let ctx = promoted_context(&[(ContextKey::Strategies, ANSWER_ID, payload.as_str())]);

        assert!(matches!(
            GroundedAnswerInvariant.check(&ctx),
            InvariantResult::Ok
        ));
        assert!(matches!(
            RecallNotEvidenceInvariant.check(&ctx),
            InvariantResult::Ok
        ));
    }
}
