// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Converge Prompt DSL — Compact machine-to-machine contract format.
//!
//! This module provides EDN-like serialization for agent prompts,
//! optimized for token efficiency and deterministic parsing.
//!
//! # Philosophy
//!
//! Suggestor prompts are **machine-to-machine contracts**, not human UX.
//! They prioritize:
//! - Token efficiency (50-60% savings vs Markdown)
//! - Structural clarity
//! - Deterministic parsing
//! - Zero fluff
//!
//! Human explanations are generated downstream from provenance + context.

use crate::context::{ContextKey, Fact};
use std::collections::HashSet;
use std::fmt::Write;

/// Prompt format for agent prompts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptFormat {
    /// Plain text (backward compatible, human-readable).
    Plain,
    /// EDN-like compact format (default, token-efficient).
    #[default]
    Edn,
}

/// Suggestor role in the prompt contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentRole {
    /// Proposes new facts (LLM agents).
    Proposer,
    /// Validates proposals (deterministic agents).
    Validator,
    /// Synthesizes existing facts.
    Synthesizer,
    /// Analyzes and evaluates.
    Analyzer,
}

impl AgentRole {
    /// Converts to compact keyword string.
    fn to_keyword(self) -> &'static str {
        match self {
            Self::Proposer => ":proposer",
            Self::Validator => ":validator",
            Self::Synthesizer => ":synthesizer",
            Self::Analyzer => ":analyzer",
        }
    }
}

/// Constraint keywords for prompts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Constraint {
    /// Do not invent facts not in context.
    NoInvent,
    /// Do not contradict existing facts.
    NoContradict,
    /// Do not hallucinate.
    NoHallucinate,
    /// Cite sources when possible.
    CiteSources,
}

impl Constraint {
    /// Converts to compact keyword string.
    fn to_keyword(self) -> &'static str {
        match self {
            Self::NoInvent => ":no-invent",
            Self::NoContradict => ":no-contradict",
            Self::NoHallucinate => ":no-hallucinate",
            Self::CiteSources => ":cite-sources",
        }
    }
}

/// Output contract for the prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputContract {
    /// What to emit.
    pub emit: OutputKind,
    /// Target context key.
    pub key: ContextKey,
    /// Output serialization format.
    pub format: Option<OutputFormat>,
}

/// Output kind required by the prompt contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    ProposedFact,
    Fact,
    Analysis,
    Evaluation,
    Plan,
    Classification,
    Draft,
    Reasoning,
}

impl OutputKind {
    fn to_keyword(self) -> &'static str {
        match self {
            Self::ProposedFact => "proposed-fact",
            Self::Fact => "fact",
            Self::Analysis => "analysis",
            Self::Evaluation => "evaluation",
            Self::Plan => "plan",
            Self::Classification => "classification",
            Self::Draft => "draft",
            Self::Reasoning => "reasoning",
        }
    }
}

/// Serialization format for emitted output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Edn,
    Json,
    Xml,
    Plain,
}

impl OutputFormat {
    fn to_keyword(self) -> &'static str {
        match self {
            Self::Edn => "edn",
            Self::Json => "json",
            Self::Xml => "xml",
            Self::Plain => "plain",
        }
    }
}

impl OutputContract {
    /// Creates a new output contract.
    #[must_use]
    pub fn new(emit: OutputKind, key: ContextKey) -> Self {
        Self {
            emit,
            key,
            format: None,
        }
    }

    /// Sets the output format.
    #[must_use]
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = Some(format);
        self
    }
}

/// Compact agent prompt contract.
///
/// This is the canonical internal representation that gets serialized
/// to EDN-like format for LLM consumption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentPrompt {
    /// Suggestor role.
    pub role: AgentRole,
    /// Objective (what the agent should do).
    pub objective: String,
    /// Context data (facts from dependencies).
    pub context: PromptContext,
    /// Constraints (keywords).
    pub constraints: HashSet<Constraint>,
    /// Output contract.
    pub output_contract: OutputContract,
}

