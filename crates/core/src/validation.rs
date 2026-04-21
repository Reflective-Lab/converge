// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! LLM and staged-proposal validation for Converge.
//!
//! The engine owns promotion. This module provides a compatibility validator
//! that can inspect staged proposals and emit diagnostic proposals when they
//! fail basic policy checks.

#![allow(clippy::unnecessary_literal_bound)]

use crate::agent::Suggestor;
use crate::context::{ContextKey, ProposedFact};
use crate::effect::AgentEffect;
use strum::IntoEnumIterator;

/// Configuration for the validation agent.
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Minimum confidence threshold (0.0 - 1.0).
    pub min_confidence: f64,
    /// Maximum content length allowed.
    pub max_content_length: usize,
    /// Forbidden terms that cause rejection.
    pub forbidden_terms: Vec<String>,
    /// Whether to require provenance information.
    pub require_provenance: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            min_confidence: 0.5,
            max_content_length: 10_000,
            forbidden_terms: vec![],
            require_provenance: true,
        }
    }
}

/// Result of validating a proposal.
#[derive(Debug, Clone)]
pub enum ValidationResult {
    /// Proposal accepted.
    Accepted(ProposedFact),
    /// Proposal rejected with reason.
    Rejected { proposal_id: String, reason: String },
}

/// Compatibility validator for staged proposals.
pub struct ValidationAgent {
    config: ValidationConfig,
}

impl ValidationAgent {
    /// Creates a new validation agent with the given config.
    #[must_use]
    pub fn new(config: ValidationConfig) -> Self {
        Self { config }
    }

    /// Creates a validation agent with default config.
    #[must_use]
    pub fn with_defaults() -> Self {
        Self::new(ValidationConfig::default())
    }

    /// Validates a single proposal against the config.
    pub fn validate_proposal(&self, proposal: &ProposedFact) -> ValidationResult {
        if proposal.confidence() < self.config.min_confidence {
            return ValidationResult::Rejected {
                proposal_id: proposal.id.to_string(),
                reason: format!(
                    "confidence {} below threshold {}",
                    proposal.confidence(),
                    self.config.min_confidence
                ),
            };
        }

        if proposal.content.len() > self.config.max_content_length {
            return ValidationResult::Rejected {
                proposal_id: proposal.id.to_string(),
                reason: format!(
                    "content length {} exceeds max {}",
                    proposal.content.len(),
                    self.config.max_content_length
                ),
            };
        }

        if proposal.content.trim().is_empty() {
            return ValidationResult::Rejected {
                proposal_id: proposal.id.to_string(),
                reason: "content is empty".into(),
            };
        }

        if self.config.require_provenance && proposal.provenance.trim().is_empty() {
            return ValidationResult::Rejected {
                proposal_id: proposal.id.to_string(),
                reason: "provenance is required but empty".into(),
            };
        }

        let content_lower = proposal.content.to_lowercase();
        for term in &self.config.forbidden_terms {
            if content_lower.contains(&term.to_lowercase()) {
                return ValidationResult::Rejected {
                    proposal_id: proposal.id.to_string(),
                    reason: format!("content contains forbidden term '{term}'"),
                };
            }
        }

        ValidationResult::Accepted(proposal.clone())
    }
}

#[async_trait::async_trait]
impl Suggestor for ValidationAgent {
    fn name(&self) -> &str {
        "ValidationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, ctx: &dyn crate::Context) -> bool {
        ContextKey::iter().any(|key| !ctx.get_proposals(key).is_empty())
    }

    async fn execute(&self, ctx: &dyn crate::Context) -> AgentEffect {
        let mut diagnostics = Vec::new();

        for key in ContextKey::iter() {
            for proposal in ctx.get_proposals(key) {
                if let ValidationResult::Rejected {
                    proposal_id,
                    reason,
                } = self.validate_proposal(proposal)
                {
                    diagnostics.push(
                        ProposedFact::new(
                            ContextKey::Diagnostic,
                            format!("validation:rejected:{proposal_id}"),
                            format!("Proposal '{proposal_id}' rejected: {reason}"),
                            self.name(),
                        )
                        .with_confidence(1.0),
                    );
                }
            }
        }

        AgentEffect::with_proposals(diagnostics)
    }
}

/// Compatibility helper retained for legacy call sites and tests.
#[must_use]
pub fn encode_proposal(proposal: &ProposedFact) -> ProposedFact {
    proposal.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_accepts_good_proposal() {
        let agent = ValidationAgent::with_defaults();
        let proposal = ProposedFact::new(
            ContextKey::Hypotheses,
            "hyp-1",
            "Market is growing",
            "gpt-4:abc123",
        )
        .with_confidence(0.8);

        match agent.validate_proposal(&proposal) {
            ValidationResult::Accepted(accepted) => {
                assert_eq!(accepted.id, "hyp-1");
            }
            ValidationResult::Rejected { reason, .. } => {
                panic!("Expected acceptance, got rejection: {reason}");
            }
        }
    }

    #[test]
    fn validation_rejects_low_confidence() {
        let agent = ValidationAgent::new(ValidationConfig {
            min_confidence: 0.7,
            ..Default::default()
        });
        let proposal =
            ProposedFact::new(ContextKey::Hypotheses, "hyp-1", "Uncertain claim", "gpt-4")
                .with_confidence(0.3);

        match agent.validate_proposal(&proposal) {
            ValidationResult::Rejected { reason, .. } => {
                assert!(reason.contains("confidence"));
            }
            ValidationResult::Accepted(_) => {
                panic!("Expected rejection for low confidence");
            }
        }
    }
}
