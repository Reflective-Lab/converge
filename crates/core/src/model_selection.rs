// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Transitional re-exports for provider selection vocabulary.
//!
//! The canonical ownership of backend and model-selection vocabulary now lives in
//! `converge-provider-api`. `converge-core` re-exports these types during the
//! migration so existing downstreams can keep compiling while moving to the
//! narrower provider contract.

pub use converge_provider_api::selection::{
    AgentRequirements, BackendRequirements, BackendSelector, ComplianceLevel, CostClass, CostTier,
    DataSovereignty, Jurisdiction, LatencyClass, ModelSelectorTrait, RequiredCapabilities,
    SelectionCriteria, TaskComplexity,
};
