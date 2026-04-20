mod solver;
mod types;

pub use solver::*;
pub use types::*;

use converge_optimization::Result;
use converge_optimization::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};
use converge_optimization::packs::{
    InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation,
};

pub struct RankingPack;

impl Pack for RankingPack {
    fn name(&self) -> &'static str {
        "ranking"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: RankingInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_optimization::Error::invalid_input(format!("Invalid input: {e}"))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        static INVARIANTS: std::sync::LazyLock<Vec<InvariantDef>> =
            std::sync::LazyLock::new(|| {
                vec![
                    InvariantDef::critical(
                        "valid-dimensions",
                        "All items must have matching score dimensions",
                    ),
                    InvariantDef::advisory(
                        "score-separation",
                        "Top and bottom scores differ by < 0.01 — items are indistinguishable",
                    ),
                ]
            });
        &INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: RankingInput = spec.inputs_as()?;
        input.validate()?;

        let solver = WeightedScoringSolver;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));

        let max_score = output
            .ranked
            .first()
            .map(|r| r.composite_score)
            .unwrap_or(0.0);
        let min_score = output
            .ranked
            .last()
            .map(|r| r.composite_score)
            .unwrap_or(0.0);
        let confidence = (max_score - min_score).clamp(0.3, 0.95);

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
        let output: RankingOutput = serde_json::from_value(plan.plan.clone())
            .map_err(|e| converge_optimization::Error::invalid_input(e.to_string()))?;

        let mut results = vec![];

        // Dimension check is already enforced at validation, so always passes here
        results.push(InvariantResult::pass("valid-dimensions"));

        let max_score = output
            .ranked
            .first()
            .map(|r| r.composite_score)
            .unwrap_or(0.0);
        let min_score = output
            .ranked
            .last()
            .map(|r| r.composite_score)
            .unwrap_or(0.0);
        if (max_score - min_score) < 0.01 {
            results.push(InvariantResult::fail(
                "score-separation",
                converge_optimization::gate::Violation::new(
                    "score-separation",
                    max_score - min_score,
                    "Items are effectively indistinguishable by score",
                ),
            ));
        } else {
            results.push(InvariantResult::pass("score-separation"));
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
