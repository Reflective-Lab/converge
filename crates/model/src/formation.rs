// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Canonical formation semantics for downstream consumers and embedders.

pub use converge_core::{FormationKind, ScoringWeights};

use converge_pack::ContextKey;
use converge_provider_api::{CostClass, LatencyClass};
use serde::{Deserialize, Serialize};

/// Describes the shape and operating envelope of a suggestor.
pub trait SuggestorProfile {
    fn role(&self) -> SuggestorRole;
    fn output_keys(&self) -> &[ContextKey];
    fn cost_hint(&self) -> CostClass;
    fn latency_hint(&self) -> LatencyClass;
    fn capabilities(&self) -> &[SuggestorCapability];
    fn confidence_range(&self) -> (f32, f32);
}

/// The coarse role a suggestor plays inside a formation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestorRole {
    Analysis,
    Planning,
    Evaluation,
    Constraint,
    Signal,
    Synthesis,
    Meta,
}

/// Capabilities formation assembly may use to filter or prefer suggestors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestorCapability {
    LlmReasoning,
    KnowledgeRetrieval,
    Analytics,
    Optimization,
    PolicyEnforcement,
    HumanInTheLoop,
    ExperienceLearning,
}

/// Serializable snapshot of a suggestor's formation metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSnapshot {
    pub name: String,
    pub role: SuggestorRole,
    pub output_keys: Vec<ContextKey>,
    pub cost_hint: CostClass,
    pub latency_hint: LatencyClass,
    pub capabilities: Vec<SuggestorCapability>,
    pub confidence_min: f32,
    pub confidence_max: f32,
}

impl ProfileSnapshot {
    #[must_use]
    pub fn from_profile(name: impl Into<String>, profile: &dyn SuggestorProfile) -> Self {
        let (min, max) = profile.confidence_range();
        Self {
            name: name.into(),
            role: profile.role(),
            output_keys: profile.output_keys().to_vec(),
            cost_hint: profile.cost_hint(),
            latency_hint: profile.latency_hint(),
            capabilities: profile.capabilities().to_vec(),
            confidence_min: min,
            confidence_max: max,
        }
    }
}

/// Classification-facing metadata for a reusable formation template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationTemplateMetadata {
    /// Stable catalog identifier.
    pub id: String,
    /// Human-readable explanation of the formation's purpose.
    pub description: String,
    /// Loose intent phrases that suggest this template is relevant.
    pub keywords: Vec<String>,
    /// Entity labels or structured fields the template expects to see.
    pub entities: Vec<String>,
    /// Roles the assembled formation must cover.
    pub required_roles: Vec<SuggestorRole>,
    /// Problem-level capabilities this template is designed to satisfy.
    pub required_capabilities: Vec<SuggestorCapability>,
}

impl FormationTemplateMetadata {
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        description: impl Into<String>,
        required_roles: impl IntoIterator<Item = SuggestorRole>,
    ) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            keywords: Vec::new(),
            entities: Vec::new(),
            required_roles: required_roles.into_iter().collect(),
            required_capabilities: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    #[must_use]
    pub fn with_entity(mut self, entity: impl Into<String>) -> Self {
        self.entities.push(entity.into());
        self
    }

    #[must_use]
    pub fn with_required_capability(mut self, capability: SuggestorCapability) -> Self {
        self.required_capabilities.push(capability);
        self
    }
}

/// Template for a static formation shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticFormationTemplate {
    pub metadata: FormationTemplateMetadata,
}

impl StaticFormationTemplate {
    #[must_use]
    pub fn new(metadata: FormationTemplateMetadata) -> Self {
        Self { metadata }
    }
}

/// Template for a scored formation shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredFormationTemplate {
    pub metadata: FormationTemplateMetadata,
    pub top_n: usize,
    pub scoring_weights: ScoringWeights,
}

impl ScoredFormationTemplate {
    #[must_use]
    pub fn new(metadata: FormationTemplateMetadata, top_n: usize) -> Self {
        Self {
            metadata,
            top_n,
            scoring_weights: ScoringWeights::default(),
        }
    }
}

/// Template for a deliberated formation shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliberatedFormationTemplate {
    pub metadata: FormationTemplateMetadata,
    pub huddle_max_cycles: u32,
    pub scoring_weights: ScoringWeights,
    pub min_confidence_threshold: f32,
}

impl DeliberatedFormationTemplate {
    #[must_use]
    pub fn new(metadata: FormationTemplateMetadata, huddle_max_cycles: u32) -> Self {
        Self {
            metadata,
            huddle_max_cycles,
            scoring_weights: ScoringWeights::default(),
            min_confidence_threshold: 0.6,
        }
    }
}

