use super::types::*;
use converge_optimization::Result;
use converge_optimization::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct ExponentialSmoothingSolver;

impl ExponentialSmoothingSolver {
    pub fn solve(
        &self,
        input: &ForecastingInput,
        spec: &ProblemSpec,
    ) -> Result<(ForecastingOutput, SolverReport)> {
        let alpha = input.alpha;
        let values = &input.values;
        let n = values.len();

        // Simple exponential smoothing
        let mut level = values[0];
        let mut residuals = Vec::with_capacity(n - 1);

        for &v in &values[1..] {
            let forecast = level;
            residuals.push(v - forecast);
            level = alpha * v + (1.0 - alpha) * level;
        }

        // Residual standard deviation for confidence intervals
        let residual_std = if residuals.is_empty() {
            0.0
        } else {
            let mean_r = residuals.iter().sum::<f64>() / residuals.len() as f64;
            let var = residuals.iter().map(|r| (r - mean_r).powi(2)).sum::<f64>()
                / residuals.len() as f64;
            var.sqrt()
        };

        // Forecast ahead
        let predictions: Vec<ForecastPoint> = (1..=input.horizon)
            .map(|step| {
                let width = 1.96 * residual_std * (step as f64).sqrt();
                ForecastPoint {
                    step,
                    value: level,
                    lower: level - width,
                    upper: level + width,
                }
            })
            .collect();

        let output = ForecastingOutput {
            predictions,
            residual_std,
            horizon: input.horizon,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let confidence = if residual_std > 0.0 {
            (1.0 / (1.0 + residual_std)).clamp(0.3, 0.95)
        } else {
            0.95
        };
        let report = SolverReport::optimal("exponential-smoothing-v1", confidence, replay);

        Ok((output, report))
    }
}
