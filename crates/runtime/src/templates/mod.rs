// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Job template system for the Converge Runtime.
//!
//! Templates define reusable job configurations including agents, validation,
//! and invariants. The hybrid model allows users to:
//!
//! 1. Pick a template (e.g., "growth-strategy")
//! 2. Override specific parts (budget, agents, validation)
//! 3. Provide seed facts
//!
//! # Example
//!
//! ```ignore
//! let registry = TemplateRegistry::with_defaults();
//! let template = registry.get("growth-strategy").unwrap();
//!
//! // Template defines agents, validation, invariants
//! // User provides seeds and optional overrides
//! ```
//!
//! # Template Format
//!
//! Templates are defined in YAML:
//!
//! ```yaml
//! name: growth-strategy
//! description: Multi-agent growth strategy analysis
//!
//! budget:
//!   max_cycles: 50
//!   max_facts: 500
//!
//! agents:
//!   - name: MarketSignalAgent
//!     type: llm
//!     requirements: fast_extraction
//!     output_key: Signals
//!     depends_on: [Seeds]
//!
//! validation:
//!   min_confidence: 0.7
//!
//! invariants:
//!   - BrandSafetyInvariant
//! ```

mod registry;
mod types;
mod validator;

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
