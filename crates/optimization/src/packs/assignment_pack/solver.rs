//! Solver for Task Assignment pack

use super::types::*;
use crate::Result;
use crate::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

/// Greedy assignment solver
///
/// Algorithm: flatten all (agent, task, cost) triples, sort by cost,
/// assign greedily ensuring each agent and task used at most once.
pub struct GreedyAssignmentSolver;

impl GreedyAssignmentSolver {
    pub fn solve(
        &self,
        input: &AssignmentInput,
        spec: &ProblemSpec,
    ) -> Result<(AssignmentOutput, SolverReport)> {
        let n = input.cost_matrix.len();

        // Build sorted list of (cost, agent, task)
        let mut edges: Vec<(f64, usize, usize)> = Vec::new();
        for (i, row) in input.cost_matrix.iter().enumerate() {
            for (j, &cost) in row.iter().enumerate() {
                edges.push((cost, i, j));
            }
        }
        edges.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let mut assigned_agents = vec![false; n];
        let mut assigned_tasks = vec![false; n];
        let mut assignments = Vec::new();
        let mut total_cost = 0.0;

        for (cost, agent, task) in edges {
            if assigned_agents[agent] || assigned_tasks[task] {
                continue;
            }
            assignments.push((agent, task));
            total_cost += cost;
            assigned_agents[agent] = true;
            assigned_tasks[task] = true;
            if assignments.len() == n {
                break;
            }
        }

        let output = AssignmentOutput {
            assignments,
            total_cost,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("greedy-assignment-v1", total_cost, replay);

        Ok((output, report))
    }
}
