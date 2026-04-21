use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectionInput {
    pub values: Vec<f64>,
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    pub labels: Option<Vec<String>>,
}

fn default_threshold() -> f64 {
    2.0
}

impl AnomalyDetectionInput {
    pub fn validate(&self) -> Result<()> {
        if self.values.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one value required",
            ));
        }
        if self.threshold <= 0.0 {
            return Err(converge_pack::GateError::invalid_input(
                "Threshold must be positive",
            ));
        }
        if let Some(labels) = &self.labels {
            if labels.len() != self.values.len() {
                return Err(converge_pack::GateError::invalid_input(
                    "Labels length must match values length",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyRecord {
    pub index: usize,
    pub value: f64,
    pub z_score: f64,
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectionOutput {
    pub anomalies: Vec<AnomalyRecord>,
    pub mean: f64,
    pub std_dev: f64,
    pub total_points: usize,
    pub anomaly_count: usize,
}

impl AnomalyDetectionOutput {
    pub fn summary(&self) -> String {
        format!(
            "Detected {} anomalies in {} points (threshold z>{})",
            self.anomaly_count, self.total_points, self.std_dev
        )
    }
}
