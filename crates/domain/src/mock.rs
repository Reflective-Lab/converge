// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
//
// Mock chat backend for domain testing.
// Uses the canonical ChatBackend trait from converge-core.

use converge_core::traits::{
    ChatBackend, ChatRequest, ChatResponse, FinishReason, LlmError, TokenUsage,
};
use std::future::Ready;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Pre-configured response for `MockChatBackend`.
#[derive(Debug, Clone)]
pub struct MockResponse {
    /// The content to return.
    pub content: String,
    /// Simulated confidence (used by callers to set `ProposedFact` confidence).
    pub confidence: f64,
    /// Whether this response should succeed.
    pub success: bool,
    /// Optional error to return.
    pub error: Option<LlmError>,
}

impl MockResponse {
    /// Creates a successful mock response.
    #[must_use]
    pub fn success(content: impl Into<String>, confidence: f64) -> Self {
        Self {
            content: content.into(),
            confidence,
            success: true,
            error: None,
        }
    }

    /// Creates a failing mock response.
    #[must_use]
    pub fn failure(error: LlmError) -> Self {
        Self {
            content: String::new(),
            confidence: 0.0,
            success: false,
            error: Some(error),
        }
    }
}

/// Mock chat backend for testing.
///
/// Returns pre-configured responses in order. Useful for deterministic tests.
pub struct MockChatBackend {
    model: String,
    responses: Mutex<Vec<MockResponse>>,
    call_count: AtomicUsize,
}

impl MockChatBackend {
    /// Creates a new mock backend with pre-configured responses.
    #[must_use]
    pub fn new(responses: Vec<MockResponse>) -> Self {
        Self {
            model: "mock-model".into(),
            responses: Mutex::new(responses),
            call_count: AtomicUsize::new(0),
        }
    }

    /// Creates a mock that always returns the same response.
    #[must_use]
    pub fn constant(content: impl Into<String>, confidence: f64) -> Self {
        let content = content.into();
        let responses = (0..100)
            .map(|_| MockResponse::success(content.clone(), confidence))
            .collect();
        Self::new(responses)
    }

    /// Returns the number of times `chat` was called.
    #[must_use]
    pub fn call_count(&self) -> usize {
        self.call_count.load(Ordering::SeqCst)
    }

    fn next_response(&self) -> Result<ChatResponse, LlmError> {
        self.call_count.fetch_add(1, Ordering::SeqCst);

        let mut responses = self.responses.lock().map_err(|_| LlmError::ProviderError {
            message: "MockChatBackend: mutex poisoned".into(),
            code: None,
        })?;

        if responses.is_empty() {
            return Err(LlmError::ProviderError {
                message: "MockChatBackend: no more responses".into(),
                code: None,
            });
        }

        let response = responses.remove(0);

        if let Some(error) = response.error {
            return Err(error);
        }

        Ok(ChatResponse {
            content: response.content,
            tool_calls: Vec::new(),
            model: Some(self.model.clone()),
            usage: Some(TokenUsage {
                prompt_tokens: 10,
                completion_tokens: 20,
                total_tokens: 30,
            }),
            finish_reason: Some(FinishReason::Stop),
            metadata: Default::default(),
        })
    }
}

impl ChatBackend for MockChatBackend {
    type ChatFut<'a> = Ready<Result<ChatResponse, LlmError>>;

    fn chat<'a>(&'a self, _req: ChatRequest) -> Self::ChatFut<'a> {
        std::future::ready(self.next_response())
    }
}
