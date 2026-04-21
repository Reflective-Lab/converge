// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Job template system for the Converge Runtime.
//!
//! Templates define reusable operational wiring for runtime packs.
//! Semantic rules stay in pack specs, not YAML. The runtime layer lets users:
//!
//! 1. Pick a template (e.g., "growth-strategy")
//! 2. Override operational parts (budget, seeds, provider preferences)
//! 3. Provide seed facts
//!
//! # Example
//!
//! ```ignore
//! let registry = TemplateRegistry::with_defaults();
//! let template = registry.get("growth-strategy").unwrap();
//!
//! // Template defines typed pack wiring
//! // User provides seeds and optional overrides
//! ```
//!
//! # Template Format
//!
//! Templates are defined in strict YAML wiring:
//!
//! ```yaml
//! name: growth-strategy
//! version: "1.0.0"
//! description: Multi-agent growth strategy analysis
//!
//! budget:
//!   max_cycles: 50
//!   max_facts: 500
//!
//! agents:
//!   - id: market_signal
//!     requirements: fast_extraction
//! ```
//!
//! Semantic keys such as `validation` and `invariants` are rejected at parse time.

mod registry;
mod types;
mod validator;

#[cfg(test)]
mod proptests;

pub use registry::TemplateRegistry;

#[allow(unused_imports)]
pub use registry::TemplateError;

// New types (preferred)
pub use types::{
    AgentOverrides, AgentWiring, BudgetConfig, CompatibilityRequirements, CustomRequirements,
    JobOverrides, PackConfig, PackJobRequest, PackSummary, ProviderPreferences, RequirementsConfig,
    SeedFact,
};

pub use types::{AgentDefinition, AgentType};

// Schema validation
pub use validator::{
    PackValidationError, allowed_keys, forbidden_keys, validate_pack_yaml, validate_pack_yaml_str,
};
