// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Root Intent — The constitution of a Converge job.
//!
//! The Root Intent is the *only* entry point into a Converge runtime.
//! It defines:
//! - The universe of discourse (what can be reasoned about)
//! - What is allowed to happen (constraints)
//! - What success means (success criteria)
//!
//! # Design Philosophy
//!
//! A Root Intent is a **typed declaration**, not a prompt.
//! Nothing may override it during execution.
//!
//! # Example
//!
//! ```
//! use converge_core::root_intent::{
//!     RootIntent, IntentKind, Objective, Scope, Budgets,
//!     ScopeConstraint, IntentConstraint, SuccessCriterion,
//! };
//!
//! let intent = RootIntent::new(IntentKind::GrowthStrategy)
//!     .with_objective(Objective::IncreaseDemand)
//!     .with_scope(Scope::new()
//!         .with_constraint(ScopeConstraint::Market("Nordic B2B".into()))
//!         .with_constraint(ScopeConstraint::TimeWindow {
//!             start: None,
//!             end: None,
//!             description: "Next quarter".into(),
//!         }))
//!     .with_constraint(IntentConstraint::hard("budget_class", "Series A"))
//!     .with_constraint(IntentConstraint::soft("brand_safety", "Family friendly"))
//!     .with_success_criterion(SuccessCriterion::AtLeastOneViableStrategy)
//!     .with_budgets(Budgets::default());
//!
//! assert!(intent.validate().is_ok());
//! ```

use crate::context::{ContextFact, ContextKey, ContextState};
use crate::types::{ConstraintName, ConstraintValue, IntentId, Timestamp};
use std::time::Duration;

/// The class of problem being solved.
///
/// Used to:
/// - Select eligible agents
/// - Load domain constraints
/// - Choose appropriate invariants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IntentKind {
    /// Growth strategy development and market analysis.
    GrowthStrategy,
    /// Scheduling and resource allocation.
    Scheduling,
    /// Resource optimization problems.
    ResourceOptimization,
    /// Risk assessment and mitigation.
    RiskAssessment,
    /// Content generation and curation.
    ContentGeneration,
    /// Custom domain (extensibility point).
    Custom,
}

impl IntentKind {
    /// Returns the canonical name for this intent kind.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::GrowthStrategy => "growth_strategy",
            Self::Scheduling => "scheduling",
            Self::ResourceOptimization => "resource_optimization",
            Self::RiskAssessment => "risk_assessment",
            Self::ContentGeneration => "content_generation",
            Self::Custom => "custom",
        }
    }

    /// Returns suggested context keys for this intent kind.
    #[must_use]
    pub fn suggested_context_keys(&self) -> &'static [ContextKey] {
        match self {
            Self::GrowthStrategy => &[
                ContextKey::Seeds,
                ContextKey::Signals,
                ContextKey::Competitors,
                ContextKey::Strategies,
                ContextKey::Evaluations,
            ],
            Self::Scheduling | Self::ContentGeneration => &[
                ContextKey::Seeds,
                ContextKey::Constraints,
                ContextKey::Strategies,
            ],
            Self::ResourceOptimization => &[
                ContextKey::Seeds,
                ContextKey::Constraints,
                ContextKey::Strategies,
                ContextKey::Evaluations,
            ],
            Self::RiskAssessment => &[
                ContextKey::Seeds,
                ContextKey::Signals,
                ContextKey::Hypotheses,
                ContextKey::Evaluations,
            ],
            Self::Custom => &[ContextKey::Seeds],
        }
    }
}

/// What the system is trying to improve or achieve.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Objective {
    /// Increase demand or market reach.
    IncreaseDemand,
    /// Minimize time to completion.
    MinimizeTime,
    /// Maximize feasibility of solutions.
    MaximizeFeasibility,
    /// Minimize cost.
    MinimizeCost,
    /// Maximize coverage.
    MaximizeCoverage,
    /// Balance multiple objectives.
    Balance(Vec<Objective>),
    /// Custom objective with description.
    Custom(String),
}

