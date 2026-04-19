// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Resource Routing agents for task allocation and optimization.
//!
//! This module implements a deterministic resource routing use case
//! that validates the Converge engine with solver integration.
//!
//! # Suggestor Pipeline
//!
//! ```text
//! Seeds (tasks, resources)
//!    │
//!    ▼
//! TaskRetrievalAgent → Signals (task definitions)
//!    │
//!    ▼
//! ResourceRetrievalAgent → Signals (resource definitions)
//!    │
//!    ▼
//! ConstraintValidationAgent → Constraints (capacity, time windows)
//!    │
//!    ▼
//! SolverAgent → Strategies (candidate assignments)
//!    │
//!    ▼
//! FeasibilityAgent → Evaluations (valid assignments ranked)
//! ```ignore
//!
//! # Example
//!
//! ```
//! use converge_core::{Engine, Context, ContextKey};
//! use converge_core::suggestors::SeedSuggestor;
//! use converge_domain::resource_routing::{
//!     TaskRetrievalAgent, ResourceRetrievalAgent, ConstraintValidationAgent,
//!     SolverAgent, FeasibilityAgent,
//! };
//!
//! let mut engine = Engine::new();
//!
//! // Seed the context with tasks and resources
//! engine.register_suggestor(SeedSuggestor::new("tasks", "Delivery A, Delivery B, Delivery C"));
//! engine.register_suggestor(SeedSuggestor::new("resources", "Vehicle 1, Vehicle 2"));
//!
//! // Register resource routing agents
//! engine.register_suggestor(TaskRetrievalAgent);
//! engine.register_suggestor(ResourceRetrievalAgent);
//! engine.register_suggestor(ConstraintValidationAgent);
//! engine.register_suggestor(SolverAgent);
//! engine.register_suggestor(FeasibilityAgent);
//!
//! let result = engine.run(Context::new()).await.expect("should converge");
//!
//! assert!(result.converged);
//! assert!(result.context.has(ContextKey::Strategies));
//! assert!(result.context.has(ContextKey::Evaluations));
//! ```

// Suggestor trait returns &str, but we return literals. This is fine.
#![allow(clippy::unnecessary_literal_bound)]

use converge_core::{AgentEffect, ContextKey, Fact, Suggestor};

/// Suggestor that retrieves and structures task definitions.
///
///
/// Extracts tasks from seeds and creates structured task facts.
pub struct TaskRetrievalAgent;

#[async_trait::async_trait]
impl Suggestor for TaskRetrievalAgent {
    fn name(&self) -> &str {
        "TaskRetrievalAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::ContextView) -> bool {
        // Run once when seeds exist but no task signals yet
        let has_tasks_seed = ctx
            .get(ContextKey::Seeds)
            .iter()
            .any(|s| s.id == "tasks" || s.content.contains("task"));
        let has_task_signals = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("task:"));

        has_tasks_seed && !has_task_signals
    }

    async fn execute(&self, ctx: &dyn converge_core::ContextView) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);

        let mut facts = Vec::new();

        // Find tasks seed
        let tasks_seed = seeds.iter().find(|s| s.id == "tasks");

        if let Some(seed) = tasks_seed {
            // Parse tasks from content (simplified: comma-separated)
            let tasks: Vec<&str> = seed.content.split(',').map(str::trim).collect();

            for (i, task) in tasks.iter().enumerate() {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Signals,
                    format!("task:{}", i + 1),
                    format!(
                        "Task {}: {} | Priority: {} | Duration: {} min",
                        i + 1,
                        task,
                        if i == 0 { "High" } else { "Medium" },
                        (i + 1) * 30,
                    ),
                ));
            }
        } else {
            // Default tasks
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Signals,
                "task:1",
                "Task 1: Delivery A | Priority: High | Duration: 30 min",
            ));
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Signals,
                "task:2",
                "Task 2: Delivery B | Priority: Medium | Duration: 60 min",
            ));
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Suggestor that retrieves and structures resource definitions.
///
///
/// Extracts resources from seeds and creates structured resource facts.
pub struct ResourceRetrievalAgent;

