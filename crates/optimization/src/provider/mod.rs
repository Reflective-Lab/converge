//! Converge platform integration
//!
//! This module provides the interface for using optimization
//! algorithms as converge-provider capabilities.
//!
//! ## Usage in Converge
//!
//! ```rust,ignore
//! use converge_optimization::provider::{OptimizationProvider, OptimizationType};
//!
//! // Register as a capability
//! let provider = OptimizationProvider::new(OptimizationType::Assignment);
//! capability_registry.register("optimize.assignment", provider);
//! ```
//!
//! ## Gate-Based Solving
//!
//! For domain-specific optimization with invariants and promotion gates:
//!
//! ```rust,ignore
//! use converge_optimization::provider::GateProvider;
//! use converge_pack::gate::ProblemSpec;
//!
//! let provider = GateProvider::new();
//! let result = provider.solve("meeting-scheduler", &spec)?;
//! assert!(result.gate.is_promoted());
//! ```

use crate::{
    SolverParams,
    assignment::{self, AssignmentProblem},
    graph::dijkstra,
    knapsack::{self, KnapsackProblem},
    packs::PackRegistry,
};
use converge_pack::InvariantResult;
use converge_pack::gate::{ProblemSpec, PromotionGate, ProposedPlan, SolverReport};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Types of optimization available
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptimizationType {
    /// Linear assignment problem
    Assignment,
    /// Knapsack problem
    Knapsack,
    /// Shortest path
    ShortestPath,
    /// Max flow
    MaxFlow,
    /// Min cost flow
    MinCostFlow,
    /// Set cover
    SetCover,
    /// Scheduling
    Scheduling,
    /// Vehicle routing (requires FFI)
    VehicleRouting,
    /// Constraint programming (requires FFI)
    ConstraintProgramming,
}

/// Request for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OptimizationRequest {
    /// Assignment problem
    Assignment {
        /// Cost matrix
        costs: Vec<Vec<i64>>,
    },
    /// Knapsack problem
    Knapsack {
        /// Item weights
        weights: Vec<i64>,
        /// Item values
        values: Vec<i64>,
        /// Capacity
        capacity: i64,
    },
    /// Shortest path problem
    ShortestPath {
        /// Edges as (from, to, cost)
        edges: Vec<(usize, usize, i64)>,
        /// Source node
        source: usize,
        /// Target node
        target: usize,
    },
}

/// Response from optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OptimizationResponse {
    /// Assignment solution
    Assignment {
        /// Assignment: assignments[agent] = task
        assignments: Vec<usize>,
        /// Total cost
        total_cost: i64,
    },
    /// Knapsack solution
    Knapsack {
        /// Selected items
        selected: Vec<usize>,
        /// Total value
        total_value: i64,
        /// Total weight
        total_weight: i64,
    },
    /// Shortest path solution
    ShortestPath {
        /// Path nodes
        path: Vec<usize>,
        /// Total cost
        cost: i64,
    },
    /// Error response
    Error {
        /// Error message
        message: String,
    },
}

/// Optimization provider for converge platform
#[derive(Debug, Clone)]
pub struct OptimizationProvider {
    /// Type of optimization
    pub optimization_type: OptimizationType,
    /// Solver parameters
    pub params: SolverParams,
}

impl Default for OptimizationProvider {
    fn default() -> Self {
        Self::new(OptimizationType::Assignment)
    }
}

impl OptimizationProvider {
    /// Create a new provider
    pub fn new(optimization_type: OptimizationType) -> Self {
        Self {
            optimization_type,
            params: SolverParams::default(),
        }
    }

    /// Set solver parameters
    pub fn with_params(mut self, params: SolverParams) -> Self {
        self.params = params;
        self
    }

    /// Solve an optimization problem
    pub fn solve(&self, request: OptimizationRequest) -> OptimizationResponse {
        match request {
            OptimizationRequest::Assignment { costs } => self.solve_assignment(costs),
            OptimizationRequest::Knapsack {
                weights,
                values,
                capacity,
            } => self.solve_knapsack(weights, values, capacity),
            OptimizationRequest::ShortestPath {
                edges,
                source,
                target,
            } => self.solve_shortest_path(edges, source, target),
        }
    }

    fn solve_assignment(&self, costs: Vec<Vec<i64>>) -> OptimizationResponse {
        let problem = AssignmentProblem::from_costs(costs);
        match assignment::solve(&problem) {
            Ok(solution) => OptimizationResponse::Assignment {
                assignments: solution.assignments,
                total_cost: solution.total_cost,
            },
            Err(e) => OptimizationResponse::Error {
                message: e.to_string(),
            },
        }
    }

