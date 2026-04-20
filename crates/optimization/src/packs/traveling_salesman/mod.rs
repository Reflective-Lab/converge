//! Traveling Salesman Pack
//!
//! Find shortest tour visiting all cities exactly once.

mod solver;
mod types;

pub use solver::*;
pub use types::*;

use crate::Result;
use crate::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};
use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};

pub struct TravelingSalesmanPack;

impl Pack for TravelingSalesmanPack {
    fn name(&self) -> &'static str {
        "traveling-salesman"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: TspInput = serde_json::from_value(inputs.clone())
            .map_err(|e| crate::Error::invalid_input(format!("Invalid input: {e}")))?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        &[]
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: TspInput = spec.inputs_as()?;
        input.validate()?;

        let solver = NearestNeighborTwoOptSolver;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));
        let confidence = 0.7;

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
