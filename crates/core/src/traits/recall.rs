// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Recall Capability Traits
//!
//! This module defines the capability boundary traits for Recall (semantic
//! memory). Recall provides **hints** to guide reasoning, not citations to
//! justify claims.
//!
//! ## Split Trait Pattern
//!
//! Recall is split by authority boundary:
//!
//! - [`RecallReader`]: Query-only read access for validation, audit, replay
//! - [`RecallWriter`]: Store/delete mutation for ingestion pipelines
//! - [`Recall`]: Umbrella combining RecallReader + RecallWriter
//!
//! This separation enables read-only contexts (validators, auditors) to depend
//! only on `RecallReader`, preventing accidental mutations.
//!
//! ## GAT Async Pattern
//!
//! All traits use Generic Associated Types (GATs) for zero-cost async:
//!
//! ```ignore
//! pub trait RecallReader: Send + Sync {
//!     type QueryFut<'a>: Future<Output = Result<Vec<RecallCandidate>, RecallError>> + Send + 'a
//!     where
//!         Self: 'a;
//!
//!     fn query<'a>(&'a self, query: &'a RecallQuery) -> Self::QueryFut<'a>;
//! }
//! ```
//!
//! This enables static dispatch async without `async_trait` proc macros or
//! tokio runtime dependency in converge-core.
//!
//! ## Thread Safety
//!
//! All traits require `Send + Sync` to enable use in concurrent contexts.
//! Implementations with non-thread-safe state use `Arc<Mutex<...>>`.
//!
//! ## Error Handling
//!
//! [`RecallError`] implements [`CapabilityError`] for generic retry logic.
//! It provides `is_transient()` and `is_retryable()` classification.

use std::future::Future;
use std::time::Duration;

use super::error::{CapabilityError, ErrorCategory};
use crate::recall::{RecallCandidate, RecallQuery};

// ============================================================================
// Recall Error
// ============================================================================

/// Error type for recall operations.
///
/// All variants implement [`CapabilityError`] for generic error handling.
#[derive(Debug, Clone)]
pub enum RecallError {
    /// Index or vector store is temporarily unavailable.
    IndexUnavailable {
        /// Human-readable description of the unavailability.
        message: String,
    },
    /// Query embedding dimensions don't match index dimensions.
    DimensionMismatch {
        /// Expected dimension count.
        expected: usize,
        /// Actual dimension count from query.
        got: usize,
    },
    /// Embedding operation failed (e.g., rate limited, timeout).
    EmbeddingFailed {
        /// Human-readable error message.
        message: String,
        /// Whether this failure is transient.
        transient: bool,
    },
    /// Query was malformed or invalid.
    InvalidQuery {
        /// Description of what was invalid.
        message: String,
    },
    /// Authentication with the recall backend failed.
    AuthFailed {
        /// Human-readable error message.
        message: String,
    },
    /// Rate limit exceeded; retry after delay.
    RateLimited {
        /// Suggested delay before retry.
        retry_after: Duration,
    },
    /// Operation timed out.
    Timeout {
        /// How long the operation ran before timing out.
        elapsed: Duration,
        /// The configured deadline.
        deadline: Duration,
    },
    /// Record not found for store/delete operation.
    NotFound {
        /// The ID that was not found.
        id: String,
    },
    /// Internal error with no specific category.
    Internal {
        /// Human-readable error message.
        message: String,
    },
}

impl std::fmt::Display for RecallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IndexUnavailable { message } => {
                write!(f, "recall index unavailable: {}", message)
            }
            Self::DimensionMismatch { expected, got } => {
                write!(
                    f,
                    "dimension mismatch: expected {} dimensions, got {}",
                    expected, got
                )
            }
            Self::EmbeddingFailed { message, .. } => {
                write!(f, "embedding failed: {}", message)
            }
            Self::InvalidQuery { message } => {
                write!(f, "invalid recall query: {}", message)
            }
            Self::AuthFailed { message } => {
                write!(f, "recall auth failed: {}", message)
            }
            Self::RateLimited { retry_after } => {
                write!(f, "rate limited, retry after {:?}", retry_after)
            }
            Self::Timeout { elapsed, deadline } => {
                write!(
                    f,
                    "recall operation timed out after {:?} (deadline: {:?})",
                    elapsed, deadline
                )
            }
            Self::NotFound { id } => {
                write!(f, "recall record not found: {}", id)
            }
            Self::Internal { message } => {
                write!(f, "internal recall error: {}", message)
            }
        }
    }
}

impl std::error::Error for RecallError {}

