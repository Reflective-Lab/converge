// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Template type definitions for job configuration.
//!
//
//!
//! # Contract: YAML = Wiring, Gherkin = Semantics
//!
//! These types represent the WIRING configuration from pack YAML files.
//! They do NOT contain semantic definitions (invariants, validation rules).
//!
//! Semantics are defined in Gherkin specs and compiled at build time.
//!
//! ## Allowed in YAML (wiring)
//! - `name`, `version`, `description` - pack identity
//! - `spec` - reference to Gherkin file
//! - `requires` - compatibility requirements
//! - `budget` - operational limits
//! - `agents` - provider wiring (name → requirements)
//! - `metadata` - informational
//!
//! ## Forbidden in YAML (must be Gherkin)
//! - `validation` - thresholds, forbidden terms
//! - `invariants` - business rules
//! - `acceptance_criteria` - done conditions
//! - `forbidden_terms` - content rules

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

use crate::semantic::{
    AgentName, PackName, PackVersion, ProviderId, QualityThreshold, RequirementPreset,
    VersionRequirement,
};
use converge_core::{ContextKey, CostClass, FactId};

/// A domain pack configuration (wiring only).
///
/// This represents the YAML pack file. Semantic definitions
/// (invariants, validation rules) come from the referenced Gherkin spec.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PackConfig {
    /// Pack name (e.g., "growth-strategy").
    #[schema(value_type = String)]
    pub name: PackName,

    /// Pack version (semver).
    #[schema(value_type = String)]
    pub version: PackVersion,

    /// Human-readable description.
    pub description: String,

    /// Reference to Gherkin spec file.
    #[serde(default)]
    pub spec: Option<String>,

    /// Compatibility requirements.
    #[serde(default)]
    pub requires: CompatibilityRequirements,

    /// Budget configuration.
    #[serde(default)]
    pub budget: BudgetConfig,

    /// Suggestor → provider wiring.
    #[serde(default, deserialize_with = "deserialize_unique_agents")]
    pub agents: Vec<AgentWiring>,

    /// Pack metadata.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Compatibility requirements for the pack.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CompatibilityRequirements {
    /// Minimum core version required (e.g., ">=0.6.0").
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub core: Option<VersionRequirement>,

    /// Minimum runtime API version required.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub runtime_api: Option<VersionRequirement>,
}

/// Budget configuration for the convergence engine.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BudgetConfig {
    /// Maximum number of convergence cycles.
    #[serde(default = "default_max_cycles")]
    pub max_cycles: u32,

    /// Maximum number of facts in context.
    #[serde(default = "default_max_facts")]
    pub max_facts: u32,
}

fn default_max_cycles() -> u32 {
    50
}

fn default_max_facts() -> u32 {
    500
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_cycles: default_max_cycles(),
            max_facts: default_max_facts(),
        }
    }
}

/// Suggestor → provider wiring.
///
/// Maps an agent ID (from Gherkin `@agent @id:xxx`) to provider requirements.
/// Does NOT contain prompts or semantic behavior (those come from compiled Gherkin).
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentWiring {
    /// Suggestor ID (must match `@agent @id:xxx` in Gherkin spec).
    /// Supports both "id" and "name" in YAML for backward compatibility.
    #[serde(alias = "name")]
    pub id: AgentName,

    /// Provider requirements for model selection.
    #[serde(default)]
    pub requirements: Option<RequirementsConfig>,
}

/// Model selection requirements configuration.
///
/// Can be either a named preset or custom requirements.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum RequirementsConfig {
    /// Named preset (e.g., "fast_extraction", "analysis", "synthesis").
    Preset(RequirementPreset),
    /// Custom requirements specification.
    Custom(CustomRequirements),
}

/// Custom model selection requirements.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CustomRequirements {
    /// Cost class: "cheap", "medium", "expensive".
    #[serde(
        default,
        deserialize_with = "deserialize_optional_cost_class",
        serialize_with = "serialize_optional_cost_class"
    )]
    #[schema(value_type = Option<String>)]
    pub cost_class: Option<CostClass>,

    /// Maximum latency in milliseconds.
    #[serde(default)]
    pub max_latency_ms: Option<u32>,

    /// Whether the model needs reasoning capabilities.
    #[serde(default)]
    pub requires_reasoning: Option<bool>,

    /// Whether the model needs web search capabilities.
    #[serde(default)]
    pub requires_web_search: Option<bool>,

    /// Minimum quality score (0.0 - 1.0).
    #[serde(default)]
    #[schema(value_type = Option<f64>)]
    pub min_quality: Option<QualityThreshold>,
}