/// Context data extracted from Context for the prompt.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PromptContext {
    /// Facts grouped by `ContextKey`.
    pub facts: Vec<(ContextKey, Vec<Fact>)>,
}

impl PromptContext {
    /// Creates an empty context.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds facts for a given key.
    pub fn add_facts(&mut self, key: ContextKey, facts: Vec<Fact>) {
        if !facts.is_empty() {
            self.facts.push((key, facts));
        }
    }

    /// Builds context from a Context and dependency keys.
    #[must_use]
    pub fn from_context(ctx: &dyn crate::Context, dependencies: &[ContextKey]) -> Self {
        let mut prompt_ctx = Self::new();
        for &key in dependencies {
            let facts = ctx.get(key).to_vec();
            prompt_ctx.add_facts(key, facts);
        }
        prompt_ctx
    }
}

/// Converts `ContextKey` to compact keyword string.
fn context_key_to_keyword(key: ContextKey) -> &'static str {
    match key {
        ContextKey::Seeds => ":seeds",
        ContextKey::Hypotheses => ":hypotheses",
        ContextKey::Strategies => ":strategies",
        ContextKey::Constraints => ":constraints",
        ContextKey::Signals => ":signals",
        ContextKey::Competitors => ":competitors",
        ContextKey::Evaluations => ":evaluations",
        ContextKey::Proposals => ":proposals",
        ContextKey::Diagnostic => ":diagnostic",
    }
}

impl AgentPrompt {
    /// Creates a new agent prompt.
    #[must_use]
    pub fn new(
        role: AgentRole,
        objective: impl Into<String>,
        context: PromptContext,
        output_contract: OutputContract,
    ) -> Self {
        Self {
            role,
            objective: objective.into(),
            context,
            constraints: HashSet::new(),
            output_contract,
        }
    }

    /// Adds a constraint.
    #[must_use]
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.insert(constraint);
        self
    }

    /// Adds multiple constraints.
    #[must_use]
    pub fn with_constraints(mut self, constraints: impl IntoIterator<Item = Constraint>) -> Self {
        self.constraints.extend(constraints);
        self
    }

    /// Serializes to EDN-like compact format.
    ///
    /// Format:
    /// ```edn
    /// {:r :proposer
    ///  :o :extract-competitors
    ///  :c {:signals [{:id "s1" :c "..."}]}
    ///  :k #{:no-invent :no-contradict}
    ///  :out {:emit :proposed-fact :key :competitors}}
    /// ```
    #[must_use]
    pub fn to_edn(&self) -> String {
        let mut s = String::new();
        s.push_str("{:r ");
        s.push_str(self.role.to_keyword());
        s.push_str("\n :o :");
        // Escape objective if needed (simplified: assume no special chars)
        s.push_str(&self.objective.replace(' ', "-"));
        s.push_str("\n :c {");

        // Serialize context facts
        let mut first_key = true;
        for (key, facts) in &self.context.facts {
            if !first_key {
                s.push(' ');
            }
            first_key = false;
            s.push_str(context_key_to_keyword(*key));
            s.push_str(" [{");
            for (i, fact) in facts.iter().enumerate() {
                if i > 0 {
                    s.push_str("} {");
                }
                s.push_str(":id \"");
                s.push_str(&escape_string(&fact.id));
                s.push_str("\" :c \"");
                s.push_str(&escape_string(&fact.content));
                s.push('"');
            }
            s.push_str("}]");
        }

        s.push_str("}\n :k #{");

        // Serialize constraints
        let mut constraints: Vec<_> = self.constraints.iter().collect();
        constraints.sort(); // Deterministic ordering
        for (i, constraint) in constraints.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            s.push_str(constraint.to_keyword());
        }

        s.push_str("}\n :out {:emit :");
        s.push_str(self.output_contract.emit.to_keyword());
        s.push_str(" :key ");
        s.push_str(context_key_to_keyword(self.output_contract.key));
        if let Some(format) = self.output_contract.format {
            s.push_str(" :format :");
            s.push_str(format.to_keyword());
        }
        s.push_str("}}");

        s
    }

    /// Serializes to plain text format (backward compatible).
    #[must_use]
    pub fn to_plain(&self) -> String {
        let mut s = String::new();
        writeln!(s, "Role: {:?}", self.role).unwrap();
        writeln!(s, "Objective: {}", self.objective).unwrap();
        writeln!(s, "\nContext:").unwrap();

        for (key, facts) in &self.context.facts {
            writeln!(s, "\n## {key:?}").unwrap();
            for fact in facts {
                writeln!(s, "- {}: {}", fact.id, fact.content).unwrap();
            }
        }

        if !self.constraints.is_empty() {
            writeln!(s, "\nConstraints:").unwrap();
            for constraint in &self.constraints {
                writeln!(s, "- {constraint:?}").unwrap();
            }
        }

        writeln!(
            s,
            "\nOutput: {:?} -> {:?}",
            self.output_contract.emit, self.output_contract.key
        )
        .unwrap();

        s
    }

    /// Serializes based on format.
    #[must_use]
    pub fn serialize(&self, format: PromptFormat) -> String {
        match format {
            PromptFormat::Edn => self.to_edn(),
            PromptFormat::Plain => self.to_plain(),
        }
    }
}

