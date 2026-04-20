//! Solver for Job Shop Scheduling pack
//!
//! Priority dispatch: shortest processing time first, earliest available machine.

use super::types::*;
use crate::Result;
use crate::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct PriorityDispatchSolver;

impl PriorityDispatchSolver {
    pub fn solve(
        &self,
        input: &JobShopInput,
        spec: &ProblemSpec,
    ) -> Result<(JobShopOutput, SolverReport)> {
        let mut machine_available = vec![0u64; input.machines];
        let mut job_next_op: Vec<usize> = vec![0; input.jobs.len()];
        let mut job_available = vec![0u64; input.jobs.len()];
        let mut schedule = Vec::new();

        let total_ops: usize = input.jobs.iter().map(|j| j.operations.len()).sum();

        while schedule.len() < total_ops {
            let mut candidates: Vec<(usize, usize, usize, u64)> = Vec::new();

            for (job_idx, job) in input.jobs.iter().enumerate() {
                let op_idx = job_next_op[job_idx];
                if op_idx < job.operations.len() {
                    let op = &job.operations[op_idx];
                    let earliest = job_available[job_idx].max(machine_available[op.machine]);
                    candidates.push((job_idx, op_idx, op.machine, earliest));
                }
            }

            // Sort by: earliest start, then shortest processing time
            candidates.sort_by(|a, b| {
                let dur_a = input.jobs[a.0].operations[a.1].duration;
                let dur_b = input.jobs[b.0].operations[b.1].duration;
                a.3.cmp(&b.3).then(dur_a.cmp(&dur_b))
            });

            if let Some(&(job_idx, op_idx, machine, start)) = candidates.first() {
                let duration = input.jobs[job_idx].operations[op_idx].duration;
                let end = start + duration;

                schedule.push(ScheduledOperation {
                    job: job_idx,
                    operation: op_idx,
                    machine,
                    start,
                });

                machine_available[machine] = end;
                job_available[job_idx] = end;
                job_next_op[job_idx] += 1;
            }
        }

        let makespan = schedule
            .iter()
            .map(|s| {
                let dur = input.jobs[s.job].operations[s.operation].duration;
                s.start + dur
            })
            .max()
            .unwrap_or(0);

        let output = JobShopOutput { schedule, makespan };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("priority-dispatch-v1", makespan as f64, replay);

        Ok((output, report))
    }
}
