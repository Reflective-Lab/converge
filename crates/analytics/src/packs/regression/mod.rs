mod solver;
mod types;

pub use solver::*;
pub use types::*;

use converge_optimization::packs::{
    InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation,
};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

pub struct RegressionPack;

impl Pack for RegressionPack {
    fn name(&self) -> &'static str {
        "regression"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: RegressionInput = serde_json::from_value(inputs.clone())
            .map_err(|e| converge_pack::GateError::invalid_input(format!("Invalid input: {e}")))?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        static INVARIANTS: std::sync::LazyLock<Vec<InvariantDef>> =
            std::sync::LazyLock::new(|| {
                vec![
                    InvariantDef::critical("finite-values", "All predicted values must be finite"),
                    InvariantDef::advisory(
                        "zero-variance",
                        "All predictions are identical — model may be degenerate",
                    ),
                ]
            });
        &INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: RegressionInput = spec.inputs_as()?;
        input.validate()?;

        let solver = LinearRegressionSolver;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));
        let confidence = if output.std_prediction > 0.0 {
            (1.0 / (1.0 + 1.0 / output.std_prediction)).clamp(0.3, 0.95)
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
        let output: RegressionOutput = serde_json::from_value(plan.plan.clone())
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;

        let mut results = vec![];

        let all_finite = output.predictions.iter().all(|p| p.value.is_finite());
        if all_finite {
            results.push(InvariantResult::pass("finite-values"));
        } else {
            results.push(InvariantResult::fail(
                "finite-values",
                converge_pack::gate::Violation::new(
                    "finite-values",
                    1.0,
                    "Non-finite predicted values",
                ),
            ));
        }

        if output.std_prediction < 1e-10 && output.total > 1 {
            results.push(InvariantResult::fail(
                "zero-variance",
                converge_pack::gate::Violation::new(
                    "zero-variance",
                    0.0,
                    "All predictions are identical",
                ),
            ));
        } else {
            results.push(InvariantResult::pass("zero-variance"));
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
