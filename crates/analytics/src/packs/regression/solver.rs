use super::types::*;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct LinearRegressionSolver;

impl LinearRegressionSolver {
    pub fn solve(
        &self,
        input: &RegressionInput,
        spec: &ProblemSpec,
    ) -> Result<(RegressionOutput, SolverReport)> {
        let predictions: Vec<PredictedValue> = input
            .records
            .iter()
            .enumerate()
            .map(|(i, record)| {
                let value: f64 = record
                    .iter()
                    .zip(&input.weights)
                    .map(|(x, w)| x * w)
                    .sum::<f64>()
                    + input.bias;
                PredictedValue { index: i, value }
            })
            .collect();

        let total = predictions.len();
        let mean_prediction = predictions.iter().map(|p| p.value).sum::<f64>() / total as f64;
        let variance = predictions
            .iter()
            .map(|p| (p.value - mean_prediction).powi(2))
            .sum::<f64>()
            / total as f64;
        let std_prediction = variance.sqrt();

        let output = RegressionOutput {
            predictions,
            mean_prediction,
            std_prediction,
            total,
        };

        // Confidence: higher when predictions have reasonable spread
        let confidence = if std_prediction > 0.0 {
            (1.0 / (1.0 + 1.0 / std_prediction)).clamp(0.3, 0.95)
        } else {
            0.5
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("linear-regression-v1", confidence, replay);

        Ok((output, report))
    }
}
