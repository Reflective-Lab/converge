use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDetectionInput {
    pub values: Vec<f64>,
    #[serde(default = "default_window")]
    pub window: usize,
    #[serde(default = "default_sensitivity")]
    pub sensitivity: f64,
}

fn default_window() -> usize {
    5
}

fn default_sensitivity() -> f64 {
    1.5
}

impl TrendDetectionInput {
    pub fn validate(&self) -> Result<()> {
        if self.values.len() < 3 {
            return Err(converge_pack::GateError::invalid_input(
                "At least 3 values required for trend detection",
            ));
        }
        if self.window < 2 {
            return Err(converge_pack::GateError::invalid_input(
                "Window must be >= 2",
            ));
        }
        if self.window > self.values.len() {
            return Err(converge_pack::GateError::invalid_input(
                "Window cannot exceed number of values",
            ));
        }
        if self.sensitivity <= 0.0 {
            return Err(converge_pack::GateError::invalid_input(
                "Sensitivity must be positive",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Rising,
    Falling,
    Stable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendSegment {
    pub start: usize,
    pub end: usize,
    pub direction: TrendDirection,
    pub slope: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Changepoint {
    pub index: usize,
    pub magnitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendDetectionOutput {
    pub segments: Vec<TrendSegment>,
    pub changepoints: Vec<Changepoint>,
    pub overall_direction: TrendDirection,
    pub overall_slope: f64,
}

impl TrendDetectionOutput {
    pub fn summary(&self) -> String {
        format!(
            "{} trend segments, {} changepoints, overall {:?} (slope: {:.4})",
            self.segments.len(),
            self.changepoints.len(),
            self.overall_direction,
            self.overall_slope,
        )
    }
}
