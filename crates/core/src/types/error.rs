// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Consolidated error types for the types module.
//!
//! All error types use thiserror for derive macros, providing
//! structured variants with proper Display implementations.
//!
//! # Error Hierarchy
//!
//! - `TypeError` - Core type system errors
//! - `PromotionError` - Errors during fact promotion
//! - `TypesValidationError` - Errors during validation
//! - `ObservationError` - Errors during observation capture
//! - `CorrectionError` - Errors during correction

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// TypeError - Core type system errors
// ============================================================================

/// Core type system errors.
#[derive(Debug, Error)]
pub enum TypeError {
    /// Invalid state transition.
    #[error("invalid state transition: {from} -> {to}")]
    InvalidStateTransition {
        /// Source state.
        from: String,
        /// Target state.
        to: String,
    },

    /// Promotion invariant violated.
    #[error("promotion invariant violated: {0}")]
    PromotionInvariant(String),

    /// Missing required field.
    #[error("missing required field: {field}")]
    MissingField {
        /// Field name.
        field: &'static str,
    },

    /// Validation failed.
    #[error("validation failed: {reason}")]
    ValidationFailed {
        /// Reason for failure.
        reason: String,
    },

    /// Serialization error.
    #[error(transparent)]
    Serialization(#[from] serde_json::Error),
}

impl TypeError {
    /// Create an invalid state transition error.
    pub fn invalid_transition(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::InvalidStateTransition {
            from: from.into(),
            to: to.into(),
        }
    }

    /// Create a promotion invariant error.
    pub fn promotion_invariant(message: impl Into<String>) -> Self {
        Self::PromotionInvariant(message.into())
    }

    /// Create a missing field error.
    pub fn missing_field(field: &'static str) -> Self {
        Self::MissingField { field }
    }

    /// Create a validation failed error.
    pub fn validation_failed(reason: impl Into<String>) -> Self {
        Self::ValidationFailed {
            reason: reason.into(),
        }
    }
}

// ============================================================================
// PromotionError - Errors during fact promotion
// ============================================================================

/// Errors during fact promotion.
#[derive(Debug, Error)]
pub enum PromotionError {
    /// Proposal not validated.
    #[error("proposal not validated: {proposal_id}")]
    NotValidated {
        /// ID of the unvalidated proposal.
        proposal_id: String,
    },

    /// Validation failed during promotion.
    #[error("validation failed: {reason}")]
    ValidationFailed {
        /// Reason for failure.
        reason: String,
    },

    /// Gate rejected the promotion.
    #[error("gate rejected: {gate_id} - {reason}")]
    GateRejected {
        /// ID of the gate that rejected.
        gate_id: String,
        /// Reason for rejection.
        reason: String,
    },

    /// Missing evidence for promotion.
    #[error("missing evidence: {0}")]
    MissingEvidence(String),

    /// Human approval required.
    #[error("human approval required")]
    HumanApprovalRequired,

    /// Policy version mismatch.
    #[error("policy version mismatch: expected {expected}, got {actual}")]
    PolicyVersionMismatch {
        /// Expected policy version.
        expected: String,
        /// Actual policy version.
        actual: String,
    },

    /// Validation report mismatch (report doesn't match proposal).
    #[error("report mismatch: proposal {proposal_id} vs report for {report_proposal_id}")]
    ReportMismatch {
        /// ID of the proposal being promoted.
        proposal_id: String,
        /// ID in the validation report.
        report_proposal_id: String,
    },
}

impl PromotionError {
    /// Create a not validated error.
    pub fn not_validated(proposal_id: impl Into<String>) -> Self {
        Self::NotValidated {
            proposal_id: proposal_id.into(),
        }
    }

    /// Create a validation failed error.
    pub fn validation_failed(reason: impl Into<String>) -> Self {
        Self::ValidationFailed {
            reason: reason.into(),
        }
    }

    /// Create a gate rejected error.
    pub fn gate_rejected(gate_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::GateRejected {
            gate_id: gate_id.into(),
            reason: reason.into(),
        }
    }

    /// Create a missing evidence error.
    pub fn missing_evidence(description: impl Into<String>) -> Self {
        Self::MissingEvidence(description.into())
    }

