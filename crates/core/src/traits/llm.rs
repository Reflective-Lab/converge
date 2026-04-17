// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # LLM Capability Boundary Traits
//!
//! This module defines the capability boundary traits for Large Language Model
//! operations. These traits abstract LLM functionality, allowing `converge-core`
//! to remain dependency-free while capability crates provide implementations.
//!
//! ## Design Philosophy
//!
//! - **Split traits by capability:** [`ChatBackend`] for chat completions,
//!   [`EmbedBackend`] for embeddings. Many providers support only subsets,
//!   so split traits make boundaries explicit and enforceable.
//!
//! - **GAT async pattern:** Uses generic associated types (GATs) for zero-cost
//!   async without proc macros or `async_trait`. This keeps the core dependency-free.
//!
//! - **Dyn-safe wrappers:** [`DynChatBackend`] and [`DynEmbedBackend`] provide
//!   `dyn Trait` compatibility when runtime polymorphism is needed.
//!
//! - **Thread safety required:** All traits require `Send + Sync` bounds for
//!   multi-threaded runtimes.
//!
//! ## Trait Hierarchy
//!
//! ```text
//! ChatBackend (GAT async)          EmbedBackend (GAT async)
//!      |                                |
//!      +------- LlmBackend -------------+  (umbrella trait)
//!
//! DynChatBackend (BoxFuture)       DynEmbedBackend (BoxFuture)
//!      |                                |
//!      +------ (for dyn Trait) ---------+
//! ```
//!
//! ## Error Handling
//!
//! [`LlmError`] implements [`CapabilityError`](super::error::CapabilityError) for
//! uniform error classification. This enables generic retry/circuit breaker logic.

use super::error::{CapabilityError, ErrorCategory};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Boxed future type for dyn-safe trait variants.
///
/// Used by [`DynChatBackend`] and [`DynEmbedBackend`] for runtime polymorphism.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request for chat completion.
///
/// Contains the conversation messages and generation parameters.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub system: Option<String>,
    pub tools: Vec<ToolDefinition>,
    pub response_format: ResponseFormat,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Requested output format for a chat completion.
///
/// Guides the model toward producing parseable structured output. Some formats
/// have native API enforcement (JSON on OpenAI/Gemini), others rely on system
/// prompt instructions.
///
/// # Choosing a format
///
/// | Format   | Use when                                       | Token cost | Parse cost |
/// |----------|------------------------------------------------|------------|------------|
/// | Text     | Free-form prose, no structure needed           | Lowest     | None       |
/// | Markdown | Human-readable reports, tables, decision briefs| Low        | Optional   |
/// | Json     | Machine-consumed data, API responses, storage  | Medium     | Cheap      |
/// | Yaml     | Human-reviewable structured data, config       | Low        | Cheap      |
/// | Toml     | Flat config, simple records, settings          | Lowest     | Cheap      |
///
/// **JSON** is the safest default for machine consumption — every model handles it
/// and most providers enforce it at the API level.
///
/// **YAML** produces ~20-30% fewer tokens than JSON for the same data (no braces,
/// no quotes on keys), while remaining parseable and human-readable. Good alternative
/// when token cost matters and the consumer can parse YAML.
///
/// **TOML** is the most compact for flat structures but breaks down on deeply nested
/// data or arrays of objects. Best for config-shaped output.
///
/// **Markdown** is for presentation, not data interchange. Use it when a human reads
/// the output directly. Some models (GPT-4o, Claude Haiku) will default to JSON
/// wrapped in code fences instead of actual Markdown structure — prefer Claude Sonnet,
/// Gemini, Llama, or Mistral for Markdown output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    /// Free-form text. No structure, no parsing. Use when the output is prose
    /// or when the model should decide its own presentation.
    #[default]
    Text,
    /// Markdown with headings, lists, tables, code blocks. Best for human-readable
    /// reports and decision briefs. Not a data interchange format.
    Markdown,
    /// Strict JSON. Most reliable structured format — all models handle it, most
    /// providers enforce it at the API level. Default choice for machine consumption.
    Json,
    /// YAML without anchors, aliases, or custom tags. More compact than JSON
    /// (~20-30% fewer tokens), human-editable. Good when a human reviews or edits
    /// the output and token cost matters.
    Yaml,
    /// TOML. Most compact for flat key-value data and simple tables. Avoid for
    /// deeply nested structures or arrays of complex objects — TOML syntax gets
    /// awkward and some models produce invalid inline tables.
    Toml,
}

