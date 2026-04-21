// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Intent types using typed-builder pattern.
//!
//! Per CONTEXT.md:
//! - IntentBuilder -> RootIntent (assembled from request, job-to-be-done, risk posture)
//! - RootIntent contains `success_criteria: Vec<Criterion>` field
//!
//! This module provides a modern TypesRootIntent using typed-builder.
//! The existing root_intent.rs RootIntent remains for backward compatibility.

use serde::{Deserialize, Serialize};
use typed_builder::TypedBuilder;

use super::frame::{ConstraintSeverity, Criterion, IntentId};
use super::id::{ConstraintName, ConstraintValue, PackId, Timestamp};

// ============================================================================
// TypesIntentKind - What problem class this intent addresses
// ============================================================================

/// Kind of intent (what problem class).
///
/// Uses "Types" prefix to avoid collision with existing IntentKind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TypesIntentKind {
    /// Growth strategy development.
    GrowthStrategy,
    /// Scheduling optimization.
    Scheduling,
    /// Resource optimization.
    ResourceOptimization,
    /// Risk assessment.
    RiskAssessment,
    /// Content generation.
    ContentGeneration,
    /// Custom intent kind.
    Custom,
}

impl Default for TypesIntentKind {
    fn default() -> Self {
        Self::Custom
    }
}

// ============================================================================
// TypesObjective - What the intent is trying to achieve
// ============================================================================

/// Objective of the intent.
///
/// Uses "Types" prefix to avoid collision with existing Objective.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum TypesObjective {
    /// Increase demand/growth.
    IncreaseDemand,
    /// Minimize time to completion.
    MinimizeTime,
    /// Maximize feasibility.
    MaximizeFeasibility,
    /// Minimize cost.
    MinimizeCost,
    /// Maximize coverage.
    MaximizeCoverage,
    /// Balance multiple objectives.
    Balance(Vec<TypesObjective>),
    /// Custom objective.
    Custom(String),
}

impl Default for TypesObjective {
    fn default() -> Self {
        Self::Custom("unspecified".into())
    }
}

// ============================================================================
// RiskPosture - How aggressive/conservative the intent execution should be
// ============================================================================

/// Risk posture for the intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskPosture {
    /// Conservative - minimize risk, prefer safe options.
    Conservative,
    /// Balanced - accept moderate risk for moderate reward.
    Balanced,
    /// Aggressive - accept higher risk for higher potential reward.
    Aggressive,
}

impl Default for RiskPosture {
    fn default() -> Self {
        Self::Balanced
    }
}

// ============================================================================
// TypesIntentConstraint - Constraint on intent execution
// ============================================================================

/// Constraint on intent execution.
///
/// Uses "Types" prefix to avoid collision with existing IntentConstraint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypesIntentConstraint {
    /// Constraint key.
    pub key: ConstraintName,
    /// Constraint value.
    pub value: ConstraintValue,
    /// How violations are handled.
    pub severity: ConstraintSeverity,
}

impl TypesIntentConstraint {
    /// Create a new constraint.
    pub fn new(
        key: impl Into<ConstraintName>,
        value: impl Into<ConstraintValue>,
        severity: ConstraintSeverity,
    ) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            severity,
        }
    }

    /// Create a hard constraint.
    pub fn hard(key: impl Into<ConstraintName>, value: impl Into<ConstraintValue>) -> Self {
        Self::new(key, value, ConstraintSeverity::Hard)
    }

    /// Create a soft constraint.
    pub fn soft(key: impl Into<ConstraintName>, value: impl Into<ConstraintValue>) -> Self {
        Self::new(key, value, ConstraintSeverity::Soft)
    }
}

// ============================================================================
// TypesBudgets - Execution budgets
// ============================================================================

/// Execution budgets.
///
/// Uses "Types" prefix to avoid collision with existing Budgets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypesBudgets {
    /// Maximum convergence cycles.
    pub max_cycles: u32,
    /// Maximum facts that can be created.
    pub max_facts: u32,
    /// Maximum tokens (for LLM operations).
    pub max_tokens: Option<u64>,
    /// Time limit in milliseconds.
    pub time_limit_ms: Option<u64>,
}