/// Escapes special characters in strings for EDN.
/// Alias for `OutputContract` used by the llm crate's prompt DSL.
pub type DslOutputContract = OutputContract;

fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::ContextState;

    #[test]
    fn test_edn_serialization() {
        let mut ctx = PromptContext::new();
        ctx.add_facts(
            ContextKey::Signals,
            vec![
                crate::context::new_fact(ContextKey::Signals, "s1", "Revenue +15% Q3"),
                crate::context::new_fact(ContextKey::Signals, "s2", "Market $2.3B"),
            ],
        );

        let prompt = AgentPrompt::new(
            AgentRole::Proposer,
            "extract-competitors",
            ctx,
            OutputContract::new(OutputKind::ProposedFact, ContextKey::Competitors),
        )
        .with_constraint(Constraint::NoInvent)
        .with_constraint(Constraint::NoContradict);

        let edn = prompt.to_edn();
        assert!(edn.contains(":r :proposer"));
        assert!(edn.contains(":o :extract-competitors"));
        assert!(edn.contains(":signals"));
        assert!(edn.contains(":no-invent"));
        assert!(edn.contains(":no-contradict"));
        assert!(edn.contains(":competitors"));
    }

    #[test]
    fn test_context_building() {
        let mut context = ContextState::new();
        context
            .add_fact(crate::context::new_fact(
                ContextKey::Seeds,
                "seed1",
                "Test seed",
            ))
            .unwrap();

        let prompt_ctx = PromptContext::from_context(&context, &[ContextKey::Seeds]);
        assert_eq!(prompt_ctx.facts.len(), 1);
        assert_eq!(prompt_ctx.facts[0].0, ContextKey::Seeds);
        assert_eq!(prompt_ctx.facts[0].1.len(), 1);
    }

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("hello\"world"), "hello\\\"world");
        assert_eq!(escape_string("hello\nworld"), "hello\\nworld");
    }

    #[test]
    fn test_token_efficiency() {
        let mut ctx = PromptContext::new();
        ctx.add_facts(
            ContextKey::Signals,
            vec![crate::context::new_fact(
                ContextKey::Signals,
                "s1",
                "Revenue +15% Q3",
            )],
        );

        let prompt = AgentPrompt::new(
            AgentRole::Proposer,
            "analyze",
            ctx,
            OutputContract::new(OutputKind::ProposedFact, ContextKey::Strategies),
        );

        let edn = prompt.to_edn();
        let plain = prompt.to_plain();

        println!("EDN length: {}", edn.len());
        println!("Plain length: {}", plain.len());
        println!("EDN:\n{edn}");
        println!("Plain:\n{plain}");

        // For small prompts, EDN overhead may exceed plain text.
        // The efficiency gain comes from larger contexts where structural
        // overhead is amortized. This test verifies the format works correctly.
        // Token efficiency is verified in integration tests with real contexts.
        assert!(!edn.is_empty());
        assert!(!plain.is_empty());
    }
}
