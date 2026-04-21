use super::types::*;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct KMeansSolver;

impl KMeansSolver {
    pub fn solve(
        &self,
        input: &SegmentationInput,
        spec: &ProblemSpec,
    ) -> Result<(SegmentationOutput, SolverReport)> {
        let n = input.records.len();
        let dim = input.records[0].len();
        let k = input.k;

        let mut centroids: Vec<Vec<f64>> = if let Some(seed) = input.seed {
            // Deterministic pseudo-random selection using seed
            (0..k)
                .map(|i| {
                    let idx = ((seed
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(i as u64))
                        % n as u64) as usize;
                    input.records[idx].clone()
                })
                .collect()
        } else {
            // Evenly spaced selection
            (0..k).map(|i| input.records[i * n / k].clone()).collect()
        };

        let mut assignments = vec![0usize; n];
        let mut iterations_used = 0;

        for _ in 0..input.max_iterations {
            iterations_used += 1;
            let mut changed = false;

            // Assign each record to nearest centroid
            for (i, record) in input.records.iter().enumerate() {
                let nearest = nearest_centroid(record, &centroids);
                if assignments[i] != nearest {
                    assignments[i] = nearest;
                    changed = true;
                }
            }

            if !changed {
                break;
            }

            // Recompute centroids
            let mut sums = vec![vec![0.0; dim]; k];
            let mut counts = vec![0usize; k];

            for (i, record) in input.records.iter().enumerate() {
                let cluster = assignments[i];
                counts[cluster] += 1;
                for (j, &val) in record.iter().enumerate() {
                    sums[cluster][j] += val;
                }
            }

            for c in 0..k {
                if counts[c] > 0 {
                    for j in 0..dim {
                        centroids[c][j] = sums[c][j] / counts[c] as f64;
                    }
                }
            }
        }

        // Compute inertia
        let inertia: f64 = input
            .records
            .iter()
            .enumerate()
            .map(|(i, record)| euclidean_distance_sq(record, &centroids[assignments[i]]))
            .sum();

        let output = SegmentationOutput {
            assignments,
            centroids,
            iterations_used,
            inertia,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("k-means-v1", inertia, replay);

        Ok((output, report))
    }
}

fn euclidean_distance_sq(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum()
}

fn nearest_centroid(record: &[f64], centroids: &[Vec<f64>]) -> usize {
    centroids
        .iter()
        .enumerate()
        .map(|(i, c)| (i, euclidean_distance_sq(record, c)))
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(i, _)| i)
        .unwrap_or(0)
}
