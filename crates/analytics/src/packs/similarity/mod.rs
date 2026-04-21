mod solver;
mod types;

pub use solver::*;
pub use types::*;

use converge_optimization::packs::{
    InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation,
};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

pub struct SimilarityPack;

impl Pack for SimilarityPack {
    fn name(&self) -> &'static str {
        "similarity"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: SimilarityInput = serde_json::from_value(inputs.clone())
            .map_err(|e| converge_pack::GateError::invalid_input(format!("Invalid input: {e}")))?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        static INVARIANTS: std::sync::LazyLock<Vec<InvariantDef>> = std::sync::LazyLock::new(
            || {
                vec![
                    InvariantDef::critical("valid-scores", "All similarity scores must be finite"),
                    InvariantDef::advisory(
                        "low-discrimination",
                        "All pairs have near-identical scores — features may not differentiate items",
                    ),
                ]
            },
        );
        &INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: SimilarityInput = spec.inputs_as()?;
        input.validate()?;

        let solver = PairwiseSimilaritySolver;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));

        let confidence = if output.pairs.len() >= 2 {
            let max_s = output.pairs.first().map(|p| p.score).unwrap_or(0.0);
            let min_s = output.pairs.last().map(|p| p.score).unwrap_or(0.0);
            (max_s - min_s).clamp(0.3, 0.95)
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
        let output: SimilarityOutput = serde_json::from_value(plan.plan.clone())
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;

        let mut results = vec![];

        let all_finite = output.pairs.iter().all(|p| p.score.is_finite());
        if all_finite {
            results.push(InvariantResult::pass("valid-scores"));
        } else {
            results.push(InvariantResult::fail(
                "valid-scores",
                converge_pack::gate::Violation::new(
                    "valid-scores",
                    1.0,
                    "Non-finite similarity scores",
                ),
            ));
        }

        if output.pairs.len() >= 2 {
            let max_s = output.pairs.first().map(|p| p.score).unwrap_or(0.0);
            let min_s = output.pairs.last().map(|p| p.score).unwrap_or(0.0);
            if (max_s - min_s).abs() < 0.01 {
                results.push(InvariantResult::fail(
                    "low-discrimination",
                    converge_pack::gate::Violation::new(
                        "low-discrimination",
                        max_s - min_s,
                        "Similarity scores are nearly identical across all pairs",
                    ),
                ));
            } else {
                results.push(InvariantResult::pass("low-discrimination"));
            }
        } else {
            results.push(InvariantResult::pass("low-discrimination"));
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
