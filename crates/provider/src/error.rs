// Copyright 2024-2026 Reflective Labs

// SPDX-License-Identifier: MIT

//! Generic backend error types.
//!
//! These errors are backend-agnostic. Any backend kind (LLM, policy,
//! optimization, analytics) uses the same error structure, making
//! error handling uniform across the platform.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::capability::Capability;

/// Error from any backend operation.
///
/// This is the universal error type for all backends. It captures the error
/// kind, a human-readable message, and whether the operation can be retried.
///
/// # Retryable Errors
///
/// Some errors are transient (network issues, rate limits) and can be retried.
/// Use [`is_retryable()`](BackendError::is_retryable) to check.
///
/// # Example
///
/// ```
/// use converge_provider::{BackendError, BackendErrorKind};
///
/// let err = BackendError::new(BackendErrorKind::Timeout, "operation timed out");
/// assert!(err.is_retryable());
///
/// let err = BackendError::new(BackendErrorKind::InvalidRequest, "missing field");
/// assert!(!err.is_retryable());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Error)]
#[error("{kind}: {message}")]
pub struct BackendError {
    /// Error category.
    pub kind: BackendErrorKind,
    /// Human-readable description.
    pub message: String,
    /// Whether this operation can be retried.
    pub retryable: bool,
}

impl BackendError {
    /// Creates a new backend error with automatic retryable detection.
    #[must_use]
    pub fn new(kind: BackendErrorKind, message: impl Into<String>) -> Self {
        let retryable = kind.is_retryable();
        Self {
            kind,
            message: message.into(),
            retryable,
        }
    }

    /// Creates a new backend error with explicit retryable flag.
    #[must_use]
    pub fn with_retryable(
        kind: BackendErrorKind,
        message: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable,
        }
    }

    /// Whether this error can be retried.
    #[must_use]
    pub fn is_retryable(&self) -> bool {
        self.retryable
    }

    // ── Convenience constructors ──────────────────────────────────────

    /// Authentication or authorization failure.
    #[must_use]
    pub fn auth(message: impl Into<String>) -> Self {
        Self::new(BackendErrorKind::Authentication, message)
    }

    /// Rate limit or quota exceeded.
    #[must_use]
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::new(BackendErrorKind::RateLimit, message)
    }

    /// Invalid request parameters.
    #[must_use]
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self::new(BackendErrorKind::InvalidRequest, message)
    }

    /// Backend not available.
    #[must_use]
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::new(BackendErrorKind::Unavailable, message)
    }

    /// Network or connection error.
    #[must_use]
    pub fn network(message: impl Into<String>) -> Self {
        Self::new(BackendErrorKind::Network, message)
    }

    /// Backend returned an error.
    #[must_use]
    pub fn backend(message: impl Into<String>) -> Self {
        Self::new(BackendErrorKind::BackendError, message)
    }

    /// Response could not be parsed.
    #[must_use]
    pub fn parse(message: impl Into<String>) -> Self {
        Self::new(BackendErrorKind::ParseError, message)
    }

    /// Operation timed out.
    #[must_use]
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::new(BackendErrorKind::Timeout, message)
    }

    /// Capability not supported.
    #[must_use]
    pub fn unsupported(capability: &Capability) -> Self {
        Self::new(
            BackendErrorKind::UnsupportedCapability,
            format!("capability not supported: {capability}"),
        )
    }

    /// Resource exhausted (budget, memory, etc.).
    #[must_use]
    pub fn resource_exhausted(message: impl Into<String>) -> Self {
        Self::new(BackendErrorKind::ResourceExhausted, message)
    }
}

/// Kind of backend error.
///
/// These categories are universal across all backend types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendErrorKind {
    /// Authentication or authorization failure.
    Authentication,
    /// Rate limit or quota exceeded.
    RateLimit,
    /// Invalid request parameters.
    InvalidRequest,
    /// Backend not available or not found.
    Unavailable,
    /// Network or connection error.
    Network,
    /// Backend returned an error.
    BackendError,
    /// Response could not be parsed.
    ParseError,
    /// Operation timed out.
    Timeout,
    /// Capability not supported by this backend.
    UnsupportedCapability,
    /// Resource exhausted (budget, memory, compute).
    ResourceExhausted,
    /// Configuration error.
    Configuration,
}

impl BackendErrorKind {
    /// Whether errors of this kind are typically retryable.
    #[must_use]
    pub fn is_retryable(self) -> bool {
        matches!(
            self,
            Self::RateLimit | Self::Unavailable | Self::Network | Self::Timeout
        )
    }
}

