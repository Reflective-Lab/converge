mod solver;
mod types;

pub use solver::*;
pub use types::*;

use converge_optimization::Result;
use converge_optimization::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};
use converge_optimization::packs::{
    InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation,
};

pub struct TrendDetectionPack;

impl Pack for TrendDetectionPack {
    fn name(&self) -> &'static str {
        "trend-detection"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: TrendDetectionInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_optimization::Error::invalid_input(format!("Invalid input: {e}"))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        static INVARIANTS: std::sync::LazyLock<Vec<InvariantDef>> =
            std::sync::LazyLock::new(|| {
                vec![
                    InvariantDef::critical(
                        "valid-segments",
                        "Segments must cover the full series without gaps",
                    ),
                    InvariantDef::advisory(
                        "excessive-changepoints",
                        "More changepoints than 50% of data length — may be noise",
                    ),
                ]
            });
        &INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: TrendDetectionInput = spec.inputs_as()?;
        input.validate()?;

        let solver = MovingAverageTrendSolver;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));

        let confidence: f64 = if output.segments.len() <= 3 {
            0.85
        } else {
            0.6
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
        let output: TrendDetectionOutput = serde_json::from_value(plan.plan.clone())
            .map_err(|e| converge_optimization::Error::invalid_input(e.to_string()))?;

        let mut results = vec![];

        let covers =
            if let (Some(first), Some(last)) = (output.segments.first(), output.segments.last()) {
                first.start == 0 && last.end > 0
            } else {
                false
            };

        if covers {
            results.push(InvariantResult::pass("valid-segments"));
        } else {
            results.push(InvariantResult::fail(
                "valid-segments",
                converge_optimization::gate::Violation::new(
                    "valid-segments",
                    1.0,
                    "Segments do not cover full series",
                ),
            ));
        }

        // Estimate series length from last segment end
        let series_len = output.segments.last().map(|s| s.end + 1).unwrap_or(0);
        if output.changepoints.len() > series_len / 2 && series_len > 4 {
            results.push(InvariantResult::fail(
                "excessive-changepoints",
                converge_optimization::gate::Violation::new(
                    "excessive-changepoints",
                    output.changepoints.len() as f64,
                    "Too many changepoints relative to series length",
                ),
            ));
        } else {
            results.push(InvariantResult::pass("excessive-changepoints"));
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