impl ResponseFormat {
    /// Returns the recommended format for structured data extraction.
    ///
    /// YAML is the default: 10% fewer tokens than JSON, 100% model compliance,
    /// human-reviewable. Use `Json` only when you need strict schema enforcement
    /// or the consumer requires it.
    #[must_use]
    pub fn default_structured() -> Self {
        Self::Yaml
    }

    /// Returns the safest fallback format for retries after a parse failure.
    ///
    /// JSON has native API enforcement on most providers and 100% compliance
    /// across all tested models.
    #[must_use]
    pub fn fallback(self) -> Option<Self> {
        match self {
            Self::Json | Self::Text => None, // already the safest / no structure
            Self::Yaml | Self::Toml | Self::Markdown => Some(Self::Json),
        }
    }

    /// Returns a system prompt instruction that tells the model to produce this format.
    ///
    /// All structured formats return `Some`. `Text` returns `None` (no constraint).
    /// Backends always include this instruction in the system prompt. For JSON,
    /// backends may additionally enable native API-level enforcement where available.
    #[must_use]
    pub fn system_instruction(self) -> Option<&'static str> {
        match self {
            Self::Text => None,
            Self::Markdown => Some(
                "You MUST respond with valid Markdown only. Use headings, lists, and tables to structure the data. Do NOT wrap output in code fences or return serialized JSON/YAML. Present data as readable Markdown.",
            ),
            Self::Json => Some("You MUST respond with valid JSON only. No other text."),
            Self::Yaml => Some(
                "You MUST respond with valid YAML only. No anchors, no aliases, no custom tags. No other text or code fences.",
            ),
            Self::Toml => Some(
                "You MUST respond with valid TOML only. Use sections and key-value pairs. No inline tables for complex data. No other text or code fences.",
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Option<TokenUsage>,
    pub model: Option<String>,
    pub finish_reason: Option<FinishReason>,
    /// Provider/gateway metadata captured from response headers and body.
    ///
    /// Populated by backends that have extra telemetry (Kong headers,
    /// OpenRouter cost fields, etc.). Empty for backends that don't.
    pub metadata: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ContentFilter,
    StopSequence,
    ToolCalls,
}

/// Request for embedding generation.
#[derive(Debug, Clone)]
pub struct EmbedRequest {
    /// Input text(s) to embed.
    pub inputs: Vec<String>,
    /// Model identifier (e.g., "text-embedding-3-small").
    pub model: Option<String>,
    /// Embedding dimensions (for models that support variable dimensions).
    pub dimensions: Option<u32>,
}

/// Response from embedding generation.
#[derive(Debug, Clone)]
pub struct EmbedResponse {
    /// Generated embeddings (one per input).
    pub embeddings: Vec<Vec<f32>>,
    /// Token usage statistics.
    pub usage: Option<TokenUsage>,
    /// Model that generated the embeddings.
    pub model: Option<String>,
}

// ============================================================================
// Error Type
// ============================================================================

/// Error type for LLM operations.
///
/// Implements [`CapabilityError`] for uniform error classification.
#[derive(Debug, Clone)]
pub enum LlmError {
    /// Rate limited by provider.
    RateLimited {
        /// Suggested delay before retry.
        retry_after: Duration,
        /// Provider's rate limit message.
        message: Option<String>,
    },
    /// Operation timed out.
    Timeout {
        /// Time elapsed before timeout.
        elapsed: Duration,
        /// Configured deadline.
        deadline: Duration,
    },
    /// Authentication or authorization denied.
    AuthDenied {
        /// Error message from provider.
        message: String,
    },
    /// Invalid request parameters.
    InvalidRequest {
        /// Description of what's invalid.
        message: String,
    },
    /// Model not found or unavailable.
    ModelNotFound {
        /// Requested model identifier.
        model: String,
    },
    /// Context length exceeded.
    ContextLengthExceeded {
        /// Maximum allowed tokens.
        max_tokens: u32,
        /// Tokens in the request.
        request_tokens: u32,
    },
    /// Content filter triggered.
    ContentFiltered {
        /// Reason for filtering.
        reason: String,
    },
    /// Provider returned content that does not match the requested response format.
    ResponseFormatMismatch {
        /// Requested response format that the provider failed to honor.
        expected: ResponseFormat,
        /// Description of the mismatch.
        message: String,
    },
    /// Provider returned an error.
    ProviderError {
        /// Error message from provider.
        message: String,
        /// Provider-specific error code.
        code: Option<String>,
    },
    /// Network or connectivity error.
    NetworkError {
        /// Error description.
        message: String,
    },
}

impl std::fmt::Display for LlmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimited {
                retry_after,
                message,
            } => {
                write!(f, "rate limited (retry after {:?})", retry_after)?;
                if let Some(msg) = message {
                    write!(f, ": {}", msg)?;
                }
                Ok(())
            }
            Self::Timeout { elapsed, deadline } => {
                write!(f, "timeout after {:?} (deadline: {:?})", elapsed, deadline)
            }
            Self::AuthDenied { message } => write!(f, "authentication denied: {}", message),
            Self::InvalidRequest { message } => write!(f, "invalid request: {}", message),
            Self::ModelNotFound { model } => write!(f, "model not found: {}", model),
            Self::ContextLengthExceeded {
                max_tokens,
                request_tokens,
            } => {
                write!(
                    f,
                    "context length exceeded: {} tokens (max: {})",
                    request_tokens, max_tokens
                )
            }
            Self::ContentFiltered { reason } => write!(f, "content filtered: {}", reason),
            Self::ResponseFormatMismatch { expected, message } => {
                write!(
                    f,
                    "response format mismatch for {:?}: {}",
                    expected, message
                )
            }
            Self::ProviderError { message, code } => {
                write!(f, "provider error: {}", message)?;
                if let Some(c) = code {
                    write!(f, " (code: {})", c)?;
                }
                Ok(())
            }
            Self::NetworkError { message } => write!(f, "network error: {}", message),
        }
    }
}

