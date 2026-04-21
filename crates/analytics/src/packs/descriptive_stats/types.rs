use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptiveStatsInput {
    pub values: Vec<f64>,
    #[serde(default)]
    pub percentiles: Vec<f64>,
}

impl DescriptiveStatsInput {
    pub fn validate(&self) -> Result<()> {
        if self.values.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one value required",
            ));
        }
        for (i, &p) in self.percentiles.iter().enumerate() {
            if !(0.0..=100.0).contains(&p) {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Percentile {} ({}) must be in [0, 100]",
                    i, p
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PercentileResult {
    pub percentile: f64,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptiveStatsOutput {
    pub count: usize,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub variance: f64,
    pub min: f64,
    pub max: f64,
    pub range: f64,
    pub skewness: f64,
    pub kurtosis: f64,
    pub percentiles: Vec<PercentileResult>,
}

impl DescriptiveStatsOutput {
    pub fn summary(&self) -> String {
        format!(
            "{} values: mean={:.3}, median={:.3}, std={:.3}, range=[{:.3}, {:.3}]",
            self.count, self.mean, self.median, self.std_dev, self.min, self.max,
        )
    }
}
