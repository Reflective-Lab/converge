// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use thiserror::Error;

#[cfg(feature = "anthropic")]
use crate::llm::AnthropicBackend;
#[cfg(feature = "gemini")]
use crate::llm::GeminiBackend;
#[cfg(feature = "kong")]
use crate::llm::KongBackend;
#[cfg(feature = "mistral")]
use crate::llm::MistralBackend;
#[cfg(feature = "openai")]
use crate::llm::OpenAiBackend;
#[cfg(feature = "openrouter")]
use crate::llm::OpenRouterBackend;
use crate::model_selection::{ProviderRegistry, SelectionResult};
use crate::secret::{EnvSecretProvider, SecretProvider};
use converge_core::model_selection::{
    ComplianceLevel, CostTier, Jurisdiction, LatencyClass, SelectionCriteria, TaskComplexity,
};
use converge_core::traits::{DynChatBackend, LlmError};

#[derive(Debug, Clone, PartialEq)]
pub struct ChatBackendSelectionConfig {
    pub criteria: SelectionCriteria,
    pub provider_override: Option<String>,
}

impl Default for ChatBackendSelectionConfig {
    fn default() -> Self {
        Self {
            criteria: SelectionCriteria::interactive(),
            provider_override: None,
        }
    }
}

impl ChatBackendSelectionConfig {
    #[must_use]
    pub fn with_criteria(mut self, criteria: SelectionCriteria) -> Self {
        self.criteria = criteria;
        self
    }

    #[must_use]
    pub fn with_provider_override(mut self, provider: impl Into<String>) -> Self {
        self.provider_override = Some(provider.into());
        self
    }

    pub fn from_env() -> Result<Self, ChatBackendSelectionConfigError> {
        let mut criteria = std::env::var("CONVERGE_LLM_PROFILE")
            .ok()
            .map(|value| parse_profile(&value))
            .transpose()?
            .unwrap_or_else(SelectionCriteria::interactive);

        if let Ok(value) = std::env::var("CONVERGE_LLM_JURISDICTION") {
            criteria = criteria.with_jurisdiction(parse_jurisdiction(&value)?);
        }
        if let Ok(value) = std::env::var("CONVERGE_LLM_LATENCY") {
            criteria = criteria.with_latency(parse_latency(&value)?);
        }
        if let Ok(value) = std::env::var("CONVERGE_LLM_COST") {
            criteria = criteria.with_cost(parse_cost(&value)?);
        }
        if let Ok(value) = std::env::var("CONVERGE_LLM_COMPLEXITY") {
            criteria = criteria.with_complexity(parse_complexity(&value)?);
        }
        if let Ok(value) = std::env::var("CONVERGE_LLM_COMPLIANCE") {
            criteria = criteria.with_compliance(parse_compliance(&value)?);
        }

        let mut capabilities = criteria.capabilities.clone();
        if env_flag("CONVERGE_LLM_TOOL_USE")? {
            capabilities.tool_use = true;
        }
        if env_flag("CONVERGE_LLM_VISION")? {
            capabilities.vision = true;
        }
        if env_flag("CONVERGE_LLM_STRUCTURED_OUTPUT")? {
            capabilities.structured_output = true;
        }
        if env_flag("CONVERGE_LLM_CODE")? {
            capabilities.code = true;
        }
        if env_flag("CONVERGE_LLM_MULTILINGUAL")? {
            capabilities.multilingual = true;
        }
        if env_flag("CONVERGE_LLM_WEB_SEARCH")? {
            capabilities.web_search = true;
        }
        if let Ok(value) = std::env::var("CONVERGE_LLM_CONTEXT_TOKENS") {
            capabilities.min_context_tokens = Some(value.parse::<usize>().map_err(|_| {
                ChatBackendSelectionConfigError::invalid(
                    "CONVERGE_LLM_CONTEXT_TOKENS",
                    value,
                    "positive integer",
                )
            })?);
        }
        criteria = criteria.with_capabilities(capabilities);

        if let (Ok(country), Ok(region)) = (
            std::env::var("CONVERGE_LLM_USER_COUNTRY"),
            std::env::var("CONVERGE_LLM_USER_REGION"),
        ) {
            criteria = criteria.with_user_location(country, region);
        }

        Ok(Self {
            criteria,
            provider_override: std::env::var("CONVERGE_LLM_FORCE_PROVIDER").ok(),
        })
    }
}

