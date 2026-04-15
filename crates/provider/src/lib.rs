// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

// Pedantic clippy lints — allow stylistic ones that don't improve correctness
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::doc_link_with_quotes)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::unused_self)]
#![allow(clippy::needless_pass_by_value)]

//! Capability adapters for the Converge runtime.
//!
//! > **Providers produce observations, never decisions.**
//! > **Converge converges; providers adapt.**
//!
//! This crate provides capability adapters (providers) that connect Converge
//! workflows to external systems. Each provider implements
//! [`ChatBackend`](converge_core::traits::ChatBackend) for LLM completions,
//! or other capability traits for embedding, search, etc.
//!
//! # What Is a Provider?
//!
//! A provider is an **adapter** that:
//! - Implements capability traits (`ChatBackend`, `Embedding`, `VectorRecall`, etc.)
//! - Returns observations (not facts, not decisions)
//! - Includes provenance metadata for tracing
//! - Is stateless (no hidden lifecycle state)
//!
//! A provider is **NOT**:
//! - An agent (agents live in `converge-core`)
//! - Orchestration (no workflows, no scheduling)
//! - Domain logic (business rules live in `converge-domain`)
//!
//! # Available Backends
//!
//! ## LLM Backends (ChatBackend implementations)
//! - [`AnthropicBackend`] - Claude API (Anthropic)
//! - [`OpenAiBackend`] - GPT-4, GPT-3.5 (OpenAI)
//! - [`OpenRouterBackend`] - Any model via OpenRouter (openrouter.ai)
//! - [`GeminiBackend`] - Gemini Pro (Google)
//! - [`MistralBackend`] - Mistral chat completions (Mistral AI)
//!
//! ## Structured Output
//!
//! All live chat backends accept [`ResponseFormat::Json`](converge_core::traits::ResponseFormat::Json),
//! but providers do not enforce it identically at request time:
//! - OpenAI and Mistral use native `response_format` API fields
//! - Gemini uses native `response_mime_type`
//! - Anthropic uses the documented system-instruction JSON pattern
//!
//! All live chat backends then apply a shared response contract before returning content:
//! - `Json`, `Yaml`, and `Toml` outputs are validated centrally
//! - trivial outer code fences are stripped for those machine formats
//! - a provider that returns prose for a structured request fails with
//!   [`LlmError::ResponseFormatMismatch`](converge_core::traits::LlmError::ResponseFormatMismatch)
//!
//! Anthropic's instruction-based JSON handling is provider-native and correct for Claude.
//! The difference is enforcement strength at request time, not post-response correctness.

// Secret management (SecretProvider trait, EnvSecretProvider default)
pub mod secret;

// Core contract types
pub mod contract;

// LLM Backend implementations (ChatBackend trait from converge-core)
pub mod llm;

// Capability registry
mod capability_registry;

// Model selection
mod model_selection;

// Search providers
#[cfg(feature = "brave")]
pub mod brave;
#[cfg(feature = "_http")]
pub mod fetch;
pub mod search;
#[cfg(feature = "tavily")]
pub mod tavily;

// Tool integration (MCP, OpenAPI, GraphQL)
pub mod tools;

// Capability providers
#[cfg(feature = "_http")]
pub mod embedding;
#[cfg(feature = "registry")]
pub mod registry_loader;
#[cfg(feature = "_http")]
pub mod reranker;
pub mod vector;

// Re-exports: capability registry
pub use capability_registry::{
    CapabilityRegistry, CapabilityRequirements, SearchProviderMeta, WebSearchRequirements,
};

// Re-exports: model selection
pub use model_selection::{
    FitnessBreakdown, ModelMetadata, ModelSelector, ProviderRegistry, RejectionReason,
    SelectionResult, is_brave_available, is_provider_available,
};

// Re-exports: secret management
pub use secret::{
    EnvSecretProvider, SecretError, SecretProvider, SecretString, StaticSecretProvider,
};

// Re-exports: LLM backends
#[cfg(feature = "anthropic")]
pub use llm::AnthropicBackend;
#[cfg(feature = "gemini")]
pub use llm::GeminiBackend;
#[cfg(feature = "kong")]
pub use llm::KongBackend;
#[cfg(feature = "mistral")]
pub use llm::MistralBackend;
#[cfg(feature = "openai")]
pub use llm::OpenAiBackend;
#[cfg(feature = "openrouter")]
pub use llm::OpenRouterBackend;
pub use llm::{
    ChatBackendSelectionConfig, ChatBackendSelectionConfigError, ResilientChatBackend,
    SelectedChatBackend, select_chat_backend, select_chat_backend_with_secret_provider,
};

// Re-exports: search providers
#[cfg(feature = "brave")]
pub use brave::{
    BraveCapability, BraveSearchError, BraveSearchProvider, BraveSearchRequest,
    BraveSearchResponse, BraveSearchResult,
};
#[cfg(feature = "_http")]
pub use fetch::HttpFetchProvider;
pub use search::{
    SearchDepth, SearchTopic, WebFetchBackend, WebFetchError, WebFetchRequest, WebFetchResponse,
    WebSearchBackend, WebSearchError, WebSearchImage, WebSearchRequest, WebSearchResponse,
    WebSearchResult,
};
#[cfg(feature = "tavily")]
pub use tavily::TavilySearchProvider;

// Re-exports: contract types
pub use contract::{
    CallTimer, Capability, ProviderCallContext, ProviderMeta, ProviderObservation, Region,
    TokenUsage, canonical_hash,
};

// Re-exports: tool integration
pub use tools::{
    GraphQlConfig, GraphQlConverter, GraphQlOperationType, InlineToolConfig, InputSchema,
    McpClient, McpClientBuilder, McpServerConfig, McpTransport, McpTransportType, OpenApiConfig,
    OpenApiConverter, SourceFilter, ToolCall, ToolDefinition, ToolError, ToolErrorKind, ToolFormat,
    ToolHandler, ToolRegistry, ToolResult, ToolResultContent, ToolSource, ToolsConfig,
    ToolsConfigError,
};
