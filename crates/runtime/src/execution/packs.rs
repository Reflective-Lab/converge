// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Pack-based agent registration for Converge Runtime.
//!
//! This module provides the generic pack registry and LLM configuration.
//! Domain-specific pack agent registration (growth-strategy, patent, SDR, etc.)
//! lives in organism-application.

use converge_core::Engine;
use converge_core::model_selection::SelectionCriteria;
use converge_provider::ChatBackendSelectionConfig;
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::error::RuntimeError;

/// Definition of a domain pack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackDefinition {
    /// Pack identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Version.
    pub version: String,
    /// Available templates.
    pub templates: Vec<String>,
    /// Invariants.
    pub invariants: Vec<String>,
    /// Whether LLM agents are available.
    pub has_llm_agents: bool,
}

/// Registry of available packs.
pub struct PackRegistry {
    packs: Vec<PackDefinition>,
}

impl Default for PackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PackRegistry {
    /// Create a new empty pack registry.
    ///
    /// Domain-specific packs (growth-strategy, patent, SDR, etc.) are registered
    /// by organism-application, not by the generic converge runtime.
    pub fn new() -> Self {
        Self { packs: Vec::new() }
    }

    /// Add a pack definition to the registry.
    pub fn add(&mut self, pack: PackDefinition) {
        self.packs.push(pack);
    }

    /// Get a pack by ID.
    pub fn get(&self, id: &str) -> Option<&PackDefinition> {
        self.packs.iter().find(|p| p.id == id)
    }

    /// List all available packs.
    pub fn list(&self) -> &[PackDefinition] {
        &self.packs
    }

    /// Check if a pack exists.
    pub fn contains(&self, id: &str) -> bool {
        self.packs.iter().any(|p| p.id == id)
    }
}

/// Configuration for LLM provider selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Use mock LLM instead of real providers.
    pub use_mock: bool,
    /// Selection criteria used to choose a backend from available providers.
    #[serde(default = "default_llm_selection_criteria")]
    pub criteria: SelectionCriteria,
    /// Optional operator override for a specific provider family.
    ///
    /// This is intended for debugging and validation. Regular callers should
    /// prefer criteria-based selection.
    #[serde(default)]
    pub provider_override: Option<String>,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            use_mock: false,
            criteria: default_llm_selection_criteria(),
            provider_override: None,
        }
    }
}

impl LlmConfig {
    /// Convert runtime LLM configuration into backend-selection input.
    #[must_use]
    pub fn selection_config(&self) -> ChatBackendSelectionConfig {
        let mut config = ChatBackendSelectionConfig::default().with_criteria(self.criteria.clone());
        if let Some(provider) = &self.provider_override {
            config = config.with_provider_override(provider.clone());
        }
        config
    }

    /// Returns whether runtime should use real host-supplied LLM adapters.
    ///
    /// Runtime no longer constructs provider registries itself. Products or
    /// deployment assembly register adapters from Manifold or another extension.
    #[must_use]
    pub fn use_real_llm(&self) -> bool {
        !self.use_mock
    }
}

fn default_llm_selection_criteria() -> SelectionCriteria {
    SelectionCriteria::analysis()
}

/// Register agents and invariants for a specific domain pack.
///
/// This is the generic bridge between the runtime and domain packs.
/// Domain-specific registration (growth-strategy, patent, etc.) should be
/// provided by the application layer (organism-application).
///
/// Override this by providing a custom registration function to the runtime.
pub fn register_pack_agents(
    _engine: &mut Engine,
    pack_id: &str,
    _llm_config: &LlmConfig,
) -> Result<(), RuntimeError> {
    warn!(
        pack = %pack_id,
        "No domain pack agents registered. Domain-specific packs belong in organism-application."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::model_selection::{CostTier, TaskComplexity};

    #[test]
    fn test_pack_registry_empty_by_default() {
        let registry = PackRegistry::new();
        assert!(!registry.contains("growth-strategy"));
        assert!(registry.list().is_empty());
    }

    #[test]
    fn test_pack_registry_add_and_get() {
        let mut registry = PackRegistry::new();
        registry.add(PackDefinition {
            id: "test-pack".to_string(),
            name: "Test Pack".to_string(),
            description: "A test pack".to_string(),
            version: "1.0.0".to_string(),
            templates: vec![],
            invariants: vec![],
            has_llm_agents: false,
        });
        assert!(registry.contains("test-pack"));
        let pack = registry.get("test-pack").unwrap();
        assert_eq!(pack.name, "Test Pack");
    }

    #[test]
    fn test_llm_config_default() {
        let config = LlmConfig::default();
        assert!(!config.use_mock);
        assert_eq!(config.criteria.cost, CostTier::Premium);
        assert_eq!(config.criteria.complexity, TaskComplexity::Reasoning);
        assert!(config.provider_override.is_none());
    }

    #[test]
    fn test_llm_config_mock_returns_none() {
        let mut config = LlmConfig::default();
        config.use_mock = true;
        assert!(!config.use_real_llm());
    }

    #[test]
    fn test_llm_config_selection_config_preserves_fields() {
        let config = LlmConfig {
            use_mock: false,
            criteria: SelectionCriteria::interactive(),
            provider_override: Some("gemini".to_string()),
        };

        let selection = config.selection_config();
        assert_eq!(selection.criteria, SelectionCriteria::interactive());
        assert_eq!(selection.provider_override.as_deref(), Some("gemini"));
    }

    #[test]
    fn test_register_pack_agents_unknown_pack() {
        let mut engine = converge_core::Engine::new();
        let config = LlmConfig::default();
        let result = register_pack_agents(&mut engine, "unknown-pack", &config);
        assert!(result.is_ok());
    }
}
