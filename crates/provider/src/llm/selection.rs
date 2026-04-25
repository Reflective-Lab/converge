// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

use std::sync::Arc;

use thiserror::Error;

#[cfg(feature = "anthropic")]
use crate::llm::AnthropicBackend;
#[cfg(feature = "arcee")]
use crate::llm::ArceeBackend;
#[cfg(feature = "gemini")]
use crate::llm::GeminiBackend;
#[cfg(feature = "kong")]
use crate::llm::KongBackend;
#[cfg(feature = "minmax")]
use crate::llm::MinMaxBackend;
#[cfg(feature = "mistral")]
use crate::llm::MistralBackend;
#[cfg(feature = "openai")]
use crate::llm::OpenAiBackend;
#[cfg(feature = "openrouter")]
use crate::llm::OpenRouterBackend;
#[cfg(feature = "staik")]
use crate::llm::StaikBackend;
#[cfg(feature = "writer")]
use crate::llm::WriterBackend;
use crate::model_selection::{ProviderRegistry, SelectionResult};
use crate::secret::{EnvSecretProvider, SecretProvider};
use converge_provider_api::selection::{
    ComplianceLevel, CostTier, Jurisdiction, LatencyClass, SelectionCriteria, TaskComplexity,
};
use converge_provider_api::{
    ChatMessage, ChatRequest, ChatRole, DynChatBackend, LlmError, ResponseFormat,
};

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
        if env_flag("CONVERGE_LLM_CONTENT_GENERATION")? {
            capabilities.content_generation = true;
        }
        if env_flag("CONVERGE_LLM_BUSINESS_ACUMEN")? {
            capabilities.business_acumen = true;
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
                "Unsupported CONVERGE_LLM_FORCE_PROVIDER={provider}. Expected one of: anthropic, openai, gemini, mistral, arcee, writer, minmax, openrouter, kong, staik."
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

/// Selects a chat backend with health probing — iterates ranked candidates and
/// returns the first one that responds to a minimal probe request.
///
/// Use this instead of [`select_chat_backend`] when you want automatic fallback
/// past providers whose API keys exist but are non-functional (e.g. exhausted
/// free-tier quotas, revoked keys, or temporary outages).
pub async fn select_healthy_chat_backend(
    config: &ChatBackendSelectionConfig,
) -> Result<SelectedChatBackend, LlmError> {
    select_healthy_chat_backend_with_secret_provider(config, &EnvSecretProvider).await
}

/// Like [`select_healthy_chat_backend`] but with an explicit secret provider.
pub async fn select_healthy_chat_backend_with_secret_provider(
    config: &ChatBackendSelectionConfig,
    secrets: &dyn SecretProvider,
) -> Result<SelectedChatBackend, LlmError> {
    let registry = if let Some(provider) = config.provider_override.as_deref() {
        let provider = normalize_provider_name(provider).ok_or_else(|| LlmError::InvalidRequest {
            message: format!(
                "Unsupported CONVERGE_LLM_FORCE_PROVIDER={provider}. Expected one of: anthropic, openai, gemini, mistral, arcee, writer, minmax, openrouter, kong, staik."
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

    let mut last_error = None;
    for (candidate, fitness) in &selection.candidates {
        let candidate_selection = SelectionResult {
            selected: candidate.clone(),
            fitness: fitness.clone(),
            candidates: selection.candidates.clone(),
            rejected: selection.rejected.clone(),
        };

        let backend = match instantiate_selected_backend(&candidate_selection, secrets) {
            Ok(b) => b,
            Err(e) => {
                tracing::debug!(
                    provider = %candidate.provider,
                    model = %candidate.model,
                    error = %e,
                    "skipping candidate: instantiation failed"
                );
                last_error = Some(e);
                continue;
            }
        };

        match probe_backend(&backend).await {
            Ok(()) => {
                tracing::info!(
                    provider = %candidate.provider,
                    model = %candidate.model,
                    "health probe passed"
                );
                return Ok(SelectedChatBackend {
                    backend,
                    selection: candidate_selection,
                });
            }
            Err(e) => {
                tracing::warn!(
                    provider = %candidate.provider,
                    model = %candidate.model,
                    error = %e,
                    "health probe failed, trying next candidate"
                );
                last_error = Some(e);
            }
        }
    }

    Err(last_error.unwrap_or_else(|| LlmError::ProviderError {
        message: "No healthy provider found among candidates".into(),
        code: None,
    }))
}

async fn probe_backend(backend: &Arc<dyn DynChatBackend>) -> Result<(), LlmError> {
    let request = ChatRequest {
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "hi".to_string(),
            tool_calls: vec![],
            tool_call_id: None,
        }],
        system: None,
        tools: vec![],
        response_format: ResponseFormat::Text,
        max_tokens: Some(1),
        temperature: None,
        stop_sequences: vec![],
        model: None,
    };
    backend.chat(request).await.map(|_| ())
}

fn chat_provider_registry(secrets: &dyn SecretProvider) -> ProviderRegistry {
    let supported: Vec<&str> = [
        "anthropic",
        "openai",
        "gemini",
        "mistral",
        "arcee",
        "writer",
        "minmax",
        "openrouter",
        "kong",
        "staik",
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
        #[cfg(feature = "staik")]
        "staik" => {
            let backend = StaikBackend::from_secret_provider(secrets)
                .map_err(backend_error)?
                .with_model(model);
            Ok(Arc::new(backend))
        }
        #[cfg(feature = "arcee")]
        "arcee" => {
            let backend = ArceeBackend::from_secret_provider(secrets)
                .map_err(backend_error)?
                .with_model(model);
            Ok(Arc::new(backend))
        }
        #[cfg(feature = "writer")]
        "writer" => {
            let backend = WriterBackend::from_secret_provider(secrets)
                .map_err(backend_error)?
                .with_model(model);
            Ok(Arc::new(backend))
        }
        #[cfg(feature = "minmax")]
        "minmax" => {
            let backend = MinMaxBackend::from_secret_provider(secrets)
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
        #[cfg(feature = "staik")]
        "staik" => secrets.has_secret("STAIK_API_KEY"),
        #[cfg(feature = "arcee")]
        "arcee" => secrets.has_secret("ARCEE_API_KEY"),
        #[cfg(feature = "writer")]
        "writer" => secrets.has_secret("WRITER_API_KEY"),
        #[cfg(feature = "minmax")]
        "minmax" => secrets.has_secret("MINMAX_API_KEY"),
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
        "staik" => Some("staik"),
        "arcee" => Some("arcee"),
        "writer" | "palmyra" => Some("writer"),
        "minmax" | "minimax" => Some("minmax"),
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
    use super::{ChatBackendSelectionConfig, select_chat_backend_with_secret_provider};
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

    // ========================================================================
    // Parsing and normalization tests
    // ========================================================================

    #[test]
    fn normalize_provider_name_aliases() {
        use super::normalize_provider_name;

        assert_eq!(normalize_provider_name("anthropic"), Some("anthropic"));
        assert_eq!(normalize_provider_name("claude"), Some("anthropic"));
        assert_eq!(normalize_provider_name("CLAUDE"), Some("anthropic"));
        assert_eq!(normalize_provider_name("openai"), Some("openai"));
        assert_eq!(normalize_provider_name("gpt"), Some("openai"));
        assert_eq!(normalize_provider_name("gemini"), Some("gemini"));
        assert_eq!(normalize_provider_name("google"), Some("gemini"));
        assert_eq!(normalize_provider_name("mistral"), Some("mistral"));
        assert_eq!(normalize_provider_name("mixtral"), Some("mistral"));
        assert_eq!(normalize_provider_name("openrouter"), Some("openrouter"));
        assert_eq!(normalize_provider_name("router"), Some("openrouter"));
        assert_eq!(normalize_provider_name("kong"), Some("kong"));
        assert_eq!(normalize_provider_name("kong_gateway"), Some("kong"));
        assert_eq!(normalize_provider_name("kong_ai"), Some("kong"));
        assert_eq!(normalize_provider_name("unknown"), None);
        assert_eq!(normalize_provider_name(""), None);
    }

    #[test]
    fn parse_profile_valid_values() {
        use super::parse_profile;

        assert!(parse_profile("interactive").is_ok());
        assert!(parse_profile("high_volume").is_ok());
        assert!(parse_profile("high-volume").is_ok());
        assert!(parse_profile("analysis").is_ok());
        assert!(parse_profile("research").is_ok());
        assert!(parse_profile("batch").is_ok());
        assert!(parse_profile("INTERACTIVE").is_ok()); // case insensitive
    }

    #[test]
    fn parse_profile_invalid_value() {
        use super::parse_profile;

        let err = parse_profile("turbo").unwrap_err();
        assert!(err.to_string().contains("turbo"));
        assert!(err.to_string().contains("CONVERGE_LLM_PROFILE"));
    }

    #[test]
    fn parse_jurisdiction_valid() {
        use super::parse_jurisdiction;

        assert!(parse_jurisdiction("unrestricted").is_ok());
        assert!(parse_jurisdiction("trusted").is_ok());
        assert!(parse_jurisdiction("same_region").is_ok());
        assert!(parse_jurisdiction("same-region").is_ok());
        assert!(parse_jurisdiction("same_country").is_ok());
        assert!(parse_jurisdiction("same-country").is_ok());
    }

    #[test]
    fn parse_jurisdiction_invalid() {
        use super::parse_jurisdiction;

        assert!(parse_jurisdiction("local").is_err());
    }

    #[test]
    fn parse_latency_valid() {
        use super::parse_latency;

        assert!(parse_latency("realtime").is_ok());
        assert!(parse_latency("interactive").is_ok());
        assert!(parse_latency("background").is_ok());
        assert!(parse_latency("batch").is_ok());
    }

    #[test]
    fn parse_cost_valid() {
        use super::parse_cost;

        assert!(parse_cost("minimal").is_ok());
        assert!(parse_cost("cheap").is_ok());
        assert!(parse_cost("standard").is_ok());
        assert!(parse_cost("balanced").is_ok());
        assert!(parse_cost("premium").is_ok());
    }

    #[test]
    fn parse_cost_invalid() {
        use super::parse_cost;

        assert!(parse_cost("free").is_err());
    }

    #[test]
    fn parse_complexity_valid() {
        use super::parse_complexity;

        assert!(parse_complexity("extraction").is_ok());
        assert!(parse_complexity("classification").is_ok());
        assert!(parse_complexity("reasoning").is_ok());
        assert!(parse_complexity("research").is_ok());
        assert!(parse_complexity("generation").is_ok());
    }

    #[test]
    fn parse_compliance_valid() {
        use super::parse_compliance;

        assert!(parse_compliance("none").is_ok());
        assert!(parse_compliance("gdpr").is_ok());
        assert!(parse_compliance("soc2").is_ok());
        assert!(parse_compliance("hipaa").is_ok());
        assert!(parse_compliance("high_explainability").is_ok());
        assert!(parse_compliance("high-explainability").is_ok());
    }

    #[test]
    fn parse_bool_valid_values() {
        use super::parse_bool;

        for v in &["1", "true", "yes", "on", "TRUE", "Yes", "ON"] {
            assert!(parse_bool("KEY", v).unwrap());
        }
        for v in &["0", "false", "no", "off", "FALSE", "No", "OFF"] {
            assert!(!parse_bool("KEY", v).unwrap());
        }
    }

    #[test]
    fn parse_bool_invalid() {
        use super::parse_bool;

        assert!(parse_bool("KEY", "maybe").is_err());
        assert!(parse_bool("KEY", "2").is_err());
    }

    #[test]
    fn unsupported_provider_override_fails() {
        let config = ChatBackendSelectionConfig::default().with_provider_override("cohere");
        let result =
            select_chat_backend_with_secret_provider(&config, &StaticSecretProvider::new("test"));
        let err = result.err().expect("should fail");
        assert!(err.to_string().contains("cohere"));
    }

    #[test]
    fn forced_provider_without_key_returns_auth_denied() {
        let config = ChatBackendSelectionConfig::default().with_provider_override("anthropic");
        let result = select_chat_backend_with_secret_provider(&config, &MissingSecretProvider);
        let err = result.err().expect("should fail");
        assert!(matches!(
            err,
            converge_core::traits::LlmError::AuthDenied { .. }
        ));
    }
}
