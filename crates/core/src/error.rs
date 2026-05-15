// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Error types for Converge.

use serde::Serialize;
use thiserror::Error;

use crate::context::ContextState;
use crate::gates::StopReason;
use crate::invariant::InvariantClass;

/// Top-level error type for Converge operations.
///
/// Note: Context is boxed in error variants to keep the error type small,
/// as recommended by clippy. Access via `error.context()` method.
#[derive(Debug, Error, Serialize)]
pub enum ConvergeError {
    /// Budget limit exceeded (cycles, facts, or time).
    #[error("budget exhausted: {kind}")]
    BudgetExhausted { kind: String },

    /// An invariant was violated during execution.
    #[error("{class:?} invariant '{name}' violated: {reason}")]
    InvariantViolation {
        /// Name of the violated invariant.
        name: String,
        /// Class of the invariant (Structural, Semantic, Acceptance).
        class: InvariantClass,
        /// Reason for the violation.
        reason: String,
        /// Final context state (including diagnostic facts). Boxed to reduce error size.
        context: Box<ContextState>,
    },

    /// Suggestor execution failed.
    #[error("agent failed: {agent_id}")]
    AgentFailed { agent_id: String },

    /// A fact-emitting suggestor returned an empty `provenance()` string.
    /// The empty-provenance contract reserves `""` for filter / observer
    /// suggestors that never emit proposals. If a suggestor produced
    /// proposals it MUST override `provenance()` to return its crate's
    /// canonical `*_PROVENANCE.as_str()`.
    #[error(
        "suggestor '{suggestor}' emitted proposals with empty provenance — \
        override Suggestor::provenance() to return your crate's canonical \
        ProvenanceSource string"
    )]
    EmptyProvenance {
        /// Name of the offending suggestor (from `Suggestor::name()`).
        suggestor: String,
    },

    /// Invalid HITL gate resume (e.g., gate_id mismatch between decision and pause).
    #[error("invalid gate resume: {reason}")]
    InvalidResume {
        /// What went wrong.
        reason: String,
    },

    /// Invalid external observation admission.
    #[error("invalid admission: {reason}")]
    InvalidAdmission {
        /// What went wrong.
        reason: String,
    },

    /// Invalid persisted context snapshot.
    #[error("invalid context snapshot: {reason}")]
    InvalidSnapshot {
        /// What went wrong.
        reason: String,
    },

    /// Conflicting facts detected for the same ID.
    #[error(
        "conflict detected for fact '{id}': existing content '{existing}' vs new content '{new}'"
    )]
    Conflict {
        /// ID of the conflicting fact.
        id: String,
        /// Existing content.
        existing: String,
        /// New conflicting content.
        new: String,
        /// Final context state. Boxed to reduce error size.
        context: Box<ContextState>,
    },
}

impl ConvergeError {
    /// Returns a reference to the context if this error variant carries one.
    #[must_use]
    pub fn context(&self) -> Option<&ContextState> {
        match self {
            Self::InvariantViolation { context, .. } | Self::Conflict { context, .. } => {
                Some(context)
            }
            Self::BudgetExhausted { .. }
            | Self::AgentFailed { .. }
            | Self::EmptyProvenance { .. }
            | Self::InvalidResume { .. }
            | Self::InvalidAdmission { .. }
            | Self::InvalidSnapshot { .. } => None,
        }
    }

    /// Convert this error into a platform-level stop reason.
    #[must_use]
    pub fn stop_reason(&self) -> StopReason {
        match self {
            Self::BudgetExhausted { kind } => StopReason::Error {
                message: format!("budget exhausted: {kind}"),
                category: crate::gates::ErrorCategory::Resource,
            },
            Self::InvariantViolation {
                name,
                class,
                reason,
                ..
            } => StopReason::invariant_violated(*class, name.clone(), reason.clone()),
            Self::AgentFailed { agent_id } => StopReason::AgentRefused {
                agent_id: agent_id.clone(),
                reason: "agent execution failed".to_string(),
            },
            Self::EmptyProvenance { suggestor } => StopReason::AgentRefused {
                agent_id: suggestor.clone(),
                reason: "suggestor emitted proposals with empty provenance".to_string(),
            },
            Self::InvalidResume { reason } => StopReason::Error {
                message: format!("invalid gate resume: {reason}"),
                category: crate::gates::ErrorCategory::Internal,
            },
            Self::InvalidAdmission { reason } => StopReason::Error {
                message: format!("invalid admission: {reason}"),
                category: crate::gates::ErrorCategory::Configuration,
            },
            Self::InvalidSnapshot { reason } => StopReason::Error {
                message: format!("invalid context snapshot: {reason}"),
                category: crate::gates::ErrorCategory::Configuration,
            },
            Self::Conflict {
                id, existing, new, ..
            } => StopReason::Error {
                message: format!("conflict for fact '{id}': existing '{existing}' vs new '{new}'"),
                category: crate::gates::ErrorCategory::Internal,
            },
        }
    }
}