impl Objective {
    /// Returns the canonical name for this objective.
    #[must_use]
    pub fn name(&self) -> String {
        match self {
            Self::IncreaseDemand => "increase_demand".into(),
            Self::MinimizeTime => "minimize_time".into(),
            Self::MaximizeFeasibility => "maximize_feasibility".into(),
            Self::MinimizeCost => "minimize_cost".into(),
            Self::MaximizeCoverage => "maximize_coverage".into(),
            Self::Balance(_) => "balanced".into(),
            Self::Custom(name) => name.clone(),
        }
    }
}

/// A constraint on the scope of the intent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScopeConstraint {
    /// Market segment (e.g., "Nordic B2B").
    Market(String),
    /// Geographic region (e.g., "EMEA").
    Geography(String),
    /// Product or service category.
    Product(String),
    /// Time window for the analysis.
    TimeWindow {
        start: Option<Timestamp>,
        end: Option<Timestamp>,
        description: String,
    },
    /// Customer segment.
    CustomerSegment(String),
    /// Custom scope constraint.
    Custom {
        key: ConstraintName,
        value: ConstraintValue,
    },
}

/// Defines what is in-bounds for the intent.
///
/// Nothing outside the scope may appear in context.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Scope {
    /// Scope constraints that define boundaries.
    constraints: Vec<ScopeConstraint>,
}

impl Scope {
    /// Creates an empty scope.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a scope constraint.
    #[must_use]
    pub fn with_constraint(mut self, constraint: ScopeConstraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Returns the scope constraints.
    #[must_use]
    pub fn constraints(&self) -> &[ScopeConstraint] {
        &self.constraints
    }

    /// Checks if the scope is defined (has at least one constraint).
    #[must_use]
    pub fn is_defined(&self) -> bool {
        !self.constraints.is_empty()
    }

    /// Validates that a fact is within scope.
    ///
    /// Returns true if the fact is allowed by the scope constraints.
    /// Default implementation is permissive; domain-specific validation
    /// should be implemented via invariants.
    #[must_use]
    pub fn allows(&self, _fact: &ContextFact) -> bool {
        // MVP: All facts are allowed. In production, this would check
        // fact content against scope constraints (e.g., market mentions,
        // geographic references, etc.)
        true
    }
}

/// Severity of an intent constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConstraintSeverity {
    /// Violation aborts convergence immediately.
    Hard,
    /// Violation is logged but doesn't abort.
    Soft,
}

/// A constraint on the intent execution.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IntentConstraint {
    /// Unique key for this constraint.
    pub key: ConstraintName,
    /// Human-readable description or value.
    pub value: ConstraintValue,
    /// Severity level.
    pub severity: ConstraintSeverity,
}

impl IntentConstraint {
    /// Creates a hard constraint (violation aborts).
    #[must_use]
    pub fn hard(key: impl Into<ConstraintName>, value: impl Into<ConstraintValue>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            severity: ConstraintSeverity::Hard,
        }
    }

    /// Creates a soft constraint (violation is logged).
    #[must_use]
    pub fn soft(key: impl Into<ConstraintName>, value: impl Into<ConstraintValue>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
            severity: ConstraintSeverity::Soft,
        }
    }
}

/// Defines when the job is considered successful.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SuccessCriterion {
    /// At least one viable strategy must exist.
    AtLeastOneViableStrategy,
    /// A valid schedule must be found.
    ValidScheduleFound,
    /// All tasks must be allocated.
    AllTasksAllocated,
    /// Minimum number of strategies.
    MinimumStrategies(usize),
    /// All evaluations must be positive.
    AllEvaluationsPositive,
    /// Custom criterion with description.
    Custom(String),
}

