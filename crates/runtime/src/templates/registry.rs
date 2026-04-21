// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Template registry for loading and managing job templates.
//!
//! The registry supports:
//! - Loading templates from YAML files in a directory
//! - Embedded default templates
//! - Thread-safe access via Arc

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tracing::{info, warn};

use super::types::{AgentWiring, PackConfig, PackSummary};
use super::validator::validate_pack_yaml_str;
use crate::semantic::{
    PackName, PackVersion, QualityThreshold, RequirementPreset, VersionRequirement,
};
use converge_core::CostClass;

// Type aliases for backward compatibility
type JobTemplate = PackConfig;
type TemplateSummary = PackSummary;

/// Errors that can occur when working with templates.
#[derive(Debug, Error)]
pub enum TemplateError {
    #[error("Template not found: {0}")]
    NotFound(String),

    #[error("Failed to read template file: {0}")]
    ReadError(#[from] std::io::Error),

    #[error("Failed to parse template: {0}")]
    ParseError(#[from] serde_yaml::Error),

    #[error("Invalid template: {0}")]
    ValidationError(String),
}

/// Template registry for managing job templates.
#[derive(Debug, Clone)]
pub struct TemplateRegistry {
    templates: HashMap<PackName, Arc<JobTemplate>>,
}

impl TemplateRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Create a registry with embedded default templates.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();

        // Register embedded templates
        if let Some(template) = Self::growth_strategy_template() {
            registry.register(template);
        }
        if let Some(template) = Self::ask_converge_template() {
            registry.register(template);
        }
        if let Some(template) = Self::patent_research_template() {
            registry.register(template);
        }
        if let Some(template) = Self::linkedin_research_template() {
            registry.register(template);
        }
        if let Some(template) = Self::release_readiness_template() {
            registry.register(template);
        }
        if let Some(template) = Self::drafting_short_template() {
            registry.register(template);
        }
        if let Some(template) = Self::novelty_search_template() {
            registry.register(template);
        }

        registry
    }

    /// Load templates from a directory.
    ///
    /// Loads all `.yaml` and `.yml` files from the directory.
    pub fn load_from_directory(path: impl AsRef<Path>) -> Result<Self, TemplateError> {
        let mut registry = Self::new();
        let path = path.as_ref();

        if !path.exists() {
            warn!("Template directory does not exist: {}", path.display());
            return Ok(registry);
        }

        let entries = std::fs::read_dir(path)?;

        for entry in entries.flatten() {
            let file_path = entry.path();
            if let Some(ext) = file_path.extension() {
                if ext == "yaml" || ext == "yml" {
                    match Self::load_template_file(&file_path) {
                        Ok(template) => {
                            info!(name = %template.name, "Loaded template from {}", file_path.display());
                            registry.register(template);
                        }
                        Err(e) => {
                            warn!("Failed to load template {}: {}", file_path.display(), e);
                        }
                    }
                }
            }
        }

        Ok(registry)
    }