impl Default for TypesBudgets {
    fn default() -> Self {
        Self {
            max_cycles: 100,
            max_facts: 10_000,
            max_tokens: None,
            time_limit_ms: None,
        }
    }
}

impl TypesBudgets {
    /// Create budgets with custom cycle limit.
    pub fn with_cycles(max_cycles: u32) -> Self {
        Self {
            max_cycles,
            ..Default::default()
        }
    }

    /// Set max tokens.
    pub fn with_max_tokens(mut self, max_tokens: u64) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set time limit.
    pub fn with_time_limit(mut self, time_limit_ms: u64) -> Self {
        self.time_limit_ms = Some(time_limit_ms);
        self
    }

    /// Convert to the engine-local budget representation.
    pub fn to_engine_budget(&self) -> crate::engine::Budget {
        crate::engine::Budget {
            max_cycles: self.max_cycles,
            max_facts: self.max_facts,
        }
    }
}

// ============================================================================
// TypesRootIntent - Root intent using builder pattern
// ============================================================================

/// Root Intent using builder pattern.
///
/// This is a modern version using typed-builder. The existing root_intent.rs
/// RootIntent remains for backward compatibility.
///
/// # Example
///
/// ```
/// use converge_core::types::{
///     TypesRootIntent, TypesIntentKind, TypesObjective, RiskPosture,
///     TypesBudgets, Criterion,
/// };
/// use converge_core::types::IntentId;
///
/// let intent = TypesRootIntent::builder()
///     .id(IntentId::new("intent-1"))
///     .kind(TypesIntentKind::GrowthStrategy)
///     .request("Find growth opportunities in Nordic B2B")
///     .objective(Some(TypesObjective::IncreaseDemand))
///     .risk_posture(RiskPosture::Balanced)
///     .success_criteria(vec![
///         Criterion::required("roi", "Achieve 15% ROI"),
///     ])
///     .build();
///
/// assert_eq!(intent.id.as_str(), "intent-1");
/// ```
#[derive(Debug, Clone, TypedBuilder, Serialize, Deserialize)]
pub struct TypesRootIntent {
    /// Unique identifier.
    pub id: IntentId,
    /// What kind of problem this addresses.
    pub kind: TypesIntentKind,
    /// The "job to be done" - the request from the user.
    #[builder(setter(into))]
    pub request: String,
    /// Objective (what to optimize for).
    #[builder(default)]
    pub objective: Option<TypesObjective>,
    /// Risk posture for execution.
    #[builder(default)]
    pub risk_posture: RiskPosture,
    /// Constraints on execution.
    #[builder(default)]
    pub constraints: Vec<TypesIntentConstraint>,
    /// Which packs should participate in this run.
    #[builder(default)]
    pub active_packs: Vec<PackId>,
    /// Success criteria (per CONTEXT.md: RootIntent contains success_criteria).
    #[builder(default)]
    pub success_criteria: Vec<Criterion>,
    /// Execution budgets.
    #[builder(default)]
    pub budgets: TypesBudgets,
    /// When the intent was created.
    #[builder(default)]
    pub created_at: Option<Timestamp>,
}

impl TypesRootIntent {
    /// Check if this intent has required success criteria.
    pub fn has_required_criteria(&self) -> bool {
        self.success_criteria.iter().any(|c| c.required)
    }

    /// Check if all budgets are within limits.
    pub fn is_within_budgets(&self, cycles: u32, facts: u32) -> bool {
        cycles <= self.budgets.max_cycles && facts <= self.budgets.max_facts
    }

    /// Get hard constraints only.
    pub fn hard_constraints(&self) -> impl Iterator<Item = &TypesIntentConstraint> {
        self.constraints
            .iter()
            .filter(|c| c.severity == ConstraintSeverity::Hard)
    }

