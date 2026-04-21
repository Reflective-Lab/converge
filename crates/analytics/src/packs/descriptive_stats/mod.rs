mod solver;
mod types;

pub use solver::*;
pub use types::*;

use converge_optimization::packs::{
    InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation,
};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

pub struct DescriptiveStatsPack;

impl Pack for DescriptiveStatsPack {
    fn name(&self) -> &'static str {
        "descriptive-stats"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: DescriptiveStatsInput = serde_json::from_value(inputs.clone())
            .map_err(|e| converge_pack::GateError::invalid_input(format!("Invalid input: {e}")))?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        static INVARIANTS: std::sync::LazyLock<Vec<InvariantDef>> =
            std::sync::LazyLock::new(|| {
                vec![
                    InvariantDef::critical(
                        "finite-statistics",
                        "All computed statistics must be finite",
                    ),
                    InvariantDef::advisory(
                        "high-skew",
                        "Absolute skewness > 2 — data is heavily skewed",
                    ),
                ]
            });
        &INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: DescriptiveStatsInput = spec.inputs_as()?;
        input.validate()?;

        let solver = DescriptiveStatsSolver;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));

        // Deterministic computation — always high confidence
        let plan = ProposedPlan::from_payload(
            format!("plan-{}", spec.problem_id),
            self.name(),
            output.summary(),
            &output,
            0.95,
            trace,
        )?;

        Ok(PackSolveResult::new(plan, report))
    }

    fn check_invariants(&self, plan: &ProposedPlan) -> Result<Vec<InvariantResult>> {
        let output: DescriptiveStatsOutput = serde_json::from_value(plan.plan.clone())
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;

        let mut results = vec![];

        let all_finite = output.mean.is_finite()
            && output.median.is_finite()
            && output.std_dev.is_finite()
            && output.variance.is_finite()
            && output.skewness.is_finite()
            && output.kurtosis.is_finite();

        if all_finite {
            results.push(InvariantResult::pass("finite-statistics"));
        } else {
            results.push(InvariantResult::fail(
                "finite-statistics",
                converge_pack::gate::Violation::new(
                    "finite-statistics",
                    1.0,
                    "Non-finite values in computed statistics",
                ),
            ));
        }

        if output.skewness.abs() > 2.0 {
            results.push(InvariantResult::fail(
                "high-skew",
                converge_pack::gate::Violation::new(
                    "high-skew",
                    output.skewness.abs(),
                    format!("Skewness of {:.2} indicates heavy skew", output.skewness),
                ),
            ));
        } else {
            results.push(InvariantResult::pass("high-skew"));
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