impl std::error::Error for LlmError {}

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
        // Transient errors are retryable
        // ProviderError may also be retryable (temporary backend issues)
        self.is_transient() || matches!(self, Self::ProviderError { .. })
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after, .. } => Some(*retry_after),
            _ => None,
        }
    }
}

// ============================================================================
// Static Dispatch Traits (GAT Async Pattern)
// ============================================================================

/// Chat completion capability.
///
/// Provides chat completion functionality using the GAT async pattern for
/// zero-cost static dispatch without runtime overhead.
///
/// # Example Implementation
///
/// ```ignore
/// struct OpenAIChatBackend { /* ... */ }
///
/// impl ChatBackend for OpenAIChatBackend {
///     type ChatFut<'a> = impl Future<Output = Result<ChatResponse, LlmError>> + Send + 'a
///     where
///         Self: 'a;
///
///     fn chat<'a>(&'a self, req: ChatRequest) -> Self::ChatFut<'a> {
///         async move {
///             // Make API call...
///             Ok(ChatResponse { /* ... */ })
///         }
///     }
/// }
/// ```
pub trait ChatBackend: Send + Sync {
    /// Associated future type for chat completion.
    ///
    /// Must be `Send` to work with multi-threaded runtimes.
    type ChatFut<'a>: Future<Output = Result<ChatResponse, LlmError>> + Send + 'a
    where
        Self: 'a;

    /// Send a chat completion request.
    ///
    /// # Arguments
    ///
    /// * `req` - The chat request containing messages and parameters.
    ///
    /// # Returns
    ///
    /// A future that resolves to the chat response or an error.
    fn chat<'a>(&'a self, req: ChatRequest) -> Self::ChatFut<'a>;
}

