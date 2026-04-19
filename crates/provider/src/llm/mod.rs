// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Remote chat backend implementations for the canonical `ChatBackend` surface.
//!
//! This module provides async multi-turn provider adapters built on
//! `converge_provider_api::ChatBackend`. These are the canonical remote
//! LLM interfaces used by application and tool layers.
//!
//! # Architecture
//!
//! ```text
//! converge-provider-api
//!     │
//!     │  ChatBackend / DynChatBackend
//!     ▼
//! converge-provider::llm
//!     │
//!     ├── AnthropicBackend
//!     ├── OpenAIBackend
//!     └── ...               → RemoteTraceLink (audit-only)
//! ```
//!
//! # Canonical Surface
//!
//! The canonical remote surface is `ChatBackend` plus its dyn-safe wrapper
//! `DynChatBackend`. Local kernel inference remains in `converge-llm`.

#[cfg(feature = "anthropic")]
mod anthropic;
mod error_classification;
mod format_contract;
#[cfg(feature = "kong")]
mod kong;
#[cfg(feature = "mistral")]
mod mistral;
#[cfg(feature = "openai")]
mod openai;
#[cfg(feature = "openrouter")]
mod openrouter;
mod resilient;
mod selection;

#[cfg(feature = "anthropic")]
pub use anthropic::AnthropicBackend;
#[cfg(feature = "kong")]
pub use kong::KongBackend;
#[cfg(feature = "gemini")]
mod gemini;
#[cfg(feature = "mistral")]
pub use mistral::MistralBackend;
#[cfg(feature = "openai")]
pub use openai::OpenAiBackend;
#[cfg(feature = "openrouter")]
pub use openrouter::OpenRouterBackend;
pub use resilient::ResilientChatBackend;
pub use selection::{
    ChatBackendSelectionConfig, ChatBackendSelectionConfigError, SelectedChatBackend,
    select_chat_backend, select_chat_backend_with_secret_provider, select_healthy_chat_backend,
    select_healthy_chat_backend_with_secret_provider,
};

#[cfg(feature = "gemini")]
pub use gemini::GeminiBackend;
