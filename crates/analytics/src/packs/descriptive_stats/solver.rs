use super::types::*;
use converge_optimization::Result;
use converge_optimization::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct DescriptiveStatsSolver;

impl DescriptiveStatsSolver {
    pub fn solve(
        &self,
        input: &DescriptiveStatsInput,
        spec: &ProblemSpec,
    ) -> Result<(DescriptiveStatsOutput, SolverReport)> {
        let n = input.values.len() as f64;
        let mut sorted = input.values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let min = sorted[0];
        let max = sorted[sorted.len() - 1];
        let range = max - min;
        let mean = input.values.iter().sum::<f64>() / n;
        let median = percentile_sorted(&sorted, 50.0);

        let variance = input.values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let std_dev = variance.sqrt();

        let skewness = if std_dev > 0.0 {
            input
                .values
                .iter()
                .map(|v| ((v - mean) / std_dev).powi(3))
                .sum::<f64>()
                / n
        } else {
            0.0
        };

        let kurtosis = if std_dev > 0.0 {
            input
                .values
                .iter()
                .map(|v| ((v - mean) / std_dev).powi(4))
                .sum::<f64>()
                / n
                - 3.0 // excess kurtosis
        } else {
            0.0
        };

        let percentiles: Vec<PercentileResult> = input
            .percentiles
            .iter()
            .map(|&p| PercentileResult {
                percentile: p,
                value: percentile_sorted(&sorted, p),
            })
            .collect();

        let output = DescriptiveStatsOutput {
            count: input.values.len(),
            mean,
            median,
            std_dev,
            variance,
            min,
            max,
            range,
            skewness,
            kurtosis,
            percentiles,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("descriptive-stats-v1", 1.0, replay);

        Ok((output, report))
    }
}

fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = p / 100.0 * (sorted.len() - 1) as f64;
    let lower = rank.floor() as usize;
    let upper = rank.ceil() as usize;
    let frac = rank - lower as f64;
    if lower == upper {
        sorted[lower]
    } else {
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}
