// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Capability Error Infrastructure
//!
//! This module defines the shared error classification interface for all capability
//! boundary traits. It provides a uniform way to categorize, classify, and handle
//! errors from external capabilities (LLM providers, vector stores, event stores, etc.).
//!
//! ## Design Philosophy
//!
//! - **Shared classification, distinct types:** Each capability defines its own error
//!   enum (e.g., `LlmError`, `RecallError`, `StoreError`), but all implement
//!   [`CapabilityError`] for uniform handling.
//!
//! - **Transient vs retryable distinction:**
//!   - `is_transient()` = the underlying condition may clear without changing the request
//!   - `is_retryable()` = it makes sense to retry given typical idempotency guarantees
//!   - These often overlap but are semantically different. A transient error (server
//!     temporarily overloaded) is usually retryable. But some retryable errors (conflict
//!     after optimistic locking) are not transient—the condition won't clear on its own.
//!
//! - **Category enables generic handling:** [`ErrorCategory`] allows middleware (retry
//!   policies, circuit breakers, rate limiters) to operate generically without knowing
//!   the specific capability or error type.
//!
//! ## Usage
//!
//! Capability error types implement [`CapabilityError`]:
//!
//! ```ignore
//! impl CapabilityError for LlmError {
//!     fn category(&self) -> ErrorCategory {
//!         match self {
//!             LlmError::RateLimited { .. } => ErrorCategory::RateLimit,
//!             LlmError::Timeout { .. } => ErrorCategory::Timeout,
//!             LlmError::AuthDenied { .. } => ErrorCategory::Auth,
//!             // ...
//!         }
//!     }
//!
//!     fn is_transient(&self) -> bool {
//!         matches!(self, LlmError::RateLimited { .. } | LlmError::Timeout { .. })
//!     }
//!
//!     fn is_retryable(&self) -> bool {
//!         self.is_transient() // Often the same, but can differ
//!     }
//!
//!     fn retry_after(&self) -> Option<Duration> {
//!         match self {
//!             LlmError::RateLimited { retry_after } => Some(*retry_after),
//!             _ => None,
//!         }
//!     }
//! }
//! ```
//!
//! Generic retry logic can then work across capabilities:
//!
//! ```ignore
//! async fn with_retry<T, E: CapabilityError>(
//!     mut f: impl FnMut() -> Result<T, E>
//! ) -> Result<T, E> {
//!     loop {
//!         match f() {
//!             Ok(v) => return Ok(v),
//!             Err(e) if e.is_retryable() => {
//!                 if let Some(delay) = e.retry_after() {
//!                     sleep(delay).await;
//!                 }
//!                 continue;
//!             }
//!             Err(e) => return Err(e),
//!         }
//!     }
//! }
//! ```

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Classification of error conditions for generic handling.
///
/// This enum enables middleware (retry policies, circuit breakers, rate limiters,
/// alerting) to operate generically without knowing the specific capability or
/// error type.
///
/// # Categories
///
/// - [`Timeout`](Self::Timeout) - Operation exceeded time limit
/// - [`RateLimit`](Self::RateLimit) - Too many requests, backoff required
/// - [`Auth`](Self::Auth) - Authentication or authorization failure
/// - [`InvalidInput`](Self::InvalidInput) - Bad request parameters
/// - [`NotFound`](Self::NotFound) - Requested resource doesn't exist
/// - [`Conflict`](Self::Conflict) - Resource state conflict (optimistic locking, etc.)
/// - [`Unavailable`](Self::Unavailable) - Service temporarily unavailable
/// - [`InvariantViolation`](Self::InvariantViolation) - System invariant broken (Converge axiom violation)
/// - [`Internal`](Self::Internal) - Unexpected internal error
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Operation exceeded time limit.
    ///
    /// Typically transient and retryable.
    Timeout,

    /// Too many requests, backoff required.
    ///
    /// Transient by nature. Check `retry_after()` for suggested delay.
    RateLimit,

    /// Authentication or authorization failure.
    ///
    /// Usually NOT transient—credentials need to be refreshed or permissions granted.
    Auth,

    /// Bad request parameters.
    ///
    /// NOT transient or retryable—the request itself is invalid.
    InvalidInput,

    /// Requested resource doesn't exist.
    ///
    /// NOT transient unless the resource might be created by another process.
    NotFound,

    /// Resource state conflict (optimistic locking, etc.).
    ///
    /// May be retryable (re-fetch and retry with new version) but not transient.
    Conflict,

    /// Service temporarily unavailable.
    ///
    /// Transient and retryable. Backend is down or unreachable.
    Unavailable,

    /// System invariant broken (Converge axiom violation).
    ///
    /// NEVER retryable. Indicates a bug or corruption. Should alert immediately.
    InvariantViolation,

    /// Unexpected internal error.
    ///
    /// May or may not be transient. Often indicates bugs or unhandled edge cases.
    Internal,
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(f, "timeout"),
            Self::RateLimit => write!(f, "rate_limit"),
            Self::Auth => write!(f, "auth"),
            Self::InvalidInput => write!(f, "invalid_input"),
            Self::NotFound => write!(f, "not_found"),
            Self::Conflict => write!(f, "conflict"),
            Self::Unavailable => write!(f, "unavailable"),
            Self::InvariantViolation => write!(f, "invariant_violation"),
            Self::Internal => write!(f, "internal"),
        }
    }
}

