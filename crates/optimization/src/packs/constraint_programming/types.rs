//! Types for Constraint Programming pack

use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpVariable {
    pub name: String,
    pub min: i64,
    pub max: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpConstraint {
    #[serde(rename = "type")]
    pub constraint_type: ConstraintType,
    pub args: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintType {
    LessThan,
    NotEqual,
    SumEquals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpObjective {
    pub variable: String,
    pub maximize: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintProgrammingInput {
    pub variables: Vec<CpVariable>,
    pub constraints: Vec<CpConstraint>,
    pub objective: Option<CpObjective>,
}

impl ConstraintProgrammingInput {
    pub fn validate(&self) -> Result<()> {
        if self.variables.is_empty() {
            return Err(crate::Error::invalid_input(
                "At least one variable required",
            ));
        }
        for var in &self.variables {
            if var.min > var.max {
                return Err(crate::Error::invalid_input(format!(
                    "Variable '{}' has min ({}) > max ({})",
                    var.name, var.min, var.max
                )));
            }
        }
        if let Some(obj) = &self.objective {
            if !self.variables.iter().any(|v| v.name == obj.variable) {
                return Err(crate::Error::invalid_input(format!(
                    "Objective variable '{}' not found",
                    obj.variable
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpAssignment {
    pub name: String,
    pub value: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstraintProgrammingOutput {
    pub assignments: Vec<CpAssignment>,
    pub feasible: bool,
    pub objective_value: Option<i64>,
}

impl ConstraintProgrammingOutput {
    pub fn summary(&self) -> String {
        if self.feasible {
            format!(
                "Found feasible solution with {} assignments",
                self.assignments.len()
            )
        } else {
            "No feasible solution found".to_string()
        }
    }
}
