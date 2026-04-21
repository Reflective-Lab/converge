// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Property-based tests for the template/pack system.
//!
//! Tests invariants that must hold for all inputs:
//! - YAML roundtrip preserves data
//! - Validator rejects forbidden keys
//! - Suggestor IDs must be unique
//! - Requirements config parses correctly

use proptest::prelude::*;

use super::types::{
    AgentWiring, BudgetConfig, CompatibilityRequirements, CustomRequirements, PackConfig,
    RequirementsConfig,
};
use super::validator::{PackValidationError, forbidden_keys, validate_pack_yaml_str};
use crate::semantic::{PackName, PackVersion, QualityThreshold, RequirementPreset};

/// Generate a valid pack name (alphanumeric with hyphens).
fn pack_name_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9-]{0,20}".prop_filter("valid pack name", |s| {
        crate::semantic::PackName::try_new(s.clone()).is_ok()
    })
}

/// Generate a semver version string.
fn version_strategy() -> impl Strategy<Value = String> {
    (1u32..10, 0u32..20, 0u32..100)
        .prop_map(|(major, minor, patch)| format!("{major}.{minor}.{patch}"))
}

/// Generate a valid agent ID (snake_case).
fn agent_id_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_]{0,30}"
}

/// Generate a description.
fn description_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 ]{1,100}"
}

/// Generate optional spec path.
fn spec_strategy() -> impl Strategy<Value = Option<String>> {
    prop_oneof![Just(None), "specs/[a-z-]+\\.feature".prop_map(Some),]
}

/// Generate budget config.
fn budget_strategy() -> impl Strategy<Value = BudgetConfig> {
    (1u32..1000, 10u32..10000).prop_map(|(max_cycles, max_facts)| BudgetConfig {
        max_cycles,
        max_facts,
    })
}

/// Generate requirements config.
fn requirements_strategy() -> impl Strategy<Value = Option<RequirementsConfig>> {
    prop_oneof![
        Just(None),
        prop_oneof![
            Just(RequirementPreset::FastExtraction),
            Just(RequirementPreset::Analysis),
            Just(RequirementPreset::Synthesis),
            Just(RequirementPreset::DeepResearch),
            Just(RequirementPreset::Deterministic),
        ]
        .prop_map(|preset| Some(RequirementsConfig::Preset(preset))),
        (
            prop_oneof![
                Just(None),
                Just(Some(converge_core::CostClass::VeryLow)),
                Just(Some(converge_core::CostClass::Medium)),
                Just(Some(converge_core::CostClass::VeryHigh))
            ],
            prop_oneof![Just(None), (1000u32..60000).prop_map(Some)],
            any::<Option<bool>>(),
            any::<Option<bool>>(),
        )
            .prop_map(
                |(cost_class, max_latency_ms, requires_reasoning, requires_web_search)| {
                    Some(RequirementsConfig::Custom(CustomRequirements {
                        cost_class,
                        max_latency_ms,
                        requires_reasoning,
                        requires_web_search,
                        min_quality: Some(QualityThreshold::new(0.7)),
                    }))
                }
            ),
    ]
}

/// Generate a single agent wiring.
fn agent_wiring_strategy() -> impl Strategy<Value = AgentWiring> {
    (agent_id_strategy(), requirements_strategy()).prop_map(|(id, requirements)| AgentWiring {
        id: id.into(),
        requirements,
    })
}

/// Generate a vector of agents with unique IDs.
fn unique_agents_strategy(max_count: usize) -> impl Strategy<Value = Vec<AgentWiring>> {
    prop::collection::vec(agent_wiring_strategy(), 0..max_count).prop_map(|agents| {
        let mut seen = std::collections::HashSet::new();
        agents
            .into_iter()
            .filter(|agent| seen.insert(agent.id.clone()))
            .collect()
    })
}

