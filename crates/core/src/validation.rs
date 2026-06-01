// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! LLM and staged-proposal validation for Converge.
//!
//! The engine owns promotion. This module provides a compatibility validator
//! that can inspect staged proposals and emit diagnostic proposals when they
//! fail basic policy checks.

#![allow(clippy::unnecessary_literal_bound)]

use crate::agent::Suggestor;
use crate::context::{ContextKey, ProposedFact, TextPayload};
use crate::effect::AgentEffect;
use crate::suggestors::CONVERGE_CORE_PROVENANCE;
use converge_pack::{Provenance, ProvenanceSource, UnitInterval};
use strum::IntoEnumIterator;

/// Configuration for the validation agent.
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Minimum confidence threshold (0.0 - 1.0).
    pub min_confidence: UnitInterval,
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
            min_confidence: UnitInterval::clamped(0.5),
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
        if proposal.confidence() < self.config.min_confidence.as_f64() {
            return ValidationResult::Rejected {
                proposal_id: proposal.id.to_string(),
                reason: format!(
                    "confidence {} below threshold {}",
                    proposal.confidence(),
                    self.config.min_confidence.as_f64()
                ),
            };
        }

        if let Err(error) = proposal.validate_payload() {
            return ValidationResult::Rejected {
                proposal_id: proposal.id.to_string(),
                reason: error.to_string(),
            };
        }

        let text = proposal.text().unwrap_or("");

        if text.len() > self.config.max_content_length {
            return ValidationResult::Rejected {
                proposal_id: proposal.id.to_string(),
                reason: format!(
                    "content length {} exceeds max {}",
                    text.len(),
                    self.config.max_content_length
                ),
            };
        }

        if text.trim().is_empty() {
            return ValidationResult::Rejected {
                proposal_id: proposal.id.to_string(),
                reason: "content is empty".into(),
            };
        }

        if self.config.require_provenance && proposal.provenance().trim().is_empty() {
            return ValidationResult::Rejected {
                proposal_id: proposal.id.to_string(),
                reason: "provenance is required but empty".into(),
            };
        }

        let content_lower = text.to_lowercase();
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

    fn provenance(&self) -> Provenance {
        CONVERGE_CORE_PROVENANCE.provenance()
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
                            TextPayload::new(format!(
                                "Proposal '{proposal_id}' rejected: {reason}"
                            )),
                            self.provenance(),
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
            TextPayload::new("Market is growing"),
            CONVERGE_CORE_PROVENANCE.provenance(),
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
            min_confidence: UnitInterval::clamped(0.7),
            ..Default::default()
        });
        let proposal = ProposedFact::new(
            ContextKey::Hypotheses,
            "hyp-1",
            TextPayload::new("Uncertain claim"),
            CONVERGE_CORE_PROVENANCE.provenance(),
        )
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