#[async_trait::async_trait]
impl Suggestor for ResourceRetrievalAgent {
    fn name(&self) -> &str {
        "ResourceRetrievalAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::ContextView) -> bool {
        // Run once when seeds exist but no resource signals yet
        let has_resources_seed = ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.id == "resources" || s.content.contains("resource") || s.content.contains("vehicle")
        });
        let has_resource_signals = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("resource:"));

        has_resources_seed && !has_resource_signals
    }

    async fn execute(&self, ctx: &dyn converge_core::ContextView) -> AgentEffect {
        let seeds = ctx.get(ContextKey::Seeds);

        let mut facts = Vec::new();

        // Find resources seed
        let resources_seed = seeds.iter().find(|s| {
            s.id == "resources" || s.content.contains("resource") || s.content.contains("vehicle")
        });

        if let Some(seed) = resources_seed {
            // Parse resources from content (simplified: comma-separated)
            let resources: Vec<&str> = seed.content.split(',').map(str::trim).collect();

            for (i, resource) in resources.iter().enumerate() {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Signals,
                    format!("resource:{}", i + 1),
                    format!(
                        "Resource {}: {} | Capacity: {} tasks | Status: Available",
                        i + 1,
                        resource,
                        if i == 0 { 3 } else { 2 },
                    ),
                ));
            }
        } else {
            // Default resources
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Signals,
                "resource:1",
                "Resource 1: Vehicle 1 | Capacity: 3 tasks | Status: Available",
            ));
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Signals,
                "resource:2",
                "Resource 2: Vehicle 2 | Capacity: 2 tasks | Status: Available",
            ));
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Suggestor that validates constraints (capacity, time windows, etc.).
///
///
/// Creates constraint facts based on tasks and resources.
pub struct ConstraintValidationAgent;

#[async_trait::async_trait]
impl Suggestor for ConstraintValidationAgent {
    fn name(&self) -> &str {
        "ConstraintValidationAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_core::ContextView) -> bool {
        // Run when we have both tasks and resources but no constraints yet
        let has_tasks = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("task:"));
        let has_resources = ctx
            .get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with("resource:"));
        let has_constraints = ctx.has(ContextKey::Constraints);

        has_tasks && has_resources && !has_constraints
    }

    async fn execute(&self, ctx: &dyn converge_core::ContextView) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);

        let mut facts = Vec::new();

        // Count tasks and resources
        let task_count = signals.iter().filter(|s| s.id.starts_with("task:")).count();
        let resource_count = signals
            .iter()
            .filter(|s| s.id.starts_with("resource:"))
            .count();

        // Define capacity constraints
        facts.push(crate::proposal(
            self.name(),
            ContextKey::Constraints,
            "constraint:capacity",
            format!(
                "Capacity constraint: {task_count} tasks must be assigned to {resource_count} resources"
            ),
        ));

        // Define objective
        facts.push(crate::proposal(
            self.name(),
            ContextKey::Constraints,
            "constraint:objective",
            "Objective: Minimize total delivery time",
        ));

        // Define feasibility requirement
        facts.push(crate::proposal(
            self.name(),
            ContextKey::Constraints,
            "constraint:feasibility",
            "All tasks must be assigned | No resource exceeds capacity",
        ));

        AgentEffect::with_proposals(facts)
    }
}

/// Suggestor that performs deterministic optimization (solver).
///
///
/// Generates candidate assignments using a simple greedy algorithm.
/// In a real system, this would integrate with a proper solver library.
pub struct SolverAgent;

#[async_trait::async_trait]
impl Suggestor for SolverAgent {
    fn name(&self) -> &str {
        "SolverAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Constraints, ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_core::ContextView) -> bool {
        // Run when constraints exist but no assignment strategies yet
        ctx.has(ContextKey::Constraints) && !ctx.has(ContextKey::Strategies)
    }

    async fn execute(&self, ctx: &dyn converge_core::ContextView) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let _constraints = ctx.get(ContextKey::Constraints);

        let mut facts = Vec::new();

