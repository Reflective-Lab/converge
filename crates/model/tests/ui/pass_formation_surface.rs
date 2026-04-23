use converge_model::{
    ContextKey, FormationCatalog, FormationPlan, FormationRequest, FormationTemplate,
    FormationTemplateMetadata, ProfileSnapshot, RoleAssignment, StaticFormationTemplate,
    SuggestorCapability, SuggestorRole,
};
use converge_provider_api::{CostClass, LatencyClass};

fn main() {
    let request = FormationRequest {
        id: "formation-1".to_string(),
        required_roles: vec![SuggestorRole::Analysis, SuggestorRole::Planning],
        required_capabilities: vec![SuggestorCapability::Analytics],
    };

    let _snapshot = ProfileSnapshot {
        name: "analysis-a".to_string(),
        role: SuggestorRole::Analysis,
        output_keys: vec![ContextKey::Hypotheses],
        cost_hint: CostClass::Medium,
        latency_hint: LatencyClass::Interactive,
        capabilities: vec![SuggestorCapability::Analytics],
        confidence_min: 0.4,
        confidence_max: 0.9,
    };

    let _plan = FormationPlan {
        request_id: request.id,
        assignments: vec![RoleAssignment {
            role: SuggestorRole::Analysis,
            suggestor: "analysis-a".to_string(),
        }],
        unmatched_roles: vec![SuggestorRole::Planning],
        coverage_ratio: 0.5,
    };

    let _catalog = FormationCatalog::new().with_template(FormationTemplate::static_template(
        StaticFormationTemplate::new(FormationTemplateMetadata::new(
            "analysis-only",
            "Single-role analysis template",
            [SuggestorRole::Analysis],
        )),
    ));
}