    /// Create a report mismatch error.
    pub fn report_mismatch(
        proposal_id: impl std::fmt::Display,
        report_proposal_id: impl std::fmt::Display,
    ) -> Self {
        Self::ReportMismatch {
            proposal_id: proposal_id.to_string(),
            report_proposal_id: report_proposal_id.to_string(),
        }
    }
}

// ============================================================================
// TypesValidationError - Errors during validation
// ============================================================================

/// Errors during validation.
///
/// Uses "Types" prefix to avoid collision with existing ValidationError.
/// Implements Clone, PartialEq, Eq for use in tests and assertions.
#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypesValidationError {
    /// Invalid confidence value.
    #[error("invalid confidence: {value} (must be 0.0-1.0)")]
    InvalidConfidence {
        /// The invalid value.
        value: String, // Using String for serialization (f32 doesn't impl Eq)
    },

    /// Empty content not allowed.
    #[error("empty content not allowed")]
    EmptyContent,

    /// Missing required field.
    #[error("missing required field: {field}")]
    MissingField {
        /// Field name.
        field: String,
    },

    /// Constraint violated.
    #[error("constraint violated: {constraint}")]
    ConstraintViolated {
        /// Constraint description.
        constraint: String,
    },

    /// Schema mismatch.
    #[error("schema mismatch: expected {expected} vs actual {actual}")]
    SchemaMismatch {
        /// Expected schema.
        expected: String,
        /// Actual schema.
        actual: String,
    },

    /// Invalid format.
    #[error("invalid format: {description}")]
    InvalidFormat {
        /// Description of the format error.
        description: String,
    },
}

impl TypesValidationError {
    /// Create an invalid confidence error.
    pub fn invalid_confidence(value: f32) -> Self {
        Self::InvalidConfidence {
            value: format!("{value}"),
        }
    }

    /// Create a missing field error.
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
        }
    }

    /// Create a constraint violated error.
    pub fn constraint_violated(constraint: impl Into<String>) -> Self {
        Self::ConstraintViolated {
            constraint: constraint.into(),
        }
    }

    /// Create a schema mismatch error.
    pub fn schema_mismatch(expected: impl Into<String>, actual: impl Into<String>) -> Self {
        Self::SchemaMismatch {
            expected: expected.into(),
            actual: actual.into(),
        }
    }

    /// Create an invalid format error.
    pub fn invalid_format(description: impl Into<String>) -> Self {
        Self::InvalidFormat {
            description: description.into(),
        }
    }
}

// ============================================================================
// ObservationError - Errors during observation capture
// ============================================================================

/// Errors during observation capture.
#[derive(Debug, Error)]
pub enum ObservationError {
    /// Invalid provider response.
    #[error("invalid provider response: {0}")]
    InvalidProviderResponse(String),

    /// Content hash mismatch.
    #[error("content hash mismatch")]
    ContentHashMismatch,

    /// Capture context missing.
    #[error("capture context missing: {field}")]
    MissingCaptureContext {
        /// Missing field name.
        field: String,
    },

    /// Provider not available.
    #[error("provider not available: {provider}")]
    ProviderNotAvailable {
        /// Provider identifier.
        provider: String,
    },

    /// Timeout during capture.
    #[error("observation capture timeout after {timeout_ms}ms")]
    Timeout {
        /// Timeout in milliseconds.
        timeout_ms: u64,
    },
}

impl ObservationError {
    /// Create an invalid provider response error.
    pub fn invalid_response(message: impl Into<String>) -> Self {
        Self::InvalidProviderResponse(message.into())
    }

    /// Create a missing capture context error.
    pub fn missing_context(field: impl Into<String>) -> Self {
        Self::MissingCaptureContext {
            field: field.into(),
        }
    }

    /// Create a provider not available error.
    pub fn provider_not_available(provider: impl Into<String>) -> Self {
        Self::ProviderNotAvailable {
            provider: provider.into(),
        }
    }

    /// Create a timeout error.
    pub fn timeout(timeout_ms: u64) -> Self {
        Self::Timeout { timeout_ms }
    }
}

// ============================================================================
// CorrectionError - Errors during correction
// ============================================================================

/// Errors during correction.
#[derive(Debug, Error)]
pub enum CorrectionError {
    /// Fact not found.
    #[error("fact not found: {fact_id}")]
    FactNotFound {
        /// ID of the missing fact.
        fact_id: String,
    },

    /// Fact already superseded.
    #[error("already superseded by: {superseding_fact_id}")]
    AlreadySuperseded {
        /// ID of the fact that superseded this one.
        superseding_fact_id: String,
    },

    /// Scope mismatch.
    #[error("scope mismatch: correction scope {correction_scope} doesn't match fact scope")]
    ScopeMismatch {
        /// Correction scope.
        correction_scope: String,
    },