impl SuccessCriterion {
    /// Checks if this criterion is satisfied by the context.
    #[must_use]
    pub fn is_satisfied(&self, ctx: &ContextState) -> bool {
        match self {
            Self::AtLeastOneViableStrategy => {
                // Check for at least one strategy with a positive evaluation
                let strategies = ctx.get(ContextKey::Strategies);
                let evaluations = ctx.get(ContextKey::Evaluations);

                if strategies.is_empty() {
                    return false;
                }

                // If no evaluations, assume strategies are viable
                if evaluations.is_empty() {
                    return true;
                }

                // Check if any evaluation is positive
                evaluations.iter().any(|e| {
                    e.content().to_lowercase().contains("viable")
                        || e.content().to_lowercase().contains("positive")
                        || e.content().to_lowercase().contains("recommended")
                })
            }
            Self::ValidScheduleFound => ctx.has(ContextKey::Strategies),
            Self::AllTasksAllocated => {
                // Check constraints are satisfied (no unallocated tasks)
                !ctx.get(ContextKey::Constraints)
                    .iter()
                    .any(|c| c.content().to_lowercase().contains("unallocated"))
            }
            Self::MinimumStrategies(n) => ctx.get(ContextKey::Strategies).len() >= *n,
            Self::AllEvaluationsPositive => {
                let evaluations = ctx.get(ContextKey::Evaluations);
                !evaluations.is_empty()
                    && evaluations.iter().all(|e| {
                        !e.content().to_lowercase().contains("negative")
                            && !e.content().to_lowercase().contains("rejected")
                    })
            }
            Self::Custom(_) => {
                // Custom criteria need domain-specific validation
                // Default to true; use invariants for real checks
                true
            }
        }
    }
}

/// Success criteria collection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SuccessCriteria {
    /// Required criteria (all must pass).
    required: Vec<SuccessCriterion>,
    /// Optional criteria (logged but don't fail).
    optional: Vec<SuccessCriterion>,
}

impl SuccessCriteria {
    /// Creates empty success criteria.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a required criterion.
    #[must_use]
    pub fn require(mut self, criterion: SuccessCriterion) -> Self {
        self.required.push(criterion);
        self
    }

    /// Adds an optional criterion.
    #[must_use]
    pub fn prefer(mut self, criterion: SuccessCriterion) -> Self {
        self.optional.push(criterion);
        self
    }

    /// Checks if all required criteria are satisfied.
    #[must_use]
    pub fn is_satisfied(&self, ctx: &ContextState) -> bool {
        self.required.iter().all(|c| c.is_satisfied(ctx))
    }

    /// Returns unsatisfied required criteria.
    #[must_use]
    pub fn unsatisfied(&self, ctx: &ContextState) -> Vec<&SuccessCriterion> {
        self.required
            .iter()
            .filter(|c| !c.is_satisfied(ctx))
            .collect()
    }

    /// Returns unsatisfied optional criteria.
    #[must_use]
    pub fn unsatisfied_optional(&self, ctx: &ContextState) -> Vec<&SuccessCriterion> {
        self.optional
            .iter()
            .filter(|c| !c.is_satisfied(ctx))
            .collect()
    }

    /// Checks if criteria are explicitly defined.
    #[must_use]
    pub fn is_explicit(&self) -> bool {
        !self.required.is_empty()
    }
}

/// Execution budgets that guarantee termination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Budgets {
    /// Maximum execution cycles.
    pub max_cycles: u32,
    /// Maximum agents that can run per cycle.
    pub max_agents_per_cycle: Option<u32>,
    /// Maximum total facts in context.
    pub max_facts: u32,
    /// Time limit for the entire job.
    pub time_limit: Option<Duration>,
    /// Maximum LLM tokens to consume.
    pub max_tokens: Option<u64>,
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            max_cycles: 100,
            max_agents_per_cycle: None,
            max_facts: 10_000,
            time_limit: None,
            max_tokens: None,
        }
    }
}

impl Budgets {
    /// Creates budgets with custom cycle limit.
    #[must_use]
    pub fn with_max_cycles(mut self, max: u32) -> Self {
        self.max_cycles = max;
        self
    }