    /// Load a single template from a file.
    pub fn load_template_file(path: impl AsRef<Path>) -> Result<JobTemplate, TemplateError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse_yaml(&content)
    }

    /// Parse a template from YAML string.
    ///
    /// Validates the YAML against the schema contract before parsing.
    /// Rejects YAML containing forbidden semantic keys.
    pub fn parse_yaml(yaml: &str) -> Result<JobTemplate, TemplateError> {
        // Validate against schema contract (no semantic keys allowed)
        validate_pack_yaml_str(yaml).map_err(|e| {
            TemplateError::ValidationError(format!("Schema validation failed: {e}"))
        })?;

        let template: JobTemplate = serde_yaml::from_str(yaml)?;
        Self::validate_template(&template)?;
        Ok(template)
    }

    /// Validate a pack config for correctness.
    fn validate_template(pack: &PackConfig) -> Result<(), TemplateError> {
        // Validate agent IDs are unique
        let mut agent_ids = std::collections::HashSet::new();
        for agent in &pack.agents {
            if !agent_ids.insert(&agent.id) {
                return Err(TemplateError::ValidationError(format!(
                    "duplicate agent id '{}'",
                    agent.id
                )));
            }
        }

        Ok(())
    }

    /// Register a template.
    pub fn register(&mut self, template: JobTemplate) {
        self.templates
            .insert(template.name.clone(), Arc::new(template));
    }

    /// Get a template by name.
    pub fn get(&self, name: &str) -> Option<Arc<JobTemplate>> {
        self.templates.get(name).cloned()
    }

    /// List all template summaries.
    pub fn list(&self) -> Vec<TemplateSummary> {
        self.templates
            .values()
            .map(|t| TemplateSummary::from(t.as_ref()))
            .collect()
    }

    /// Get template names.
    pub fn names(&self) -> Vec<&str> {
        self.templates.keys().map(|s| s.as_str()).collect()
    }

    /// Check if a template exists.
    pub fn contains(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    /// Number of registered templates.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Create the embedded growth-strategy pack config.
    ///
    /// Note: This loads from converge-domain packs.
    /// The Gherkin spec (semantics) is in converge-domain/specs/growth-strategy.feature
    /// The YAML config (wiring) is in converge-domain/packs/growth-strategy.yaml
    ///
    /// Semantics (invariants, validation rules, forbidden terms) are in Gherkin,
    /// NOT in this wiring config. Suggestor IDs match @agent @id:xxx tags.
    fn growth_strategy_template() -> Option<PackConfig> {
        Some(PackConfig {
            name: PackName::new("growth-strategy"),
            version: PackVersion::new("1.0.0"),
            description: "Multi-agent growth strategy analysis".to_string(),
            spec: Some("specs/growth-strategy.feature".to_string()),
            requires: super::types::CompatibilityRequirements {
                core: Some(VersionRequirement::new(">=0.6.0")),
                runtime_api: Some(VersionRequirement::new(">=0.1.0")),
            },
            budget: super::types::BudgetConfig {
                max_cycles: 50,
                max_facts: 500,
            },
            agents: vec![
                AgentWiring {
                    id: "market_signal".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::FastExtraction,
                    )),
                },
                AgentWiring {
                    id: "competitor_analysis".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: Some(15000),
                            requires_reasoning: None,
                            requires_web_search: Some(true),
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "strategy_synthesis".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: None,
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "evaluation".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::VeryHigh),
                            max_latency_ms: None,
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
            ],
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Create the embedded ask-converge pack config.
    fn ask_converge_template() -> Option<PackConfig> {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "grounded_answering".to_string(),
            serde_json::Value::Bool(true),
        );
        metadata.insert(
            "recall_not_evidence".to_string(),
            serde_json::Value::Bool(true),
        );
        metadata.insert(
            "pack_type".to_string(),
            serde_json::Value::String("ask".to_string()),
        );

        Some(PackConfig {
            name: PackName::new("ask-converge"),
            version: PackVersion::new("0.1.0"),
            description: "Grounded ask-converge Q&A with recall-only sources".to_string(),
            spec: Some("specs/ask-converge.feature".to_string()),
            requires: super::types::CompatibilityRequirements {
                core: Some(VersionRequirement::new(">=0.6.0")),
                runtime_api: Some(VersionRequirement::new(">=0.1.0")),
            },
            budget: super::types::BudgetConfig {
                max_cycles: 6,
                max_facts: 64,
            },
            agents: vec![AgentWiring {
                id: "ask_converge".into(),
                requirements: Some(super::types::RequirementsConfig::Custom(
                    super::types::CustomRequirements {
                        cost_class: Some(CostClass::Medium),
                        max_latency_ms: Some(20000),
                        requires_reasoning: Some(true),
                        requires_web_search: None,
                        min_quality: Some(QualityThreshold::new(0.6)),
                    },
                )),
            }],
            metadata,
        })
    }

    /// Create the embedded patent-research pack config.
    fn patent_research_template() -> Option<PackConfig> {
        Some(PackConfig {
            name: PackName::new("patent-research"),
            version: PackVersion::new("1.0.0"),
            description: "Governed patent research with evidence and approvals".to_string(),
            spec: Some("specs/patent-research.feature".to_string()),
            requires: super::types::CompatibilityRequirements {
                core: Some(VersionRequirement::new(">=0.6.0")),
                runtime_api: Some(VersionRequirement::new(">=0.1.0")),
            },
            budget: super::types::BudgetConfig {
                max_cycles: 20,
                max_facts: 200,
            },
            agents: vec![
                AgentWiring {
                    id: "patent_query_builder".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::FastExtraction,
                    )),
                },
                AgentWiring {
                    id: "patent_operator_planner".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Low),
                            max_latency_ms: Some(8000),
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "patent_search_executor".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: Some(20000),
                            requires_reasoning: Some(false),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "patent_evidence_collector".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: Some(15000),
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "patent_claims_analyzer".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: Some(15000),
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "patent_landscape_analyzer".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: Some(15000),
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "patent_report_assembler".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: Some(15000),
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "patent_alert_agent".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::FastExtraction,
                    )),
                },
                AgentWiring {
                    id: "patent_submission_agent".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::Deterministic,
                    )),
                },
                AgentWiring {
                    id: "patent_approval_recorder".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::Deterministic,
                    )),
                },
            ],
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Create the embedded linkedin-research pack config.
    fn linkedin_research_template() -> Option<PackConfig> {
        Some(PackConfig {
            name: PackName::new("linkedin-research"),
            version: PackVersion::new("1.0.0"),
            description: "Governed LinkedIn research with evidence and approvals".to_string(),
            spec: Some("specs/linkedin-research.feature".to_string()),
            requires: super::types::CompatibilityRequirements {
                core: Some(VersionRequirement::new(">=0.6.0")),
                runtime_api: Some(VersionRequirement::new(">=0.1.0")),
            },
            budget: super::types::BudgetConfig {
                max_cycles: 30,
                max_facts: 300,
            },
            agents: vec![
                AgentWiring {
                    id: "signal_ingest".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::FastExtraction,
                    )),
                },
                AgentWiring {
                    id: "evidence_validator".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::Deterministic,
                    )),
                },
                AgentWiring {
                    id: "dossier_builder".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: Some(15000),
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "path_verifier".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: Some(15000),
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "approval_recorder".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::Deterministic,
                    )),
                },
                AgentWiring {
                    id: "linkedin_target_discovery".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: Some(15000),
                            requires_reasoning: Some(false),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
            ],
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Create the embedded drafting-short pack config.
    fn drafting_short_template() -> Option<PackConfig> {
        Some(PackConfig {
            name: PackName::new("drafting-short"),
            version: PackVersion::new("1.0.0"),
            description: "Short drafting flow with Perplexity research and Anthropic drafting."
                .to_string(),
            spec: Some("specs/drafting-short.feature".to_string()),
            requires: super::types::CompatibilityRequirements {
                core: Some(VersionRequirement::new(">=0.6.0")),
                runtime_api: Some(VersionRequirement::new(">=0.1.0")),
            },
            budget: super::types::BudgetConfig {
                max_cycles: 10,
                max_facts: 100,
            },
            agents: vec![
                AgentWiring {
                    id: "DraftingResearchAgent".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: None,
                            requires_reasoning: None,
                            requires_web_search: Some(true),
                            min_quality: Some(QualityThreshold::new(0.75)),
                        },
                    )),
                },
                AgentWiring {
                    id: "DraftingComposerAgent".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: Some(3000),
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: Some(QualityThreshold::new(0.85)),
                        },
                    )),
                },
            ],
            metadata: std::collections::HashMap::new(),
        })
    }

    /// Create the embedded release-readiness pack config.
    fn release_readiness_template() -> Option<PackConfig> {
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "approval_required".to_string(),
            serde_json::Value::Bool(true),
        );
        metadata.insert(
            "deterministic_only".to_string(),
            serde_json::Value::Bool(true),
        );

        Some(PackConfig {
            name: PackName::new("release-readiness"),
            version: PackVersion::new("1.0.0"),
            description: "Engineering dependency and release quality gates.".to_string(),
            spec: Some("specs/release-readiness.feature".to_string()),
            requires: super::types::CompatibilityRequirements {
                core: Some(VersionRequirement::new(">=0.6.0")),
                runtime_api: Some(VersionRequirement::new(">=0.1.0")),
            },
            budget: super::types::BudgetConfig {
                max_cycles: 12,
                max_facts: 128,
            },
            agents: vec![
                AgentWiring {
                    id: "DependencyGraphAgent".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::Deterministic,
                    )),
                },
                AgentWiring {
                    id: "TestCoverageAgent".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::Deterministic,
                    )),
                },
                AgentWiring {
                    id: "SecurityScanAgent".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::Deterministic,
                    )),
                },
                AgentWiring {
                    id: "PerformanceRegressionAgent".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::Deterministic,
                    )),
                },
                AgentWiring {
                    id: "DocumentationAgent".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::Deterministic,
                    )),
                },
                AgentWiring {
                    id: "RiskSummaryAgent".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::Deterministic,
                    )),
                },
                AgentWiring {
                    id: "ReleaseReadyAgent".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::Deterministic,
                    )),
                },
            ],
            metadata,
        })
    }

    /// Create the embedded novelty-search pack config.
    fn novelty_search_template() -> Option<PackConfig> {
        Some(PackConfig {
            name: PackName::new("novelty-search"),
            version: PackVersion::new("1.0.0"),
            description: "Short novelty search flow for patent prior art.".to_string(),
            spec: Some("specs/novelty-search.feature".to_string()),
            requires: super::types::CompatibilityRequirements {
                core: Some(VersionRequirement::new(">=0.6.0")),
                runtime_api: Some(VersionRequirement::new(">=0.1.0")),
            },
            budget: super::types::BudgetConfig {
                max_cycles: 15,
                max_facts: 150,
            },
            agents: vec![
                AgentWiring {
                    id: "patent_query_builder".into(),
                    requirements: Some(super::types::RequirementsConfig::Preset(
                        RequirementPreset::FastExtraction,
                    )),
                },
                AgentWiring {
                    id: "patent_operator_planner".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Low),
                            max_latency_ms: None,
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "patent_search_executor".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: Some(20000),
                            requires_reasoning: Some(false),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "patent_evidence_collector".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: None,
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
                AgentWiring {
                    id: "prior_art_shortlist".into(),
                    requirements: Some(super::types::RequirementsConfig::Custom(
                        super::types::CustomRequirements {
                            cost_class: Some(CostClass::Medium),
                            max_latency_ms: None,
                            requires_reasoning: Some(true),
                            requires_web_search: None,
                            min_quality: None,
                        },
                    )),
                },
            ],
            metadata: std::collections::HashMap::new(),
        })
    }
}