/// Generate a complete PackConfig.
fn pack_config_strategy() -> impl Strategy<Value = PackConfig> {
    (
        pack_name_strategy(),
        version_strategy(),
        description_strategy(),
        spec_strategy(),
        budget_strategy(),
        unique_agents_strategy(10),
    )
        .prop_map(
            |(name, version, description, spec, budget, agents)| PackConfig {
                name: PackName::new(name),
                version: PackVersion::new(version),
                description,
                spec,
                requires: CompatibilityRequirements::default(),
                budget,
                agents,
                metadata: std::collections::HashMap::new(),
            },
        )
}

proptest! {
    #[test]
    fn pack_config_yaml_roundtrip(pack in pack_config_strategy()) {
        let yaml = serde_yaml::to_string(&pack).expect("should serialize");
        let parsed: PackConfig = serde_yaml::from_str(&yaml).expect("should deserialize");

        prop_assert_eq!(pack.name, parsed.name);
        prop_assert_eq!(pack.version, parsed.version);
        prop_assert_eq!(pack.description, parsed.description);
        prop_assert_eq!(pack.spec, parsed.spec);
        prop_assert_eq!(pack.budget.max_cycles, parsed.budget.max_cycles);
        prop_assert_eq!(pack.budget.max_facts, parsed.budget.max_facts);
        prop_assert_eq!(pack.agents.len(), parsed.agents.len());

        for (orig, parsed) in pack.agents.iter().zip(parsed.agents.iter()) {
            prop_assert_eq!(&orig.id, &parsed.id);
        }
    }

    #[test]
    fn budget_config_roundtrip(budget in budget_strategy()) {
        let yaml = serde_yaml::to_string(&budget).expect("should serialize");
        let parsed: BudgetConfig = serde_yaml::from_str(&yaml).expect("should deserialize");

        prop_assert_eq!(budget.max_cycles, parsed.max_cycles);
        prop_assert_eq!(budget.max_facts, parsed.max_facts);
    }

    #[test]
    fn agent_wiring_roundtrip(agent in agent_wiring_strategy()) {
        let yaml = serde_yaml::to_string(&agent).expect("should serialize");
        let parsed: AgentWiring = serde_yaml::from_str(&yaml).expect("should deserialize");

        prop_assert_eq!(agent.id, parsed.id);
    }
}

proptest! {
    #[test]
    fn validator_rejects_forbidden_keys(
        name in pack_name_strategy(),
        version in version_strategy(),
        description in description_strategy(),
        forbidden_key in prop::sample::select(forbidden_keys().to_vec()),
    ) {
        let yaml = format!(
            r#"
name: {name}
version: "{version}"
description: {description}

{forbidden_key}:
  some_value: true
"#
        );

        let result = validate_pack_yaml_str(&yaml);
        prop_assert!(result.is_err(), "Should reject YAML with forbidden key: {}", forbidden_key);

        match result.unwrap_err() {
            PackValidationError::ForbiddenKey { key } => {
                prop_assert_eq!(key, forbidden_key);
            }
            PackValidationError::ForbiddenKeys { keys } => {
                prop_assert!(keys.contains(&forbidden_key.to_string()));
            }
            other => {
                prop_assert!(false, "Expected ForbiddenKey error, got: {:?}", other);
            }
        }
    }

    #[test]
    fn validator_accepts_valid_yaml(pack in pack_config_strategy()) {
        let yaml = serde_yaml::to_string(&pack).expect("should serialize");
        let result = validate_pack_yaml_str(&yaml);
        prop_assert!(result.is_ok(), "Valid pack should pass validation: {:?}", result);
    }
}