#[derive(Debug, Error)]
pub enum ChatBackendSelectionConfigError {
    #[error("invalid value for {key}: {value} (expected {expected})")]
    InvalidValue {
        key: &'static str,
        value: String,
        expected: &'static str,
    },
}

impl ChatBackendSelectionConfigError {
    fn invalid(key: &'static str, value: impl Into<String>, expected: &'static str) -> Self {
        Self::InvalidValue {
            key,
            value: value.into(),
            expected,
        }
    }
}

#[derive(Clone)]
pub struct SelectedChatBackend {
    pub backend: Arc<dyn DynChatBackend>,
    pub selection: SelectionResult,
}

impl SelectedChatBackend {
    #[must_use]
    pub fn provider(&self) -> &str {
        &self.selection.selected.provider
    }

    #[must_use]
    pub fn model(&self) -> &str {
        &self.selection.selected.model
    }
}

pub fn select_chat_backend(
    config: &ChatBackendSelectionConfig,
) -> Result<SelectedChatBackend, LlmError> {
    select_chat_backend_with_secret_provider(config, &EnvSecretProvider)
}

pub fn select_chat_backend_with_secret_provider(
    config: &ChatBackendSelectionConfig,
    secrets: &dyn SecretProvider,
) -> Result<SelectedChatBackend, LlmError> {
    let registry = if let Some(provider) = config.provider_override.as_deref() {
        let provider = normalize_provider_name(provider).ok_or_else(|| LlmError::InvalidRequest {
            message: format!(
                "Unsupported CONVERGE_LLM_FORCE_PROVIDER={provider}. Expected one of: anthropic, openai, gemini, mistral, openrouter, kong."
            ),
        })?;

        if !is_chat_provider_available(provider, secrets) {
            return Err(LlmError::AuthDenied {
                message: format!(
                    "Requested provider {provider} is not available. Configure the matching API key first."
                ),
            });
        }

        ProviderRegistry::with_providers(&[provider])
    } else {
        chat_provider_registry(secrets)
    };

    let selection = registry.select_with_details(&config.criteria.to_agent_requirements())?;
    let backend = instantiate_selected_backend(&selection, secrets)?;
    Ok(SelectedChatBackend { backend, selection })
}

fn chat_provider_registry(secrets: &dyn SecretProvider) -> ProviderRegistry {
    let supported: Vec<&str> = [
        "anthropic",
        "openai",
        "gemini",
        "mistral",
        "openrouter",
        "kong",
    ]
    .into_iter()
    .filter(|provider| is_chat_provider_available(provider, secrets))
    .collect();
    ProviderRegistry::with_providers(&supported)
}

fn instantiate_selected_backend(
    selection: &SelectionResult,
    secrets: &dyn SecretProvider,
) -> Result<Arc<dyn DynChatBackend>, LlmError> {
    let provider = selection.selected.provider.as_str();
    let model = selection.selected.model.clone();

    match provider {
        #[cfg(feature = "anthropic")]
        "anthropic" => {
            let backend = AnthropicBackend::from_secret_provider(secrets)
                .map_err(backend_error)?
                .with_model(model);
            Ok(Arc::new(backend))
        }
        #[cfg(feature = "openai")]
        "openai" => {
            let backend = OpenAiBackend::from_secret_provider(secrets)
                .map_err(backend_error)?
                .with_model(model);
            Ok(Arc::new(backend))
        }
        #[cfg(feature = "gemini")]
        "gemini" => {
            let backend = GeminiBackend::from_secret_provider(secrets)
                .map_err(backend_error)?
                .with_model(model);
            Ok(Arc::new(backend))
        }
        #[cfg(feature = "mistral")]
        "mistral" => {
            let backend = MistralBackend::from_secret_provider(secrets)
                .map_err(backend_error)?
                .with_model(model);
            Ok(Arc::new(backend))
        }
        #[cfg(feature = "openrouter")]
        "openrouter" => {
            let backend = OpenRouterBackend::from_secret_provider(secrets)
                .map_err(backend_error)?
                .with_model(model);
            Ok(Arc::new(backend))
        }
        #[cfg(feature = "kong")]
        "kong" => {
            let backend = KongBackend::from_secret_provider(secrets)
                .map_err(backend_error)?
                .with_model(model);
            Ok(Arc::new(backend))
        }
        _ => Err(LlmError::ProviderError {
            message: format!("Selected provider {provider} does not have a chat backend"),
            code: None,
        }),
    }
}

