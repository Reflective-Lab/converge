//! Types for Task Assignment pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for task assignment optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentInput {
    /// Cost matrix: cost_matrix[i][j] = cost of assigning agent i to task j
    pub cost_matrix: Vec<Vec<f64>>,
}

impl AssignmentInput {
    pub fn validate(&self) -> Result<()> {
        if self.cost_matrix.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "Cost matrix must not be empty",
            ));
        }
        let n = self.cost_matrix.len();
        for (i, row) in self.cost_matrix.iter().enumerate() {
            if row.len() != n {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Cost matrix row {} has {} columns, expected {}",
                    i,
                    row.len(),
                    n
                )));
            }
        }
        Ok(())
    }
}

/// Output for task assignment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentOutput {
    /// Assignments as (agent_index, task_index) pairs
    pub assignments: Vec<(usize, usize)>,
    /// Total cost of all assignments
    pub total_cost: f64,
}

impl AssignmentOutput {
    pub fn summary(&self) -> String {
        format!(
            "Assigned {} agents to tasks with total cost {:.2}",
            self.assignments.len(),
            self.total_cost
        )
    }
}