impl Default for TemplateRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_yaml_with_id() {
        let yaml = r#"
name: test
version: "1.0.0"
description: Test template
agents:
  - id: test_agent
    requirements: fast_extraction
"#;

        let template = TemplateRegistry::parse_yaml(yaml).expect("should parse");
        assert_eq!(template.name, "test");
        assert_eq!(template.version, "1.0.0");
        assert_eq!(template.agents.len(), 1);
        assert_eq!(template.agents[0].id, "test_agent");
    }

    #[test]
    fn test_parse_yaml_with_name_alias() {
        // "name" is an alias for "id" for backward compatibility
        let yaml = r#"
name: test
version: "1.0.0"
description: Test template
agents:
  - name: legacy_agent
    requirements: fast_extraction
"#;

        let template = TemplateRegistry::parse_yaml(yaml).expect("should parse");
        assert_eq!(template.agents[0].id, "legacy_agent");
    }

    #[test]
    fn test_validate_duplicate_agent_ids() {
        let yaml = r#"
name: test
version: "1.0.0"
description: Test template
agents:
  - id: duplicate_id
    requirements: fast_extraction
  - id: duplicate_id
    requirements: fast_extraction
"#;

        let result = TemplateRegistry::parse_yaml(yaml);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("duplicate agent id")
        );
    }

    #[test]
    fn test_registry_operations() {
        let mut registry = TemplateRegistry::new();
        assert!(registry.is_empty());

        let template = TemplateRegistry::parse_yaml(
            r#"
name: test
version: "1.0.0"
description: Test
agents: []
"#,
        )
        .unwrap();

        registry.register(template);
        assert!(!registry.is_empty());
        assert!(registry.contains("test"));
        assert!(!registry.contains("nonexistent"));

        let retrieved = registry.get("test").expect("should exist");
        assert_eq!(retrieved.name, "test");
    }

    #[test]
    fn test_with_defaults_includes_release_readiness() {
        let registry = TemplateRegistry::with_defaults();
        assert!(registry.contains("release-readiness"));

        let template = registry
            .get("release-readiness")
            .expect("template should exist");
        assert_eq!(template.name, "release-readiness");
        assert_eq!(
            template.spec.as_deref(),
            Some("specs/release-readiness.feature")
        );
        assert_eq!(template.agents.len(), 7);
        assert!(template.agents.iter().any(|a| a.id == "ReleaseReadyAgent"));
    }
}