/// Shared classification interface for capability errors.
///
/// All capability-specific error types (e.g., `LlmError`, `RecallError`, `StoreError`)
/// implement this trait to enable uniform error handling across the system.
///
/// # Semantic Distinction: Transient vs Retryable
///
/// - **`is_transient()`**: The underlying condition may clear without changing the request.
///   Examples: rate limiting (quota resets), timeout (server was busy), network blip.
///
/// - **`is_retryable()`**: It makes sense to retry the operation given typical idempotency.
///   Examples: transient errors are usually retryable, but also conflicts (re-fetch and retry
///   with updated version), or certain auth errors (token expired, can refresh).
///
/// These often overlap but serve different purposes:
/// - A circuit breaker cares about transient errors (to detect unhealthy backends).
/// - A retry loop cares about retryable errors (to know whether to attempt again).
///
/// # Implementation Notes
///
/// All implementations must also implement `std::error::Error` and be `Send + Sync`
/// to ensure thread-safe error handling in async contexts.
///
/// # Example
///
/// ```ignore
/// impl CapabilityError for MyError {
///     fn category(&self) -> ErrorCategory {
///         match self {
///             Self::TimedOut => ErrorCategory::Timeout,
///             Self::BadInput(_) => ErrorCategory::InvalidInput,
///             // ...
///         }
///     }
///
///     fn is_transient(&self) -> bool {
///         matches!(self.category(), ErrorCategory::Timeout | ErrorCategory::Unavailable)
///     }
///
///     fn is_retryable(&self) -> bool {
///         self.is_transient() || matches!(self.category(), ErrorCategory::Conflict)
///     }
///
///     fn retry_after(&self) -> Option<Duration> {
///         None // Override when rate limit info available
///     }
/// }
/// ```
pub trait CapabilityError: std::error::Error + Send + Sync {
    /// Returns the category of this error for generic handling.
    ///
    /// Categories enable middleware to operate without knowing specific error types.
    fn category(&self) -> ErrorCategory;

    /// Returns `true` if the underlying condition may clear without changing the request.
    ///
    /// Transient errors indicate temporary conditions like rate limiting, network
    /// issues, or service overload. Circuit breakers use this to detect unhealthy backends.
    fn is_transient(&self) -> bool;

    /// Returns `true` if retrying the operation makes sense given typical idempotency.
    ///
    /// Retryable errors include transient errors, but also cases like conflicts where
    /// re-fetching and retrying with updated state may succeed.
    fn is_retryable(&self) -> bool;

    /// Returns the suggested delay before retrying, if known.
    ///
    /// Primarily used for rate limiting where the backend specifies a backoff period.
    /// Returns `None` if no specific delay is suggested.
    fn retry_after(&self) -> Option<Duration>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_category_display() {
        assert_eq!(ErrorCategory::Timeout.to_string(), "timeout");
        assert_eq!(ErrorCategory::RateLimit.to_string(), "rate_limit");
        assert_eq!(ErrorCategory::Auth.to_string(), "auth");
        assert_eq!(ErrorCategory::InvalidInput.to_string(), "invalid_input");
        assert_eq!(ErrorCategory::NotFound.to_string(), "not_found");
        assert_eq!(ErrorCategory::Conflict.to_string(), "conflict");
        assert_eq!(ErrorCategory::Unavailable.to_string(), "unavailable");
        assert_eq!(
            ErrorCategory::InvariantViolation.to_string(),
            "invariant_violation"
        );
        assert_eq!(ErrorCategory::Internal.to_string(), "internal");
    }

    #[test]
    fn error_category_equality() {
        assert_eq!(ErrorCategory::Timeout, ErrorCategory::Timeout);
        assert_ne!(ErrorCategory::Timeout, ErrorCategory::RateLimit);
    }

    #[test]
    fn error_category_serde_roundtrip() {
        let categories = [
            ErrorCategory::Timeout,
            ErrorCategory::RateLimit,
            ErrorCategory::Auth,
            ErrorCategory::InvalidInput,
            ErrorCategory::NotFound,
            ErrorCategory::Conflict,
            ErrorCategory::Unavailable,
            ErrorCategory::InvariantViolation,
            ErrorCategory::Internal,
        ];
        for cat in categories {
            let json = serde_json::to_string(&cat).unwrap();
            let back: ErrorCategory = serde_json::from_str(&json).unwrap();
            assert_eq!(cat, back);
        }
    }

    #[derive(Debug)]
    struct TestError {
        cat: ErrorCategory,
        transient: bool,
    }

    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "test error: {}", self.cat)
        }
    }

    impl std::error::Error for TestError {}

    impl CapabilityError for TestError {
        fn category(&self) -> ErrorCategory {
            self.cat
        }
        fn is_transient(&self) -> bool {
            self.transient
        }
        fn is_retryable(&self) -> bool {
            self.transient
        }
        fn retry_after(&self) -> Option<Duration> {
            if self.cat == ErrorCategory::RateLimit {
                Some(Duration::from_secs(5))
            } else {
                None
            }
        }
    }

    #[test]
    fn capability_error_trait_implementation() {
        let err = TestError {
            cat: ErrorCategory::RateLimit,
            transient: true,
        };
        assert_eq!(err.category(), ErrorCategory::RateLimit);
        assert!(err.is_transient());
        assert!(err.is_retryable());
        assert_eq!(err.retry_after(), Some(Duration::from_secs(5)));
    }

    #[test]
    fn non_transient_error_not_retryable() {
        let err = TestError {
            cat: ErrorCategory::InvalidInput,
            transient: false,
        };
        assert!(!err.is_transient());
        assert!(!err.is_retryable());
        assert!(err.retry_after().is_none());
    }
}