    /// Not authorized to correct.
    #[error("not authorized to correct fact: {fact_id}")]
    NotAuthorized {
        /// ID of the fact.
        fact_id: String,
    },

    /// Invalid correction chain.
    #[error("invalid correction chain: {reason}")]
    InvalidChain {
        /// Reason for invalid chain.
        reason: String,
    },
}

impl CorrectionError {
    /// Create a fact not found error.
    pub fn fact_not_found(fact_id: impl Into<String>) -> Self {
        Self::FactNotFound {
            fact_id: fact_id.into(),
        }
    }

    /// Create an already superseded error.
    pub fn already_superseded(superseding_fact_id: impl Into<String>) -> Self {
        Self::AlreadySuperseded {
            superseding_fact_id: superseding_fact_id.into(),
        }
    }

    /// Create a scope mismatch error.
    pub fn scope_mismatch(correction_scope: impl Into<String>) -> Self {
        Self::ScopeMismatch {
            correction_scope: correction_scope.into(),
        }
    }

    /// Create a not authorized error.
    pub fn not_authorized(fact_id: impl Into<String>) -> Self {
        Self::NotAuthorized {
            fact_id: fact_id.into(),
        }
    }

    /// Create an invalid chain error.
    pub fn invalid_chain(reason: impl Into<String>) -> Self {
        Self::InvalidChain {
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_error_display() {
        let err = TypeError::invalid_transition("Draft", "Fact");
        assert_eq!(err.to_string(), "invalid state transition: Draft -> Fact");

        let err = TypeError::missing_field("content");
        assert_eq!(err.to_string(), "missing required field: content");
    }

    #[test]
    fn promotion_error_display() {
        let err = PromotionError::not_validated("prop-123");
        assert_eq!(err.to_string(), "proposal not validated: prop-123");

        let err = PromotionError::gate_rejected("gate-1", "confidence too low");
        assert_eq!(
            err.to_string(),
            "gate rejected: gate-1 - confidence too low"
        );
    }

    #[test]
    fn validation_error_display() {
        let err = TypesValidationError::invalid_confidence(1.5);
        assert_eq!(err.to_string(), "invalid confidence: 1.5 (must be 0.0-1.0)");

        let err = TypesValidationError::EmptyContent;
        assert_eq!(err.to_string(), "empty content not allowed");
    }

    #[test]
    fn validation_error_serialization() {
        let err = TypesValidationError::missing_field("required_field");
        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("\"MissingField\""));
        assert!(json.contains("\"required_field\""));

        let deserialized: TypesValidationError = serde_json::from_str(&json).unwrap();
        assert_eq!(err, deserialized);
    }

    #[test]
    fn observation_error_display() {
        let err = ObservationError::invalid_response("malformed JSON");
        assert_eq!(err.to_string(), "invalid provider response: malformed JSON");

        let err = ObservationError::timeout(5000);
        assert_eq!(err.to_string(), "observation capture timeout after 5000ms");
    }

    #[test]
    fn correction_error_display() {
        let err = CorrectionError::fact_not_found("fact-123");
        assert_eq!(err.to_string(), "fact not found: fact-123");

        let err = CorrectionError::already_superseded("fact-456");
        assert_eq!(err.to_string(), "already superseded by: fact-456");
    }

    #[test]
    fn type_error_from_serde() {
        // Test transparent conversion from serde_json::Error
        let bad_json = "not json";
        let serde_err: Result<serde_json::Value, _> = serde_json::from_str(bad_json);
        let type_err: TypeError = serde_err.unwrap_err().into();
        assert!(type_err.to_string().contains("expected"));
    }

    // ========================================================================
    // TypeError — all variants
    // ========================================================================

    #[test]
    fn type_error_promotion_invariant() {
        let err = TypeError::promotion_invariant("must have evidence");
        assert_eq!(
            err.to_string(),
            "promotion invariant violated: must have evidence"
        );
    }

    #[test]
    fn type_error_validation_failed() {
        let err = TypeError::validation_failed("confidence below threshold");
        assert_eq!(
            err.to_string(),
            "validation failed: confidence below threshold"
        );
    }

    #[test]
    fn type_error_is_std_error() {
        let err = TypeError::missing_field("id");
        let _: &dyn std::error::Error = &err;
    }

    // ========================================================================
    // PromotionError — all variants
    // ========================================================================

    #[test]
    fn promotion_error_validation_failed() {
        let err = PromotionError::validation_failed("schema mismatch");
        assert_eq!(err.to_string(), "validation failed: schema mismatch");
    }

    #[test]
    fn promotion_error_missing_evidence() {
        let err = PromotionError::missing_evidence("no supporting observations");
        assert_eq!(
            err.to_string(),
            "missing evidence: no supporting observations"
        );
    }

    #[test]
    fn promotion_error_human_approval_required() {
        let err = PromotionError::HumanApprovalRequired;
        assert_eq!(err.to_string(), "human approval required");
    }

    #[test]
    fn promotion_error_policy_version_mismatch() {
        let err = PromotionError::PolicyVersionMismatch {
            expected: "v2".into(),
            actual: "v1".into(),
        };
        assert_eq!(
            err.to_string(),
            "policy version mismatch: expected v2, got v1"
        );
    }

    #[test]
    fn promotion_error_report_mismatch() {
        let err = PromotionError::report_mismatch("prop-1", "prop-2");
        assert_eq!(
            err.to_string(),
            "report mismatch: proposal prop-1 vs report for prop-2"
        );
    }

    #[test]
    fn promotion_error_is_std_error() {
        let err = PromotionError::HumanApprovalRequired;
        let _: &dyn std::error::Error = &err;
    }

    // ========================================================================
    // ObservationError — all variants
    // ========================================================================

    #[test]
    fn observation_error_content_hash_mismatch() {
        let err = ObservationError::ContentHashMismatch;
        assert_eq!(err.to_string(), "content hash mismatch");
    }

    #[test]
    fn observation_error_missing_context() {
        let err = ObservationError::missing_context("session_id");
        assert_eq!(err.to_string(), "capture context missing: session_id");
    }

    #[test]
    fn observation_error_provider_not_available() {
        let err = ObservationError::provider_not_available("anthropic");
        assert_eq!(err.to_string(), "provider not available: anthropic");
    }

    #[test]
    fn observation_error_is_std_error() {
        let err = ObservationError::ContentHashMismatch;
        let _: &dyn std::error::Error = &err;
    }

    // ========================================================================
    // TypesValidationError — all variants
    // ========================================================================

    #[test]
    fn validation_error_constraint_violated() {
        let err = TypesValidationError::constraint_violated("budget exceeded");
        assert_eq!(err.to_string(), "constraint violated: budget exceeded");
    }

    #[test]
    fn validation_error_schema_mismatch() {
        let err = TypesValidationError::schema_mismatch("json", "xml");
        assert_eq!(
            err.to_string(),
            "schema mismatch: expected json vs actual xml"
        );
    }

    #[test]
    fn validation_error_invalid_format() {
        let err = TypesValidationError::invalid_format("expected ISO-8601");
        assert_eq!(err.to_string(), "invalid format: expected ISO-8601");
    }

    #[test]
    fn validation_error_missing_field() {
        let err = TypesValidationError::missing_field("name");
        assert_eq!(err.to_string(), "missing required field: name");
    }

    #[test]
    fn validation_error_all_variants_serialize() {
        let variants: Vec<TypesValidationError> = vec![
            TypesValidationError::invalid_confidence(2.0),
            TypesValidationError::EmptyContent,
            TypesValidationError::missing_field("x"),
            TypesValidationError::constraint_violated("c"),
            TypesValidationError::schema_mismatch("a", "b"),
            TypesValidationError::invalid_format("f"),
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let round: TypesValidationError = serde_json::from_str(&json).unwrap();
            assert_eq!(*v, round);
        }
    }

    #[test]
    fn validation_error_equality() {
        let a = TypesValidationError::EmptyContent;
        let b = TypesValidationError::EmptyContent;
        assert_eq!(a, b);

        let c = TypesValidationError::missing_field("x");
        let d = TypesValidationError::missing_field("y");
        assert_ne!(c, d);
    }

    // ========================================================================
    // CorrectionError — all variants
    // ========================================================================

    #[test]
    fn correction_error_scope_mismatch() {
        let err = CorrectionError::scope_mismatch("global");
        assert_eq!(
            err.to_string(),
            "scope mismatch: correction scope global doesn't match fact scope"
        );
    }

    #[test]
    fn correction_error_not_authorized() {
        let err = CorrectionError::not_authorized("fact-99");
        assert_eq!(err.to_string(), "not authorized to correct fact: fact-99");
    }

    #[test]
    fn correction_error_invalid_chain() {
        let err = CorrectionError::invalid_chain("cycle detected");
        assert_eq!(err.to_string(), "invalid correction chain: cycle detected");
    }

    #[test]
    fn correction_error_is_std_error() {
        let err = CorrectionError::fact_not_found("f-1");
        let _: &dyn std::error::Error = &err;
    }
}