impl From<crate::AdmissionError> for ConvergeError {
    fn from(value: crate::AdmissionError) -> Self {
        Self::InvalidAdmission {
            reason: value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_context() -> ContextState {
        ContextState::default()
    }

    #[test]
    fn budget_exhausted_display() {
        let err = ConvergeError::BudgetExhausted {
            kind: "cycles".into(),
        };
        assert_eq!(err.to_string(), "budget exhausted: cycles");
    }

    #[test]
    fn budget_exhausted_has_no_context() {
        let err = ConvergeError::BudgetExhausted {
            kind: "tokens".into(),
        };
        assert!(err.context().is_none());
    }

    #[test]
    fn agent_failed_display() {
        let err = ConvergeError::AgentFailed {
            agent_id: "agent-x".into(),
        };
        assert_eq!(err.to_string(), "agent failed: agent-x");
    }

    #[test]
    fn agent_failed_has_no_context() {
        let err = ConvergeError::AgentFailed {
            agent_id: "a".into(),
        };
        assert!(err.context().is_none());
    }

    #[test]
    fn invariant_violation_has_context() {
        let err = ConvergeError::InvariantViolation {
            name: "no_empty".into(),
            class: InvariantClass::Structural,
            reason: "empty found".into(),
            context: Box::new(empty_context()),
        };
        assert!(err.context().is_some());
    }

    #[test]
    fn invariant_violation_display() {
        let err = ConvergeError::InvariantViolation {
            name: "no_empty".into(),
            class: InvariantClass::Semantic,
            reason: "bad".into(),
            context: Box::new(empty_context()),
        };
        assert_eq!(
            err.to_string(),
            "Semantic invariant 'no_empty' violated: bad"
        );
    }

    #[test]
    fn conflict_has_context() {
        let err = ConvergeError::Conflict {
            id: "fact-1".into(),
            existing: "old".into(),
            new: "new".into(),
            context: Box::new(empty_context()),
        };
        assert!(err.context().is_some());
    }

    #[test]
    fn conflict_display() {
        let err = ConvergeError::Conflict {
            id: "f".into(),
            existing: "a".into(),
            new: "b".into(),
            context: Box::new(empty_context()),
        };
        assert!(err.to_string().contains("conflict detected for fact 'f'"));
    }

    #[test]
    fn stop_reason_budget_exhausted() {
        let err = ConvergeError::BudgetExhausted {
            kind: "time".into(),
        };
        let reason = err.stop_reason();
        assert!(matches!(reason, StopReason::Error { .. }));
    }

    #[test]
    fn stop_reason_invariant_violated() {
        let err = ConvergeError::InvariantViolation {
            name: "inv".into(),
            class: InvariantClass::Acceptance,
            reason: "fail".into(),
            context: Box::new(empty_context()),
        };
        let reason = err.stop_reason();
        assert!(matches!(reason, StopReason::InvariantViolated { .. }));
    }

    #[test]
    fn stop_reason_agent_refused() {
        let err = ConvergeError::AgentFailed {
            agent_id: "bot".into(),
        };
        let reason = err.stop_reason();
        assert!(matches!(reason, StopReason::AgentRefused { .. }));
    }

    #[test]
    fn invalid_resume_display() {
        let err = ConvergeError::InvalidResume {
            reason: "gate_id mismatch".into(),
        };
        assert_eq!(err.to_string(), "invalid gate resume: gate_id mismatch");
    }

    #[test]
    fn invalid_resume_has_no_context() {
        let err = ConvergeError::InvalidResume {
            reason: "test".into(),
        };
        assert!(err.context().is_none());
    }

    #[test]
    fn stop_reason_invalid_resume() {
        let err = ConvergeError::InvalidResume {
            reason: "wrong gate".into(),
        };
        let reason = err.stop_reason();
        assert!(matches!(reason, StopReason::Error { .. }));
    }

    #[test]
    fn stop_reason_conflict() {
        let err = ConvergeError::Conflict {
            id: "x".into(),
            existing: "old".into(),
            new: "new".into(),
            context: Box::new(empty_context()),
        };
        let reason = err.stop_reason();
        assert!(matches!(reason, StopReason::Error { .. }));
    }
}
