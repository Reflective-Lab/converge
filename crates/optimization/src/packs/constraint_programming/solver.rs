//! Solver for Constraint Programming pack
//!
//! Simple backtracking with constraint propagation.

use super::types::*;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct BacktrackingSolver;

impl BacktrackingSolver {
    pub fn solve(
        &self,
        input: &ConstraintProgrammingInput,
        spec: &ProblemSpec,
    ) -> Result<(ConstraintProgrammingOutput, SolverReport)> {
        let n = input.variables.len();
        let mut best_assignment: Option<Vec<i64>> = None;
        let mut best_obj: Option<i64> = None;
        let mut current = vec![0i64; n];

        Self::backtrack(input, 0, &mut current, &mut best_assignment, &mut best_obj);

        let (assignments, feasible, objective_value) = match best_assignment {
            Some(values) => {
                let assignments: Vec<CpAssignment> = input
                    .variables
                    .iter()
                    .zip(values.iter())
                    .map(|(var, &val)| CpAssignment {
                        name: var.name.clone(),
                        value: val,
                    })
                    .collect();
                (assignments, true, best_obj)
            }
            None => (vec![], false, None),
        };

        let output = ConstraintProgrammingOutput {
            assignments,
            feasible,
            objective_value,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let obj_val = objective_value.unwrap_or(0) as f64;
        let report = SolverReport::optimal("backtracking-cp-v1", obj_val, replay);

        Ok((output, report))
    }

    fn backtrack(
        input: &ConstraintProgrammingInput,
        depth: usize,
        current: &mut Vec<i64>,
        best: &mut Option<Vec<i64>>,
        best_obj: &mut Option<i64>,
    ) {
        let n = input.variables.len();
        if depth == n {
            if Self::all_constraints_satisfied(input, current) {
                let obj = input.objective.as_ref().map(|o| {
                    let idx = input
                        .variables
                        .iter()
                        .position(|v| v.name == o.variable)
                        .unwrap_or(0);
                    current[idx]
                });

                let dominated = match (&input.objective, obj, *best_obj) {
                    (Some(o), Some(cur), Some(prev)) => {
                        if o.maximize {
                            cur <= prev
                        } else {
                            cur >= prev
                        }
                    }
                    _ => false,
                };

                if !dominated {
                    *best = Some(current.clone());
                    *best_obj = obj;
                }
            }
            return;
        }

        let var = &input.variables[depth];
        for val in var.min..=var.max {
            current[depth] = val;
            if Self::partial_constraints_satisfied(input, current, depth) {
                Self::backtrack(input, depth + 1, current, best, best_obj);
            }
        }
    }

    fn all_constraints_satisfied(input: &ConstraintProgrammingInput, vals: &[i64]) -> bool {
        input
            .constraints
            .iter()
            .all(|c| Self::check_constraint(input, vals, c, input.variables.len()))
    }

    fn partial_constraints_satisfied(
        input: &ConstraintProgrammingInput,
        vals: &[i64],
        depth: usize,
    ) -> bool {
        input
            .constraints
            .iter()
            .all(|c| Self::check_constraint(input, vals, c, depth + 1))
    }

    fn check_constraint(
        input: &ConstraintProgrammingInput,
        vals: &[i64],
        constraint: &CpConstraint,
        assigned_count: usize,
    ) -> bool {
        let var_index =
            |name: &str| -> Option<usize> { input.variables.iter().position(|v| v.name == name) };

        match constraint.constraint_type {
            ConstraintType::LessThan => {
                let left = constraint.args.get("left").and_then(|v| v.as_str());
                let right = constraint.args.get("right").and_then(|v| v.as_str());
                match (left.and_then(&var_index), right.and_then(&var_index)) {
                    (Some(li), Some(ri)) if li < assigned_count && ri < assigned_count => {
                        vals[li] < vals[ri]
                    }
                    _ => true,
                }
            }
            ConstraintType::NotEqual => {
                let left = constraint.args.get("left").and_then(|v| v.as_str());
                let right = constraint.args.get("right").and_then(|v| v.as_str());
                match (left.and_then(&var_index), right.and_then(&var_index)) {
                    (Some(li), Some(ri)) if li < assigned_count && ri < assigned_count => {
                        vals[li] != vals[ri]
                    }
                    _ => true,
                }
            }
            ConstraintType::SumEquals => {
                let vars: Vec<usize> = constraint
                    .args
                    .get("variables")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().and_then(&var_index))
                            .collect()
                    })
                    .unwrap_or_default();
                let target = constraint
                    .args
                    .get("value")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);

                if vars.iter().all(|&i| i < assigned_count) {
                    let sum: i64 = vars.iter().map(|&i| vals[i]).sum();
                    sum == target
                } else {
                    true
                }
            }
        }
    }
}