    /// Check whether a pack is active for this intent.
    pub fn activates_pack(&self, pack_id: &PackId) -> bool {
        self.active_packs.iter().any(|active| active == pack_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_kind_default() {
        let kind = TypesIntentKind::default();
        assert!(matches!(kind, TypesIntentKind::Custom));
    }

    #[test]
    fn risk_posture_default() {
        let posture = RiskPosture::default();
        assert_eq!(posture, RiskPosture::Balanced);
    }

    #[test]
    fn budgets_default() {
        let budgets = TypesBudgets::default();
        assert_eq!(budgets.max_cycles, 100);
        assert_eq!(budgets.max_facts, 10_000);
        assert!(budgets.max_tokens.is_none());
    }

    #[test]
    fn budgets_builder() {
        let budgets = TypesBudgets::with_cycles(50)
            .with_max_tokens(100_000)
            .with_time_limit(30_000);

        assert_eq!(budgets.max_cycles, 50);
        assert_eq!(budgets.max_tokens, Some(100_000));
        assert_eq!(budgets.time_limit_ms, Some(30_000));
    }

    #[test]
    fn constraint_helpers() {
        let hard = TypesIntentConstraint::hard("budget", "1000000");
        assert_eq!(hard.severity, ConstraintSeverity::Hard);

        let soft = TypesIntentConstraint::soft("preference", "local_vendors");
        assert_eq!(soft.severity, ConstraintSeverity::Soft);
    }

    #[test]
    fn root_intent_builder() {
        let intent = TypesRootIntent::builder()
            .id(IntentId::new("intent-1"))
            .kind(TypesIntentKind::GrowthStrategy)
            .request("Find growth opportunities")
            .objective(Some(TypesObjective::IncreaseDemand))
            .risk_posture(RiskPosture::Conservative)
            .active_packs(vec!["growth-pack".into()])
            .success_criteria(vec![Criterion::required("growth", "10% growth")])
            .build();

        assert_eq!(intent.id.as_str(), "intent-1");
        assert!(matches!(intent.kind, TypesIntentKind::GrowthStrategy));
        assert!(matches!(
            intent.objective,
            Some(TypesObjective::IncreaseDemand)
        ));
        assert_eq!(intent.risk_posture, RiskPosture::Conservative);
        assert!(intent.activates_pack(&PackId::new("growth-pack")));
        assert!(intent.has_required_criteria());
    }

    #[test]
    fn root_intent_budget_check() {
        let intent = TypesRootIntent::builder()
            .id(IntentId::new("intent-1"))
            .kind(TypesIntentKind::Custom)
            .request("Test")
            .budgets(TypesBudgets::with_cycles(10))
            .build();

        assert!(intent.is_within_budgets(5, 100));
        assert!(!intent.is_within_budgets(15, 100));
    }

    #[test]
    fn root_intent_hard_constraints() {
        let intent = TypesRootIntent::builder()
            .id(IntentId::new("intent-1"))
            .kind(TypesIntentKind::Custom)
            .request("Test")
            .constraints(vec![
                TypesIntentConstraint::hard("budget", "1000"),
                TypesIntentConstraint::soft("region", "EU"),
            ])
            .build();

        assert_eq!(intent.hard_constraints().count(), 1);
    }

    #[test]
    fn root_intent_serialization() {
        let intent = TypesRootIntent::builder()
            .id(IntentId::new("intent-1"))
            .kind(TypesIntentKind::Scheduling)
            .request("Schedule meetings")
            .build();

        let json = serde_json::to_string(&intent).unwrap();
        assert!(json.contains("\"id\":\"intent-1\""));
        assert!(json.contains("\"kind\":\"Scheduling\""));

        let deserialized: TypesRootIntent = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id.as_str(), "intent-1");
    }

    #[test]
    fn objective_balance() {
        let balanced = TypesObjective::Balance(vec![
            TypesObjective::MinimizeCost,
            TypesObjective::MaximizeCoverage,
        ]);

        assert!(matches!(balanced, TypesObjective::Balance(_)));
    }
}
