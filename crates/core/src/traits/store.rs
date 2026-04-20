// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Store Capability Traits
//!
//! Durable context snapshot storage for applications with state spanning
//! multiple runs.

use std::future::Future;
use std::time::Duration;

use super::error::{CapabilityError, ErrorCategory};
use crate::context::ContextState;

// ============================================================================
// Store Error
// ============================================================================

/// Error type for store operations.
#[derive(Debug, Clone)]
pub enum StoreError {
    /// Storage backend is temporarily unavailable.
    Unavailable { message: String },
    /// Serialization or deserialization failed.
    SerializationFailed { message: String },
    /// ID already exists (duplicate).
    Conflict { event_id: String },
    /// Query was malformed or invalid.
    InvalidQuery { message: String },
    /// Authentication with the store backend failed.
    AuthFailed { message: String },
    /// Rate limit exceeded; retry after delay.
    RateLimited { retry_after: Duration },
    /// Operation timed out.
    Timeout {
        elapsed: Duration,
        deadline: Duration,
    },
    /// Record not found.
    NotFound { message: String },
    /// Invariant violation in store (should not happen).
    InvariantViolation { message: String },
    /// Internal error with no specific category.
    Internal { message: String },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { message } => write!(f, "store unavailable: {message}"),
            Self::SerializationFailed { message } => write!(f, "serialization failed: {message}"),
            Self::Conflict { event_id } => write!(f, "already exists: {event_id}"),
            Self::InvalidQuery { message } => write!(f, "invalid query: {message}"),
            Self::AuthFailed { message } => write!(f, "store auth failed: {message}"),
            Self::RateLimited { retry_after } => {
                write!(f, "rate limited, retry after {retry_after:?}")
            }
            Self::Timeout { elapsed, deadline } => {
                write!(f, "timed out after {elapsed:?} (deadline: {deadline:?})")
            }
            Self::NotFound { message } => write!(f, "not found: {message}"),
            Self::InvariantViolation { message } => write!(f, "invariant violation: {message}"),
            Self::Internal { message } => write!(f, "internal store error: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl CapabilityError for StoreError {
    fn category(&self) -> ErrorCategory {
        match self {
            Self::Unavailable { .. } => ErrorCategory::Unavailable,
            Self::SerializationFailed { .. } | Self::InvalidQuery { .. } => {
                ErrorCategory::InvalidInput
            }
            Self::Conflict { .. } => ErrorCategory::Conflict,
            Self::AuthFailed { .. } => ErrorCategory::Auth,
            Self::RateLimited { .. } => ErrorCategory::RateLimit,
            Self::Timeout { .. } => ErrorCategory::Timeout,
            Self::NotFound { .. } => ErrorCategory::NotFound,
            Self::InvariantViolation { .. } => ErrorCategory::InvariantViolation,
            Self::Internal { .. } => ErrorCategory::Internal,
        }
    }

    fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Unavailable { .. } | Self::RateLimited { .. } | Self::Timeout { .. }
        )
    }

    fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Unavailable { .. }
                | Self::RateLimited { .. }
                | Self::Timeout { .. }
                | Self::Conflict { .. }
        )
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after } => Some(*retry_after),
            _ => None,
        }
    }
}

// ============================================================================
// ContextStore Trait
// ============================================================================

/// Durable context snapshot storage.
///
/// Applications with state that spans multiple runs need a place to persist
/// and reconstruct the engine context. This trait defines that boundary
/// without prescribing a storage backend.
pub trait ContextStore: Send + Sync {
    /// Future type for loading a context snapshot.
    type LoadFut<'a>: Future<Output = Result<Option<ContextState>, StoreError>> + Send + 'a
    where
        Self: 'a;

    /// Future type for saving a context snapshot.
    type SaveFut<'a>: Future<Output = Result<(), StoreError>> + Send + 'a
    where
        Self: 'a;

    /// Load the latest snapshot for a run, tenant, or application-defined scope.
    fn load_context<'a>(&'a self, scope_id: &'a str) -> Self::LoadFut<'a>;

    /// Persist the latest snapshot for a run, tenant, or application-defined scope.
    fn save_context<'a>(
        &'a self,
        scope_id: &'a str,
        context: &'a ContextState,
    ) -> Self::SaveFut<'a>;
}

// ============================================================================
// Dyn-Safe Wrapper
// ============================================================================

/// Boxed future type for dyn-safe trait objects.
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Dyn-safe context store for runtime polymorphism.
pub trait DynContextStore: Send + Sync {
    /// Load a stored context snapshot.
    fn load_context<'a>(
        &'a self,
        scope_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<ContextState>, StoreError>>;

    /// Save a context snapshot.
    fn save_context<'a>(
        &'a self,
        scope_id: &'a str,
        context: &'a ContextState,
    ) -> BoxFuture<'a, Result<(), StoreError>>;
}

impl<T: ContextStore> DynContextStore for T {
    fn load_context<'a>(
        &'a self,
        scope_id: &'a str,
    ) -> BoxFuture<'a, Result<Option<ContextState>, StoreError>> {
        Box::pin(ContextStore::load_context(self, scope_id))
    }

    fn save_context<'a>(
        &'a self,
        scope_id: &'a str,
        context: &'a ContextState,
    ) -> BoxFuture<'a, Result<(), StoreError>> {
        Box::pin(ContextStore::save_context(self, scope_id, context))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_error_display() {
        let err = StoreError::Conflict {
            event_id: "evt-123".to_string(),
        };
        assert!(err.to_string().contains("evt-123"));
    }

    #[test]
    fn store_error_category_classification() {
        assert_eq!(
            StoreError::Unavailable {
                message: "test".to_string()
            }
            .category(),
            ErrorCategory::Unavailable
        );
        assert_eq!(
            StoreError::Conflict {
                event_id: "test".to_string()
            }
            .category(),
            ErrorCategory::Conflict
        );
    }

    #[test]
    fn store_error_transient_classification() {
        assert!(
            StoreError::Unavailable {
                message: "test".to_string()
            }
            .is_transient()
        );
        assert!(
            !StoreError::Conflict {
                event_id: "test".to_string()
            }
            .is_transient()
        );
    }

    #[test]
    fn store_error_retryable_classification() {
        assert!(
            StoreError::Unavailable {
                message: "test".to_string()
            }
            .is_retryable()
        );
        assert!(
            !StoreError::AuthFailed {
                message: "test".to_string()
            }
            .is_retryable()
        );
    }

    #[test]
    fn store_error_retry_after() {
        let err = StoreError::RateLimited {
            retry_after: Duration::from_secs(60),
        };
        assert_eq!(err.retry_after(), Some(Duration::from_secs(60)));
    }
}