/// Seed fact definition for job requests.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SeedFact {
    /// Unique identifier for the seed (e.g., "market:nordic-b2b").
    #[schema(value_type = String)]
    pub id: FactId,
    /// Content of the seed fact.
    pub content: String,
}

/// Job overrides that can be applied to a pack.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct JobOverrides {
    /// Override budget configuration.
    #[serde(default)]
    pub budget: Option<BudgetConfig>,

    /// Seed facts to inject.
    #[serde(default)]
    pub seeds: Vec<SeedFact>,

    /// Per-agent requirement overrides keyed by agent name.
    #[serde(default)]
    pub agents: HashMap<AgentName, AgentOverrides>,

    /// Whether to use real LLM providers (requires API keys).
    ///
    /// - `true`: Use real LLM providers based on available API keys
    /// - `false` (default): Use deterministic mock providers
    ///
    /// When enabled, the runtime will check for API keys in environment
    /// variables (ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.) and use the
    /// best available provider based on agent requirements.
    #[serde(default)]
    pub use_llm: bool,
}

/// Overrides for a specific agent's provider requirements.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentOverrides {
    /// Override requirements.
    #[serde(default)]
    pub requirements: Option<RequirementsConfig>,
}

/// Provider preferences for job execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProviderPreferences {
    /// Preferred providers in order of preference.
    #[serde(default)]
    pub prefer: Vec<ProviderId>,

    /// Providers to exclude.
    #[serde(default)]
    pub exclude: Vec<ProviderId>,
}

/// Job request using a domain pack.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PackJobRequest {
    /// Pack to use (e.g., "growth-strategy").
    pub pack: PackName,

    /// Overrides to apply.
    #[serde(default)]
    pub overrides: JobOverrides,

    /// Provider preferences.
    #[serde(default)]
    pub providers: ProviderPreferences,
}

/// Summary of a pack for listing.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PackSummary {
    /// Pack name.
    #[schema(value_type = String)]
    pub name: PackName,
    /// Pack description.
    pub description: String,
    /// Pack version.
    #[schema(value_type = String)]
    pub version: PackVersion,
    /// Number of agents.
    pub agent_count: usize,
    /// Gherkin spec reference.
    pub spec: Option<String>,
}

impl From<&PackConfig> for PackSummary {
    fn from(pack: &PackConfig) -> Self {
        Self {
            name: pack.name.clone(),
            description: pack.description.clone(),
            version: pack.version.clone(),
            agent_count: pack.agents.len(),
            spec: pack.spec.clone(),
        }
    }
}

/// Suggestor definition within a template.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AgentDefinition {
    /// Suggestor name (unique within template).
    #[schema(value_type = String)]
    pub name: AgentName,

    /// Suggestor type: "seed", "llm", or "validation".
    #[serde(rename = "type")]
    pub agent_type: AgentType,

    /// Model selection requirements (for LLM agents).
    #[serde(default)]
    pub requirements: Option<RequirementsConfig>,

    /// Context key this agent writes to.
    #[serde(default)]
    #[schema(value_type = Option<String>)]
    pub output_key: Option<ContextKey>,

    /// Context keys this agent depends on.
    #[serde(default)]
    #[schema(value_type = Vec<String>)]
    pub depends_on: Vec<ContextKey>,

    /// System prompt (for LLM agents).
    #[serde(default)]
    pub system_prompt: Option<String>,

    /// Prompt template with {context} placeholder (for LLM agents).
    #[serde(default)]
    pub prompt_template: Option<String>,

    /// Suggestor-specific configuration.
    #[serde(default)]
    pub config: HashMap<String, serde_json::Value>,
}

/// Suggestor type enumeration.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// Seed agent that provides initial facts.
    Seed,
    /// LLM-powered agent.
    Llm,
    /// Validation agent that promotes proposals to facts.
    Validation,
}

fn deserialize_unique_agents<'de, D>(deserializer: D) -> Result<Vec<AgentWiring>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let agents = Vec::<AgentWiring>::deserialize(deserializer)?;
    let mut seen = std::collections::HashSet::new();
    for agent in &agents {
        if !seen.insert(agent.id.clone()) {
            return Err(serde::de::Error::custom(format!(
                "duplicate agent id '{}'",
                agent.id
            )));
        }
    }
    Ok(agents)
}

fn serialize_optional_cost_class<S>(
    value: &Option<CostClass>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(cost) => serializer.serialize_some(cost_class_label(*cost)),
        None => serializer.serialize_none(),
    }
}

