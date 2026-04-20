//! Solver for Task Assignment pack
//!
//! Delegates to the Hungarian algorithm from `crate::assignment::hungarian`
//! for optimal O(n³) assignment.

use super::types::*;
use crate::Result;
use crate::assignment::{self, AssignmentProblem};
use crate::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

/// Optimal assignment solver (Hungarian algorithm wrapper)
pub struct HungarianAssignmentSolver;

impl HungarianAssignmentSolver {
    pub fn solve(
        &self,
        input: &AssignmentInput,
        spec: &ProblemSpec,
    ) -> Result<(AssignmentOutput, SolverReport)> {
        let n = input.cost_matrix.len();

        // Convert f64 cost matrix to i64 for the Hungarian solver.
        // Scale by 1000 to preserve 3 decimal places of precision.
        let scale = 1000i64;
        let int_costs: Vec<Vec<i64>> = input
            .cost_matrix
            .iter()
            .map(|row| {
                row.iter()
                    .map(|&c| (c * scale as f64).round() as i64)
                    .collect()
            })
            .collect();

        let problem = AssignmentProblem::from_costs(int_costs);
        let solution = assignment::solve(&problem)?;

        // Convert back: assignments[agent] = task → Vec<(agent, task)>
        let assignments: Vec<(usize, usize)> = solution
            .assignments
            .iter()
            .enumerate()
            .filter(|&(agent, _)| agent < n)
            .map(|(agent, &task)| (agent, task))
            .collect();

        let total_cost: f64 = assignments
            .iter()
            .map(|&(a, t)| input.cost_matrix[a][t])
            .sum();

        let output = AssignmentOutput {
            assignments,
            total_cost,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("hungarian-assignment-v1", total_cost, replay);

        Ok((output, report))
    }
}
