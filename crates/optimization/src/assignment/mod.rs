//! Linear Assignment Problem solvers
//!
//! The assignment problem finds the optimal one-to-one matching between
//! agents and tasks that minimizes total cost.
//!
//! ## Problem Definition
//!
//! Given:
//! - n agents and m tasks (typically n = m)
//! - Cost matrix C where C[i][j] is cost of assigning agent i to task j
//!
//! Find:
//! - Assignment minimizing sum of costs
//! - Each agent assigned to exactly one task
//! - Each task assigned to at most one agent
//!
//! ## Algorithms
//!
//! - [`hungarian`] - O(n⁴) Hungarian algorithm, simple and correct
//! - [`auction`] - O(n³) Auction algorithm, faster for sparse problems
//!
//! ## Example
//!
//! ```rust,ignore
//! use converge_optimization::assignment::{solve, AssignmentProblem};
//!
//! let problem = AssignmentProblem::from_costs(vec![
//!     vec![10, 5, 13],
//!     vec![3, 9, 18],
//!     vec![14, 8, 7],
//! ]);
//!
//! let solution = solve(&problem).unwrap();
//! println!("Total cost: {}", solution.total_cost);
//! for (agent, task) in solution.assignments.iter().enumerate() {
//!     println!("Suggestor {} -> Task {}", agent, task);
//! }
//! ```

pub mod auction;
pub mod hungarian;

use crate::{Cost, Error, Result, SolverParams, SolverStats, SolverStatus};
use serde::{Deserialize, Serialize};

/// An assignment problem instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentProblem {
    /// Cost matrix: costs[agent][task]
    pub costs: Vec<Vec<Cost>>,
    /// Number of agents
    pub num_agents: usize,
    /// Number of tasks
    pub num_tasks: usize,
}

impl AssignmentProblem {
    /// Create a problem from a cost matrix
    pub fn from_costs(costs: Vec<Vec<Cost>>) -> Self {
        let num_agents = costs.len();
        let num_tasks = costs.first().map_or(0, Vec::len);
        Self {
            costs,
            num_agents,
            num_tasks,
        }
    }

    /// Create a square problem from a flat cost vector
    pub fn from_flat(costs: Vec<Cost>, n: usize) -> Result<Self> {
        if costs.len() != n * n {
            return Err(Error::dimension_mismatch(n * n, costs.len()));
        }
        let matrix: Vec<Vec<Cost>> = costs.chunks(n).map(|c| c.to_vec()).collect();
        Ok(Self::from_costs(matrix))
    }

    /// Check if the problem is square (n agents = n tasks)
    pub fn is_square(&self) -> bool {
        self.num_agents == self.num_tasks
    }

    /// Get cost of assigning agent to task
    pub fn cost(&self, agent: usize, task: usize) -> Cost {
        self.costs[agent][task]
    }

    /// Validate the problem
    pub fn validate(&self) -> Result<()> {
        if self.num_agents == 0 {
            return Err(Error::invalid_input("no agents"));
        }
        if self.num_tasks == 0 {
            return Err(Error::invalid_input("no tasks"));
        }
        for row in &self.costs {
            if row.len() != self.num_tasks {
                return Err(Error::dimension_mismatch(self.num_tasks, row.len()));
            }
        }
        Ok(())
    }
}

/// Solution to an assignment problem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentSolution {
    /// Assignment: assignments[agent] = task
    pub assignments: Vec<usize>,
    /// Total cost of the assignment
    pub total_cost: Cost,
    /// Solver status
    pub status: SolverStatus,
    /// Solver statistics
    pub stats: SolverStats,
}

impl AssignmentSolution {
    /// Get the task assigned to an agent
    pub fn task_for_agent(&self, agent: usize) -> Option<usize> {
        self.assignments.get(agent).copied()
    }

    /// Iterate over (agent, task) pairs
    pub fn iter(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.assignments.iter().enumerate().map(|(a, &t)| (a, t))
    }
}

/// Trait for assignment solvers
pub trait AssignmentSolver {
    /// Solve the assignment problem
    fn solve(
        &self,
        problem: &AssignmentProblem,
        params: &SolverParams,
    ) -> Result<AssignmentSolution>;

    /// Solver name
    fn name(&self) -> &'static str;
}

/// Solve an assignment problem using the default solver (Hungarian)
pub fn solve(problem: &AssignmentProblem) -> Result<AssignmentSolution> {
    solve_with_params(problem, &SolverParams::default())
}

/// Solve an assignment problem with custom parameters
pub fn solve_with_params(
    problem: &AssignmentProblem,
    params: &SolverParams,
) -> Result<AssignmentSolution> {
    problem.validate()?;
    hungarian::HungarianSolver.solve(problem, params)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_assignment() {
        let problem =
            AssignmentProblem::from_costs(vec![vec![10, 5, 13], vec![3, 9, 18], vec![14, 8, 7]]);

        let solution = solve(&problem).unwrap();
        assert_eq!(solution.status, SolverStatus::Optimal);
        // Optimal: agent 0->task 1 (5), agent 1->task 0 (3), agent 2->task 2 (7) = 15
        assert_eq!(solution.total_cost, 15);
    }

    #[test]
    fn test_from_flat() {
        let costs = vec![1, 2, 3, 4];
        let problem = AssignmentProblem::from_flat(costs, 2).unwrap();
        assert_eq!(problem.num_agents, 2);
        assert_eq!(problem.num_tasks, 2);
        assert_eq!(problem.cost(0, 0), 1);
        assert_eq!(problem.cost(1, 1), 4);
    }
}
