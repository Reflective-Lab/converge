//! Solver for Network Flow pack
//!
//! Delegates to `crate::graph::flow::min_cost_flow` (Successive Shortest Paths
//! with Bellman-Ford) for the actual computation.

use super::types::*;
use crate::graph::flow::{FlowNetwork, MinCostFlowProblem, min_cost_flow};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport, StopReason};

/// Min-cost flow solver delegating to `crate::graph::flow`
pub struct MinCostFlowSolver;

impl MinCostFlowSolver {
    pub fn solve(
        &self,
        input: &NetworkFlowInput,
        spec: &ProblemSpec,
    ) -> Result<(NetworkFlowOutput, SolverReport)> {
        // Scale f64 costs/capacities to i64 (multiply by 1000 for 3 decimal places)
        let scale = 1000i64;

        let mut network = FlowNetwork::new(input.nodes);
        for edge in &input.edges {
            let cap = (edge.capacity * scale as f64).round() as i64;
            let cost = (edge.cost * scale as f64).round() as i64;
            network.add_edge(edge.from, edge.to, cap, cost);
        }

        let demand = (input.demand * scale as f64).round() as i64;
        let problem = MinCostFlowProblem::source_sink(network, input.source, input.sink, demand)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;

        match min_cost_flow(&problem) {
            Ok(result) => {
                // Convert flows back to f64
                let flows: Vec<f64> = result
                    .edge_flows
                    .iter()
                    .map(|&f| f as f64 / scale as f64)
                    .collect();
                let total_flow = result.flow as f64 / scale as f64;
                // Cost was scaled twice (flow * cost), so divide by scale²
                let total_cost = result.cost as f64 / (scale as f64 * scale as f64);

                let output = NetworkFlowOutput {
                    flows,
                    total_cost,
                    total_flow,
                };

                let replay = ReplayEnvelope::minimal(spec.seed());
                let report = if total_flow >= input.demand - 1e-9 {
                    SolverReport::optimal("min-cost-flow-v1", total_cost, replay)
                } else {
                    SolverReport::infeasible(
                        "min-cost-flow-v1",
                        vec![],
                        StopReason::NoFeasible,
                        replay,
                    )
                };

                Ok((output, report))
            }
            Err(_) => {
                let replay = ReplayEnvelope::minimal(spec.seed());
                let report = SolverReport::infeasible(
                    "min-cost-flow-v1",
                    vec![],
                    StopReason::NoFeasible,
                    replay,
                );
                let output = NetworkFlowOutput {
                    flows: vec![0.0; input.edges.len()],
                    total_cost: 0.0,
                    total_flow: 0.0,
                };
                Ok((output, report))
            }
        }
    }
}
