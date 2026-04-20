//! Network Flow Pack
//!
//! Send flow through a network at minimum cost using successive shortest paths.

mod solver;
mod types;

pub use solver::*;
pub use types::*;

use crate::Result;
use crate::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};
use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};

pub struct NetworkFlowPack;

impl Pack for NetworkFlowPack {
    fn name(&self) -> &'static str {
        "network-flow"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: NetworkFlowInput = serde_json::from_value(inputs.clone())
            .map_err(|e| crate::Error::invalid_input(format!("Invalid input: {e}")))?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        &[]
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: NetworkFlowInput = spec.inputs_as()?;
        input.validate()?;

        let solver = SuccessiveShortestPathSolver;
        let (output, report) = solver.solve(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));
        let confidence = if output.total_flow >= input.demand - 1e-9 {
            0.9
        } else {
            0.3
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
