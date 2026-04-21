use super::types::*;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct WeightedScoringSolver;

impl WeightedScoringSolver {
    pub fn solve(
        &self,
        input: &RankingInput,
        spec: &ProblemSpec,
    ) -> Result<(RankingOutput, SolverReport)> {
        let dim = input.weights.len();
        let n = input.items.len();

        // Normalize weights to sum to 1.0
        let weight_sum: f64 = input.weights.iter().sum();
        let weights: Vec<f64> = if weight_sum > 0.0 {
            input.weights.iter().map(|w| w / weight_sum).collect()
        } else {
            vec![1.0 / dim as f64; dim]
        };

        // Min-max normalize each criterion
        let mut mins = vec![f64::INFINITY; dim];
        let mut maxs = vec![f64::NEG_INFINITY; dim];
        for item in &input.items {
            for (j, &score) in item.scores.iter().enumerate() {
                mins[j] = mins[j].min(score);
                maxs[j] = maxs[j].max(score);
            }
        }

        let mut scored: Vec<(usize, f64, Vec<f64>)> = Vec::with_capacity(n);
        for (i, item) in input.items.iter().enumerate() {
            let mut normalized = Vec::with_capacity(dim);
            let mut composite = 0.0;

            for j in 0..dim {
                let range = maxs[j] - mins[j];
                let norm = if range > 0.0 {
                    (item.scores[j] - mins[j]) / range
                } else {
                    0.5
                };
                let directed = if input.higher_is_better[j] {
                    norm
                } else {
                    1.0 - norm
                };
                normalized.push(directed);
                composite += weights[j] * directed;
            }

            scored.push((i, composite, normalized));
        }

        // Sort descending by composite score
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let total_items = n;
        let mut ranked: Vec<RankedItem> = scored
            .into_iter()
            .enumerate()
            .map(|(rank, (idx, composite, normalized))| RankedItem {
                id: input.items[idx].id.clone(),
                rank: rank + 1,
                composite_score: composite,
                normalized_scores: normalized,
            })
            .collect();

        if let Some(top_k) = input.top_k {
            ranked.truncate(top_k);
        }

        let max_score = ranked.first().map(|r| r.composite_score).unwrap_or(0.0);
        let min_score = ranked.last().map(|r| r.composite_score).unwrap_or(0.0);
        let confidence = (max_score - min_score).clamp(0.3, 0.95);

        let output = RankingOutput {
            ranked,
            total_items,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("weighted-scoring-v1", confidence, replay);

        Ok((output, report))
    }
}
