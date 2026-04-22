// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Canonical backend selection vocabulary for provider consumers and adapters.

use serde::{Deserialize, Serialize};

use crate::backend::BackendKind;
use crate::capability::Capability;
use crate::chat::LlmError;
use crate::error::BackendError;

/// Requirements for generic backend selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendRequirements {
    pub kind: BackendKind,
    pub required_capabilities: Vec<Capability>,
    pub max_cost_class: CostClass,
    pub max_latency_ms: u32,
    pub data_sovereignty: DataSovereignty,
    pub compliance: ComplianceLevel,
    pub requires_replay: bool,
    pub requires_offline: bool,
}

impl BackendRequirements {
    #[must_use]
    pub fn new(kind: BackendKind) -> Self {
        Self {
            kind,
            required_capabilities: Vec::new(),
            max_cost_class: CostClass::VeryHigh,
            max_latency_ms: 0,
            data_sovereignty: DataSovereignty::Any,
            compliance: ComplianceLevel::None,
            requires_replay: false,
            requires_offline: false,
        }
    }

    #[must_use]
    pub fn with_capability(mut self, capability: Capability) -> Self {
        self.required_capabilities.push(capability);
        self
    }

    #[must_use]
    pub fn with_max_cost(mut self, cost: CostClass) -> Self {
        self.max_cost_class = cost;
        self
    }

    #[must_use]
    pub fn with_max_latency_ms(mut self, ms: u32) -> Self {
        self.max_latency_ms = ms;
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
    pub fn with_replay(mut self) -> Self {
        self.requires_replay = true;
        self
    }

    #[must_use]
    pub fn with_offline(mut self) -> Self {
        self.requires_offline = true;
        self
    }

    #[must_use]
    pub fn fast_llm() -> Self {
        Self::new(BackendKind::Llm)
            .with_capability(Capability::TextGeneration)
            .with_max_cost(CostClass::Low)
            .with_max_latency_ms(2_000)
    }

    #[must_use]
    pub fn reasoning_llm() -> Self {
        Self::new(BackendKind::Llm)
            .with_capability(Capability::TextGeneration)
            .with_capability(Capability::Reasoning)
            .with_max_cost(CostClass::High)
            .with_max_latency_ms(30_000)
    }

    #[must_use]
    pub fn access_policy() -> Self {
        Self::new(BackendKind::Policy)
            .with_capability(Capability::AccessControl)
            .with_max_latency_ms(100)
    }

    #[must_use]
    pub fn constraint_solver() -> Self {
        Self::new(BackendKind::Optimization).with_capability(Capability::ConstraintSolving)
    }

    #[must_use]
    pub fn embedding_pipeline() -> Self {
        Self::new(BackendKind::Analytics).with_capability(Capability::Embedding)
    }

    #[must_use]
    pub fn vector_search() -> Self {
        Self::new(BackendKind::Search).with_capability(Capability::VectorSearch)
    }
}

/// Trait for selecting a backend that satisfies generic requirements.
pub trait BackendSelector: Send + Sync {
    fn select(&self, requirements: &BackendRequirements) -> Result<String, BackendError>;
}

/// Structured request for provider selection inside a convergence loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRequest {
    /// Stable request identifier used for idempotency.
    pub id: String,
    /// Capabilities that must each be covered by at least one backend.
    /// Duplicates request multiple independent backends for the same capability.
    pub required_capabilities: Vec<Capability>,
}

/// Structured result of provider selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAssignment {
    /// The request this assignment answers.
    pub request_id: String,
    /// Matched capability-to-backend assignments.
    pub assignments: Vec<CapabilityAssignment>,
    /// Capabilities that no registered backend could satisfy.
    pub unmatched: Vec<Capability>,
    /// `assignments.len() / required_capabilities.len()` — 1.0 is full coverage.
    pub coverage_ratio: f64,
}

/// A single capability-to-backend assignment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityAssignment {
    pub capability: Capability,
    pub backend_name: String,
}

/// Data jurisdiction requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Jurisdiction {
    #[default]
    Unrestricted,
    Trusted,
    SameRegion,
    SameCountry,
}

