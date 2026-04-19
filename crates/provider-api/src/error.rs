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
/// use converge_provider_api::{BackendError, BackendErrorKind};
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
