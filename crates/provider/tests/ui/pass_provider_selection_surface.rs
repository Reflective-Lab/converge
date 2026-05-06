use converge_provider::{
    BackendKind, BackendRequirements, Capability, CapabilityAssignment, ChatBackend,
    ChatBackendCapabilities, ChatBackendDescriptor, ChatRequest, ChatResponse,
    ChatBackendRegistry, ContextWindowTokens, CostClass, FinishReason, LatencyMillis,
    ModelName, ProviderAssignment, ProviderName, ProviderRequest, QualityScore,
    RegisteredChatBackend, TokenUsage,
};
use std::future;
use std::sync::Arc;

struct MockChatBackend;

impl ChatBackend for MockChatBackend {
    type ChatFut<'a>
        = future::Ready<Result<ChatResponse, converge_provider::LlmError>>
    where
        Self: 'a;

    fn chat<'a>(&'a self, _req: ChatRequest) -> Self::ChatFut<'a> {
        future::ready(Ok(ChatResponse {
            content: "ok".to_string(),
            tool_calls: Vec::new(),
            usage: Some(TokenUsage::default()),
            model: Some("mock-model".to_string()),
            finish_reason: Some(FinishReason::Stop),
            metadata: std::collections::HashMap::new(),
        }))
    }
}

fn main() {
    let _requirements = BackendRequirements::new(BackendKind::Llm)
        .with_capability(Capability::Reasoning)
        .with_capability(Capability::TextGeneration);

    let request = ProviderRequest {
        id: "provider-1".to_string(),
        required_capabilities: vec![Capability::Reasoning],
        backend_requirements: None,
    };

    let _assignment = ProviderAssignment {
        request_id: request.id,
        assignments: vec![CapabilityAssignment {
            capability: Capability::Reasoning,
            backend_name: "mock-backend".to_string(),
        }],
        unmatched: Vec::new(),
        coverage_ratio: 1.0,
    };

    let descriptor = ChatBackendDescriptor::new(
        ProviderName::new("mock").unwrap(),
        ModelName::new("mock-model").unwrap(),
        CostClass::Low,
        LatencyMillis::new(100).unwrap(),
        QualityScore::new(0.9).unwrap(),
    )
    .with_capabilities(
        ChatBackendCapabilities::new()
            .with_tool_use(true)
            .with_context_tokens(ContextWindowTokens::new(8_192).unwrap()),
    );
    let registered = RegisteredChatBackend::new(descriptor, Arc::new(MockChatBackend));
    let _registry = ChatBackendRegistry::new().with_backend(registered);
}