/// Template for an open-claw formation shape.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawFormationTemplate {
    pub metadata: FormationTemplateMetadata,
    pub max_extra_loops: u32,
}

impl OpenClawFormationTemplate {
    #[must_use]
    pub fn new(metadata: FormationTemplateMetadata, max_extra_loops: u32) -> Self {
        Self {
            metadata,
            max_extra_loops,
        }
    }
}

/// Reusable formation choice before concrete suggestor assembly happens.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FormationTemplate {
    Static(StaticFormationTemplate),
    Scored(ScoredFormationTemplate),
    Deliberated(DeliberatedFormationTemplate),
    OpenClaw(OpenClawFormationTemplate),
}

impl FormationTemplate {
    #[must_use]
    pub fn static_template(template: StaticFormationTemplate) -> Self {
        Self::Static(template)
    }

    #[must_use]
    pub fn scored(template: ScoredFormationTemplate) -> Self {
        Self::Scored(template)
    }

    #[must_use]
    pub fn deliberated(template: DeliberatedFormationTemplate) -> Self {
        Self::Deliberated(template)
    }

    #[must_use]
    pub fn open_claw(template: OpenClawFormationTemplate) -> Self {
        Self::OpenClaw(template)
    }

    #[must_use]
    pub fn metadata(&self) -> &FormationTemplateMetadata {
        match self {
            Self::Static(template) => &template.metadata,
            Self::Scored(template) => &template.metadata,
            Self::Deliberated(template) => &template.metadata,
            Self::OpenClaw(template) => &template.metadata,
        }
    }

    #[must_use]
    pub fn id(&self) -> &str {
        &self.metadata().id
    }

    #[must_use]
    pub fn kind(&self) -> FormationKind {
        match self {
            Self::Static(_) => FormationKind::Static,
            Self::Scored(_) => FormationKind::Scored,
            Self::Deliberated(_) => FormationKind::Deliberated,
            Self::OpenClaw(_) => FormationKind::OpenClaw,
        }
    }

    /// Compile this template into the current request surface.
    ///
    /// The template's `required_capabilities` remain catalog metadata for
    /// problem classification and later guru/tournament logic. They are not
    /// copied into [`FormationRequest::required_capabilities`] because the
    /// current assembly suggestor interprets that field as a global
    /// per-suggestor eligibility gate.
    #[must_use]
    pub fn to_request(&self, request_id: impl Into<String>) -> FormationRequest {
        FormationRequest {
            id: request_id.into(),
            required_roles: self.metadata().required_roles.clone(),
            required_capabilities: Vec::new(),
        }
    }

    fn match_score(&self, query: &FormationTemplateQuery) -> Option<usize> {
        let metadata = self.metadata();

        if !query
            .required_capabilities
            .iter()
            .all(|capability| metadata.required_capabilities.contains(capability))
        {
            return None;
        }

        if query.is_empty() {
            return Some(0);
        }

        let keyword_hits = string_matches(&metadata.keywords, &query.keywords);
        let entity_hits = string_matches(&metadata.entities, &query.entities);
        let capability_hits = query.required_capabilities.len() * 2;
        let score = keyword_hits + entity_hits + capability_hits;

        (score > 0).then_some(score)
    }
}

/// Query surface a classifier can emit before choosing a formation template.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormationTemplateQuery {
    pub keywords: Vec<String>,
    pub entities: Vec<String>,
    pub required_capabilities: Vec<SuggestorCapability>,
}

impl FormationTemplateQuery {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    #[must_use]
    pub fn with_entity(mut self, entity: impl Into<String>) -> Self {
        self.entities.push(entity.into());
        self
    }

    #[must_use]
    pub fn with_required_capability(mut self, capability: SuggestorCapability) -> Self {
        self.required_capabilities.push(capability);
        self
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.keywords.is_empty()
            && self.entities.is_empty()
            && self.required_capabilities.is_empty()
    }
}

/// Registry of reusable formation templates.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FormationCatalog {
    templates: Vec<FormationTemplate>,
}

impl FormationCatalog {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_template(mut self, template: FormationTemplate) -> Self {
        self.register(template);
        self
    }

