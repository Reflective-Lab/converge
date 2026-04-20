mod solver;
mod types;

pub use solver::*;
pub use types::*;

use converge_optimization::Result;
use converge_optimization::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};
use converge_optimization::packs::{
    InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation,
};

pub struct AnomalyDetectionPack;

impl Pack for AnomalyDetectionPack {
    fn name(&self) -> &'static str {
        "anomaly-detection"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: AnomalyDetectionInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_optimization::Error::invalid_input(format!("Invalid input: {e}"))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        static INVARIANTS: std::sync::LazyLock<Vec<InvariantDef>> =
            std::sync::LazyLock::new(|| {
                vec![
                    InvariantDef::critical(
                        "valid-statistics",
                        "Standard deviation must be > 0 (constant data cannot detect anomalies)",
                    ),
                    InvariantDef::advisory(
                        "anomaly-ratio",
                        "Anomaly count exceeds 50% of total points — threshold may be too low",
                    ),
                ]
            });
        &INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: AnomalyDetectionInput = spec.inputs_as()?;
        input.validate()?;

        let solver = ZScoreSolver;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));
        let confidence =
            (1.0 - (output.anomaly_count as f64 / output.total_points as f64)).clamp(0.3, 0.95);

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
        let output: AnomalyDetectionOutput = serde_json::from_value(plan.plan.clone())
            .map_err(|e| converge_optimization::Error::invalid_input(e.to_string()))?;

        let mut results = vec![];

        if output.std_dev <= 0.0 {
            results.push(InvariantResult::fail(
                "valid-statistics",
                converge_optimization::gate::Violation::new(
                    "valid-statistics",
                    1.0,
                    "std_dev is zero",
                ),
            ));
        } else {
            results.push(InvariantResult::pass("valid-statistics"));
        }

        let ratio = output.anomaly_count as f64 / output.total_points as f64;
        if ratio > 0.5 {
            results.push(InvariantResult::fail(
                "anomaly-ratio",
                converge_optimization::gate::Violation::new(
                    "anomaly-ratio",
                    ratio,
                    format!("{:.0}% of points flagged as anomalies", ratio * 100.0),
                ),
            ));
        } else {
            results.push(InvariantResult::pass("anomaly-ratio"));
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
