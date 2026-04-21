use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionInput {
    pub records: Vec<Vec<f64>>,
    pub weights: Vec<f64>,
    pub bias: f64,
}

impl RegressionInput {
    pub fn validate(&self) -> Result<()> {
        if self.records.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one record required",
            ));
        }
        let dim = self.weights.len();
        if dim == 0 {
            return Err(converge_pack::GateError::invalid_input(
                "At least one weight (feature) required",
            ));
        }
        for (i, record) in self.records.iter().enumerate() {
            if record.len() != dim {
                return Err(converge_pack::GateError::invalid_input(format!(
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
pub struct PredictedValue {
    pub index: usize,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionOutput {
    pub predictions: Vec<PredictedValue>,
    pub mean_prediction: f64,
    pub std_prediction: f64,
    pub total: usize,
}

impl RegressionOutput {
    pub fn summary(&self) -> String {
        format!(
            "Predicted {} values (mean: {:.3}, std: {:.3})",
            self.total, self.mean_prediction, self.std_prediction,
        )
    }
}
