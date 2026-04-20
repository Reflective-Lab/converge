mod solver;
mod types;

pub use solver::*;
pub use types::*;

use converge_optimization::Result;
use converge_optimization::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};
use converge_optimization::packs::{
    InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation,
};

pub struct SegmentationPack;

impl Pack for SegmentationPack {
    fn name(&self) -> &'static str {
        "segmentation"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: SegmentationInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_optimization::Error::invalid_input(format!("Invalid input: {e}"))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        static INVARIANTS: std::sync::LazyLock<Vec<InvariantDef>> =
            std::sync::LazyLock::new(|| {
                vec![
                    InvariantDef::critical(
                        "non-empty-clusters",
                        "Every cluster must have at least one member",
                    ),
                    InvariantDef::advisory(
                        "balanced-clusters",
                        "Cluster size below 10% of expected proportion",
                    ),
                ]
            });
        &INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: SegmentationInput = spec.inputs_as()?;
        input.validate()?;

        let solver = KMeansSolver;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));

        // Confidence from cluster quality: lower inertia relative to spread = higher confidence
        let global_mean: Vec<f64> = {
            let dim = input.records[0].len();
            let n = input.records.len() as f64;
            let mut mean = vec![0.0; dim];
            for record in &input.records {
                for (j, &v) in record.iter().enumerate() {
                    mean[j] += v;
                }
            }
            for v in &mut mean {
                *v /= n;
            }
            mean
        };
        let total_variance: f64 = input
            .records
            .iter()
            .map(|r| {
                r.iter()
                    .zip(&global_mean)
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
            })
            .sum();
        let confidence = if total_variance > 0.0 {
            (1.0 - output.inertia / total_variance).clamp(0.3, 0.95)
        } else {
            0.5
        };

        let plan = ProposedPlan::from_payload(
            format!("plan-{}", spec.problem_id),
            self.name(),
            output.summary(),
            &output,
            confidence,
            trace,
        )?;

        Ok(PackSolveResult::new(plan, report))
    }

    fn check_invariants(&self, plan: &ProposedPlan) -> Result<Vec<InvariantResult>> {
        let output: SegmentationOutput = serde_json::from_value(plan.plan.clone())
            .map_err(|e| converge_optimization::Error::invalid_input(e.to_string()))?;

        let k = output.centroids.len();
        let n = output.assignments.len();
        let mut counts = vec![0usize; k];
        for &a in &output.assignments {
            if a < k {
                counts[a] += 1;
            }
        }

        let mut results = vec![];

        let empty_clusters: Vec<usize> = counts
            .iter()
            .enumerate()
            .filter(|(_, c)| **c == 0)
            .map(|(i, _)| i)
            .collect();

        if empty_clusters.is_empty() {
            results.push(InvariantResult::pass("non-empty-clusters"));
        } else {
            results.push(InvariantResult::fail(
                "non-empty-clusters",
                converge_optimization::gate::Violation::new(
                    "non-empty-clusters",
                    empty_clusters.len() as f64,
                    format!("Empty clusters: {:?}", empty_clusters),
                ),
            ));
        }

        let expected_size = n as f64 / k as f64;
        let threshold = expected_size * 0.1;
        let undersized: Vec<usize> = counts
            .iter()
            .enumerate()
            .filter(|(_, c)| (**c as f64) < threshold)
            .map(|(i, _)| i)
            .collect();

        if undersized.is_empty() {
            results.push(InvariantResult::pass("balanced-clusters"));
        } else {
            results.push(InvariantResult::fail(
                "balanced-clusters",
                converge_optimization::gate::Violation::new(
                    "balanced-clusters",
                    undersized.len() as f64,
                    format!("Undersized clusters: {:?}", undersized),
                ),
            ));
        }

        Ok(results)
    }

    fn evaluate_gate(
        &self,
        _plan: &ProposedPlan,
        invariant_results: &[InvariantResult],
    ) -> PromotionGate {
        default_gate_evaluation(invariant_results, self.invariants())
    }
}
