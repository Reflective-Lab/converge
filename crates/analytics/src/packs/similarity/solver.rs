use super::types::*;
use converge_optimization::Result;
use converge_optimization::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct PairwiseSimilaritySolver;

impl PairwiseSimilaritySolver {
    pub fn solve(
        &self,
        input: &SimilarityInput,
        spec: &ProblemSpec,
    ) -> Result<(SimilarityOutput, SolverReport)> {
        let n = input.items.len();
        let total_pairs = n * (n - 1) / 2;

        let mut pairs: Vec<SimilarityPair> = Vec::with_capacity(total_pairs);

        for i in 0..n {
            for j in (i + 1)..n {
                let score = similarity_score(
                    &input.items[i].features,
                    &input.items[j].features,
                    input.metric,
                );
                pairs.push(SimilarityPair {
                    id_a: input.items[i].id.clone(),
                    id_b: input.items[j].id.clone(),
                    score,
                });
            }
        }

        // Sort by score descending (most similar first)
        pairs.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(top_k) = input.top_k {
            pairs.truncate(top_k);
        }

        let output = SimilarityOutput { pairs, total_pairs };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("pairwise-similarity-v1", total_pairs as f64, replay);

        Ok((output, report))
    }
}

fn similarity_score(a: &[f64], b: &[f64], metric: DistanceMetric) -> f64 {
    match metric {
        DistanceMetric::Euclidean => {
            let dist: f64 = a
                .iter()
                .zip(b.iter())
                .map(|(x, y)| (x - y).powi(2))
                .sum::<f64>()
                .sqrt();
            // Convert distance to similarity: 1 / (1 + dist)
            1.0 / (1.0 + dist)
        }
        DistanceMetric::Cosine => {
            let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            let mag_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
            let mag_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
            if mag_a == 0.0 || mag_b == 0.0 {
                0.0
            } else {
                dot / (mag_a * mag_b)
            }
        }
        DistanceMetric::Manhattan => {
            let dist: f64 = a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum();
            1.0 / (1.0 + dist)
        }
    }
}
