// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Canonical chat capability contracts for provider consumers and adapters.

use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

/// Boxed future type for dyn-safe capability traits.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Request for chat completion.
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    Text,
    Markdown,
    Json,
    Yaml,
    Toml,
}

impl ResponseFormat {
    #[must_use]
    pub fn default_structured() -> Self {
        Self::Yaml
    }

    #[must_use]
    pub fn fallback(self) -> Option<Self> {
        match self {
            Self::Json | Self::Text => None,
            Self::Yaml | Self::Toml | Self::Markdown => Some(Self::Json),
        }
    }

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

/// Error type for chat operations.
#[derive(Debug, Clone)]
pub enum LlmError {
    RateLimited {
        retry_after: Duration,
        message: Option<String>,
    },
    Timeout {
        elapsed: Duration,
        deadline: Duration,
    },
    AuthDenied {
        message: String,
    },
    InvalidRequest {
        message: String,
    },
    ModelNotFound {
        model: String,
    },
    ContextLengthExceeded {
        max_tokens: u32,
        request_tokens: u32,
    },
    ContentFiltered {
        reason: String,
    },
    ResponseFormatMismatch {
        expected: ResponseFormat,
        message: String,
    },
    ProviderError {
        message: String,
        code: Option<String>,
    },
    NetworkError {
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
                if let Some(message) = message {
                    write!(f, ": {message}")?;
                }
                Ok(())
            }
            Self::Timeout { elapsed, deadline } => {
                write!(f, "timeout after {:?} (deadline: {:?})", elapsed, deadline)
            }
            Self::AuthDenied { message } => write!(f, "authentication denied: {message}"),
            Self::InvalidRequest { message } => write!(f, "invalid request: {message}"),
            Self::ModelNotFound { model } => write!(f, "model not found: {model}"),
            Self::ContextLengthExceeded {
                max_tokens,
                request_tokens,
            } => {
                write!(
                    f,
                    "context length exceeded: {request_tokens} tokens (max: {max_tokens})"
                )
            }
            Self::ContentFiltered { reason } => write!(f, "content filtered: {reason}"),
            Self::ResponseFormatMismatch { expected, message } => {
                write!(f, "response format mismatch for {:?}: {message}", expected)
            }
            Self::ProviderError { message, code } => {
                write!(f, "provider error: {message}")?;
                if let Some(code) = code {
                    write!(f, " (code: {code})")?;
                }
                Ok(())
            }
            Self::NetworkError { message } => write!(f, "network error: {message}"),
        }
    }
}

impl std::error::Error for LlmError {}

/// Chat completion capability.
pub trait ChatBackend: Send + Sync {
    type ChatFut<'a>: Future<Output = Result<ChatResponse, LlmError>> + Send + 'a
    where
        Self: 'a;

    fn chat<'a>(&'a self, req: ChatRequest) -> Self::ChatFut<'a>;
}

/// Dyn-safe chat backend for runtime polymorphism.
pub trait DynChatBackend: Send + Sync {
    fn chat(&self, req: ChatRequest) -> BoxFuture<'_, Result<ChatResponse, LlmError>>;
}

impl<T: ChatBackend> DynChatBackend for T {
    fn chat(&self, req: ChatRequest) -> BoxFuture<'_, Result<ChatResponse, LlmError>> {
        Box::pin(ChatBackend::chat(self, req))
    }
}
