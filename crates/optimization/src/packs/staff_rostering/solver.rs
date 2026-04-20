//! Solver for Staff Rostering pack
//!
//! Greedy skill-matching with load balancing.

use super::types::*;
use crate::Result;
use crate::gate::{ProblemSpec, ReplayEnvelope, SolverReport};
use std::collections::HashMap;

pub struct GreedyRosteringSolver;

impl GreedyRosteringSolver {
    pub fn solve(
        &self,
        input: &StaffRosteringInput,
        spec: &ProblemSpec,
    ) -> Result<(StaffRosteringOutput, SolverReport)> {
        let mut staff_hours: HashMap<&str, u64> = HashMap::new();
        let mut staff_shift_count: HashMap<&str, usize> = HashMap::new();
        for s in &input.staff {
            staff_hours.insert(&s.id, 0);
            staff_shift_count.insert(&s.id, 0);
        }

        let mut assignments = Vec::new();
        let mut unassigned_shifts = Vec::new();

        for shift in &input.shifts {
            // Find qualified staff sorted by current load (least-loaded first)
            let mut candidates: Vec<&StaffMember> = input
                .staff
                .iter()
                .filter(|s| s.skills.contains(&shift.required_skill))
                .filter(|s| {
                    let used = staff_hours.get(s.id.as_str()).copied().unwrap_or(0);
                    used + shift.hours <= s.max_hours
                })
                .filter(|s| {
                    if let Some(max_consec) = input.max_consecutive_shifts {
                        let count = staff_shift_count.get(s.id.as_str()).copied().unwrap_or(0);
                        count < max_consec
                    } else {
                        true
                    }
                })
                .collect();

            candidates.sort_by_key(|s| staff_hours.get(s.id.as_str()).copied().unwrap_or(0));

            if let Some(best) = candidates.first() {
                *staff_hours.get_mut(best.id.as_str()).unwrap() += shift.hours;
                *staff_shift_count.get_mut(best.id.as_str()).unwrap() += 1;
                assignments.push(RosterAssignment {
                    staff_id: best.id.clone(),
                    shift_id: shift.id.clone(),
                });
            } else {
                unassigned_shifts.push(shift.id.clone());
            }
        }

        let total = input.shifts.len() as f64;
        let coverage = if total > 0.0 {
            assignments.len() as f64 / total
        } else {
            1.0
        };

        let output = StaffRosteringOutput {
            assignments,
            coverage,
            unassigned_shifts,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("greedy-rostering-v1", coverage, replay);

        Ok((output, report))
    }
}
