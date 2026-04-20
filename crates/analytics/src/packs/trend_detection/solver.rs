use super::types::*;
use converge_optimization::Result;
use converge_optimization::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct MovingAverageTrendSolver;

impl MovingAverageTrendSolver {
    pub fn solve(
        &self,
        input: &TrendDetectionInput,
        spec: &ProblemSpec,
    ) -> Result<(TrendDetectionOutput, SolverReport)> {
        let n = input.values.len();
        let w = input.window;

        // Compute moving average slopes
        let mut slopes: Vec<f64> = Vec::with_capacity(n.saturating_sub(w));
        for i in 0..n.saturating_sub(w) {
            let window_start = &input.values[i..i + w];
            let slope = linear_slope(window_start);
            slopes.push(slope);
        }

        // Overall slope across entire series
        let overall_slope = linear_slope(&input.values);
        let slope_threshold = slopes.iter().map(|s| s.abs()).sum::<f64>()
            / slopes.len().max(1) as f64
            * input.sensitivity;
        let slope_threshold = slope_threshold.max(1e-10);

        let overall_direction = classify_slope(overall_slope, slope_threshold);

        // Build segments by detecting direction changes
        let mut segments: Vec<TrendSegment> = Vec::new();
        let mut changepoints: Vec<Changepoint> = Vec::new();

        if slopes.is_empty() {
            segments.push(TrendSegment {
                start: 0,
                end: n - 1,
                direction: overall_direction,
                slope: overall_slope,
            });
        } else {
            let mut seg_start = 0;
            let mut seg_direction = classify_slope(slopes[0], slope_threshold);
            let mut seg_slopes = vec![slopes[0]];

            for (i, &slope) in slopes.iter().enumerate().skip(1) {
                let dir = classify_slope(slope, slope_threshold);
                if dir != seg_direction {
                    let avg_slope = seg_slopes.iter().sum::<f64>() / seg_slopes.len() as f64;
                    segments.push(TrendSegment {
                        start: seg_start,
                        end: i + w / 2,
                        direction: seg_direction,
                        slope: avg_slope,
                    });
                    changepoints.push(Changepoint {
                        index: i + w / 2,
                        magnitude: (slope - seg_slopes.last().copied().unwrap_or(0.0)).abs(),
                    });
                    seg_start = i + w / 2;
                    seg_direction = dir;
                    seg_slopes.clear();
                }
                seg_slopes.push(slope);
            }

            let avg_slope = seg_slopes.iter().sum::<f64>() / seg_slopes.len() as f64;
            segments.push(TrendSegment {
                start: seg_start,
                end: n - 1,
                direction: seg_direction,
                slope: avg_slope,
            });
        }

        let output = TrendDetectionOutput {
            segments,
            changepoints,
            overall_direction,
            overall_slope,
        };

        let confidence = if slopes.len() >= 2 {
            let variance = slopes
                .iter()
                .map(|s| (s - overall_slope).powi(2))
                .sum::<f64>()
                / slopes.len() as f64;
            (1.0 / (1.0 + variance.sqrt())).clamp(0.3, 0.95)
        } else {
            0.5
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("moving-average-trend-v1", confidence, replay);

        Ok((output, report))
    }
}

fn linear_slope(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    if n < 2.0 {
        return 0.0;
    }
    let x_mean = (n - 1.0) / 2.0;
    let y_mean = values.iter().sum::<f64>() / n;
    let numerator: f64 = values
        .iter()
        .enumerate()
        .map(|(i, &y)| (i as f64 - x_mean) * (y - y_mean))
        .sum();
    let denominator: f64 = (0..values.len()).map(|i| (i as f64 - x_mean).powi(2)).sum();
    if denominator.abs() < 1e-15 {
        0.0
    } else {
        numerator / denominator
    }
}

fn classify_slope(slope: f64, threshold: f64) -> TrendDirection {
    if slope > threshold {
        TrendDirection::Rising
    } else if slope < -threshold {
        TrendDirection::Falling
    } else {
        TrendDirection::Stable
    }
}