impl CapabilityError for RecallError {
    fn category(&self) -> ErrorCategory {
        match self {
            Self::IndexUnavailable { .. } => ErrorCategory::Unavailable,
            Self::DimensionMismatch { .. } => ErrorCategory::InvalidInput,
            Self::EmbeddingFailed { .. } => ErrorCategory::Internal,
            Self::InvalidQuery { .. } => ErrorCategory::InvalidInput,
            Self::AuthFailed { .. } => ErrorCategory::Auth,
            Self::RateLimited { .. } => ErrorCategory::RateLimit,
            Self::Timeout { .. } => ErrorCategory::Timeout,
            Self::NotFound { .. } => ErrorCategory::NotFound,
            Self::Internal { .. } => ErrorCategory::Internal,
        }
    }

    fn is_transient(&self) -> bool {
        match self {
            Self::IndexUnavailable { .. } => true,
            Self::DimensionMismatch { .. } => false,
            Self::EmbeddingFailed { transient, .. } => *transient,
            Self::InvalidQuery { .. } => false,
            Self::AuthFailed { .. } => false,
            Self::RateLimited { .. } => true,
            Self::Timeout { .. } => true,
            Self::NotFound { .. } => false,
            Self::Internal { .. } => false,
        }
    }

    fn is_retryable(&self) -> bool {
        self.is_transient()
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after } => Some(*retry_after),
            _ => None,
        }
    }
}

// ============================================================================
// Recall Record (for storage operations)
// ============================================================================

/// A record to store in the recall index.
///
/// This type is used by [`RecallWriter::store`] to add new records to the
/// semantic memory. It contains the content, embedding, and metadata needed
/// for later retrieval.
#[derive(Debug, Clone)]
pub struct RecallRecord {
    /// Unique identifier for this record.
    pub id: String,
    /// The text content to store.
    pub content: String,
    /// Pre-computed embedding vector (optional; backend may compute if missing).
    pub embedding: Option<Vec<f32>>,
    /// Source metadata for provenance.
    pub metadata: RecallRecordMetadata,
}

/// Metadata for a recall record.
#[derive(Debug, Clone, Default)]
pub struct RecallRecordMetadata {
    /// Source type (e.g., "failure", "runbook", "adapter").
    pub source_type: Option<String>,
    /// Source chain ID if from a chain execution.
    pub source_chain_id: Option<String>,
    /// Tenant scope for multi-tenant isolation.
    pub tenant_id: Option<String>,
    /// ISO 8601 timestamp when record was created.
    pub created_at: Option<String>,
}

// ============================================================================
// Recall Traits
// ============================================================================

/// Read-only recall capability trait.
///
/// This trait provides query access to semantic memory. It is designed for
/// read-only contexts like validators, auditors, and replay engines.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` for use in concurrent contexts.
///
/// # GAT Async Pattern
///
/// The `QueryFut` associated type enables static dispatch async:
///
/// ```ignore
/// impl RecallReader for MyRecallBackend {
///     type QueryFut<'a> = impl Future<Output = Result<Vec<RecallCandidate>, RecallError>> + Send + 'a;
///
///     fn query<'a>(&'a self, query: &'a RecallQuery) -> Self::QueryFut<'a> {
///         async move {
///             // ... implementation
///         }
///     }
/// }
/// ```
pub trait RecallReader: Send + Sync {
    /// Future type for query operations.
    type QueryFut<'a>: Future<Output = Result<Vec<RecallCandidate>, RecallError>> + Send + 'a
    where
        Self: 'a;

    /// Query the recall index for similar content.
    ///
    /// # Arguments
    ///
    /// * `query` - The recall query containing search text and parameters
    ///
    /// # Returns
    ///
    /// A vector of recall candidates ranked by relevance.
    fn query<'a>(&'a self, query: &'a RecallQuery) -> Self::QueryFut<'a>;
}

/// Write access recall capability trait.
///
/// This trait provides mutation access to semantic memory. It is designed for
/// ingestion pipelines and administrative operations.
///
/// # Authority Boundary
///
/// Store and delete operations are governance boundaries. Most runtime contexts
/// should use only [`RecallReader`] to prevent accidental data modification.
///
/// # Thread Safety
///
/// Implementations must be `Send + Sync` for use in concurrent contexts.
pub trait RecallWriter: Send + Sync {
    /// Future type for store operations.
    type StoreFut<'a>: Future<Output = Result<(), RecallError>> + Send + 'a
    where
        Self: 'a;

    /// Future type for delete operations.
    type DeleteFut<'a>: Future<Output = Result<(), RecallError>> + Send + 'a
    where
        Self: 'a;