    fn solve_knapsack(
        &self,
        weights: Vec<i64>,
        values: Vec<i64>,
        capacity: i64,
    ) -> OptimizationResponse {
        match KnapsackProblem::new(weights, values, capacity) {
            Ok(problem) => match knapsack::solve(&problem) {
                Ok(solution) => OptimizationResponse::Knapsack {
                    selected: solution.selected,
                    total_value: solution.total_value,
                    total_weight: solution.total_weight,
                },
                Err(e) => OptimizationResponse::Error {
                    message: e.to_string(),
                },
            },
            Err(e) => OptimizationResponse::Error {
                message: e.to_string(),
            },
        }
    }

    fn solve_shortest_path(
        &self,
        edges: Vec<(usize, usize, i64)>,
        source: usize,
        target: usize,
    ) -> OptimizationResponse {
        use petgraph::graph::DiGraph;

        // Build graph
        let mut graph: DiGraph<(), i64> = DiGraph::new();
        let max_node = edges
            .iter()
            .flat_map(|(a, b, _)| [*a, *b])
            .max()
            .unwrap_or(0);

        // Add nodes
        let nodes: Vec<_> = (0..=max_node).map(|_| graph.add_node(())).collect();

        // Add edges
        for (from, to, cost) in edges {
            if from <= max_node && to <= max_node {
                graph.add_edge(nodes[from], nodes[to], cost);
            }
        }

        if source > max_node || target > max_node {
            return OptimizationResponse::Error {
                message: "source or target node out of range".to_string(),
            };
        }

        match dijkstra::shortest_path(&graph, nodes[source], nodes[target], |&w| w) {
            Ok(Some(path)) => OptimizationResponse::ShortestPath {
                path: vec![source, target], // Simplified
                cost: path.cost,
            },
            Ok(None) => OptimizationResponse::Error {
                message: "no path exists".to_string(),
            },
            Err(e) => OptimizationResponse::Error {
                message: e.to_string(),
            },
        }
    }
}

// ============================================================================
// Gate-Based Provider
// ============================================================================

/// Gate-based optimization provider
///
/// Provides access to domain packs through the solver gate architecture.
/// Each solve returns a complete result including the proposed plan,
/// solver reports, invariant checks, and promotion gate decision.
#[derive(Clone)]
pub struct GateProvider {
    registry: Arc<PackRegistry>,
}

impl Default for GateProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GateProvider {
    /// Create with default built-in packs
    pub fn new() -> Self {
        Self {
            registry: Arc::new(PackRegistry::with_builtins()),
        }
    }

    /// Create with custom registry
    pub fn with_registry(registry: Arc<PackRegistry>) -> Self {
        Self { registry }
    }

    /// Get the underlying registry
    pub fn registry(&self) -> &PackRegistry {
        &self.registry
    }

    /// List available packs
    pub fn list_packs(&self) -> Vec<&str> {
        self.registry.list()
    }

    /// Check if a pack is available
    pub fn has_pack(&self, name: &str) -> bool {
        self.registry.contains(name)
    }

    /// Solve a problem through the gate
    ///
    /// This method:
    /// 1. Validates inputs against the pack schema
    /// 2. Solves the problem using the pack's solver
    /// 3. Checks all invariants
    /// 4. Evaluates the promotion gate
    ///
    /// Returns a complete result including all artifacts.
    pub fn solve(&self, pack_name: &str, spec: &ProblemSpec) -> crate::Result<GateSolveResult> {
        // Get the pack
        let pack = self
            .registry
            .get(pack_name)
            .ok_or_else(|| crate::Error::invalid_input(format!("unknown pack: {}", pack_name)))?;

        // Validate inputs against pack schema
        pack.validate_inputs(&spec.inputs)?;

        // Solve
        let solve_result = pack.solve(spec)?;

        // Check invariants
        let invariant_results = pack.check_invariants(&solve_result.plan)?;

        // Evaluate gate
        let gate = pack.evaluate_gate(&solve_result.plan, &invariant_results);

        Ok(GateSolveResult {
            plan: solve_result.plan,
            reports: solve_result.reports,
            invariant_results,
            gate,
        })
    }
}

impl std::fmt::Debug for GateProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GateProvider")
            .field("packs", &self.list_packs())
            .finish()
    }
}

/// Complete result from gate-based solving
#[derive(Debug)]
pub struct GateSolveResult {
    /// The proposed plan
    pub plan: ProposedPlan,
    /// Solver reports (may have tried multiple solvers)
    pub reports: Vec<SolverReport>,
    /// Results of invariant checks
    pub invariant_results: Vec<InvariantResult>,
    /// The promotion gate decision
    pub gate: PromotionGate,
}

impl GateSolveResult {
    /// Check if the solution is feasible
    pub fn is_feasible(&self) -> bool {
        self.reports.iter().any(|r| r.feasible)
    }

    /// Check if the plan was promoted
    pub fn is_promoted(&self) -> bool {
        self.gate.is_promoted()
    }