        // Extract tasks and resources
        let tasks: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("task:"))
            .collect();
        let resources: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("resource:"))
            .collect();

        // Simple greedy assignment: assign tasks to resources in order
        // respecting capacity constraints
        let mut resource_loads = vec![0; resources.len()];
        let mut assignment_id = 1;

        for task in &tasks {
            // Find resource with lowest load that has capacity
            let mut best_resource = None;
            let mut best_load = usize::MAX;

            for (res_idx, resource) in resources.iter().enumerate() {
                // Extract capacity from resource content
                let capacity = resource
                    .content
                    .split("Capacity: ")
                    .nth(1)
                    .and_then(|s| s.split_whitespace().next())
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(2);

                if resource_loads[res_idx] < capacity && resource_loads[res_idx] < best_load {
                    best_load = resource_loads[res_idx];
                    best_resource = Some(res_idx);
                }
            }

            if let Some(res_idx) = best_resource {
                resource_loads[res_idx] += 1;
                let resource_id = resources[res_idx]
                    .id
                    .strip_prefix("resource:")
                    .unwrap_or("unknown");

                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Strategies,
                    format!("assignment:{assignment_id}"),
                    format!(
                        "Assignment {}: {} → {} | Load: {}/{}",
                        assignment_id,
                        task.id.strip_prefix("task:").unwrap_or("unknown"),
                        resource_id,
                        resource_loads[res_idx],
                        resources[res_idx]
                            .content
                            .split("Capacity: ")
                            .nth(1)
                            .and_then(|s| s.split_whitespace().next())
                            .and_then(|s| s.parse::<usize>().ok())
                            .unwrap_or(2)
                    ),
                ));
                assignment_id += 1;
            }
        }

        // If no assignments were made, create a fallback
        if facts.is_empty() {
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Strategies,
                "assignment:infeasible",
                "Assignment: INFEASIBLE | Reason: Insufficient capacity",
            ));
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Suggestor that validates feasibility and ranks assignments.
///
///
/// Evaluates assignments against constraints and ranks them.
pub struct FeasibilityAgent;

