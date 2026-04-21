//! Types for Job Shop Scheduling pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub machine: usize,
    pub duration: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobShopInput {
    pub jobs: Vec<Job>,
    pub machines: usize,
}

impl JobShopInput {
    pub fn validate(&self) -> Result<()> {
        if self.jobs.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one job required",
            ));
        }
        if self.machines == 0 {
            return Err(converge_pack::GateError::invalid_input(
                "At least one machine required",
            ));
        }
        for (i, job) in self.jobs.iter().enumerate() {
            if job.operations.is_empty() {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Job {} has no operations",
                    i
                )));
            }
            for (j, op) in job.operations.iter().enumerate() {
                if op.machine >= self.machines {
                    return Err(converge_pack::GateError::invalid_input(format!(
                        "Job {} operation {} references machine {} but only {} machines exist",
                        i, j, op.machine, self.machines
                    )));
                }
                if op.duration == 0 {
                    return Err(converge_pack::GateError::invalid_input(format!(
                        "Job {} operation {} has zero duration",
                        i, j
                    )));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledOperation {
    pub job: usize,
    pub operation: usize,
    pub machine: usize,
    pub start: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobShopOutput {
    pub schedule: Vec<ScheduledOperation>,
    pub makespan: u64,
}

impl JobShopOutput {
    pub fn summary(&self) -> String {
        format!(
            "Scheduled {} operations with makespan {}",
            self.schedule.len(),
            self.makespan
        )
    }
}