fn backend_error(error: converge_core::backend::BackendError) -> LlmError {
    LlmError::ProviderError {
        message: error.to_string(),
        code: None,
    }
}

fn is_chat_provider_available(provider: &str, secrets: &dyn SecretProvider) -> bool {
    match provider {
        #[cfg(feature = "anthropic")]
        "anthropic" => secrets.has_secret("ANTHROPIC_API_KEY"),
        #[cfg(feature = "openai")]
        "openai" => secrets.has_secret("OPENAI_API_KEY"),
        #[cfg(feature = "gemini")]
        "gemini" => secrets.has_secret("GEMINI_API_KEY"),
        #[cfg(feature = "mistral")]
        "mistral" => secrets.has_secret("MISTRAL_API_KEY"),
        #[cfg(feature = "openrouter")]
        "openrouter" => secrets.has_secret("OPENROUTER_API_KEY"),
        #[cfg(feature = "kong")]
        "kong" => {
            secrets.has_secret("KONG_API_KEY") && std::env::var("KONG_AI_GATEWAY_URL").is_ok()
        }
        _ => false,
    }
}

fn normalize_provider_name(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "anthropic" | "claude" => Some("anthropic"),
        "openai" | "gpt" => Some("openai"),
        "gemini" | "google" => Some("gemini"),
        "mistral" | "mixtral" => Some("mistral"),
        "openrouter" | "router" => Some("openrouter"),
        "kong" | "kong_gateway" | "kong_ai" => Some("kong"),
        _ => None,
    }
}

fn parse_profile(value: &str) -> Result<SelectionCriteria, ChatBackendSelectionConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "high_volume" | "high-volume" => Ok(SelectionCriteria::high_volume()),
        "interactive" => Ok(SelectionCriteria::interactive()),
        "analysis" | "research" => Ok(SelectionCriteria::analysis()),
        "batch" => Ok(SelectionCriteria::batch()),
        _ => Err(ChatBackendSelectionConfigError::invalid(
            "CONVERGE_LLM_PROFILE",
            value,
            "high_volume, interactive, analysis, or batch",
        )),
    }
}

fn parse_jurisdiction(value: &str) -> Result<Jurisdiction, ChatBackendSelectionConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "unrestricted" => Ok(Jurisdiction::Unrestricted),
        "trusted" => Ok(Jurisdiction::Trusted),
        "same_region" | "same-region" => Ok(Jurisdiction::SameRegion),
        "same_country" | "same-country" => Ok(Jurisdiction::SameCountry),
        _ => Err(ChatBackendSelectionConfigError::invalid(
            "CONVERGE_LLM_JURISDICTION",
            value,
            "unrestricted, trusted, same_region, or same_country",
        )),
    }
}

fn parse_latency(value: &str) -> Result<LatencyClass, ChatBackendSelectionConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "realtime" => Ok(LatencyClass::Realtime),
        "interactive" => Ok(LatencyClass::Interactive),
        "background" => Ok(LatencyClass::Background),
        "batch" => Ok(LatencyClass::Batch),
        _ => Err(ChatBackendSelectionConfigError::invalid(
            "CONVERGE_LLM_LATENCY",
            value,
            "realtime, interactive, background, or batch",
        )),
    }
}

