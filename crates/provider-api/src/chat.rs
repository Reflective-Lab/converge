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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_format_default_structured_is_yaml() {
        assert_eq!(ResponseFormat::default_structured(), ResponseFormat::Yaml);
    }

    #[test]
    fn response_format_fallback() {
        assert_eq!(ResponseFormat::Text.fallback(), None);
        assert_eq!(ResponseFormat::Json.fallback(), None);
        assert_eq!(ResponseFormat::Yaml.fallback(), Some(ResponseFormat::Json));
        assert_eq!(ResponseFormat::Toml.fallback(), Some(ResponseFormat::Json));
        assert_eq!(
            ResponseFormat::Markdown.fallback(),
            Some(ResponseFormat::Json)
        );
    }

    #[test]
    fn response_format_system_instruction_text_is_none() {
        assert!(ResponseFormat::Text.system_instruction().is_none());
    }

    #[test]
    fn response_format_system_instruction_json() {
        let instr = ResponseFormat::Json.system_instruction().unwrap();
        assert!(instr.contains("JSON"));
    }

    #[test]
    fn response_format_system_instruction_yaml() {
        let instr = ResponseFormat::Yaml.system_instruction().unwrap();
        assert!(instr.contains("YAML"));
    }

    #[test]
    fn response_format_system_instruction_toml() {
        let instr = ResponseFormat::Toml.system_instruction().unwrap();
        assert!(instr.contains("TOML"));
    }

    #[test]
    fn response_format_system_instruction_markdown() {
        let instr = ResponseFormat::Markdown.system_instruction().unwrap();
        assert!(instr.contains("Markdown"));
    }

    #[test]
    fn response_format_default_is_text() {
        assert_eq!(ResponseFormat::default(), ResponseFormat::Text);
    }

    #[test]
    fn chat_role_variants_exist() {
        let _system = ChatRole::System;
        let _user = ChatRole::User;
        let _assistant = ChatRole::Assistant;
        let _tool = ChatRole::Tool;
    }

    #[test]
    fn llm_error_display_rate_limited() {
        let err = LlmError::RateLimited {
            retry_after: Duration::from_secs(30),
            message: Some("too many requests".into()),
        };
        let s = err.to_string();
        assert!(s.contains("rate limited"));
        assert!(s.contains("too many requests"));
    }

    #[test]
    fn llm_error_display_rate_limited_no_message() {
        let err = LlmError::RateLimited {
            retry_after: Duration::from_secs(5),
            message: None,
        };
        let s = err.to_string();
        assert!(s.contains("rate limited"));
        assert!(!s.contains(":"));
    }

    #[test]
    fn llm_error_display_timeout() {
        let err = LlmError::Timeout {
            elapsed: Duration::from_secs(10),
            deadline: Duration::from_secs(5),
        };
        let s = err.to_string();
        assert!(s.contains("timeout"));
        assert!(s.contains("deadline"));
    }

    #[test]
    fn llm_error_display_auth_denied() {
        let err = LlmError::AuthDenied {
            message: "bad key".into(),
        };
        assert!(err.to_string().contains("authentication denied"));
    }

    #[test]
    fn llm_error_display_invalid_request() {
        let err = LlmError::InvalidRequest {
            message: "missing model".into(),
        };
        assert!(err.to_string().contains("invalid request"));
    }

    #[test]
    fn llm_error_display_model_not_found() {
        let err = LlmError::ModelNotFound {
            model: "gpt-5".into(),
        };
        assert!(err.to_string().contains("gpt-5"));
    }

    #[test]
    fn llm_error_display_context_length() {
        let err = LlmError::ContextLengthExceeded {
            max_tokens: 4096,
            request_tokens: 8000,
        };
        let s = err.to_string();
        assert!(s.contains("8000"));
        assert!(s.contains("4096"));
    }

    #[test]
    fn llm_error_display_content_filtered() {
        let err = LlmError::ContentFiltered {
            reason: "safety".into(),
        };
        assert!(err.to_string().contains("safety"));
    }

    #[test]
    fn llm_error_display_response_format_mismatch() {
        let err = LlmError::ResponseFormatMismatch {
            expected: ResponseFormat::Json,
            message: "got yaml".into(),
        };
        let s = err.to_string();
        assert!(s.contains("format mismatch"));
        assert!(s.contains("got yaml"));
    }

    #[test]
    fn llm_error_display_provider_error_with_code() {
        let err = LlmError::ProviderError {
            message: "internal".into(),
            code: Some("500".into()),
        };
        let s = err.to_string();
        assert!(s.contains("provider error"));
        assert!(s.contains("500"));
    }

    #[test]
    fn llm_error_display_provider_error_no_code() {
        let err = LlmError::ProviderError {
            message: "oops".into(),
            code: None,
        };
        let s = err.to_string();
        assert!(s.contains("oops"));
        assert!(!s.contains("code"));
    }

    #[test]
    fn llm_error_display_network() {
        let err = LlmError::NetworkError {
            message: "dns failed".into(),
        };
        assert!(err.to_string().contains("dns failed"));
    }
}
