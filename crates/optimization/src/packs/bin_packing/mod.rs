//! Bin Packing Pack
//!
//! Pack items into bins of fixed capacity minimizing bins used.

mod solver;
mod types;

pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

pub struct BinPackingPack;

impl Pack for BinPackingPack {
    fn name(&self) -> &'static str {
        "bin-packing"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: BinPackingInput = serde_json::from_value(inputs.clone())
            .map_err(|e| converge_pack::GateError::invalid_input(format!("Invalid input: {e}")))?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        &[]
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: BinPackingInput = spec.inputs_as()?;
        input.validate()?;

        let solver = FirstFitDecreasingSolver;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));
        let confidence = 0.8;

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

    fn check_invariants(&self, _plan: &ProposedPlan) -> Result<Vec<InvariantResult>> {
        Ok(vec![])
    }

    fn evaluate_gate(
        &self,
        _plan: &ProposedPlan,
        invariant_results: &[InvariantResult],
    ) -> PromotionGate {
        default_gate_evaluation(invariant_results, self.invariants())
    }
}
