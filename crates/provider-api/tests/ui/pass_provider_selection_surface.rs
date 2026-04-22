use converge_provider_api::{
    BackendKind, BackendRequirements, Capability, CapabilityAssignment, ProviderAssignment,
    ProviderRequest,
};

fn main() {
    let _requirements = BackendRequirements::new(BackendKind::Llm)
        .with_capability(Capability::Reasoning)
        .with_capability(Capability::TextGeneration);

    let request = ProviderRequest {
        id: "provider-1".to_string(),
        required_capabilities: vec![Capability::Reasoning],
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
}
