use std::sync::Arc;

use converge_kernel::{
    ContextKey,
    formation::{
        Capability, CostClass, FormationAssemblySuggestor, FormationCatalog, FormationRequest,
        FormationTemplate, FormationTemplateMetadata, LatencyClass, ProfileSnapshot,
        ProviderRequest, ProviderSelectionSuggestor, StaticFormationTemplate,
        SuggestorCapability, SuggestorProfile, SuggestorRole,
    },
};
use converge_provider::{Backend, BackendKind};

struct AnalysisProfile;

impl SuggestorProfile for AnalysisProfile {
    fn role(&self) -> SuggestorRole {
        SuggestorRole::Analysis
    }

    fn output_keys(&self) -> &[ContextKey] {
        &[ContextKey::Hypotheses]
    }

    fn cost_hint(&self) -> CostClass {
        CostClass::Medium
    }

    fn latency_hint(&self) -> LatencyClass {
        LatencyClass::Interactive
    }

    fn capabilities(&self) -> &[SuggestorCapability] {
        &[SuggestorCapability::Analytics]
    }

    fn confidence_range(&self) -> (f32, f32) {
        (0.4, 0.9)
    }
}

struct MockBackend;

impl Backend for MockBackend {
    fn name(&self) -> &str {
        "mock-backend"
    }

    fn kind(&self) -> BackendKind {
        BackendKind::Llm
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::Reasoning]
    }
}

fn main() {
    let _formation = FormationRequest {
        id: "formation-1".to_string(),
        required_roles: vec![SuggestorRole::Analysis],
        required_capabilities: vec![SuggestorCapability::Analytics],
    };
    let _provider = ProviderRequest {
        id: "provider-1".to_string(),
        required_capabilities: vec![Capability::Reasoning],
        backend_requirements: None,
    };

    let catalog = vec![ProfileSnapshot::from_profile("analysis-a", &AnalysisProfile)];
    let _templates = FormationCatalog::new().with_template(FormationTemplate::static_template(
        StaticFormationTemplate::new(FormationTemplateMetadata::new(
            "analysis-only",
            "Single-role analysis template",
            [SuggestorRole::Analysis],
        )),
    ));
    let backends: Vec<Arc<dyn Backend>> = vec![Arc::new(MockBackend)];

    let _assembly = FormationAssemblySuggestor::new(catalog);
    let _selection = ProviderSelectionSuggestor::new(backends);
}