impl Jurisdiction {
    #[must_use]
    pub fn satisfied_by(
        self,
        provider_country: &str,
        provider_region: &str,
        user_country: &str,
        user_region: &str,
    ) -> bool {
        match self {
            Self::Unrestricted => true,
            Self::Trusted => is_trusted_jurisdiction(provider_region),
            Self::SameRegion => provider_region == user_region,
            Self::SameCountry => provider_country == user_country,
        }
    }
}

fn is_trusted_jurisdiction(region: &str) -> bool {
    matches!(
        region.to_uppercase().as_str(),
        "EU" | "EEA" | "CH" | "UK" | "JP" | "CA" | "NZ" | "IL" | "KR" | "AR" | "UY"
    )
}

/// Latency class requirements.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub enum LatencyClass {
    Realtime,
    #[default]
    Interactive,
    Background,
    Batch,
}

impl LatencyClass {
    #[must_use]
    pub fn max_latency_ms(self) -> u32 {
        match self {
            Self::Realtime => 100,
            Self::Interactive => 2_000,
            Self::Background => 30_000,
            Self::Batch => 300_000,
        }
    }

    #[must_use]
    pub fn satisfied_by(self, provider_latency_ms: u32) -> bool {
        provider_latency_ms <= self.max_latency_ms()
    }
}

/// Cost tier preference.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default, Serialize, Deserialize,
)]
pub enum CostTier {
    Minimal,
    #[default]
    Standard,
    Premium,
}

/// Task complexity hint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum TaskComplexity {
    Extraction,
    #[default]
    Classification,
    Reasoning,
    Generation,
}

impl TaskComplexity {
    #[must_use]
    pub fn min_quality_hint(self) -> f64 {
        match self {
            Self::Extraction => 0.5,
            Self::Classification => 0.6,
            Self::Reasoning => 0.8,
            Self::Generation => 0.7,
        }
    }

    #[must_use]
    pub fn requires_reasoning(self) -> bool {
        matches!(self, Self::Reasoning)
    }
}

/// Required model capabilities.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct RequiredCapabilities {
    pub tool_use: bool,
    pub vision: bool,
    pub min_context_tokens: Option<usize>,
    pub structured_output: bool,
    pub code: bool,
    pub multilingual: bool,
    pub web_search: bool,
}

impl RequiredCapabilities {
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_tool_use(mut self) -> Self {
        self.tool_use = true;
        self
    }

    #[must_use]
    pub fn with_vision(mut self) -> Self {
        self.vision = true;
        self
    }

    #[must_use]
    pub fn with_min_context(mut self, tokens: usize) -> Self {
        self.min_context_tokens = Some(tokens);
        self
    }

    #[must_use]
    pub fn with_structured_output(mut self) -> Self {
        self.structured_output = true;
        self
    }

    #[must_use]
    pub fn with_code(mut self) -> Self {
        self.code = true;
        self
    }

    #[must_use]
    pub fn with_multilingual(mut self) -> Self {
        self.multilingual = true;
        self
    }

    #[must_use]
    pub fn with_web_search(mut self) -> Self {
        self.web_search = true;
        self
    }
}

/// Cost classification — how expensive is this backend to use?
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CostClass {
    Free,
    VeryLow,
    Low,
    Medium,
    High,
    VeryHigh,
}

impl CostClass {
    #[must_use]
    pub fn allowed_classes(self) -> Vec<CostClass> {
        let all = [
            CostClass::Free,
            CostClass::VeryLow,
            CostClass::Low,
            CostClass::Medium,
            CostClass::High,
            CostClass::VeryHigh,
        ];
        all.iter().copied().filter(|&c| c <= self).collect()
    }

    #[must_use]
    pub fn from_tier(tier: CostTier) -> Self {
        match tier {
            CostTier::Minimal => Self::Low,
            CostTier::Standard => Self::Medium,
            CostTier::Premium => Self::VeryHigh,
        }
    }
}

/// Data sovereignty requirements — where can data legally reside?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataSovereignty {
    Any,
    EU,
    US,
    Switzerland,
    China,
    OnPremises,
}

