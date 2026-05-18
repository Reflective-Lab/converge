// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Error types for Converge Runtime.

use converge_core::ConvergeError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// Runtime-level errors.
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// Converge engine error.
    #[error("converge error: {0}")]
    Converge(#[from] ConvergeError),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP server error.
    #[error("HTTP error: {0}")]
    Http(#[from] axum::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),

    /// Resource not found.
    #[error("not found: {0}")]
    NotFound(String),

    /// Conflict (e.g., job not in expected state).
    #[error("conflict: {0}")]
    Conflict(String),

    /// Functionality is intentionally not implemented by this runtime.
    #[error("not implemented: {0}")]
    Unimplemented(String),

    /// Authentication error.
    #[error("authentication error: {0}")]
    Authentication(String),

    /// Authorization error.
    #[error("authorization error: {0}")]
    Authorization(String),

    /// Billing/payment error.
    #[cfg(feature = "billing")]
    #[error("billing error: {0}")]
    Billing(String),
}

/// Error response for API.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuntimeErrorResponse {
    /// Error message.
    pub error: String,
    /// HTTP status code.
    pub status: u16,
}

/// Result type for runtime operations.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

impl axum::response::IntoResponse for RuntimeError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            RuntimeError::Converge(e) => {
                let status = match e {
                    ConvergeError::BudgetExhausted { .. } => {
                        axum::http::StatusCode::PAYLOAD_TOO_LARGE
                    }
                    ConvergeError::InvariantViolation { .. } => {
                        axum::http::StatusCode::UNPROCESSABLE_ENTITY
                    }
                    ConvergeError::AgentFailed { .. } => {
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
                    }
                    ConvergeError::EmptyProvenance { .. } => {
                        axum::http::StatusCode::INTERNAL_SERVER_ERROR
                    }
                    ConvergeError::InvalidResume { .. } => axum::http::StatusCode::BAD_REQUEST,
                    ConvergeError::InvalidAdmission { .. } => axum::http::StatusCode::BAD_REQUEST,
                    ConvergeError::InvalidSnapshot { .. } => axum::http::StatusCode::BAD_REQUEST,
                    ConvergeError::Conflict { .. } => axum::http::StatusCode::CONFLICT,
                };
                (status, format!("Converge error: {e}"))
            }
            RuntimeError::Serialization(e) => (
                axum::http::StatusCode::BAD_REQUEST,
                format!("Invalid JSON: {e}"),
            ),
            RuntimeError::Http(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("HTTP error: {e}"),
            ),
            RuntimeError::Io(e) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("I/O error: {e}"),
            ),
            RuntimeError::Config(msg) => (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                format!("Configuration error: {msg}"),
            ),
            RuntimeError::NotFound(msg) => (
                axum::http::StatusCode::NOT_FOUND,
                format!("Not found: {msg}"),
            ),
            RuntimeError::Conflict(msg) => {
                (axum::http::StatusCode::CONFLICT, format!("Conflict: {msg}"))
            }
            RuntimeError::Unimplemented(msg) => (
                axum::http::StatusCode::NOT_IMPLEMENTED,
                format!("Not implemented: {msg}"),
            ),
            RuntimeError::Authentication(msg) => (
                axum::http::StatusCode::UNAUTHORIZED,
                format!("Authentication failed: {msg}"),
            ),
            RuntimeError::Authorization(msg) => (
                axum::http::StatusCode::FORBIDDEN,
                format!("Access denied: {msg}"),
            ),
            #[cfg(feature = "billing")]
            RuntimeError::Billing(msg) => (
                axum::http::StatusCode::PAYMENT_REQUIRED,
                format!("Billing error: {msg}"),
            ),
        };

        let body = RuntimeErrorResponse {
            error: message,
            status: status.as_u16(),
        };

        (status, axum::Json(body)).into_response()
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;

    // -------------------------------------------------------------------------
    // Unit Tests: Error Display
    // -------------------------------------------------------------------------

    #[test]
    fn test_config_error_display() {
        let error = RuntimeError::Config("invalid port".to_string());
        assert_eq!(error.to_string(), "configuration error: invalid port");
    }

    #[test]
    fn test_not_found_error_display() {
        let error = RuntimeError::NotFound("job-123".to_string());
        assert_eq!(error.to_string(), "not found: job-123");
    }

    #[test]
    fn test_conflict_error_display() {
        let error = RuntimeError::Conflict("job already running".to_string());
        assert_eq!(error.to_string(), "conflict: job already running");
    }

    #[test]
    fn test_unimplemented_error_display() {
        let error = RuntimeError::Unimplemented("job store not configured".to_string());
        assert_eq!(
            error.to_string(),
            "not implemented: job store not configured"
        );
    }

    #[test]
    fn test_serialization_error_display() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid").unwrap_err();
        let error = RuntimeError::Serialization(json_err);
        assert!(error.to_string().starts_with("serialization error:"));
    }

    #[test]
    fn test_io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let error = RuntimeError::Io(io_err);
        assert_eq!(error.to_string(), "I/O error: file not found");
    }

    // -------------------------------------------------------------------------
    // Unit Tests: Error Debug
    // -------------------------------------------------------------------------

    #[test]
    fn test_error_debug_format() {
        let error = RuntimeError::Config("test".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_not_found_debug_format() {
        let error = RuntimeError::NotFound("resource-456".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("NotFound"));
        assert!(debug_str.contains("resource-456"));
    }

    // -------------------------------------------------------------------------
    // Unit Tests: Error Conversion (From trait)
    // -------------------------------------------------------------------------

    #[test]
    fn test_from_serde_json_error() {
        let json_err = serde_json::from_str::<serde_json::Value>("}{").unwrap_err();
        let error: RuntimeError = json_err.into();
        assert!(matches!(error, RuntimeError::Serialization(_)));
    }

    #[test]
    fn test_from_io_error() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied");
        let error: RuntimeError = io_err.into();
        assert!(matches!(error, RuntimeError::Io(_)));
    }

    #[test]
    fn test_from_converge_error_budget() {
        let converge_err = ConvergeError::BudgetExhausted {
            kind: "cycles exceeded (limit: 100)".to_string(),
        };
        let error: RuntimeError = converge_err.into();
        assert!(matches!(error, RuntimeError::Converge(_)));
    }

    // -------------------------------------------------------------------------
    // Unit Tests: RuntimeErrorResponse
    // -------------------------------------------------------------------------

    #[test]
    fn test_runtime_error_response_serialization() {
        let response = RuntimeErrorResponse {
            error: "test error".to_string(),
            status: 404,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test error"));
        assert!(json.contains("404"));
    }

    #[test]
    fn test_runtime_error_response_debug() {
        let response = RuntimeErrorResponse {
            error: "debug test".to_string(),
            status: 500,
        };
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("RuntimeErrorResponse"));
        assert!(debug_str.contains("debug test"));
    }

    // -------------------------------------------------------------------------
    // Integration Tests: IntoResponse
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_config_error_response_status() {
        let error = RuntimeError::Config("bad config".to_string());
        let response = error.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn test_not_found_error_response_status() {
        let error = RuntimeError::NotFound("missing resource".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_conflict_error_response_status() {
        let error = RuntimeError::Conflict("state conflict".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_serialization_error_response_status() {
        let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let error = RuntimeError::Serialization(json_err);
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_io_error_response_status() {
        let io_err = std::io::Error::other("io failure");
        let error = RuntimeError::Io(io_err);
        let response = error.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn test_error_response_body_contains_message() {
        let error = RuntimeError::NotFound("job-xyz".to_string());
        let response = error.into_response();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("job-xyz"));
        assert!(body_str.contains("404"));
    }

    #[tokio::test]
    async fn test_error_response_body_is_json() {
        let error = RuntimeError::Config("test".to_string());
        let response = error.into_response();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert!(parsed.get("error").is_some());
        assert!(parsed.get("status").is_some());
    }

    // -------------------------------------------------------------------------
    // Tests: ConvergeError variants mapping to HTTP status codes
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_budget_exhausted_returns_payload_too_large() {
        let converge_err = ConvergeError::BudgetExhausted {
            kind: "facts exceeded (limit: 50)".to_string(),
        };
        let error = RuntimeError::Converge(converge_err);
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn test_invariant_violation_returns_unprocessable_entity() {
        use converge_core::{ContextState, invariant::InvariantClass};
        let converge_err = ConvergeError::InvariantViolation {
            name: "test_invariant".to_string(),
            class: InvariantClass::Structural,
            reason: "failed check".to_string(),
            context: Box::new(ContextState::new()),
        };
        let error = RuntimeError::Converge(converge_err);
        let response = error.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[tokio::test]
    async fn test_agent_failed_returns_internal_server_error() {
        let converge_err = ConvergeError::AgentFailed {
            agent_id: "agent-1".to_string(),
        };
        let error = RuntimeError::Converge(converge_err);
        let response = error.into_response();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn test_invalid_resume_returns_bad_request() {
        let converge_err = ConvergeError::InvalidResume {
            reason: "decision gate_id does not match pause gate_id".to_string(),
        };
        let error = RuntimeError::Converge(converge_err);
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_converge_conflict_returns_conflict() {
        use converge_core::ContextState;
        let converge_err = ConvergeError::Conflict {
            id: "fact-1".to_string(),
            existing: "value1".to_string(),
            new: "value2".to_string(),
            context: Box::new(ContextState::new()),
        };
        let error = RuntimeError::Converge(converge_err);
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_unimplemented_returns_not_implemented() {
        let error = RuntimeError::Unimplemented("job execution is not wired".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), axum::http::StatusCode::NOT_IMPLEMENTED);
    }

    // -------------------------------------------------------------------------
    // Negative Tests: Edge cases
    // -------------------------------------------------------------------------

    #[test]
    fn test_empty_config_error() {
        let error = RuntimeError::Config(String::new());
        assert_eq!(error.to_string(), "configuration error: ");
    }

    #[test]
    fn test_empty_not_found_error() {
        let error = RuntimeError::NotFound(String::new());
        assert_eq!(error.to_string(), "not found: ");
    }

    #[test]
    fn test_unicode_in_error_message() {
        let error = RuntimeError::Config("配置错误 🔧".to_string());
        assert!(error.to_string().contains("配置错误"));
        assert!(error.to_string().contains("🔧"));
    }

    #[test]
    fn test_very_long_error_message() {
        let long_msg = "x".repeat(10000);
        let error = RuntimeError::Config(long_msg.clone());
        assert!(error.to_string().contains(&long_msg));
    }

    #[test]
    fn test_special_characters_in_error() {
        let error = RuntimeError::NotFound("path/with<special>&chars\"".to_string());
        let display = error.to_string();
        assert!(display.contains("path/with<special>&chars\""));
    }
}