impl std::fmt::Display for BackendErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Authentication => write!(f, "authentication"),
            Self::RateLimit => write!(f, "rate_limit"),
            Self::InvalidRequest => write!(f, "invalid_request"),
            Self::Unavailable => write!(f, "unavailable"),
            Self::Network => write!(f, "network"),
            Self::BackendError => write!(f, "backend_error"),
            Self::ParseError => write!(f, "parse_error"),
            Self::Timeout => write!(f, "timeout"),
            Self::UnsupportedCapability => write!(f, "unsupported_capability"),
            Self::ResourceExhausted => write!(f, "resource_exhausted"),
            Self::Configuration => write!(f, "configuration"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_sets_retryable_from_kind() {
        let err = BackendError::new(BackendErrorKind::RateLimit, "slow down");
        assert_eq!(err.kind, BackendErrorKind::RateLimit);
        assert_eq!(err.message, "slow down");
        assert!(err.is_retryable());

        let err = BackendError::new(BackendErrorKind::InvalidRequest, "bad");
        assert!(!err.is_retryable());
    }

    #[test]
    fn with_retryable_overrides_default() {
        let err = BackendError::with_retryable(BackendErrorKind::InvalidRequest, "retry me", true);
        assert!(err.is_retryable());

        let err = BackendError::with_retryable(BackendErrorKind::RateLimit, "no retry", false);
        assert!(!err.is_retryable());
    }

    #[test]
    fn convenience_auth() {
        let err = BackendError::auth("denied");
        assert_eq!(err.kind, BackendErrorKind::Authentication);
        assert!(!err.is_retryable());
    }

    #[test]
    fn convenience_rate_limit() {
        let err = BackendError::rate_limit("quota exceeded");
        assert_eq!(err.kind, BackendErrorKind::RateLimit);
        assert!(err.is_retryable());
    }

    #[test]
    fn convenience_invalid_request() {
        let err = BackendError::invalid_request("missing field");
        assert_eq!(err.kind, BackendErrorKind::InvalidRequest);
        assert!(!err.is_retryable());
    }

    #[test]
    fn convenience_unavailable() {
        let err = BackendError::unavailable("down");
        assert_eq!(err.kind, BackendErrorKind::Unavailable);
        assert!(err.is_retryable());
    }

    #[test]
    fn convenience_network() {
        let err = BackendError::network("connection refused");
        assert_eq!(err.kind, BackendErrorKind::Network);
        assert!(err.is_retryable());
    }

    #[test]
    fn convenience_backend() {
        let err = BackendError::backend("500");
        assert_eq!(err.kind, BackendErrorKind::BackendError);
        assert!(!err.is_retryable());
    }

    #[test]
    fn convenience_parse() {
        let err = BackendError::parse("invalid json");
        assert_eq!(err.kind, BackendErrorKind::ParseError);
        assert!(!err.is_retryable());
    }

    #[test]
    fn convenience_timeout() {
        let err = BackendError::timeout("10s elapsed");
        assert_eq!(err.kind, BackendErrorKind::Timeout);
        assert!(err.is_retryable());
    }

    #[test]
    fn convenience_unsupported() {
        let err = BackendError::unsupported(&Capability::ImageUnderstanding);
        assert_eq!(err.kind, BackendErrorKind::UnsupportedCapability);
        assert!(!err.is_retryable());
        assert!(err.message.contains("ImageUnderstanding"));
    }

    #[test]
    fn convenience_resource_exhausted() {
        let err = BackendError::resource_exhausted("out of memory");
        assert_eq!(err.kind, BackendErrorKind::ResourceExhausted);
        assert!(!err.is_retryable());
    }

    #[test]
    fn kind_is_retryable() {
        assert!(!BackendErrorKind::Authentication.is_retryable());
        assert!(BackendErrorKind::RateLimit.is_retryable());
        assert!(!BackendErrorKind::InvalidRequest.is_retryable());
        assert!(BackendErrorKind::Unavailable.is_retryable());
        assert!(BackendErrorKind::Network.is_retryable());
        assert!(!BackendErrorKind::BackendError.is_retryable());
        assert!(!BackendErrorKind::ParseError.is_retryable());
        assert!(BackendErrorKind::Timeout.is_retryable());
        assert!(!BackendErrorKind::UnsupportedCapability.is_retryable());
        assert!(!BackendErrorKind::ResourceExhausted.is_retryable());
        assert!(!BackendErrorKind::Configuration.is_retryable());
    }

    #[test]
    fn kind_display() {
        assert_eq!(
            BackendErrorKind::Authentication.to_string(),
            "authentication"
        );
        assert_eq!(BackendErrorKind::RateLimit.to_string(), "rate_limit");
        assert_eq!(
            BackendErrorKind::InvalidRequest.to_string(),
            "invalid_request"
        );
        assert_eq!(BackendErrorKind::Unavailable.to_string(), "unavailable");
        assert_eq!(BackendErrorKind::Network.to_string(), "network");
        assert_eq!(BackendErrorKind::BackendError.to_string(), "backend_error");
        assert_eq!(BackendErrorKind::ParseError.to_string(), "parse_error");
        assert_eq!(BackendErrorKind::Timeout.to_string(), "timeout");
        assert_eq!(
            BackendErrorKind::UnsupportedCapability.to_string(),
            "unsupported_capability"
        );
        assert_eq!(
            BackendErrorKind::ResourceExhausted.to_string(),
            "resource_exhausted"
        );
        assert_eq!(BackendErrorKind::Configuration.to_string(), "configuration");
    }

    #[test]
    fn backend_error_display() {
        let err = BackendError::new(BackendErrorKind::Timeout, "operation timed out");
        assert_eq!(err.to_string(), "timeout: operation timed out");
    }
}