    /// Check if the plan was rejected
    pub fn is_rejected(&self) -> bool {
        self.gate.is_rejected()
    }

    /// Check if escalation is required
    pub fn requires_escalation(&self) -> bool {
        self.gate.requires_escalation()
    }

    /// Get failed invariants
    pub fn failed_invariants(&self) -> Vec<&str> {
        self.invariant_results
            .iter()
            .filter(|r| !r.passed)
            .map(|r| r.invariant.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::meeting_scheduler::{
        Attendee, MeetingRequirements, MeetingSchedulerInput, SlotPreference, TimeSlot,
    };
    use converge_pack::gate::ObjectiveSpec;

    #[test]
    fn test_assignment_provider() {
        let provider = OptimizationProvider::new(OptimizationType::Assignment);
        let request = OptimizationRequest::Assignment {
            costs: vec![vec![10, 5], vec![3, 8]],
        };

        let response = provider.solve(request);
        match response {
            OptimizationResponse::Assignment { total_cost, .. } => {
                // (0,1)=5, (1,0)=3 -> 8
                // (0,0)=10, (1,1)=8 -> 18
                // Optimal is 8
                assert_eq!(total_cost, 8);
            }
            _ => panic!("unexpected response"),
        }
    }

    #[test]
    fn test_knapsack_provider() {
        let provider = OptimizationProvider::new(OptimizationType::Knapsack);
        let request = OptimizationRequest::Knapsack {
            weights: vec![10, 20, 30],
            values: vec![60, 100, 120],
            capacity: 50,
        };

        let response = provider.solve(request);
        match response {
            OptimizationResponse::Knapsack { total_value, .. } => {
                assert_eq!(total_value, 220);
            }
            _ => panic!("unexpected response"),
        }
    }

    #[test]
    fn test_gate_provider_new() {
        let provider = GateProvider::new();
        assert!(provider.has_pack("meeting-scheduler"));
        assert!(provider.has_pack("inventory-rebalancing"));
        assert!(!provider.has_pack("nonexistent"));
    }

    #[test]
    fn test_gate_provider_list_packs() {
        let provider = GateProvider::new();
        let packs = provider.list_packs();
        assert!(packs.contains(&"meeting-scheduler"));
        assert!(packs.contains(&"inventory-rebalancing"));
    }

    #[test]
    fn test_gate_provider_solve_meeting_scheduler() {
        let provider = GateProvider::new();

        let input = MeetingSchedulerInput {
            slots: vec![TimeSlot {
                id: "slot-1".to_string(),
                start: 1000,
                end: 2000,
                room: Some("Room A".to_string()),
                capacity: 10,
            }],
            attendees: vec![Attendee {
                id: "alice".to_string(),
                name: "Alice".to_string(),
                required: true,
                available_slots: vec!["slot-1".to_string()],
                preferences: vec![SlotPreference {
                    slot_id: "slot-1".to_string(),
                    score: 10.0,
                }],
            }],
            requirements: MeetingRequirements {
                duration_minutes: 60,
                min_attendees: 1,
                require_room: false,
            },
        };

        let spec = ProblemSpec::builder("test-gate-001", "test-tenant")
            .objective(ObjectiveSpec::maximize("attendance"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = provider.solve("meeting-scheduler", &spec).unwrap();

        assert!(result.is_feasible());
        assert!(result.is_promoted());
        assert!(result.failed_invariants().is_empty());
    }

    #[test]
    fn test_gate_provider_unknown_pack() {
        let provider = GateProvider::new();

        let spec = ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs_raw(serde_json::json!({}))
            .build()
            .unwrap();

        let result = provider.solve("nonexistent-pack", &spec);
        assert!(result.is_err());
    }

    #[test]
    fn test_gate_solve_result_methods() {
        let provider = GateProvider::new();

        let input = MeetingSchedulerInput {
            slots: vec![TimeSlot {
                id: "slot-1".to_string(),
                start: 1000,
                end: 2000,
                room: None,
                capacity: 10,
            }],
            attendees: vec![Attendee {
                id: "alice".to_string(),
                name: "Alice".to_string(),
                required: true,
                available_slots: vec!["slot-1".to_string()],
                preferences: vec![SlotPreference {
                    slot_id: "slot-1".to_string(),
                    score: 10.0, // Add preference to pass advisory invariant
                }],
            }],
            requirements: MeetingRequirements {
                duration_minutes: 60,
                min_attendees: 1,
                require_room: false,
            },
        };

        let spec = ProblemSpec::builder("test-methods", "tenant")
            .objective(ObjectiveSpec::maximize("attendance"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = provider.solve("meeting-scheduler", &spec).unwrap();

        // Test all helper methods
        assert!(result.is_feasible());
        assert!(result.is_promoted());
        assert!(!result.is_rejected());
        assert!(!result.requires_escalation());
    }
}
