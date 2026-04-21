// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use converge_pack::ContextKey;

use crate::model_selection::{CostClass, LatencyClass};

pub trait SuggestorProfile {
    fn role(&self) -> SuggestorRole;
    fn output_keys(&self) -> &[ContextKey];
    fn cost_hint(&self) -> CostClass;
    fn latency_hint(&self) -> LatencyClass;
    fn capabilities(&self) -> &[SuggestorCapability];
    fn confidence_range(&self) -> (f32, f32);
}

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
    pub fn from_profile(name: impl Into<String>, p: &dyn SuggestorProfile) -> Self {
        let (min, max) = p.confidence_range();
        Self {
            name: name.into(),
            role: p.role(),
            output_keys: p.output_keys().to_vec(),
            cost_hint: p.cost_hint(),
            latency_hint: p.latency_hint(),
            capabilities: p.capabilities().to_vec(),
            confidence_min: min,
            confidence_max: max,
        }
    }
}

#[cfg(test)]
mod tests {
    use converge_provider_api::selection::{CostClass, LatencyClass};

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
        let s = AnalysisSuggestor;
        let snap = ProfileSnapshot::from_profile("analysis-1", &s);

        assert_eq!(snap.name, "analysis-1");
        assert_eq!(snap.role, SuggestorRole::Analysis);
        assert_eq!(snap.output_keys, vec![ContextKey::Hypotheses]);
        assert_eq!(snap.confidence_min, 0.5);
        assert_eq!(snap.confidence_max, 0.95);
        assert_eq!(snap.capabilities, vec![SuggestorCapability::LlmReasoning]);
    }

    #[test]
    fn profile_snapshot_serde_roundtrip() {
        let s = AnalysisSuggestor;
        let snap = ProfileSnapshot::from_profile("analysis-1", &s);
        let json = serde_json::to_string(&snap).unwrap();
        let back: ProfileSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, snap.name);
        assert_eq!(back.role, snap.role);
        assert_eq!(back.confidence_min, snap.confidence_min);
    }

    #[test]
    fn suggestor_role_serde_lowercase() {
        let json = serde_json::to_string(&SuggestorRole::Planning).unwrap();
        assert_eq!(json, "\"planning\"");
        let back: SuggestorRole = serde_json::from_str(&json).unwrap();
        assert_eq!(back, SuggestorRole::Planning);
    }

    #[test]
    fn suggestor_capability_serde_lowercase() {
        let json = serde_json::to_string(&SuggestorCapability::PolicyEnforcement).unwrap();
        assert_eq!(json, "\"policy_enforcement\"");
        let back: SuggestorCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(back, SuggestorCapability::PolicyEnforcement);
    }

    #[test]
    fn all_roles_are_distinct() {
        use std::collections::HashSet;
        let roles = [
            SuggestorRole::Analysis,
            SuggestorRole::Planning,
            SuggestorRole::Evaluation,
            SuggestorRole::Constraint,
            SuggestorRole::Signal,
            SuggestorRole::Synthesis,
            SuggestorRole::Meta,
        ];
        let set: HashSet<_> = roles.iter().collect();
        assert_eq!(set.len(), roles.len());
    }

    #[test]
    fn all_capabilities_are_distinct() {
        use std::collections::HashSet;
        let caps = [
            SuggestorCapability::LlmReasoning,
            SuggestorCapability::KnowledgeRetrieval,
            SuggestorCapability::Analytics,
            SuggestorCapability::Optimization,
            SuggestorCapability::PolicyEnforcement,
            SuggestorCapability::HumanInTheLoop,
            SuggestorCapability::ExperienceLearning,
        ];
        let set: HashSet<_> = caps.iter().collect();
        assert_eq!(set.len(), caps.len());
    }
}
