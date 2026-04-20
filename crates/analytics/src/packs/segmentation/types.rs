use converge_optimization::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentationInput {
    pub records: Vec<Vec<f64>>,
    pub k: usize,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: usize,
    pub seed: Option<u64>,
}

fn default_max_iterations() -> usize {
    100
}

impl SegmentationInput {
    pub fn validate(&self) -> Result<()> {
        if self.records.is_empty() {
            return Err(converge_optimization::Error::invalid_input(
                "At least one record required",
            ));
        }
        if self.k == 0 {
            return Err(converge_optimization::Error::invalid_input(
                "k must be >= 1",
            ));
        }
        if self.k > self.records.len() {
            return Err(converge_optimization::Error::invalid_input(
                "k cannot exceed number of records",
            ));
        }
        if self.max_iterations == 0 {
            return Err(converge_optimization::Error::invalid_input(
                "max_iterations must be >= 1",
            ));
        }
        let dim = self.records[0].len();
        if dim == 0 {
            return Err(converge_optimization::Error::invalid_input(
                "Records must have at least one feature",
            ));
        }
        for (i, record) in self.records.iter().enumerate() {
            if record.len() != dim {
                return Err(converge_optimization::Error::invalid_input(format!(
                    "Record {} has {} features, expected {}",
                    i,
                    record.len(),
                    dim
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SegmentationOutput {
    pub assignments: Vec<usize>,
    pub centroids: Vec<Vec<f64>>,
    pub iterations_used: usize,
    pub inertia: f64,
}

impl SegmentationOutput {
    pub fn summary(&self) -> String {
        format!(
            "Segmented {} records into {} clusters in {} iterations (inertia: {:.2})",
            self.assignments.len(),
            self.centroids.len(),
            self.iterations_used,
            self.inertia,
        )
    }
}
