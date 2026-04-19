// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! LLM capability traits.
//!
//! The canonical chat contract now lives in `converge-provider-api`. `converge-core`
//! re-exports that surface during the migration so downstreams can move without
//! a flag day. Embedding remains here for now.

use super::error::{CapabilityError, ErrorCategory};
pub use converge_provider_api::chat::{
    BoxFuture, ChatBackend, ChatMessage, ChatRequest, ChatResponse, ChatRole, DynChatBackend,
    FinishReason, LlmError, ResponseFormat, TokenUsage, ToolCall, ToolDefinition,
};
use std::future::Future;
use std::time::Duration;

/// Request for embedding generation.
#[derive(Debug, Clone)]
pub struct EmbedRequest {
    pub inputs: Vec<String>,
    pub model: Option<String>,
    pub dimensions: Option<u32>,
}

/// Response from embedding generation.
#[derive(Debug, Clone)]
pub struct EmbedResponse {
    pub embeddings: Vec<Vec<f32>>,
    pub usage: Option<TokenUsage>,
    pub model: Option<String>,
}

impl CapabilityError for LlmError {
    fn category(&self) -> ErrorCategory {
        match self {
            Self::RateLimited { .. } => ErrorCategory::RateLimit,
            Self::Timeout { .. } => ErrorCategory::Timeout,
            Self::AuthDenied { .. } => ErrorCategory::Auth,
            Self::InvalidRequest { .. } => ErrorCategory::InvalidInput,
            Self::ModelNotFound { .. } => ErrorCategory::NotFound,
            Self::ContextLengthExceeded { .. } => ErrorCategory::InvalidInput,
            Self::ContentFiltered { .. } => ErrorCategory::InvalidInput,
            Self::ResponseFormatMismatch { .. } => ErrorCategory::Internal,
            Self::ProviderError { .. } => ErrorCategory::Internal,
            Self::NetworkError { .. } => ErrorCategory::Unavailable,
        }
    }

    fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::RateLimited { .. } | Self::Timeout { .. } | Self::NetworkError { .. }
        )
    }

    fn is_retryable(&self) -> bool {
        self.is_transient() || matches!(self, Self::ProviderError { .. })
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after, .. } => Some(*retry_after),
            _ => None,
        }
    }
}

/// Embedding generation capability.
pub trait EmbedBackend: Send + Sync {
    type EmbedFut<'a>: Future<Output = Result<EmbedResponse, LlmError>> + Send + 'a
    where
        Self: 'a;

    fn embed<'a>(&'a self, req: EmbedRequest) -> Self::EmbedFut<'a>;
}

/// Umbrella trait combining chat and embedding capabilities.
pub trait LlmBackend: ChatBackend + EmbedBackend {}

impl<T: ChatBackend + EmbedBackend> LlmBackend for T {}

/// Dyn-safe embed backend for runtime polymorphism.
pub trait DynEmbedBackend: Send + Sync {
    fn embed(&self, req: EmbedRequest) -> BoxFuture<'_, Result<EmbedResponse, LlmError>>;
}

impl<T: EmbedBackend> DynEmbedBackend for T {
    fn embed(&self, req: EmbedRequest) -> BoxFuture<'_, Result<EmbedResponse, LlmError>> {
        Box::pin(EmbedBackend::embed(self, req))
    }
}