impl DataSovereignty {
    #[must_use]
    pub fn from_jurisdiction(jurisdiction: Jurisdiction, user_region: &str) -> Self {
        match jurisdiction {
            Jurisdiction::Unrestricted | Jurisdiction::Trusted => Self::Any,
            Jurisdiction::SameRegion => match user_region.to_uppercase().as_str() {
                "EU" | "EEA" => Self::EU,
                "CH" => Self::Switzerland,
                "CN" => Self::China,
                "US" => Self::US,
                _ => Self::Any,
            },
            Jurisdiction::SameCountry => Self::OnPremises,
        }
    }
}

/// Compliance level requirements.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceLevel {
    None,
    GDPR,
    HIPAA,
    SOC2,
    HighExplainability,
}

/// Selection criteria using orthogonal dimensions.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SelectionCriteria {
    pub jurisdiction: Jurisdiction,
    pub latency: LatencyClass,
    pub cost: CostTier,
    pub complexity: TaskComplexity,
    pub capabilities: RequiredCapabilities,
    pub compliance: Option<ComplianceLevel>,
    pub user_country: Option<String>,
    pub user_region: Option<String>,
}

impl SelectionCriteria {
    #[must_use]
    pub fn high_volume() -> Self {
        Self {
            latency: LatencyClass::Interactive,
            cost: CostTier::Minimal,
            complexity: TaskComplexity::Extraction,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn interactive() -> Self {
        Self {
            latency: LatencyClass::Interactive,
            cost: CostTier::Minimal,
            complexity: TaskComplexity::Classification,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn analysis() -> Self {
        Self {
            latency: LatencyClass::Background,
            cost: CostTier::Premium,
            complexity: TaskComplexity::Reasoning,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn batch() -> Self {
        Self {
            latency: LatencyClass::Batch,
            cost: CostTier::Minimal,
            complexity: TaskComplexity::Extraction,
            ..Default::default()
        }
    }

    #[must_use]
    pub fn with_jurisdiction(mut self, jurisdiction: Jurisdiction) -> Self {
        self.jurisdiction = jurisdiction;
        self
    }

    #[must_use]
    pub fn with_latency(mut self, latency: LatencyClass) -> Self {
        self.latency = latency;
        self
    }

    #[must_use]
    pub fn with_cost(mut self, cost: CostTier) -> Self {
        self.cost = cost;
        self
    }

    #[must_use]
    pub fn with_complexity(mut self, complexity: TaskComplexity) -> Self {
        self.complexity = complexity;
        self
    }

    #[must_use]
    pub fn with_capabilities(mut self, capabilities: RequiredCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    #[must_use]
    pub fn with_compliance(mut self, compliance: ComplianceLevel) -> Self {
        self.compliance = Some(compliance);
        self
    }

    #[must_use]
    pub fn with_user_location(
        mut self,
        country: impl Into<String>,
        region: impl Into<String>,
    ) -> Self {
        self.user_country = Some(country.into());
        self.user_region = Some(region.into());
        self
    }

    #[must_use]
    pub fn to_agent_requirements(&self) -> AgentRequirements {
        let user_region = self.user_region.as_deref().unwrap_or("US");
        AgentRequirements {
            max_cost_class: CostClass::from_tier(self.cost),
            max_latency_ms: self.latency.max_latency_ms(),
            requires_reasoning: self.complexity.requires_reasoning(),
            requires_web_search: self.capabilities.web_search,
            requires_tool_use: self.capabilities.tool_use,
            requires_vision: self.capabilities.vision,
            min_context_tokens: self.capabilities.min_context_tokens,
            requires_structured_output: self.capabilities.structured_output,
            requires_code: self.capabilities.code,
            min_quality: self.complexity.min_quality_hint(),
            data_sovereignty: DataSovereignty::from_jurisdiction(self.jurisdiction, user_region),
            compliance: self.compliance.unwrap_or(ComplianceLevel::None),
            requires_multilingual: self.capabilities.multilingual,
        }
    }
}

/// Requirements for an agent's LLM usage.
#[derive(Debug, Clone, PartialEq)]
pub struct AgentRequirements {
    pub max_cost_class: CostClass,
    pub max_latency_ms: u32,
    pub requires_reasoning: bool,
    pub requires_web_search: bool,
    pub requires_tool_use: bool,
    pub requires_vision: bool,
    pub min_context_tokens: Option<usize>,
    pub requires_structured_output: bool,
    pub requires_code: bool,
    pub min_quality: f64,
    pub data_sovereignty: DataSovereignty,
    pub compliance: ComplianceLevel,
    pub requires_multilingual: bool,
}

impl AgentRequirements {
    #[must_use]
    pub fn fast_cheap() -> Self {
        Self {
            max_cost_class: CostClass::VeryLow,
            max_latency_ms: 2_000,
            requires_reasoning: false,
            requires_web_search: false,
            requires_tool_use: false,
            requires_vision: false,
            min_context_tokens: None,
            requires_structured_output: false,
            requires_code: false,
            min_quality: 0.6,
            data_sovereignty: DataSovereignty::Any,
            compliance: ComplianceLevel::None,
            requires_multilingual: false,
        }
    }

    #[must_use]
    pub fn deep_research() -> Self {
        Self {
            max_cost_class: CostClass::High,
            max_latency_ms: 30_000,
            requires_reasoning: true,
            requires_web_search: true,
            requires_tool_use: false,
            requires_vision: false,
            min_context_tokens: None,
            requires_structured_output: false,
            requires_code: false,
            min_quality: 0.9,
            data_sovereignty: DataSovereignty::Any,
            compliance: ComplianceLevel::None,
            requires_multilingual: false,
        }
    }

    #[must_use]
    pub fn balanced() -> Self {
        Self {
            max_cost_class: CostClass::Medium,
            max_latency_ms: 5_000,
            requires_reasoning: false,
            requires_web_search: false,
            requires_tool_use: false,
            requires_vision: false,
            min_context_tokens: None,
            requires_structured_output: false,
            requires_code: false,
            min_quality: 0.7,
            data_sovereignty: DataSovereignty::Any,
            compliance: ComplianceLevel::None,
            requires_multilingual: false,
        }
    }

    #[must_use]
    pub fn new(max_cost_class: CostClass, max_latency_ms: u32, requires_reasoning: bool) -> Self {
        Self {
            max_cost_class,
            max_latency_ms,
            requires_reasoning,
            requires_web_search: false,
            requires_tool_use: false,
            requires_vision: false,
            min_context_tokens: None,
            requires_structured_output: false,
            requires_code: false,
            min_quality: 0.7,
            data_sovereignty: DataSovereignty::Any,
            compliance: ComplianceLevel::None,
            requires_multilingual: false,
        }
    }

    #[must_use]
    pub fn powerful() -> Self {
        Self {
            max_cost_class: CostClass::High,
            max_latency_ms: 10_000,
            requires_reasoning: true,
            requires_web_search: false,
            requires_tool_use: false,
            requires_vision: false,
            min_context_tokens: None,
            requires_structured_output: false,
            requires_code: false,
            min_quality: 0.9,
            data_sovereignty: DataSovereignty::Any,
            compliance: ComplianceLevel::None,
            requires_multilingual: false,
        }
    }

    #[must_use]
    pub fn with_quality(self, quality: f64) -> Self {
        self.with_min_quality(quality)
    }

    #[must_use]
    pub fn with_web_search(mut self, requires: bool) -> Self {
        self.requires_web_search = requires;
        self
    }

    #[must_use]
    pub fn with_tool_use(mut self, requires: bool) -> Self {
        self.requires_tool_use = requires;
        self
    }

    #[must_use]
    pub fn with_vision(mut self, requires: bool) -> Self {
        self.requires_vision = requires;
        self
    }

    #[must_use]
    pub fn with_min_context(mut self, tokens: usize) -> Self {
        self.min_context_tokens = Some(tokens);
        self
    }

    #[must_use]
    pub fn with_structured_output(mut self, requires: bool) -> Self {
        self.requires_structured_output = requires;
        self
    }

    #[must_use]
    pub fn with_code(mut self, requires: bool) -> Self {
        self.requires_code = requires;
        self
    }

    #[must_use]
    pub fn with_min_quality(mut self, quality: f64) -> Self {
        self.min_quality = quality.clamp(0.0, 1.0);
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
    pub fn with_multilingual(mut self, requires: bool) -> Self {
        self.requires_multilingual = requires;
        self
    }

    #[must_use]
    pub fn from_criteria(criteria: &SelectionCriteria) -> Self {
        criteria.to_agent_requirements()
    }
}

/// Trait for model selection based on LLM requirements.
pub trait ModelSelectorTrait: Send + Sync {
    fn select(&self, requirements: &AgentRequirements) -> Result<(String, String), LlmError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_class_ordering() {
        assert!(CostClass::Free < CostClass::VeryLow);
        assert!(CostClass::VeryLow < CostClass::Low);
        assert!(CostClass::Low < CostClass::Medium);
        assert!(CostClass::Medium < CostClass::High);
        assert!(CostClass::High < CostClass::VeryHigh);
    }

    #[test]
    fn requirements_builder() {
        let reqs = BackendRequirements::new(BackendKind::Llm)
            .with_capability(Capability::TextGeneration)
            .with_capability(Capability::Reasoning)
            .with_max_cost(CostClass::Medium)
            .with_max_latency_ms(5_000);

        assert_eq!(reqs.kind, BackendKind::Llm);
        assert_eq!(reqs.required_capabilities.len(), 2);
        assert_eq!(reqs.max_cost_class, CostClass::Medium);
        assert_eq!(reqs.max_latency_ms, 5_000);
    }

    #[test]
    fn selection_criteria_presets() {
        let high_vol = SelectionCriteria::high_volume();
        assert_eq!(high_vol.cost, CostTier::Minimal);
        assert_eq!(high_vol.complexity, TaskComplexity::Extraction);

        let analysis = SelectionCriteria::analysis();
        assert_eq!(analysis.cost, CostTier::Premium);
        assert_eq!(analysis.complexity, TaskComplexity::Reasoning);
    }

    #[test]
    fn selection_criteria_to_agent_requirements() {
        let criteria = SelectionCriteria::default()
            .with_latency(LatencyClass::Background)
            .with_cost(CostTier::Premium)
            .with_complexity(TaskComplexity::Reasoning)
            .with_capabilities(
                RequiredCapabilities::none()
                    .with_tool_use()
                    .with_vision()
                    .with_min_context(128_000)
                    .with_structured_output()
                    .with_code(),
            );
        let requirements = criteria.to_agent_requirements();
        assert_eq!(requirements.max_latency_ms, 30_000);
        assert!(requirements.requires_reasoning);
        assert!(requirements.min_quality >= 0.8);
        assert!(requirements.requires_tool_use);
        assert!(requirements.requires_vision);
        assert_eq!(requirements.min_context_tokens, Some(128_000));
        assert!(requirements.requires_structured_output);
        assert!(requirements.requires_code);
    }

    #[test]
    fn preset_constructors() {
        let fast = BackendRequirements::fast_llm();
        assert_eq!(fast.kind, BackendKind::Llm);
        assert_eq!(fast.max_cost_class, CostClass::Low);

        let policy = BackendRequirements::access_policy();
        assert_eq!(policy.kind, BackendKind::Policy);
        assert!(
            policy
                .required_capabilities
                .contains(&Capability::AccessControl)
        );

        let solver = BackendRequirements::constraint_solver();
        assert_eq!(solver.kind, BackendKind::Optimization);
    }

    #[test]
    fn preset_reasoning_llm() {
        let r = BackendRequirements::reasoning_llm();
        assert_eq!(r.kind, BackendKind::Llm);
        assert_eq!(r.max_cost_class, CostClass::High);
        assert_eq!(r.max_latency_ms, 30_000);
        assert!(
            r.required_capabilities
                .contains(&Capability::TextGeneration)
        );
        assert!(r.required_capabilities.contains(&Capability::Reasoning));
    }

    #[test]
    fn preset_embedding_pipeline() {
        let r = BackendRequirements::embedding_pipeline();
        assert_eq!(r.kind, BackendKind::Analytics);
        assert!(r.required_capabilities.contains(&Capability::Embedding));
    }

    #[test]
    fn preset_vector_search() {
        let r = BackendRequirements::vector_search();
        assert_eq!(r.kind, BackendKind::Search);
        assert!(r.required_capabilities.contains(&Capability::VectorSearch));
    }

    #[test]
    fn jurisdiction_unrestricted_always_satisfied() {
        assert!(Jurisdiction::Unrestricted.satisfied_by("US", "US", "SE", "EU"));
    }

    #[test]
    fn jurisdiction_trusted_eu() {
        assert!(Jurisdiction::Trusted.satisfied_by("SE", "EU", "SE", "EU"));
    }

    #[test]
    fn jurisdiction_trusted_us_not_trusted() {
        assert!(!Jurisdiction::Trusted.satisfied_by("US", "US", "SE", "EU"));
    }

    #[test]
    fn jurisdiction_trusted_various() {
        for region in &["CH", "UK", "JP", "CA", "NZ", "IL", "KR", "AR", "UY", "EEA"] {
            assert!(
                Jurisdiction::Trusted.satisfied_by("X", region, "Y", "Z"),
                "expected {region} to be trusted"
            );
        }
    }

    #[test]
    fn jurisdiction_same_region() {
        assert!(Jurisdiction::SameRegion.satisfied_by("SE", "EU", "DE", "EU"));
        assert!(!Jurisdiction::SameRegion.satisfied_by("SE", "EU", "US", "US"));
    }

    #[test]
    fn jurisdiction_same_country() {
        assert!(Jurisdiction::SameCountry.satisfied_by("SE", "EU", "SE", "EU"));
        assert!(!Jurisdiction::SameCountry.satisfied_by("SE", "EU", "DE", "EU"));
    }

    #[test]
    fn latency_class_max_values() {
        assert_eq!(LatencyClass::Realtime.max_latency_ms(), 100);
        assert_eq!(LatencyClass::Interactive.max_latency_ms(), 2_000);
        assert_eq!(LatencyClass::Background.max_latency_ms(), 30_000);
        assert_eq!(LatencyClass::Batch.max_latency_ms(), 300_000);
    }

    #[test]
    fn latency_class_satisfied_by() {
        assert!(LatencyClass::Realtime.satisfied_by(50));
        assert!(LatencyClass::Realtime.satisfied_by(100));
        assert!(!LatencyClass::Realtime.satisfied_by(101));

        assert!(LatencyClass::Interactive.satisfied_by(2_000));
        assert!(!LatencyClass::Interactive.satisfied_by(2_001));
    }

    #[test]
    fn latency_class_ordering() {
        assert!(LatencyClass::Realtime < LatencyClass::Interactive);
        assert!(LatencyClass::Interactive < LatencyClass::Background);
        assert!(LatencyClass::Background < LatencyClass::Batch);
    }

    #[test]
    fn data_sovereignty_from_jurisdiction_unrestricted() {
        assert_eq!(
            DataSovereignty::from_jurisdiction(Jurisdiction::Unrestricted, "EU"),
            DataSovereignty::Any
        );
    }

    #[test]
    fn data_sovereignty_from_jurisdiction_trusted() {
        assert_eq!(
            DataSovereignty::from_jurisdiction(Jurisdiction::Trusted, "EU"),
            DataSovereignty::Any
        );
    }

    #[test]
    fn data_sovereignty_from_jurisdiction_same_region_eu() {
        assert_eq!(
            DataSovereignty::from_jurisdiction(Jurisdiction::SameRegion, "EU"),
            DataSovereignty::EU
        );
        assert_eq!(
            DataSovereignty::from_jurisdiction(Jurisdiction::SameRegion, "eea"),
            DataSovereignty::EU
        );
    }

    #[test]
    fn data_sovereignty_from_jurisdiction_same_region_other() {
        assert_eq!(
            DataSovereignty::from_jurisdiction(Jurisdiction::SameRegion, "CH"),
            DataSovereignty::Switzerland
        );
        assert_eq!(
            DataSovereignty::from_jurisdiction(Jurisdiction::SameRegion, "CN"),
            DataSovereignty::China
        );
        assert_eq!(
            DataSovereignty::from_jurisdiction(Jurisdiction::SameRegion, "US"),
            DataSovereignty::US
        );
        assert_eq!(
            DataSovereignty::from_jurisdiction(Jurisdiction::SameRegion, "XX"),
            DataSovereignty::Any
        );
    }

    #[test]
    fn data_sovereignty_from_jurisdiction_same_country() {
        assert_eq!(
            DataSovereignty::from_jurisdiction(Jurisdiction::SameCountry, "SE"),
            DataSovereignty::OnPremises
        );
    }

    #[test]
    fn cost_class_allowed_contains_self() {
        for class in [
            CostClass::Free,
            CostClass::VeryLow,
            CostClass::Low,
            CostClass::Medium,
            CostClass::High,
            CostClass::VeryHigh,
        ] {
            let allowed = class.allowed_classes();
            assert!(
                allowed.contains(&class),
                "{class:?} should contain itself in allowed_classes"
            );
        }
    }

    #[test]
    fn cost_class_allowed_counts() {
        assert_eq!(CostClass::Free.allowed_classes().len(), 1);
        assert_eq!(CostClass::VeryLow.allowed_classes().len(), 2);
        assert_eq!(CostClass::Low.allowed_classes().len(), 3);
        assert_eq!(CostClass::Medium.allowed_classes().len(), 4);
        assert_eq!(CostClass::High.allowed_classes().len(), 5);
        assert_eq!(CostClass::VeryHigh.allowed_classes().len(), 6);
    }

    #[test]
    fn cost_class_from_tier() {
        assert_eq!(CostClass::from_tier(CostTier::Minimal), CostClass::Low);
        assert_eq!(CostClass::from_tier(CostTier::Standard), CostClass::Medium);
        assert_eq!(CostClass::from_tier(CostTier::Premium), CostClass::VeryHigh);
    }

    #[test]
    fn task_complexity_min_quality() {
        assert!((TaskComplexity::Extraction.min_quality_hint() - 0.5).abs() < f64::EPSILON);
        assert!((TaskComplexity::Classification.min_quality_hint() - 0.6).abs() < f64::EPSILON);
        assert!((TaskComplexity::Reasoning.min_quality_hint() - 0.8).abs() < f64::EPSILON);
        assert!((TaskComplexity::Generation.min_quality_hint() - 0.7).abs() < f64::EPSILON);
    }

    #[test]
    fn task_complexity_requires_reasoning() {
        assert!(!TaskComplexity::Extraction.requires_reasoning());
        assert!(!TaskComplexity::Classification.requires_reasoning());
        assert!(TaskComplexity::Reasoning.requires_reasoning());
        assert!(!TaskComplexity::Generation.requires_reasoning());
    }

    #[test]
    fn agent_requirements_fast_cheap() {
        let r = AgentRequirements::fast_cheap();
        assert_eq!(r.max_cost_class, CostClass::VeryLow);
        assert_eq!(r.max_latency_ms, 2_000);
        assert!(!r.requires_reasoning);
    }

    #[test]
    fn agent_requirements_deep_research() {
        let r = AgentRequirements::deep_research();
        assert!(r.requires_reasoning);
        assert!(r.requires_web_search);
        assert!(r.min_quality >= 0.9);
    }

    #[test]
    fn agent_requirements_with_min_quality_clamped() {
        let r = AgentRequirements::fast_cheap().with_min_quality(2.0);
        assert!((r.min_quality - 1.0).abs() < f64::EPSILON);

        let r = AgentRequirements::fast_cheap().with_min_quality(-1.0);
        assert!(r.min_quality.abs() < f64::EPSILON);
    }

    #[test]
    fn provider_request_and_assignment_roundtrip() {
        let request = ProviderRequest {
            id: "req-1".to_string(),
            required_capabilities: vec![Capability::Reasoning, Capability::Scheduling],
        };
        let assignment = ProviderAssignment {
            request_id: request.id.clone(),
            assignments: vec![CapabilityAssignment {
                capability: Capability::Reasoning,
                backend_name: "solver-a".to_string(),
            }],
            unmatched: vec![Capability::Scheduling],
            coverage_ratio: 0.5,
        };

        let request_back: ProviderRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
        let assignment_back: ProviderAssignment =
            serde_json::from_str(&serde_json::to_string(&assignment).unwrap()).unwrap();

        assert_eq!(
            request_back.required_capabilities,
            request.required_capabilities
        );
        assert_eq!(assignment_back.assignments, assignment.assignments);
        assert_eq!(assignment_back.unmatched, assignment.unmatched);
    }
}
