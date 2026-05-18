// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Host-supplied capability registries.
//!
//! Registries in this module hold already-constructed backend handles. They do
//! not inspect credentials, instantiate vendor SDKs, or import adapter crates.

use std::sync::Arc;

use thiserror::Error;

use crate::chat::{DynChatBackend, LlmError};
use crate::selection::{
    AgentRequirements, ChatBackendSelectionConfig, ComplianceLevel, CostClass, DataSovereignty,
};

/// Validated provider family name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProviderName(String);

impl ProviderName {
    /// Creates a non-empty provider name.
    ///
    /// # Errors
    ///
    /// Returns an error when the provider name is empty after trimming.
    pub fn new(value: impl Into<String>) -> Result<Self, RegistryValueError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(RegistryValueError::Empty { field: "provider" });
        }
        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated model identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModelName(String);

impl ModelName {
    /// Creates a non-empty model name.
    ///
    /// # Errors
    ///
    /// Returns an error when the model name is empty after trimming.
    pub fn new(value: impl Into<String>) -> Result<Self, RegistryValueError> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(RegistryValueError::Empty { field: "model" });
        }
        Ok(Self(trimmed.to_string()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Validated latency value in milliseconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LatencyMillis(u32);

impl LatencyMillis {
    /// Creates a strictly positive latency value.
    ///
    /// # Errors
    ///
    /// Returns an error when `ms` is zero.
    pub fn new(ms: u32) -> Result<Self, RegistryValueError> {
        if ms == 0 {
            return Err(RegistryValueError::Zero {
                field: "latency_ms",
            });
        }
        Ok(Self(ms))
    }

    #[must_use]
    pub fn get(self) -> u32 {
        self.0
    }
}

/// Validated context-window size in tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContextWindowTokens(usize);

impl ContextWindowTokens {
    /// Creates a strictly positive context-window size.
    ///
    /// # Errors
    ///
    /// Returns an error when `tokens` is zero.
    pub fn new(tokens: usize) -> Result<Self, RegistryValueError> {
        if tokens == 0 {
            return Err(RegistryValueError::Zero {
                field: "context_tokens",
            });
        }
        Ok(Self(tokens))
    }

    #[must_use]
    pub fn get(self) -> usize {
        self.0
    }
}

/// Validated quality score in the closed interval `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct QualityScore(f64);

impl QualityScore {
    /// Creates a finite score between `0.0` and `1.0`, inclusive.
    ///
    /// # Errors
    ///
    /// Returns an error when the score is not finite or is outside the closed
    /// range.
    pub fn new(value: f64) -> Result<Self, RegistryValueError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(RegistryValueError::InvalidQuality);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

/// Errors raised while constructing validated registry contract values.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RegistryValueError {
    #[error("{field} cannot be empty")]
    Empty { field: &'static str },
    #[error("{field} must be greater than zero")]
    Zero { field: &'static str },
    #[error("quality must be finite and between 0.0 and 1.0")]
    InvalidQuality,
}

/// Capabilities offered by a registered chat backend.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatBackendCapabilities {
    reasoning: bool,
    web_search: bool,
    tool_use: bool,
    vision: bool,
    context_tokens: Option<ContextWindowTokens>,
    structured_output: bool,
    code: bool,
    multilingual: bool,
    content_generation: bool,
    business_acumen: bool,
}

impl ChatBackendCapabilities {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_reasoning(mut self, supports: bool) -> Self {
        self.reasoning = supports;
        self
    }

    #[must_use]
    pub fn with_web_search(mut self, supports: bool) -> Self {
        self.web_search = supports;
        self
    }

    #[must_use]
    pub fn with_tool_use(mut self, supports: bool) -> Self {
        self.tool_use = supports;
        self
    }

    #[must_use]
    pub fn with_vision(mut self, supports: bool) -> Self {
        self.vision = supports;
        self
    }

    #[must_use]
    pub fn with_context_tokens(mut self, tokens: ContextWindowTokens) -> Self {
        self.context_tokens = Some(tokens);
        self
    }

    #[must_use]
    pub fn with_structured_output(mut self, supports: bool) -> Self {
        self.structured_output = supports;
        self
    }

    #[must_use]
    pub fn with_code(mut self, supports: bool) -> Self {
        self.code = supports;
        self
    }

    #[must_use]
    pub fn with_multilingual(mut self, supports: bool) -> Self {
        self.multilingual = supports;
        self
    }

    #[must_use]
    pub fn with_content_generation(mut self, supports: bool) -> Self {
        self.content_generation = supports;
        self
    }

    #[must_use]
    pub fn with_business_acumen(mut self, supports: bool) -> Self {
        self.business_acumen = supports;
        self
    }

    #[must_use]
    pub fn reasoning(&self) -> bool {
        self.reasoning
    }

    #[must_use]
    pub fn web_search(&self) -> bool {
        self.web_search
    }

    #[must_use]
    pub fn tool_use(&self) -> bool {
        self.tool_use
    }

    #[must_use]
    pub fn vision(&self) -> bool {
        self.vision
    }

    #[must_use]
    pub fn context_tokens(&self) -> Option<ContextWindowTokens> {
        self.context_tokens
    }

    #[must_use]
    pub fn structured_output(&self) -> bool {
        self.structured_output
    }

    #[must_use]
    pub fn code(&self) -> bool {
        self.code
    }

    #[must_use]
    pub fn multilingual(&self) -> bool {
        self.multilingual
    }

    #[must_use]
    pub fn content_generation(&self) -> bool {
        self.content_generation
    }

    #[must_use]
    pub fn business_acumen(&self) -> bool {
        self.business_acumen
    }

    fn satisfies(&self, requirements: &AgentRequirements) -> bool {
        if requirements.requires_reasoning && !self.reasoning {
            return false;
        }
        if requirements.requires_web_search && !self.web_search {
            return false;
        }
        if requirements.requires_tool_use && !self.tool_use {
            return false;
        }
        if requirements.requires_vision && !self.vision {
            return false;
        }
        if let Some(required) = requirements.min_context_tokens {
            let Some(available) = self.context_tokens else {
                return false;
            };
            if available.get() < required {
                return false;
            }
        }
        if requirements.requires_structured_output && !self.structured_output {
            return false;
        }
        if requirements.requires_code && !self.code {
            return false;
        }
        if requirements.requires_multilingual && !self.multilingual {
            return false;
        }
        if requirements.requires_content_generation && !self.content_generation {
            return false;
        }
        if requirements.requires_business_acumen && !self.business_acumen {
            return false;
        }
        true
    }
}

/// Metadata used to match a registered chat backend to selection criteria.
#[derive(Debug, Clone, PartialEq)]
pub struct ChatBackendDescriptor {
    provider: ProviderName,
    model: ModelName,
    cost_class: CostClass,
    typical_latency: LatencyMillis,
    quality: QualityScore,
    capabilities: ChatBackendCapabilities,
    data_sovereignty: DataSovereignty,
    compliance: ComplianceLevel,
}

impl ChatBackendDescriptor {
    #[must_use]
    pub fn new(
        provider: ProviderName,
        model: ModelName,
        cost_class: CostClass,
        typical_latency: LatencyMillis,
        quality: QualityScore,
    ) -> Self {
        Self {
            provider,
            model,
            cost_class,
            typical_latency,
            quality,
            capabilities: ChatBackendCapabilities::default(),
            data_sovereignty: DataSovereignty::Any,
            compliance: ComplianceLevel::None,
        }
    }

    #[must_use]
    pub fn with_capabilities(mut self, capabilities: ChatBackendCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    #[must_use]
    pub fn with_data_sovereignty(mut self, sovereignty: DataSovereignty) -> Self {
        self.data_sovereignty = sovereignty;
        self
    }

    #[must_use]
    pub fn with_compliance(mut self, compliance: ComplianceLevel) -> Self {
        self.compliance = compliance;
        self
    }

    #[must_use]
    pub fn provider(&self) -> &ProviderName {
        &self.provider
    }

    #[must_use]
    pub fn model(&self) -> &ModelName {
        &self.model
    }

    #[must_use]
    pub fn cost_class(&self) -> CostClass {
        self.cost_class
    }

    #[must_use]
    pub fn typical_latency(&self) -> LatencyMillis {
        self.typical_latency
    }

    #[must_use]
    pub fn quality(&self) -> QualityScore {
        self.quality
    }

    #[must_use]
    pub fn capabilities(&self) -> &ChatBackendCapabilities {
        &self.capabilities
    }

    #[must_use]
    pub fn data_sovereignty(&self) -> DataSovereignty {
        self.data_sovereignty
    }

    #[must_use]
    pub fn compliance(&self) -> ComplianceLevel {
        self.compliance
    }

    #[must_use]
    pub fn satisfies(&self, requirements: &AgentRequirements) -> bool {
        if !requirements
            .max_cost_class
            .allowed_classes()
            .contains(&self.cost_class)
        {
            return false;
        }
        if self.typical_latency.get() > requirements.max_latency_ms {
            return false;
        }
        if self.quality.get() < requirements.min_quality {
            return false;
        }
        if requirements.data_sovereignty != DataSovereignty::Any
            && self.data_sovereignty != requirements.data_sovereignty
        {
            return false;
        }
        if requirements.compliance != ComplianceLevel::None
            && self.compliance != requirements.compliance
        {
            return false;
        }
        self.capabilities.satisfies(requirements)
    }

    #[must_use]
    pub fn fitness_score(&self, requirements: &AgentRequirements) -> Option<f64> {
        if !self.satisfies(requirements) {
            return None;
        }

        let cost_score = match self.cost_class {
            CostClass::Free | CostClass::VeryLow => 1.0,
            CostClass::Low => 0.8,
            CostClass::Medium => 0.6,
            CostClass::High => 0.4,
            CostClass::VeryHigh => 0.2,
        };
        let latency_ratio =
            f64::from(self.typical_latency.get()) / f64::from(requirements.max_latency_ms);
        let latency_score = 1.0 - latency_ratio.min(1.0);
        Some(0.4 * cost_score + 0.3 * latency_score + 0.3 * self.quality.get())
    }
}

/// A host-registered chat backend handle.
#[derive(Clone)]
pub struct RegisteredChatBackend {
    descriptor: ChatBackendDescriptor,
    backend: Arc<dyn DynChatBackend>,
}

impl RegisteredChatBackend {
    #[must_use]
    pub fn new(descriptor: ChatBackendDescriptor, backend: Arc<dyn DynChatBackend>) -> Self {
        Self {
            descriptor,
            backend,
        }
    }

    #[must_use]
    pub fn descriptor(&self) -> &ChatBackendDescriptor {
        &self.descriptor
    }

    #[must_use]
    pub fn backend(&self) -> Arc<dyn DynChatBackend> {
        Arc::clone(&self.backend)
    }
}

/// A chat backend selected from a host-supplied registry.
#[derive(Clone)]
pub struct ResolvedChatBackend {
    descriptor: ChatBackendDescriptor,
    backend: Arc<dyn DynChatBackend>,
}

impl ResolvedChatBackend {
    #[must_use]
    pub fn descriptor(&self) -> &ChatBackendDescriptor {
        &self.descriptor
    }

    #[must_use]
    pub fn backend(&self) -> Arc<dyn DynChatBackend> {
        Arc::clone(&self.backend)
    }

    #[must_use]
    pub fn provider(&self) -> &str {
        self.descriptor.provider().as_str()
    }

    #[must_use]
    pub fn model(&self) -> &str {
        self.descriptor.model().as_str()
    }
}

/// Host-populated registry for chat capability handles.
#[derive(Clone, Default)]
pub struct ChatBackendRegistry {
    backends: Vec<RegisteredChatBackend>,
}

impl ChatBackendRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, backend: RegisteredChatBackend) {
        self.backends.push(backend);
    }

    #[must_use]
    pub fn with_backend(mut self, backend: RegisteredChatBackend) -> Self {
        self.register(backend);
        self
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.backends.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.backends.is_empty()
    }

    #[must_use]
    pub fn descriptors(&self) -> Vec<&ChatBackendDescriptor> {
        self.backends
            .iter()
            .map(RegisteredChatBackend::descriptor)
            .collect()
    }

    /// Selects a registered backend handle using the contract-level criteria.
    ///
    /// # Errors
    ///
    /// Returns an error if no registered backend satisfies the requested
    /// criteria or provider override.
    pub fn select(
        &self,
        config: &ChatBackendSelectionConfig,
    ) -> Result<ResolvedChatBackend, LlmError> {
        let requirements = config.criteria.to_agent_requirements();
        let provider_override = config.provider_override.as_deref();

        let mut candidates: Vec<(&RegisteredChatBackend, f64)> = self
            .backends
            .iter()
            .filter(|backend| {
                provider_override.is_none_or(|provider| {
                    backend
                        .descriptor()
                        .provider()
                        .as_str()
                        .eq_ignore_ascii_case(provider)
                })
            })
            .filter_map(|backend| {
                backend
                    .descriptor()
                    .fitness_score(&requirements)
                    .map(|score| (backend, score))
            })
            .collect();

        if candidates.is_empty() {
            return Err(LlmError::ProviderError {
                message: no_registered_backend_message(provider_override),
                code: None,
            });
        }

        candidates.sort_by(|left, right| {
            right
                .1
                .total_cmp(&left.1)
                .then_with(|| {
                    left.0
                        .descriptor()
                        .provider()
                        .as_str()
                        .cmp(right.0.descriptor().provider().as_str())
                })
                .then_with(|| {
                    left.0
                        .descriptor()
                        .model()
                        .as_str()
                        .cmp(right.0.descriptor().model().as_str())
                })
        });

        let selected = candidates[0].0;
        Ok(ResolvedChatBackend {
            descriptor: selected.descriptor().clone(),
            backend: selected.backend(),
        })
    }
}

fn no_registered_backend_message(provider_override: Option<&str>) -> String {
    match provider_override {
        Some(provider) => format!(
            "No registered chat backend satisfied the selection criteria for provider {provider}"
        ),
        None => "No registered chat backend satisfied the selection criteria".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use std::future;

    use crate::chat::{ChatBackend, ChatRequest, ChatResponse, FinishReason, LlmError, TokenUsage};
    use crate::selection::{
        ChatBackendSelectionConfig, RequiredCapabilities, SelectionCriteria, TaskComplexity,
    };

    use super::*;

    struct TestBackend;

    impl ChatBackend for TestBackend {
        type ChatFut<'a>
            = future::Ready<Result<ChatResponse, LlmError>>
        where
            Self: 'a;

        fn chat<'a>(&'a self, _req: ChatRequest) -> Self::ChatFut<'a> {
            future::ready(Ok(ChatResponse {
                content: "ok".to_string(),
                tool_calls: Vec::new(),
                usage: Some(TokenUsage::default()),
                model: Some("test-model".to_string()),
                finish_reason: Some(FinishReason::Stop),
                metadata: std::collections::HashMap::new(),
            }))
        }
    }

    fn provider(value: &str) -> ProviderName {
        ProviderName::new(value).unwrap()
    }

    fn model(value: &str) -> ModelName {
        ModelName::new(value).unwrap()
    }

    fn latency(value: u32) -> LatencyMillis {
        LatencyMillis::new(value).unwrap()
    }

    fn quality(value: f64) -> QualityScore {
        QualityScore::new(value).unwrap()
    }

    fn backend(
        provider_name: &str,
        model_name: &str,
        cost: CostClass,
        latency_ms: u32,
        quality_score: f64,
        capabilities: ChatBackendCapabilities,
    ) -> RegisteredChatBackend {
        let descriptor = ChatBackendDescriptor::new(
            provider(provider_name),
            model(model_name),
            cost,
            latency(latency_ms),
            quality(quality_score),
        )
        .with_capabilities(capabilities);
        RegisteredChatBackend::new(descriptor, Arc::new(TestBackend))
    }

    #[test]
    fn semantic_names_reject_empty_strings() {
        assert!(ProviderName::new(" ").is_err());
        assert!(ModelName::new("").is_err());
    }

    #[test]
    fn numeric_contract_values_validate_boundaries() {
        assert!(LatencyMillis::new(0).is_err());
        assert!(ContextWindowTokens::new(0).is_err());
        assert!(QualityScore::new(-0.01).is_err());
        assert!(QualityScore::new(1.01).is_err());
        assert!(QualityScore::new(f64::NAN).is_err());
        assert_eq!(QualityScore::new(1.0).unwrap().get(), 1.0);
    }

    #[test]
    fn registry_selects_best_registered_handle() {
        let registry = ChatBackendRegistry::new()
            .with_backend(backend(
                "slow",
                "slow-model",
                CostClass::High,
                1_500,
                0.7,
                ChatBackendCapabilities::new(),
            ))
            .with_backend(backend(
                "fast",
                "fast-model",
                CostClass::Low,
                100,
                0.8,
                ChatBackendCapabilities::new(),
            ));

        let selected = registry
            .select(&ChatBackendSelectionConfig::default())
            .unwrap();
        assert_eq!(selected.provider(), "fast");
        assert_eq!(selected.model(), "fast-model");
    }

    #[test]
    fn registry_honors_provider_override() {
        let registry = ChatBackendRegistry::new()
            .with_backend(backend(
                "fast",
                "fast-model",
                CostClass::Low,
                100,
                0.8,
                ChatBackendCapabilities::new(),
            ))
            .with_backend(backend(
                "forced",
                "forced-model",
                CostClass::Low,
                100,
                0.8,
                ChatBackendCapabilities::new(),
            ));

        let selected = registry
            .select(&ChatBackendSelectionConfig::default().with_provider_override("forced"))
            .unwrap();
        assert_eq!(selected.provider(), "forced");
    }

    #[test]
    fn registry_filters_by_required_capabilities() {
        let registry = ChatBackendRegistry::new()
            .with_backend(backend(
                "basic",
                "basic-model",
                CostClass::Low,
                100,
                0.9,
                ChatBackendCapabilities::new(),
            ))
            .with_backend(backend(
                "reasoning",
                "reasoning-model",
                CostClass::Low,
                100,
                0.9,
                ChatBackendCapabilities::new().with_reasoning(true),
            ));
        let config = ChatBackendSelectionConfig::default().with_criteria(
            SelectionCriteria::interactive().with_complexity(TaskComplexity::Reasoning),
        );

        let selected = registry.select(&config).unwrap();
        assert_eq!(selected.provider(), "reasoning");
    }

    #[test]
    fn registry_filters_by_context_window() {
        let registry = ChatBackendRegistry::new()
            .with_backend(backend(
                "small",
                "small-model",
                CostClass::Low,
                100,
                0.9,
                ChatBackendCapabilities::new()
                    .with_context_tokens(ContextWindowTokens::new(8_192).unwrap()),
            ))
            .with_backend(backend(
                "large",
                "large-model",
                CostClass::Low,
                100,
                0.9,
                ChatBackendCapabilities::new()
                    .with_context_tokens(ContextWindowTokens::new(128_000).unwrap()),
            ));
        let config = ChatBackendSelectionConfig::default().with_criteria(
            SelectionCriteria::interactive()
                .with_capabilities(RequiredCapabilities::none().with_min_context(64_000)),
        );

        let selected = registry.select(&config).unwrap();
        assert_eq!(selected.provider(), "large");
    }

    #[test]
    fn empty_registry_returns_selection_error() {
        let err = ChatBackendRegistry::new()
            .select(&ChatBackendSelectionConfig::default())
            .err()
            .unwrap();
        assert!(err.to_string().contains("No registered chat backend"));
    }
}