    /// Creates budgets with custom fact limit.
    #[must_use]
    pub fn with_max_facts(mut self, max: u32) -> Self {
        self.max_facts = max;
        self
    }

    /// Sets a time limit.
    #[must_use]
    pub fn with_time_limit(mut self, limit: Duration) -> Self {
        self.time_limit = Some(limit);
        self
    }

    /// Sets a token limit.
    #[must_use]
    pub fn with_max_tokens(mut self, max: u64) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Converts to engine Budget.
    #[must_use]
    pub fn to_engine_budget(&self) -> crate::engine::Budget {
        crate::engine::Budget {
            max_cycles: self.max_cycles,
            max_facts: self.max_facts,
        }
    }
}

/// Error when Root Intent validation fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentValidationError {
    /// What failed validation.
    pub field: String,
    /// Why it failed.
    pub reason: String,
}

impl std::fmt::Display for IntentValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid {}: {}", self.field, self.reason)
    }
}

impl std::error::Error for IntentValidationError {}

/// The Root Intent — constitution of a Converge job.
///
/// This is the *only* entry point into a Converge runtime.
/// It defines the universe of discourse, what is allowed,
/// and what success means.
#[derive(Debug, Clone)]
pub struct RootIntent {
    /// Unique identifier.
    pub id: IntentId,
    /// The class of problem.
    pub kind: IntentKind,
    /// What the system should optimize.
    pub objective: Option<Objective>,
    /// What is in-bounds.
    pub scope: Scope,
    /// Hard and soft constraints.
    pub constraints: Vec<IntentConstraint>,
    /// Success criteria.
    pub success_criteria: SuccessCriteria,
    /// Execution budgets.
    pub budgets: Budgets,
}

impl RootIntent {
    /// Creates a new Root Intent with the given kind.
    #[must_use]
    pub fn new(kind: IntentKind) -> Self {
        Self {
            id: IntentId::generate(),
            kind,
            objective: None,
            scope: Scope::new(),
            constraints: Vec::new(),
            success_criteria: SuccessCriteria::new(),
            budgets: Budgets::default(),
        }
    }

    /// Sets the intent ID.
    #[must_use]
    pub fn with_id(mut self, id: IntentId) -> Self {
        self.id = id;
        self
    }

    /// Sets the objective.
    #[must_use]
    pub fn with_objective(mut self, objective: Objective) -> Self {
        self.objective = Some(objective);
        self
    }

    /// Sets the scope.
    #[must_use]
    pub fn with_scope(mut self, scope: Scope) -> Self {
        self.scope = scope;
        self
    }

