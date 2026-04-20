use super::types::*;
use converge_optimization::Result;
use converge_optimization::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct ZScoreSolver;

impl ZScoreSolver {
    pub fn solve(
        &self,
        input: &AnomalyDetectionInput,
        spec: &ProblemSpec,
    ) -> Result<(AnomalyDetectionOutput, SolverReport)> {
        let n = input.values.len() as f64;
        let mean = input.values.iter().sum::<f64>() / n;
        let variance = input.values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();

        let anomalies = if std_dev > 0.0 {
            input
                .values
                .iter()
                .enumerate()
                .filter_map(|(i, &v)| {
                    let z = (v - mean).abs() / std_dev;
                    if z > input.threshold {
                        Some(AnomalyRecord {
                            index: i,
                            value: v,
                            z_score: z,
                            label: input.labels.as_ref().map(|l| l[i].clone()),
                        })
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            vec![]
        };

        let anomaly_count = anomalies.len();
        let total_points = input.values.len();

        let output = AnomalyDetectionOutput {
            anomalies,
            mean,
            std_dev,
            total_points,
            anomaly_count,
        };

        let confidence = (1.0 - (anomaly_count as f64 / total_points as f64)).clamp(0.3, 0.95);

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("z-score-v1", confidence, replay);

        Ok((output, report))
    }
}
