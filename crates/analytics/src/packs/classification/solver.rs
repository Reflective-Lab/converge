use super::types::*;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct LogisticClassifier;

impl LogisticClassifier {
    pub fn solve(
        &self,
        input: &ClassificationInput,
        spec: &ProblemSpec,
    ) -> Result<(ClassificationOutput, SolverReport)> {
        let (pos_label, neg_label) = input
            .labels
            .clone()
            .unwrap_or_else(|| ("positive".to_string(), "negative".to_string()));

        let mut positive_count = 0usize;
        let mut negative_count = 0usize;

        let predictions: Vec<ClassifiedRecord> = input
            .records
            .iter()
            .enumerate()
            .map(|(i, record)| {
                let logit: f64 = record
                    .iter()
                    .zip(&input.weights)
                    .map(|(x, w)| x * w)
                    .sum::<f64>()
                    + input.bias;

                let probability = sigmoid(logit);
                let label = if probability >= input.threshold {
                    positive_count += 1;
                    pos_label.clone()
                } else {
                    negative_count += 1;
                    neg_label.clone()
                };

                ClassifiedRecord {
                    index: i,
                    probability,
                    label,
                }
            })
            .collect();

        let total = predictions.len();
        let output = ClassificationOutput {
            predictions,
            positive_count,
            negative_count,
            total,
        };

        // Confidence based on how decisive the classifications are
        let avg_decisiveness: f64 = output
            .predictions
            .iter()
            .map(|p| (p.probability - 0.5).abs() * 2.0)
            .sum::<f64>()
            / total as f64;

        let confidence = avg_decisiveness.clamp(0.3, 0.95);

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("logistic-classifier-v1", confidence, replay);

        Ok((output, report))
    }
}

fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}
