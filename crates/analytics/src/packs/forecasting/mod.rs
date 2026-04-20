mod solver;
mod types;

pub use solver::*;
pub use types::*;

use converge_optimization::Result;
use converge_optimization::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};
use converge_optimization::packs::{
    InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation,
};

pub struct ForecastingPack;

impl Pack for ForecastingPack {
    fn name(&self) -> &'static str {
        "forecasting"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: ForecastingInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_optimization::Error::invalid_input(format!("Invalid input: {e}"))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        static INVARIANTS: std::sync::LazyLock<Vec<InvariantDef>> =
            std::sync::LazyLock::new(|| {
                vec![
                    InvariantDef::critical(
                        "finite-predictions",
                        "All predicted values must be finite",
                    ),
                    InvariantDef::advisory(
                        "wide-intervals",
                        "Confidence intervals exceed 2x the data range — low predictive power",
                    ),
                ]
            });
        &INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: ForecastingInput = spec.inputs_as()?;
        input.validate()?;

        let solver = ExponentialSmoothingSolver;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));
        let confidence = if output.residual_std > 0.0 {
            (1.0 / (1.0 + output.residual_std)).clamp(0.3, 0.95)
        } else {
            0.95
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
        let output: ForecastingOutput = serde_json::from_value(plan.plan.clone())
            .map_err(|e| converge_optimization::Error::invalid_input(e.to_string()))?;

        let mut results = vec![];

        let all_finite = output
            .predictions
            .iter()
            .all(|p| p.value.is_finite() && p.lower.is_finite() && p.upper.is_finite());

        if all_finite {
            results.push(InvariantResult::pass("finite-predictions"));
        } else {
            results.push(InvariantResult::fail(
                "finite-predictions",
                converge_optimization::gate::Violation::new(
                    "finite-predictions",
                    1.0,
                    "Non-finite values in predictions",
                ),
            ));
        }

        if let (Some(last), Some(first)) = (output.predictions.last(), output.predictions.first()) {
            let max_width = last.upper - last.lower;
            let first_width = first.upper - first.lower;
            if max_width > first_width * 4.0 && first_width > 0.0 {
                results.push(InvariantResult::fail(
                    "wide-intervals",
                    converge_optimization::gate::Violation::new(
                        "wide-intervals",
                        max_width,
                        "Confidence intervals grow too wide over horizon",
                    ),
                ));
            } else {
                results.push(InvariantResult::pass("wide-intervals"));
            }
        } else {
            results.push(InvariantResult::pass("wide-intervals"));
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