fn parse_cost(value: &str) -> Result<CostTier, ChatBackendSelectionConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "minimal" | "cheap" => Ok(CostTier::Minimal),
        "standard" | "balanced" => Ok(CostTier::Standard),
        "premium" => Ok(CostTier::Premium),
        _ => Err(ChatBackendSelectionConfigError::invalid(
            "CONVERGE_LLM_COST",
            value,
            "minimal, standard, or premium",
        )),
    }
}

fn parse_complexity(value: &str) -> Result<TaskComplexity, ChatBackendSelectionConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "extraction" => Ok(TaskComplexity::Extraction),
        "classification" => Ok(TaskComplexity::Classification),
        "reasoning" | "research" => Ok(TaskComplexity::Reasoning),
        "generation" => Ok(TaskComplexity::Generation),
        _ => Err(ChatBackendSelectionConfigError::invalid(
            "CONVERGE_LLM_COMPLEXITY",
            value,
            "extraction, classification, reasoning, or generation",
        )),
    }
}

fn parse_compliance(value: &str) -> Result<ComplianceLevel, ChatBackendSelectionConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => Ok(ComplianceLevel::None),
        "gdpr" => Ok(ComplianceLevel::GDPR),
        "soc2" => Ok(ComplianceLevel::SOC2),
        "hipaa" => Ok(ComplianceLevel::HIPAA),
        "high_explainability" | "high-explainability" => Ok(ComplianceLevel::HighExplainability),
        _ => Err(ChatBackendSelectionConfigError::invalid(
            "CONVERGE_LLM_COMPLIANCE",
            value,
            "none, gdpr, soc2, hipaa, or high_explainability",
        )),
    }
}

fn env_flag(key: &'static str) -> Result<bool, ChatBackendSelectionConfigError> {
    match std::env::var(key) {
        Ok(value) => parse_bool(key, &value),
        Err(_) => Ok(false),
    }
}

fn parse_bool(key: &'static str, value: &str) -> Result<bool, ChatBackendSelectionConfigError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Ok(true),
        "0" | "false" | "no" | "off" => Ok(false),
        _ => Err(ChatBackendSelectionConfigError::invalid(
            key,
            value,
            "boolean (true/false/1/0/yes/no/on/off)",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{select_chat_backend_with_secret_provider, ChatBackendSelectionConfig};
    use crate::secret::{SecretError, SecretProvider, StaticSecretProvider};
    use converge_core::model_selection::{RequiredCapabilities, SelectionCriteria};

    #[derive(Debug, Default)]
    struct MissingSecretProvider;

    impl SecretProvider for MissingSecretProvider {
        fn get_secret(&self, key: &str) -> Result<crate::secret::SecretString, SecretError> {
            Err(SecretError::NotFound(key.to_string()))
        }
    }

    #[test]
    fn provider_override_selects_requested_backend_family() {
        let config = ChatBackendSelectionConfig::default().with_provider_override("gemini");
        let selected =
            select_chat_backend_with_secret_provider(&config, &StaticSecretProvider::new("test"))
                .unwrap();
        assert_eq!(selected.provider(), "gemini");
    }

    #[test]
    fn missing_secrets_fail_selection() {
        let config = ChatBackendSelectionConfig::default();
        let error = select_chat_backend_with_secret_provider(&config, &MissingSecretProvider)
            .err()
            .unwrap();
        assert!(matches!(
            error,
            converge_core::traits::LlmError::ProviderError { .. }
        ));
    }

    #[test]
    fn capability_driven_selection_stays_with_instantiable_backends() {
        let config = ChatBackendSelectionConfig::default().with_criteria(
            SelectionCriteria::analysis().with_capabilities(
                RequiredCapabilities::none()
                    .with_structured_output()
                    .with_tool_use(),
            ),
        );
        let selected =
            select_chat_backend_with_secret_provider(&config, &StaticSecretProvider::new("test"))
                .unwrap();
        assert!(matches!(
            selected.provider(),
            "anthropic" | "openai" | "gemini"
        ));
    }
}