    pub fn register(&mut self, template: FormationTemplate) {
        let template_id = template.id().to_string();

        if let Some(existing) = self
            .templates
            .iter_mut()
            .find(|existing| existing.id() == template_id)
        {
            *existing = template;
        } else {
            self.templates.push(template);
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    #[must_use]
    pub fn get(&self, template_id: &str) -> Option<&FormationTemplate> {
        self.templates
            .iter()
            .find(|template| template.id() == template_id)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, FormationTemplate> {
        self.templates.iter()
    }

    #[must_use]
    pub fn matches(&self, query: &FormationTemplateQuery) -> Vec<&FormationTemplate> {
        let mut matches = self
            .templates
            .iter()
            .enumerate()
            .filter_map(|(index, template)| {
                template
                    .match_score(query)
                    .map(|score| (score, index, template))
            })
            .collect::<Vec<_>>();

        matches.sort_by(
            |(left_score, left_index, _), (right_score, right_index, _)| {
                right_score
                    .cmp(left_score)
                    .then_with(|| left_index.cmp(right_index))
            },
        );

        matches
            .into_iter()
            .map(|(_, _, template)| template)
            .collect()
    }

    #[must_use]
    pub fn top_match(&self, query: &FormationTemplateQuery) -> Option<&FormationTemplate> {
        self.matches(query).into_iter().next()
    }
}

impl<'a> IntoIterator for &'a FormationCatalog {
    type Item = &'a FormationTemplate;
    type IntoIter = std::slice::Iter<'a, FormationTemplate>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

fn string_matches(catalog_values: &[String], query_values: &[String]) -> usize {
    query_values
        .iter()
        .filter(|query| {
            catalog_values
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(query))
        })
        .count()
}

/// Structured request for formation assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationRequest {
    /// Stable request identifier used for idempotency.
    pub id: String,
    /// Roles that must be covered. Duplicates mean multiple seats of the same role.
    pub required_roles: Vec<SuggestorRole>,
    /// Extra capability constraints all eligible suggestors must satisfy.
    pub required_capabilities: Vec<SuggestorCapability>,
}

/// Structured result of formation assembly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationPlan {
    /// The request this plan answers.
    pub request_id: String,
    /// Matched role-to-suggestor assignments.
    pub assignments: Vec<RoleAssignment>,
    /// Roles that could not be filled from the catalog.
    pub unmatched_roles: Vec<SuggestorRole>,
    /// `assignments.len() / required_roles.len()` — 1.0 is full coverage.
    pub coverage_ratio: f64,
}

/// A single role-to-suggestor assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoleAssignment {
    pub role: SuggestorRole,
    pub suggestor: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AnalysisSuggestor;

    impl SuggestorProfile for AnalysisSuggestor {
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
            &[SuggestorCapability::LlmReasoning]
        }