    /// Store a record in the recall index.
    ///
    /// # Arguments
    ///
    /// * `record` - The recall record to store
    ///
    /// # Idempotency
    ///
    /// Storing a record with the same ID should overwrite the existing record.
    fn store<'a>(&'a self, record: RecallRecord) -> Self::StoreFut<'a>;

    /// Delete a record from the recall index by ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the record to delete
    ///
    /// # Errors
    ///
    /// Returns `RecallError::NotFound` if the record does not exist.
    fn delete<'a>(&'a self, id: &'a str) -> Self::DeleteFut<'a>;
}

/// Umbrella trait combining read and write recall capabilities.
///
/// This trait is for contexts that need both read and write access.
/// Most contexts should prefer the narrower [`RecallReader`] or
/// [`RecallWriter`] traits.
///
/// # Blanket Implementation
///
/// Any type implementing both `RecallReader` and `RecallWriter` automatically
/// implements `Recall`.
pub trait Recall: RecallReader + RecallWriter {}

// Blanket impl: any type with both Reader and Writer is Recall
impl<T: RecallReader + RecallWriter> Recall for T {}

// ============================================================================
// Dyn-Safe Wrapper (for runtime polymorphism)
// ============================================================================

/// Boxed future type for dyn-safe trait objects.
pub type BoxFuture<'a, T> = std::pin::Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Dyn-safe recall reader trait for runtime polymorphism.
///
/// Use this when you need `dyn RecallReader` (e.g., heterogeneous backends,
/// plugin systems). The cost is one heap allocation per call.
///
/// # Blanket Implementation
///
/// All `RecallReader` implementations automatically implement `DynRecallReader`.
pub trait DynRecallReader: Send + Sync {
    /// Query the recall index for similar content.
    fn query<'a>(
        &'a self,
        query: &'a RecallQuery,
    ) -> BoxFuture<'a, Result<Vec<RecallCandidate>, RecallError>>;
}

impl<T: RecallReader> DynRecallReader for T {
    fn query<'a>(
        &'a self,
        query: &'a RecallQuery,
    ) -> BoxFuture<'a, Result<Vec<RecallCandidate>, RecallError>> {
        Box::pin(RecallReader::query(self, query))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recall_error_display() {
        let err = RecallError::DimensionMismatch {
            expected: 1536,
            got: 768,
        };
        assert!(err.to_string().contains("1536"));
        assert!(err.to_string().contains("768"));
    }

    #[test]
    fn recall_error_category_classification() {
        assert_eq!(
            RecallError::IndexUnavailable {
                message: "test".to_string()
            }
            .category(),
            ErrorCategory::Unavailable
        );
        assert_eq!(
            RecallError::DimensionMismatch {
                expected: 1536,
                got: 768
            }
            .category(),
            ErrorCategory::InvalidInput
        );
        assert_eq!(
            RecallError::AuthFailed {
                message: "test".to_string()
            }
            .category(),
            ErrorCategory::Auth
        );
        assert_eq!(
            RecallError::RateLimited {
                retry_after: Duration::from_mins(1)
            }
            .category(),
            ErrorCategory::RateLimit
        );
    }

    #[test]
    fn recall_error_transient_classification() {
        assert!(
            RecallError::IndexUnavailable {
                message: "test".to_string()
            }
            .is_transient()
        );
        assert!(
            RecallError::RateLimited {
                retry_after: Duration::from_mins(1)
            }
            .is_transient()
        );
        assert!(
            RecallError::Timeout {
                elapsed: Duration::from_secs(30),
                deadline: Duration::from_secs(30),
            }
            .is_transient()
        );

        assert!(
            !RecallError::DimensionMismatch {
                expected: 1536,
                got: 768
            }
            .is_transient()
        );
        assert!(
            !RecallError::AuthFailed {
                message: "test".to_string()
            }
            .is_transient()
        );
        assert!(
            !RecallError::NotFound {
                id: "test".to_string()
            }
            .is_transient()
        );
    }

    #[test]
    fn recall_error_retry_after() {
        let err = RecallError::RateLimited {
            retry_after: Duration::from_mins(1),
        };
        assert_eq!(err.retry_after(), Some(Duration::from_mins(1)));

        let err2 = RecallError::IndexUnavailable {
            message: "test".to_string(),
        };
        assert_eq!(err2.retry_after(), None);
    }

    #[test]
    fn recall_record_metadata_default() {
        let meta = RecallRecordMetadata::default();
        assert!(meta.source_type.is_none());
        assert!(meta.tenant_id.is_none());
    }
}
