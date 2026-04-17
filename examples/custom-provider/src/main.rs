// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

use converge_core::traits::{
    ChatBackend, ChatMessage, ChatRequest, ChatResponse, ChatRole, FinishReason, LlmError,
    ResponseFormat, TokenUsage,
};

struct EchoBackend {
    model_name: String,
}

impl EchoBackend {
    fn new(model: &str) -> Self {
        Self {
            model_name: model.to_string(),
        }
    }
}

impl ChatBackend for EchoBackend {
    type ChatFut<'a>
        = std::future::Ready<Result<ChatResponse, LlmError>>
    where
        Self: 'a;

    fn chat(&self, request: ChatRequest) -> Self::ChatFut<'_> {
        let user_content: String = request
            .messages
            .iter()
            .filter(|m| m.role == ChatRole::User)
            .map(|m| m.content.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let len = user_content.len() as u32;

        std::future::ready(Ok(ChatResponse {
            content: format!("Echo: {user_content}"),
            tool_calls: Vec::new(),
            model: Some(self.model_name.clone()),
            usage: Some(TokenUsage {
                prompt_tokens: len,
                completion_tokens: len,
                total_tokens: len * 2,
            }),
            finish_reason: Some(FinishReason::Stop),
            metadata: Default::default(),
        }))
    }
}

#[tokio::main]
async fn main() {
    println!("=== Custom Provider Example ===\n");

    let backend = EchoBackend::new("echo-v1");

    let request = ChatRequest {
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "What is the convergence model?".to_string(),
            tool_calls: Vec::new(),
            tool_call_id: None,
        }],
        system: None,
        tools: Vec::new(),
        response_format: ResponseFormat::default(),
        max_tokens: None,
        temperature: None,
        stop_sequences: Vec::new(),
        model: None,
    };

    match backend.chat(request).await {
        Ok(response) => {
            println!("Response: {}", response.content);
            if let Some(usage) = &response.usage {
                println!(
                    "Tokens:   {} in / {} out",
                    usage.prompt_tokens, usage.completion_tokens
                );
            }
        }
        Err(e) => println!("Error: {e}"),
    }

    println!("\n=== Done ===");
}