        fn confidence_range(&self) -> (f32, f32) {
            (0.5, 0.95)
        }
    }

    #[test]
    fn profile_snapshot_captures_all_fields() {
        let suggestor = AnalysisSuggestor;
        let snap = ProfileSnapshot::from_profile("analysis-1", &suggestor);

        assert_eq!(snap.name, "analysis-1");
        assert_eq!(snap.role, SuggestorRole::Analysis);
        assert_eq!(snap.output_keys, vec![ContextKey::Hypotheses]);
        assert_eq!(snap.confidence_min, 0.5);
        assert_eq!(snap.confidence_max, 0.95);
        assert_eq!(snap.capabilities, vec![SuggestorCapability::LlmReasoning]);
    }

    #[test]
    fn profile_snapshot_serde_roundtrip() {
        let suggestor = AnalysisSuggestor;
        let snap = ProfileSnapshot::from_profile("analysis-1", &suggestor);
        let json = serde_json::to_string(&snap).unwrap();
        let back: ProfileSnapshot = serde_json::from_str(&json).unwrap();

        assert_eq!(back.name, snap.name);
        assert_eq!(back.role, snap.role);
        assert_eq!(back.confidence_min, snap.confidence_min);
    }

    #[test]
    fn template_compiles_to_current_request_surface() {
        let template = FormationTemplate::deliberated(DeliberatedFormationTemplate::new(
            FormationTemplateMetadata::new(
                "market-entry",
                "Go or no-go market launch formation",
                [SuggestorRole::Analysis, SuggestorRole::Planning],
            )
            .with_keyword("launch")
            .with_entity("market")
            .with_required_capability(SuggestorCapability::LlmReasoning),
            4,
        ));

        let request = template.to_request("req-1");

        assert_eq!(template.id(), "market-entry");
        assert_eq!(template.kind(), FormationKind::Deliberated);
        assert_eq!(
            request.required_roles,
            vec![SuggestorRole::Analysis, SuggestorRole::Planning]
        );
        assert!(
            request.required_capabilities.is_empty(),
            "template capabilities stay in the catalog for selection"
        );

        match &template {
            FormationTemplate::Deliberated(inner) => {
                assert_eq!(inner.huddle_max_cycles, 4);
                assert!((inner.min_confidence_threshold - 0.6).abs() < f32::EPSILON);
            }
            other => panic!("expected deliberated template, got {other:?}"),
        }
    }

    #[test]
    fn catalog_prefers_more_specific_matching_template() {
        let broad = FormationTemplate::static_template(StaticFormationTemplate::new(
            FormationTemplateMetadata::new(
                "general-market",
                "General market analysis formation",
                [SuggestorRole::Analysis],
            )
            .with_keyword("market")
            .with_entity("region"),
        ));
        let specific = FormationTemplate::deliberated(DeliberatedFormationTemplate::new(
            FormationTemplateMetadata::new(
                "market-entry",
                "Launch decision formation",
                [
                    SuggestorRole::Analysis,
                    SuggestorRole::Planning,
                    SuggestorRole::Constraint,
                ],
            )
            .with_keyword("market")
            .with_keyword("launch")
            .with_entity("region")
            .with_entity("competitors")
            .with_required_capability(SuggestorCapability::LlmReasoning),
            3,
        ));
        let catalog = FormationCatalog::new()
            .with_template(broad)
            .with_template(specific);
        let query = FormationTemplateQuery::new()
            .with_keyword("launch")
            .with_entity("competitors")
            .with_required_capability(SuggestorCapability::LlmReasoning);

        let matches = catalog.matches(&query);

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].id(), "market-entry");
        assert_eq!(
            catalog.top_match(&query).map(FormationTemplate::kind),
            Some(FormationKind::Deliberated)
        );
    }

    #[test]
    fn catalog_register_replaces_existing_template_by_id() {
        let mut catalog = FormationCatalog::new();
        catalog.register(FormationTemplate::static_template(
            StaticFormationTemplate::new(FormationTemplateMetadata::new(
                "market-entry",
                "First revision",
                [SuggestorRole::Analysis],
            )),
        ));
        catalog.register(FormationTemplate::scored(ScoredFormationTemplate::new(
            FormationTemplateMetadata::new(
                "market-entry",
                "Second revision",
                [SuggestorRole::Analysis, SuggestorRole::Planning],
            ),
            2,
        )));

        assert_eq!(catalog.len(), 1);
        assert_eq!(
            catalog.get("market-entry").map(FormationTemplate::kind),
            Some(FormationKind::Scored)
        );
    }

    #[test]
    fn catalog_serde_roundtrip_preserves_templates() {
        let catalog = FormationCatalog::new().with_template(FormationTemplate::open_claw(
            OpenClawFormationTemplate::new(
                FormationTemplateMetadata::new(
                    "stress-test",
                    "Open-claw escalation formation",
                    [
                        SuggestorRole::Analysis,
                        SuggestorRole::Evaluation,
                        SuggestorRole::Constraint,
                    ],
                )
                .with_keyword("stress")
                .with_entity("scenario")
                .with_required_capability(SuggestorCapability::ExperienceLearning),
                2,
            ),
        ));

        let json = serde_json::to_string(&catalog).unwrap();
        let roundtrip: FormationCatalog = serde_json::from_str(&json).unwrap();

        assert_eq!(roundtrip.len(), 1);
        assert_eq!(
            roundtrip.get("stress-test").map(FormationTemplate::kind),
            Some(FormationKind::OpenClaw)
        );
    }

    #[test]
    fn formation_request_and_plan_roundtrip() {
        let request = FormationRequest {
            id: "req-1".to_string(),
            required_roles: vec![SuggestorRole::Analysis, SuggestorRole::Planning],
            required_capabilities: vec![SuggestorCapability::Analytics],
        };
        let plan = FormationPlan {
            request_id: request.id.clone(),
            assignments: vec![RoleAssignment {
                role: SuggestorRole::Analysis,
                suggestor: "analysis-1".to_string(),
            }],
            unmatched_roles: vec![SuggestorRole::Planning],
            coverage_ratio: 0.5,
        };

        let request_back: FormationRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        let plan_back: FormationPlan =
            serde_json::from_str(&serde_json::to_string(&plan).unwrap()).unwrap();

        assert_eq!(request_back.required_roles, request.required_roles);
        assert_eq!(plan_back.assignments, plan.assignments);
        assert_eq!(plan_back.unmatched_roles, plan.unmatched_roles);
    }
}