proptest! {
    #[test]
    fn duplicate_agent_ids_rejected(
        name in pack_name_strategy(),
        version in version_strategy(),
        duplicate_id in agent_id_strategy(),
    ) {
        let yaml = format!(
            r#"
name: {name}
version: "{version}"
description: Test pack with duplicate agents

agents:
  - id: {duplicate_id}
    requirements: fast_extraction
  - id: {duplicate_id}
    requirements: analysis
"#
        );

        let result = super::TemplateRegistry::parse_yaml(&yaml);
        prop_assert!(result.is_err(), "Should reject duplicate agent IDs");
        prop_assert!(
            result.unwrap_err().to_string().contains("duplicate agent id"),
            "Error should mention duplicate agent id"
        );
    }

    #[test]
    fn unique_agent_ids_accepted(
        name in pack_name_strategy(),
        version in version_strategy(),
        id1 in agent_id_strategy(),
        id2 in agent_id_strategy().prop_filter("different from id1", |s| s.len() > 0),
    ) {
        prop_assume!(id1 != id2);

        let yaml = format!(
            r#"
name: {name}
version: "{version}"
description: Test pack with unique agents

agents:
  - id: {id1}
    requirements: fast_extraction
  - id: {id2}
    requirements: analysis
"#
        );

        let result = super::TemplateRegistry::parse_yaml(&yaml);
        prop_assert!(result.is_ok(), "Unique agent IDs should be accepted: {:?}", result);
    }
}

proptest! {
    #[test]
    fn preset_requirements_parse(
        preset in prop_oneof![
            Just(RequirementPreset::FastExtraction),
            Just(RequirementPreset::Analysis),
            Just(RequirementPreset::Synthesis),
            Just(RequirementPreset::DeepResearch),
            Just(RequirementPreset::Deterministic),
        ]
    ) {
        let yaml = format!(
            r#"
name: test
version: "1.0.0"
description: Test

agents:
  - id: test_agent
    requirements: {}
"#,
            preset.as_str()
        );

        let result = super::TemplateRegistry::parse_yaml(&yaml);
        prop_assert!(result.is_ok());

        let pack = result.unwrap();
        match &pack.agents[0].requirements {
            Some(RequirementsConfig::Preset(parsed)) => {
                prop_assert_eq!(*parsed, preset);
            }
            other => {
                prop_assert!(false, "Expected Preset, got: {:?}", other);
            }
        }
    }

    #[test]
    fn custom_requirements_parse(
        cost_class in prop_oneof![Just("cheap"), Just("medium"), Just("expensive")],
        latency in 1000u32..60000,
    ) {
        let yaml = format!(
            r#"
name: test
version: "1.0.0"
description: Test

agents:
  - id: test_agent
    requirements:
      cost_class: {cost_class}
      max_latency_ms: {latency}
"#
        );

        let result = super::TemplateRegistry::parse_yaml(&yaml);
        prop_assert!(result.is_ok());

        let pack = result.unwrap();
        match &pack.agents[0].requirements {
            Some(RequirementsConfig::Custom(custom)) => {
                let expected_cost = super::types::parse_cost_class(cost_class)
                    .expect("generated cost class should parse");
                prop_assert_eq!(custom.cost_class, Some(expected_cost));
                prop_assert_eq!(custom.max_latency_ms, Some(latency));
            }
            other => {
                prop_assert!(false, "Expected Custom, got: {:?}", other);
            }
        }
    }
}

proptest! {
    #[test]
    fn missing_version_rejected(
        name in pack_name_strategy(),
        description in description_strategy(),
    ) {
        let yaml = format!(
            r#"
name: {name}
description: {description}
"#
        );

        let result = validate_pack_yaml_str(&yaml);
        prop_assert!(result.is_err());
        match result.unwrap_err() {
            PackValidationError::MissingKey { key } => {
                prop_assert_eq!(key, "version");
            }
            other => {
                prop_assert!(false, "Expected MissingKey error, got: {:?}", other);
            }
        }
    }

    #[test]
    fn missing_name_rejected(
        version in version_strategy(),
        description in description_strategy(),
    ) {
        let yaml = format!(
            r#"
version: "{version}"
description: {description}
"#
        );

        let result = validate_pack_yaml_str(&yaml);
        prop_assert!(result.is_err());
        match result.unwrap_err() {
            PackValidationError::MissingKey { key } => {
                prop_assert_eq!(key, "name");
            }
            other => {
                prop_assert!(false, "Expected MissingKey error, got: {:?}", other);
            }
        }
    }
}
