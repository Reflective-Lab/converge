mod solver;
mod types;

pub use solver::*;
pub use types::*;

use converge_optimization::packs::{
    InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation,
};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

pub struct ClassificationPack;

impl Pack for ClassificationPack {
    fn name(&self) -> &'static str {
        "classification"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: ClassificationInput = serde_json::from_value(inputs.clone())
            .map_err(|e| converge_pack::GateError::invalid_input(format!("Invalid input: {e}")))?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        static INVARIANTS: std::sync::LazyLock<Vec<InvariantDef>> =
            std::sync::LazyLock::new(|| {
                vec![
                    InvariantDef::critical(
                        "valid-probabilities",
                        "All probabilities must be in [0, 1]",
                    ),
                    InvariantDef::advisory(
                        "class-imbalance",
                        "One class has > 90% of predictions — model may be degenerate",
                    ),
                ]
            });
        &INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: ClassificationInput = spec.inputs_as()?;
        input.validate()?;

        let solver = LogisticClassifier;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));

        let avg_decisiveness: f64 = output
            .predictions
            .iter()
            .map(|p| (p.probability - 0.5).abs() * 2.0)
            .sum::<f64>()
            / output.total as f64;
        let confidence = avg_decisiveness.clamp(0.3, 0.95);

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
        let output: ClassificationOutput = serde_json::from_value(plan.plan.clone())
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;

        let mut results = vec![];

        let all_valid = output
            .predictions
            .iter()
            .all(|p| (0.0..=1.0).contains(&p.probability));

        if all_valid {
            results.push(InvariantResult::pass("valid-probabilities"));
        } else {
            results.push(InvariantResult::fail(
                "valid-probabilities",
                converge_pack::gate::Violation::new(
                    "valid-probabilities",
                    1.0,
                    "Probability outside [0, 1] range",
                ),
            ));
        }

        let majority = output.positive_count.max(output.negative_count) as f64;
        let ratio = majority / output.total as f64;
        if ratio > 0.9 {
            results.push(InvariantResult::fail(
                "class-imbalance",
                converge_pack::gate::Violation::new(
                    "class-imbalance",
                    ratio,
                    format!("{:.0}% of predictions in one class", ratio * 100.0),
                ),
            ));
        } else {
            results.push(InvariantResult::pass("class-imbalance"));
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