#[async_trait::async_trait]
impl Suggestor for FeasibilityAgent {
    fn name(&self) -> &str {
        "FeasibilityAgent"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Strategies, ContextKey::Constraints]
    }

    fn accepts(&self, ctx: &dyn converge_core::ContextView) -> bool {
        // Run when assignments exist but no evaluations yet
        ctx.has(ContextKey::Strategies) && !ctx.has(ContextKey::Evaluations)
    }

    async fn execute(&self, ctx: &dyn converge_core::ContextView) -> AgentEffect {
        let strategies = ctx.get(ContextKey::Strategies);
        let _constraints = ctx.get(ContextKey::Constraints);
        let signals = ctx.get(ContextKey::Signals);

        let mut facts = Vec::new();

        // Count tasks and check if all are assigned
        let task_count = signals.iter().filter(|s| s.id.starts_with("task:")).count();
        let assignment_count = strategies
            .iter()
            .filter(|s| !s.content.contains("INFEASIBLE"))
            .count();

        // Check feasibility
        let is_feasible = assignment_count >= task_count;
        let all_tasks_assigned = assignment_count == task_count;

        if is_feasible {
            // Evaluate each assignment
            for (i, assignment) in strategies.iter().enumerate() {
                if assignment.content.contains("INFEASIBLE") {
                    continue;
                }

                let (score, rationale) = evaluate_assignment(assignment, i, all_tasks_assigned);

                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Evaluations,
                    format!(
                        "eval:{}",
                        assignment
                            .id
                            .strip_prefix("assignment:")
                            .unwrap_or(&assignment.id)
                    ),
                    format!(
                        "Score: {}/100 | {} | Rationale: {}",
                        score,
                        if i == 0 && all_tasks_assigned {
                            "FEASIBLE"
                        } else {
                            "PARTIAL"
                        },
                        rationale
                    ),
                ));
            }
        } else {
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Evaluations,
                "eval:infeasible",
                format!(
                    "Score: 0/100 | INFEASIBLE | Rationale: Only {assignment_count}/{task_count} tasks assigned"
                ),
            ));
        }

        // Ensure at least one evaluation
        if facts.is_empty() {
            facts.push(crate::proposal(
                self.name(),
                ContextKey::Evaluations,
                "eval:unknown",
                "Score: 0/100 | UNKNOWN | Rationale: Unable to evaluate assignments",
            ));
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Deterministic assignment evaluation function.
fn evaluate_assignment(assignment: &Fact, _rank: usize, all_assigned: bool) -> (u32, &'static str) {
    let content = &assignment.content;

    if !all_assigned {
        return (50, "Partial assignment, not all tasks assigned");
    }

    // Prefer balanced load distribution
    if content.contains("Load: 1/") || content.contains("Load: 2/") {
        (95, "Optimal assignment with balanced resource utilization")
    } else if content.contains("Load: 3/") {
        (85, "Good assignment, resource fully utilized")
    } else {
        (75, "Valid assignment within capacity constraints")
    }
}

// =============================================================================
// RESOURCE ROUTING INVARIANTS
// =============================================================================

use converge_core::{Invariant, InvariantClass, InvariantResult, Violation};

/// Acceptance invariant: All tasks must be assigned.
///
/// From Gherkin spec:
/// ```gherkin
/// Scenario: Feasible and optimal allocation
///   When the system converges
///   Then all tasks are assigned to resources
/// ```
pub struct RequireAllTasksAssigned;

impl Invariant for RequireAllTasksAssigned {
    fn name(&self) -> &str {
        "require_all_tasks_assigned"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Acceptance
    }

    fn check(&self, ctx: &dyn converge_core::ContextView) -> InvariantResult {
        let signals = ctx.get(ContextKey::Signals);
        let strategies = ctx.get(ContextKey::Strategies);

        let task_count = signals.iter().filter(|s| s.id.starts_with("task:")).count();
        let assignment_count = strategies
            .iter()
            .filter(|s| !s.content.contains("INFEASIBLE"))
            .count();

        if assignment_count < task_count {
            return InvariantResult::Violated(Violation::new(format!(
                "only {assignment_count}/{task_count} tasks assigned"
            )));
        }
        InvariantResult::Ok
    }
}

/// Semantic invariant: No resource exceeds capacity.
///
/// From Gherkin spec:
/// ```gherkin
/// Scenario: Capacity constraints
///   When the system converges
///   Then no resource exceeds its capacity
/// ```
pub struct RequireCapacityRespected;

impl Invariant for RequireCapacityRespected {
    fn name(&self) -> &str {
        "require_capacity_respected"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &dyn converge_core::ContextView) -> InvariantResult {
        let signals = ctx.get(ContextKey::Signals);
        let strategies = ctx.get(ContextKey::Strategies);

        // Extract resource capacities
        let resources: Vec<_> = signals
            .iter()
            .filter(|s| s.id.starts_with("resource:"))
            .collect();

        let mut resource_loads: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        // Count assignments per resource
        for assignment in strategies
            .iter()
            .filter(|s| !s.content.contains("INFEASIBLE"))
        {
            // Extract resource from assignment content
            if let Some(resource_part) = assignment.content.split("→").nth(1) {
                let resource_id = resource_part.split('|').next().unwrap_or("").trim();
                *resource_loads.entry(resource_id.to_string()).or_insert(0) += 1;
            }
        }

        // Check each resource's capacity
        for resource in resources {
            let resource_id = resource.id.strip_prefix("resource:").unwrap_or("unknown");
            let capacity = resource
                .content
                .split("Capacity: ")
                .nth(1)
                .and_then(|s| s.split_whitespace().next())
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(0);

            let load = resource_loads.get(resource_id).copied().unwrap_or(0);

            if load > capacity {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("resource {resource_id} exceeds capacity: {load}/{capacity}"),
                    vec![resource.id.clone()],
                ));
            }
        }

        InvariantResult::Ok
    }
}

/// Structural invariant: Valid task and resource definitions.
///
/// From Gherkin spec:
/// ```gherkin
/// Scenario: Valid definitions
///   Given tasks and resources are defined
///   Then task and resource definitions are valid
/// ```
pub struct RequireValidDefinitions;

impl Invariant for RequireValidDefinitions {
    fn name(&self) -> &str {
        "require_valid_definitions"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &dyn converge_core::ContextView) -> InvariantResult {
        let signals = ctx.get(ContextKey::Signals);

        // Only check when signals exist (pipeline has started producing output)
        if signals.is_empty() {
            return InvariantResult::Ok;
        }

        let task_count = signals.iter().filter(|s| s.id.starts_with("task:")).count();
        let resource_count = signals
            .iter()
            .filter(|s| s.id.starts_with("resource:"))
            .count();

        if task_count == 0 {
            return InvariantResult::Violated(Violation::new("no tasks defined"));
        }

        if resource_count == 0 {
            return InvariantResult::Violated(Violation::new("no resources defined"));
        }

        InvariantResult::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::Context;
    use converge_core::Engine;
    use converge_core::suggestors::SeedSuggestor;

    #[tokio::test]
    async fn task_retrieval_agent_extracts_tasks() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new(
            "tasks",
            "Delivery A, Delivery B, Delivery C",
        ));
        engine.register_suggestor(TaskRetrievalAgent);