fn deserialize_optional_cost_class<'de, D>(deserializer: D) -> Result<Option<CostClass>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = Option::<String>::deserialize(deserializer)?;
    raw.map(|value| parse_cost_class(&value).map_err(serde::de::Error::custom))
        .transpose()
}

pub(super) fn parse_cost_class(value: &str) -> Result<CostClass, String> {
    match value.trim().to_ascii_lowercase().as_str() {
        "free" => Ok(CostClass::Free),
        "cheap" | "very_low" | "very-low" => Ok(CostClass::VeryLow),
        "low" => Ok(CostClass::Low),
        "medium" => Ok(CostClass::Medium),
        "high" => Ok(CostClass::High),
        "expensive" | "very_high" | "very-high" => Ok(CostClass::VeryHigh),
        other => Err(format!("unsupported cost class '{other}'")),
    }
}

fn cost_class_label(value: CostClass) -> &'static str {
    match value {
        CostClass::Free => "free",
        CostClass::VeryLow => "cheap",
        CostClass::Low => "low",
        CostClass::Medium => "medium",
        CostClass::High => "high",
        CostClass::VeryHigh => "expensive",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_pack_config() {
        let yaml = r#"
name: test-pack
version: "1.0.0"
description: A test pack
spec: specs/test.feature

requires:
  core: ">=0.6.0"
  runtime_api: ">=0.1.0"

budget:
  max_cycles: 100
  max_facts: 1000

agents:
  - id: test_agent
    requirements: fast_extraction

  - id: analysis_agent
    requirements:
      cost_class: medium
      requires_reasoning: true

metadata:
  category: test
  tags:
    - test
    - example
"#;

        let pack: PackConfig = serde_yaml::from_str(yaml).expect("should parse");
        assert_eq!(pack.name, "test-pack");
        assert_eq!(pack.version, "1.0.0");
        assert_eq!(pack.spec, Some("specs/test.feature".to_string()));
        assert_eq!(pack.requires.core, Some(VersionRequirement::new(">=0.6.0")));
        assert_eq!(pack.budget.max_cycles, 100);
        assert_eq!(pack.agents.len(), 2);
        assert_eq!(pack.agents[0].id, "test_agent");
        assert_eq!(pack.agents[1].id, "analysis_agent");

        // Check preset requirements
        match &pack.agents[0].requirements {
            Some(RequirementsConfig::Preset(preset)) => {
                assert_eq!(*preset, RequirementPreset::FastExtraction);
            }
            _ => panic!("Expected preset requirements"),
        }

        // Check custom requirements
        match &pack.agents[1].requirements {
            Some(RequirementsConfig::Custom(custom)) => {
                assert_eq!(custom.cost_class, Some(CostClass::Medium));
                assert_eq!(custom.requires_reasoning, Some(true));
            }
            _ => panic!("Expected custom requirements"),
        }
    }

    #[test]
    fn test_deserialize_job_request() {
        let json = r#"{
            "pack": "growth-strategy",
            "overrides": {
                "budget": { "max_cycles": 100 },
                "seeds": [
                    { "id": "market:fintech", "content": "European fintech" }
                ],
                "agents": {
                    "CompetitorAgent": {
                        "requirements": { "requires_web_search": true }
                    }
                }
            },
            "providers": {
                "prefer": ["anthropic"],
                "exclude": ["openai"]
            }
        }"#;

        let request: PackJobRequest = serde_json::from_str(json).expect("should parse");
        assert_eq!(request.pack, "growth-strategy");
        assert_eq!(request.overrides.seeds.len(), 1);
        assert_eq!(request.providers.prefer, vec![ProviderId::new("anthropic")]);
    }

    #[test]
    fn test_wiring_only_rejects_semantic_keys() {
        let yaml_with_semantics = r#"
name: bad-pack
version: "1.0.0"
description: Should not have validation

validation:
  min_confidence: 0.8

invariants:
  - SomeInvariant
"#;

        let error = serde_yaml::from_str::<PackConfig>(yaml_with_semantics).unwrap_err();
        assert!(error.to_string().contains("unknown field"));
    }

    #[test]
    fn test_invalid_pack_version_rejected() {
        let yaml = r#"
name: test-pack
version: "v1"
description: Invalid version
"#;

        let error = serde_yaml::from_str::<PackConfig>(yaml).unwrap_err();
        assert!(error.to_string().contains("invalid pack version"));
    }

    #[test]
    fn test_invalid_min_quality_rejected() {
        let error = serde_yaml::from_str::<CustomRequirements>("min_quality: 1.5").unwrap_err();
        assert!(error.to_string().contains("quality threshold"));
    }
}
