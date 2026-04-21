//! Backlog Prioritization Pack
//!
//! JTBD: "Prioritize work under value-risk-effort tradeoffs."
//!
//! ## Problem
//!
//! Given:
//! - Work items with value, risk, effort estimates
//! - Dependencies between items
//! - Capacity constraints
//!
//! Find:
//! - Prioritized backlog using WSJF (Weighted Shortest Job First)
//!
//! ## Solver
//!
//! Uses WSJF scoring:
//! 1. Calculate WSJF = (Business Value + Time Criticality + Risk Reduction) / Effort
//! 2. Sort by WSJF descending
//! 3. Respect dependency ordering
//! 4. Mark items within capacity as included

mod invariants;
mod solver;
mod types;

pub use invariants::*;
pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};
use converge_pack::{CONFIDENCE_STEP_MAJOR, CONFIDENCE_STEP_MINOR};

/// Backlog Prioritization Pack
pub struct BacklogPrioritizationPack;

impl Pack for BacklogPrioritizationPack {
    fn name(&self) -> &'static str {
        "backlog-prioritization"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: BacklogPrioritizationInput =
            serde_json::from_value(inputs.clone()).map_err(|e| {
                converge_pack::GateError::invalid_input(format!("Invalid input: {}", e))
            })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: BacklogPrioritizationInput = spec.inputs_as()?;
        input.validate()?;

        let solver = WsjfSolver;
        let (output, report) = solver.solve_backlog(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));
        let confidence = calculate_confidence(&output, &input);

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
        let output: BacklogPrioritizationOutput = plan.plan_as()?;
        Ok(check_all_invariants(&output))
    }

    fn evaluate_gate(
        &self,
        _plan: &ProposedPlan,
        invariant_results: &[InvariantResult],
    ) -> PromotionGate {
        default_gate_evaluation(invariant_results, self.invariants())
    }
}

fn calculate_confidence(
    output: &BacklogPrioritizationOutput,
    input: &BacklogPrioritizationInput,
) -> f64 {
    if output.ranked_items.is_empty() {
        return 0.0;
    }

    let mut confidence: f64 = 0.5;

    // Higher confidence if we included items in capacity
    if output.included_count > 0 {
        confidence += CONFIDENCE_STEP_MAJOR;
    }

    // Higher confidence if capacity is well utilized
    if output.total_effort > 0 {
        let utilization = output.total_effort as f64 / input.capacity_points as f64;
        if utilization >= 0.7 {
            confidence += CONFIDENCE_STEP_MAJOR;
        } else if utilization >= 0.5 {
            confidence += CONFIDENCE_STEP_MINOR;
        }
    }

    // Higher confidence if total value is good
    if output.total_value >= 100.0 {
        confidence += CONFIDENCE_STEP_MINOR;
    }

    confidence.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> BacklogPrioritizationInput {
        BacklogPrioritizationInput {
            items: vec![
                BacklogItem {
                    id: "feat-1".to_string(),
                    title: "Feature 1".to_string(),
                    business_value: 80.0,
                    time_criticality: 60.0,
                    risk_reduction: 40.0,
                    effort_points: 5,
                    dependencies: vec![],
                },
                BacklogItem {
                    id: "feat-2".to_string(),
                    title: "Feature 2".to_string(),
                    business_value: 40.0,
                    time_criticality: 80.0,
                    risk_reduction: 30.0,
                    effort_points: 2,
                    dependencies: vec![],
                },
            ],
            capacity_points: 10,
        }
    }

    #[test]
    fn test_pack_name() {
        let pack = BacklogPrioritizationPack;
        assert_eq!(pack.name(), "backlog-prioritization");
        assert_eq!(pack.version(), "1.0.0");
    }

    #[test]
    fn test_validate_inputs() {
        let pack = BacklogPrioritizationPack;
        let input = create_test_input();
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_ok());
    }

    #[test]
    fn test_solve_basic() {
        let pack = BacklogPrioritizationPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-001", "test-tenant")
            .objective(ObjectiveSpec::maximize("value"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        assert!(result.is_feasible());

        let output: BacklogPrioritizationOutput = result.plan.plan_as().unwrap();
        assert_eq!(output.ranked_items.len(), 2);
    }

    #[test]
    fn test_check_invariants() {
        let pack = BacklogPrioritizationPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-002", "test-tenant")
            .objective(ObjectiveSpec::maximize("value"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();

        let all_pass = invariants.iter().all(|r| r.passed);
        assert!(all_pass);
    }

    #[test]
    fn test_gate_promotes() {
        let pack = BacklogPrioritizationPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-003", "test-tenant")
            .objective(ObjectiveSpec::maximize("value"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();
        let gate = pack.evaluate_gate(&result.plan, &invariants);

        assert!(gate.is_promoted());
    }

    #[test]
    fn test_determinism() {
        let pack = BacklogPrioritizationPack;
        let input = create_test_input();

        let spec1 = ProblemSpec::builder("test-a", "tenant")
            .objective(ObjectiveSpec::maximize("value"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let spec2 = ProblemSpec::builder("test-b", "tenant")
            .objective(ObjectiveSpec::maximize("value"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let result1 = pack.solve(&spec1).unwrap();
        let result2 = pack.solve(&spec2).unwrap();

        let output1: BacklogPrioritizationOutput = result1.plan.plan_as().unwrap();
        let output2: BacklogPrioritizationOutput = result2.plan.plan_as().unwrap();

        for (a, b) in output1.ranked_items.iter().zip(output2.ranked_items.iter()) {
            assert_eq!(a.item_id, b.item_id);
            assert_eq!(a.rank, b.rank);
        }
    }
}