        let result = engine.run(Context::new()).await.expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Signals));

        let signals = result.context.get(ContextKey::Signals);
        assert!(signals.iter().any(|s| s.id.starts_with("task:")));
    }

    #[tokio::test]
    async fn resource_retrieval_agent_extracts_resources() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("resources", "Vehicle 1, Vehicle 2"));
        engine.register_suggestor(ResourceRetrievalAgent);

        let result = engine.run(Context::new()).await.expect("should converge");

        assert!(result.converged);
        let signals = result.context.get(ContextKey::Signals);
        assert!(signals.iter().any(|s| s.id.starts_with("resource:")));
    }

    #[tokio::test]
    async fn constraint_validation_agent_creates_constraints() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("tasks", "Delivery A, Delivery B"));
        engine.register_suggestor(SeedSuggestor::new("resources", "Vehicle 1"));
        engine.register_suggestor(TaskRetrievalAgent);
        engine.register_suggestor(ResourceRetrievalAgent);
        engine.register_suggestor(ConstraintValidationAgent);

        let result = engine.run(Context::new()).await.expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Constraints));
    }

    #[tokio::test]
    async fn solver_agent_generates_assignments() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("tasks", "Delivery A, Delivery B"));
        engine.register_suggestor(SeedSuggestor::new("resources", "Vehicle 1, Vehicle 2"));
        engine.register_suggestor(TaskRetrievalAgent);
        engine.register_suggestor(ResourceRetrievalAgent);
        engine.register_suggestor(ConstraintValidationAgent);
        engine.register_suggestor(SolverAgent);

        let result = engine.run(Context::new()).await.expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Strategies));

        let assignments = result.context.get(ContextKey::Strategies);
        assert!(!assignments.is_empty());
    }

    #[tokio::test]
    async fn feasibility_agent_evaluates_assignments() {
        let mut engine = Engine::new();
        engine.register_suggestor(SeedSuggestor::new("tasks", "Delivery A, Delivery B"));
        engine.register_suggestor(SeedSuggestor::new("resources", "Vehicle 1, Vehicle 2"));
        engine.register_suggestor(TaskRetrievalAgent);
        engine.register_suggestor(ResourceRetrievalAgent);
        engine.register_suggestor(ConstraintValidationAgent);
        engine.register_suggestor(SolverAgent);
        engine.register_suggestor(FeasibilityAgent);

        let result = engine.run(Context::new()).await.expect("should converge");

        assert!(result.converged);
        assert!(result.context.has(ContextKey::Evaluations));

        let evals = result.context.get(ContextKey::Evaluations);
        assert!(!evals.is_empty());
    }

    #[tokio::test]
    async fn full_pipeline_converges_deterministically() {
        let run = || async {
            let mut engine = Engine::new();
            engine.register_suggestor(SeedSuggestor::new(
                "tasks",
                "Delivery A, Delivery B, Delivery C",
            ));
            engine.register_suggestor(SeedSuggestor::new("resources", "Vehicle 1, Vehicle 2"));
            engine.register_suggestor(TaskRetrievalAgent);
            engine.register_suggestor(ResourceRetrievalAgent);
            engine.register_suggestor(ConstraintValidationAgent);
            engine.register_suggestor(SolverAgent);
            engine.register_suggestor(FeasibilityAgent);
            engine.run(Context::new()).await.expect("should converge")
        };

        let r1 = run().await;
        let r2 = run().await;

        // Same number of cycles
        assert_eq!(r1.cycles, r2.cycles);

        // Same assignments (compare content, ignoring timestamps that may differ across runs)
        let content = |facts: &[converge_core::Fact]| -> Vec<(String, String)> {
            facts
                .iter()
                .map(|f| (f.id.clone(), f.content.clone()))
                .collect()
        };
        assert_eq!(
            content(r1.context.get(ContextKey::Strategies)),
            content(r2.context.get(ContextKey::Strategies))
        );

        // Same evaluations
        assert_eq!(
            content(r1.context.get(ContextKey::Evaluations)),
            content(r2.context.get(ContextKey::Evaluations))
        );
    }
}