    /// Adds a constraint.
    #[must_use]
    pub fn with_constraint(mut self, constraint: IntentConstraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Adds a success criterion (required).
    #[must_use]
    pub fn with_success_criterion(mut self, criterion: SuccessCriterion) -> Self {
        self.success_criteria = self.success_criteria.require(criterion);
        self
    }

    /// Sets the success criteria.
    #[must_use]
    pub fn with_success_criteria(mut self, criteria: SuccessCriteria) -> Self {
        self.success_criteria = criteria;
        self
    }

    /// Sets the budgets.
    #[must_use]
    pub fn with_budgets(mut self, budgets: Budgets) -> Self {
        self.budgets = budgets;
        self
    }

    /// Validates the Root Intent.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Scope is not defined
    /// - Success criteria are not explicit
    pub fn validate(&self) -> Result<(), IntentValidationError> {
        // Scope must be defined
        if !self.scope.is_defined() {
            return Err(IntentValidationError {
                field: "scope".into(),
                reason: "scope must have at least one constraint".into(),
            });
        }

        // Success criteria must be explicit
        if !self.success_criteria.is_explicit() {
            return Err(IntentValidationError {
                field: "success_criteria".into(),
                reason: "at least one success criterion must be defined".into(),
            });
        }

        Ok(())
    }

    /// Creates initial seed facts from this intent.
    ///
    /// These facts are added to the context at the start of execution.
    #[must_use]
    pub fn to_seed_facts(&self) -> Vec<ContextFact> {
        let mut facts = Vec::new();

        // Intent metadata as seed
        facts.push(crate::context::new_fact(
            ContextKey::Seeds,
            format!("intent:{}", self.id),
            format!(
                "kind={} objective={}",
                self.kind.name(),
                self.objective
                    .as_ref()
                    .map_or("unspecified".to_string(), Objective::name)
            ),
        ));

        // Scope as seeds
        for (i, constraint) in self.scope.constraints().iter().enumerate() {
            let content = match constraint {
                ScopeConstraint::Market(m) => format!("market={m}"),
                ScopeConstraint::Geography(g) => format!("geography={g}"),
                ScopeConstraint::Product(p) => format!("product={p}"),
                ScopeConstraint::TimeWindow { description, .. } => {
                    format!("timewindow={description}")
                }
                ScopeConstraint::CustomerSegment(s) => format!("segment={s}"),
                ScopeConstraint::Custom { key, value } => format!("{key}={value}"),
            };
            facts.push(crate::context::new_fact(
                ContextKey::Seeds,
                format!("scope:{}:{i}", self.id),
                content,
            ));
        }

        // Hard constraints as constraint facts
        for constraint in &self.constraints {
            if constraint.severity == ConstraintSeverity::Hard {
                facts.push(crate::context::new_fact(
                    ContextKey::Constraints,
                    format!("constraint:{}:{}", self.id, constraint.key),
                    format!("{}={}", constraint.key, constraint.value),
                ));
            }
        }

        facts
    }

    /// Returns hard constraints.
    #[must_use]
    pub fn hard_constraints(&self) -> Vec<&IntentConstraint> {
        self.constraints
            .iter()
            .filter(|c| c.severity == ConstraintSeverity::Hard)
            .collect()
    }

    /// Returns soft constraints.
    #[must_use]
    pub fn soft_constraints(&self) -> Vec<&IntentConstraint> {
        self.constraints
            .iter()
            .filter(|c| c.severity == ConstraintSeverity::Soft)
            .collect()
    }

    /// Checks if success criteria are satisfied by the context.
    #[must_use]
    pub fn is_successful(&self, ctx: &ContextState) -> bool {
        self.success_criteria.is_satisfied(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intent_id_generates_unique_ids() {
        let id1 = IntentId::generate();
        let id2 = IntentId::generate();
        assert_ne!(id1, id2);
    }

    #[test]
    fn intent_kind_has_names() {
        assert_eq!(IntentKind::GrowthStrategy.name(), "growth_strategy");
        assert_eq!(IntentKind::Scheduling.name(), "scheduling");
    }

    #[test]
    fn intent_kind_suggests_context_keys() {
        let keys = IntentKind::GrowthStrategy.suggested_context_keys();
        assert!(keys.contains(&ContextKey::Strategies));
        assert!(keys.contains(&ContextKey::Competitors));
    }

    #[test]
    fn scope_tracks_constraints() {
        let scope = Scope::new()
            .with_constraint(ScopeConstraint::Market("Nordic B2B".into()))
            .with_constraint(ScopeConstraint::Geography("EMEA".into()));

        assert!(scope.is_defined());
        assert_eq!(scope.constraints().len(), 2);
    }

    #[test]
    fn intent_constraint_severities() {
        let hard = IntentConstraint::hard("budget", "1M");
        let soft = IntentConstraint::soft("brand", "friendly");

        assert_eq!(hard.severity, ConstraintSeverity::Hard);
        assert_eq!(soft.severity, ConstraintSeverity::Soft);
    }

    #[test]
    fn success_criteria_checks_satisfaction() {
        let mut ctx = ContextState::new();
        ctx.add_fact(crate::context::new_fact(
            ContextKey::Strategies,
            "strat-1",
            "growth strategy",
        ))
        .unwrap();
        ctx.add_fact(crate::context::new_fact(
            ContextKey::Evaluations,
            "eval-1",
            "viable and recommended",
        ))
        .unwrap();

        let criteria = SuccessCriteria::new().require(SuccessCriterion::AtLeastOneViableStrategy);

        assert!(criteria.is_satisfied(&ctx));
    }

    #[test]
    fn success_criteria_reports_unsatisfied() {
        let ctx = ContextState::new();
        let criteria = SuccessCriteria::new().require(SuccessCriterion::MinimumStrategies(2));

        assert!(!criteria.is_satisfied(&ctx));
        assert_eq!(criteria.unsatisfied(&ctx).len(), 1);
    }

    #[test]
    fn root_intent_validates_scope() {
        let intent = RootIntent::new(IntentKind::GrowthStrategy)
            .with_success_criterion(SuccessCriterion::AtLeastOneViableStrategy);

        let result = intent.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field, "scope");
    }

    #[test]
    fn root_intent_validates_success_criteria() {
        let intent = RootIntent::new(IntentKind::GrowthStrategy)
            .with_scope(Scope::new().with_constraint(ScopeConstraint::Market("B2B".into())));

        let result = intent.validate();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().field, "success_criteria");
    }

    #[test]
    fn root_intent_passes_validation() {
        let intent = RootIntent::new(IntentKind::GrowthStrategy)
            .with_objective(Objective::IncreaseDemand)
            .with_scope(Scope::new().with_constraint(ScopeConstraint::Market("B2B".into())))
            .with_success_criterion(SuccessCriterion::AtLeastOneViableStrategy);

        assert!(intent.validate().is_ok());
    }

    #[test]
    fn root_intent_generates_seed_facts() {
        let intent = RootIntent::new(IntentKind::GrowthStrategy)
            .with_id(IntentId::new("test-intent"))
            .with_objective(Objective::IncreaseDemand)
            .with_scope(
                Scope::new()
                    .with_constraint(ScopeConstraint::Market("Nordic".into()))
                    .with_constraint(ScopeConstraint::Geography("EMEA".into())),
            )
            .with_constraint(IntentConstraint::hard("budget", "1M"));

        let facts = intent.to_seed_facts();

        // Should have: 1 intent fact + 2 scope facts + 1 constraint fact
        assert_eq!(facts.len(), 4);

        // Check intent fact
        let intent_fact = facts
            .iter()
            .find(|f| f.id().as_str().starts_with("intent:"))
            .unwrap();
        assert!(intent_fact.content().contains("growth_strategy"));
        assert!(intent_fact.content().contains("increase_demand"));

        // Check constraint fact
        let constraint_fact = facts
            .iter()
            .find(|f| f.key() == ContextKey::Constraints)
            .unwrap();
        assert!(constraint_fact.content().contains("budget=1M"));
    }

    #[test]
    fn budgets_converts_to_engine_budget() {
        let budgets = Budgets::default().with_max_cycles(50).with_max_facts(5000);

        let engine_budget = budgets.to_engine_budget();
        assert_eq!(engine_budget.max_cycles, 50);
        assert_eq!(engine_budget.max_facts, 5000);
    }

    #[test]
    fn root_intent_checks_success() {
        let intent = RootIntent::new(IntentKind::GrowthStrategy)
            .with_success_criterion(SuccessCriterion::MinimumStrategies(1));

        let mut ctx = ContextState::new();
        assert!(!intent.is_successful(&ctx));

        ctx.add_fact(crate::context::new_fact(
            ContextKey::Strategies,
            "s1",
            "strategy",
        ))
        .unwrap();

        assert!(intent.is_successful(&ctx));
    }

    #[test]
    fn hard_and_soft_constraints_filtered() {
        let intent = RootIntent::new(IntentKind::GrowthStrategy)
            .with_constraint(IntentConstraint::hard("budget", "1M"))
            .with_constraint(IntentConstraint::soft("brand", "friendly"))
            .with_constraint(IntentConstraint::hard("compliance", "GDPR"));

        assert_eq!(intent.hard_constraints().len(), 2);
        assert_eq!(intent.soft_constraints().len(), 1);
    }
}
