// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Resilient chat: automatic format and model fallback on failure.
//!
//! Wraps a `DynChatBackend` with retry logic that:
//! 1. On parse/format failure: retries with JSON (native API enforcement)
//! 2. On model error (rate limit, auth, provider error): retries with a fallback backend
//!
//! This is the recommended way to call LLMs when you need structured output.

use std::sync::Arc;

use tracing::{info, warn};

use converge_provider_api::{
    BoxFuture, ChatBackend, ChatRequest, ChatResponse, DynChatBackend, LlmError,
};

/// A chat backend that retries with format and model fallbacks.
///
/// On the first attempt, uses the primary backend with the requested format.
/// If the response fails to parse as the requested format, retries with JSON.
/// If the primary backend errors, falls back to the secondary backend.
pub struct ResilientChatBackend {
    primary: Arc<dyn DynChatBackend>,
    fallback: Option<Arc<dyn DynChatBackend>>,
    primary_label: String,
    fallback_label: String,
}

impl ResilientChatBackend {
    #[must_use]
    pub fn new(primary: Arc<dyn DynChatBackend>, label: impl Into<String>) -> Self {
        Self {
            primary,
            fallback: None,
            primary_label: label.into(),
            fallback_label: String::new(),
        }
    }

    #[must_use]
    pub fn with_fallback(
        mut self,
        fallback: Arc<dyn DynChatBackend>,
        label: impl Into<String>,
    ) -> Self {
        self.fallback = Some(fallback);
        self.fallback_label = label.into();
        self
    }

    async fn chat_async(&self, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        let original_format = req.response_format;

        // Attempt 1: primary backend, requested format
        match self.primary.chat(req.clone()).await {
            Ok(response) => Ok(response),
            Err(e) if is_retryable_with_format_change(&e) => {
                // Format-related failure — try JSON fallback
                if let Some(fallback_format) = original_format.fallback() {
                    warn!(
                        primary = %self.primary_label,
                        original_format = ?original_format,
                        fallback_format = ?fallback_format,
                        "Format failure, retrying with fallback format"
                    );

                    let mut retry_req = req.clone();
                    retry_req.response_format = fallback_format;

                    self.primary.chat(retry_req).await
                } else {
                    Err(e)
                }
            }
            Err(e) if is_retryable_with_model_change(&e) => {
                // Model/provider failure — try fallback backend
                if let Some(fallback) = &self.fallback {
                    warn!(
                        primary = %self.primary_label,
                        fallback = %self.fallback_label,
                        error = %e,
                        "Model failure, retrying with fallback backend"
                    );

                    match fallback.chat(req.clone()).await {
                        Ok(response) => {
                            info!(
                                fallback = %self.fallback_label,
                                "Fallback backend succeeded"
                            );
                            Ok(response)
                        }
                        Err(fallback_err) => {
                            warn!(
                                fallback = %self.fallback_label,
                                error = %fallback_err,
                                "Fallback backend also failed"
                            );
                            Err(e)
                        }
                    }
                } else {
                    Err(e)
                }
            }
            Err(e) => Err(e),
        }
    }
}

impl ChatBackend for ResilientChatBackend {
    type ChatFut<'a>
        = BoxFuture<'a, Result<ChatResponse, LlmError>>
    where
        Self: 'a;

    fn chat(&self, req: ChatRequest) -> Self::ChatFut<'_> {
        Box::pin(async move { self.chat_async(req).await })
    }
}

fn is_retryable_with_format_change(error: &LlmError) -> bool {
    matches!(
        error,
        LlmError::InvalidRequest { .. }
            | LlmError::ContentFiltered { .. }
            | LlmError::ResponseFormatMismatch { .. }
    )
}

fn is_retryable_with_model_change(error: &LlmError) -> bool {
    matches!(
        error,
        LlmError::RateLimited { .. }
            | LlmError::ProviderError { .. }
            | LlmError::ModelNotFound { .. }
            | LlmError::NetworkError { .. }
            | LlmError::Timeout { .. }
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use converge_core::traits::{ChatMessage, ChatRole, ResponseFormat};

    use super::*;

    struct FormatAwareBackend {
        seen_formats: Mutex<Vec<ResponseFormat>>,
        fail_json: bool,
    }

    impl FormatAwareBackend {
        fn new(fail_json: bool) -> Self {
            Self {
                seen_formats: Mutex::new(Vec::new()),
                fail_json,
            }
        }

        fn seen_formats(&self) -> Vec<ResponseFormat> {
            self.seen_formats.lock().unwrap().clone()
        }
    }

    impl ChatBackend for FormatAwareBackend {
        type ChatFut<'a>
            = BoxFuture<'a, Result<ChatResponse, LlmError>>
        where
            Self: 'a;

        fn chat(&self, req: ChatRequest) -> Self::ChatFut<'_> {
            self.seen_formats.lock().unwrap().push(req.response_format);

            Box::pin(async move {
                match req.response_format {
                    ResponseFormat::Yaml => Err(LlmError::ResponseFormatMismatch {
                        expected: ResponseFormat::Yaml,
                        message: "yaml parse failed".to_string(),
                    }),
                    ResponseFormat::Json => {
                        if self.fail_json {
                            Err(LlmError::ResponseFormatMismatch {
                                expected: ResponseFormat::Json,
                                message: "json parse failed".to_string(),
                            })
                        } else {
                            Ok(ChatResponse {
                                content: "{\"facts\":[]}".to_string(),
                                tool_calls: Vec::new(),
                                usage: None,
                                model: None,
                                finish_reason: None,
                                metadata: Default::default(),
                            })
                        }
                    }
                    _ => unreachable!(),
                }
            })
        }
    }

    fn request(response_format: ResponseFormat) -> ChatRequest {
        ChatRequest {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Return structured output".to_string(),
                tool_calls: Vec::new(),
                tool_call_id: None,
            }],
            system: None,
            tools: Vec::new(),
            response_format,
            max_tokens: None,
            temperature: None,
            stop_sequences: Vec::new(),
            model: None,
        }
    }

    #[tokio::test(flavor = "current_thread")]
    async fn retries_with_json_after_format_mismatch() {
        let primary = Arc::new(FormatAwareBackend::new(false));
        let backend = ResilientChatBackend::new(primary.clone(), "primary");

        let response = ChatBackend::chat(&backend, request(ResponseFormat::Yaml))
            .await
            .unwrap();

        assert_eq!(response.content, "{\"facts\":[]}");
        assert_eq!(
            primary.seen_formats(),
            vec![ResponseFormat::Yaml, ResponseFormat::Json]
        );
    }

    #[tokio::test(flavor = "current_thread")]
    async fn preserves_json_format_mismatch_when_no_fallback_exists() {
        let primary = Arc::new(FormatAwareBackend::new(true));
        let backend = ResilientChatBackend::new(primary, "primary");

        let error = ChatBackend::chat(&backend, request(ResponseFormat::Json))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            LlmError::ResponseFormatMismatch {
                expected: ResponseFormat::Json,
                ..
            }
        ));
    }
}
