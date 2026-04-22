// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Canonical formation semantics for downstream consumers and embedders.

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