/// Embedding generation capability.
///
/// Provides embedding generation functionality using the GAT async pattern for
/// zero-cost static dispatch without runtime overhead.
///
/// # Example Implementation
///
/// ```ignore
/// struct OpenAIEmbedBackend { /* ... */ }
///
/// impl EmbedBackend for OpenAIEmbedBackend {
///     type EmbedFut<'a> = impl Future<Output = Result<EmbedResponse, LlmError>> + Send + 'a
///     where
///         Self: 'a;
///
///     fn embed<'a>(&'a self, req: EmbedRequest) -> Self::EmbedFut<'a> {
///         async move {
///             // Make API call...
///             Ok(EmbedResponse { /* ... */ })
///         }
///     }
/// }
/// ```
pub trait EmbedBackend: Send + Sync {
    /// Associated future type for embedding generation.
    ///
    /// Must be `Send` to work with multi-threaded runtimes.
    type EmbedFut<'a>: Future<Output = Result<EmbedResponse, LlmError>> + Send + 'a
    where
        Self: 'a;

    /// Generate embeddings for input text(s).
    ///
    /// # Arguments
    ///
    /// * `req` - The embedding request containing inputs and parameters.
    ///
    /// # Returns
    ///
    /// A future that resolves to the embedding response or an error.
    fn embed<'a>(&'a self, req: EmbedRequest) -> Self::EmbedFut<'a>;
}

/// Umbrella trait combining chat and embedding capabilities.
///
/// Provides a convenience trait for backends that support both chat completions
/// and embeddings. This is an optional umbrella—implementations can choose to
/// implement only [`ChatBackend`] or [`EmbedBackend`] if they don't support both.
///
/// # Automatic Implementation
///
/// Any type implementing both [`ChatBackend`] and [`EmbedBackend`] automatically
/// implements [`LlmBackend`] via a blanket impl.
pub trait LlmBackend: ChatBackend + EmbedBackend {}

// Blanket implementation: any type with both traits gets LlmBackend
impl<T: ChatBackend + EmbedBackend> LlmBackend for T {}

// ============================================================================
// Dyn-Safe Wrappers (Runtime Polymorphism)
// ============================================================================

/// Dyn-safe chat backend for runtime polymorphism.
///
/// Use this trait when you need `dyn Trait` compatibility, such as:
/// - Storing multiple backend types in a collection
/// - Runtime routing between different providers
/// - Plugin systems with dynamic loading
///
/// For static dispatch (better performance, no allocation), use [`ChatBackend`].
///
/// # Blanket Implementation
///
/// Any type implementing [`ChatBackend`] automatically implements [`DynChatBackend`]
/// via a blanket impl that boxes the future.
pub trait DynChatBackend: Send + Sync {
    /// Send a chat completion request.
    ///
    /// Returns a boxed future for dyn-safety.
    fn chat(&self, req: ChatRequest) -> BoxFuture<'_, Result<ChatResponse, LlmError>>;
}

// Blanket implementation: ChatBackend -> DynChatBackend
impl<T: ChatBackend> DynChatBackend for T {
    fn chat(&self, req: ChatRequest) -> BoxFuture<'_, Result<ChatResponse, LlmError>> {
        Box::pin(ChatBackend::chat(self, req))
    }
}

/// Dyn-safe embed backend for runtime polymorphism.
///
/// Use this trait when you need `dyn Trait` compatibility, such as:
/// - Storing multiple backend types in a collection
/// - Runtime routing between different providers
/// - Plugin systems with dynamic loading
///
/// For static dispatch (better performance, no allocation), use [`EmbedBackend`].
///
/// # Blanket Implementation
///
/// Any type implementing [`EmbedBackend`] automatically implements [`DynEmbedBackend`]
/// via a blanket impl that boxes the future.
pub trait DynEmbedBackend: Send + Sync {
    /// Generate embeddings for input text(s).
    ///
    /// Returns a boxed future for dyn-safety.
    fn embed(&self, req: EmbedRequest) -> BoxFuture<'_, Result<EmbedResponse, LlmError>>;
}

// Blanket implementation: EmbedBackend -> DynEmbedBackend
impl<T: EmbedBackend> DynEmbedBackend for T {
    fn embed(&self, req: EmbedRequest) -> BoxFuture<'_, Result<EmbedResponse, LlmError>> {
        Box::pin(EmbedBackend::embed(self, req))
    }
}
