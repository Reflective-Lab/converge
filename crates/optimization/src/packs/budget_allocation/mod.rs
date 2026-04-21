//! Budget Allocation Pack
//!
//! JTBD: "Allocate budgets across categories maximizing ROI under constraints."
//!
//! ## Problem
//!
//! Given:
//! - Total budget to allocate
//! - Categories with ROI estimates and constraints
//! - Min/max allocation limits
//!
//! Find:
//! - Optimal allocation maximizing expected return
//!
//! ## Solver
//!
//! Uses efficiency-based allocation:
//! 1. Filter categories meeting ROI threshold
//! 2. Sort by efficiency score (ROI * priority)
//! 3. Allocate minimum to each qualifying category
//! 4. Distribute remaining proportionally by efficiency

mod invariants;
mod solver;
mod types;

pub use invariants::*;
pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

/// Budget Allocation Pack
pub struct BudgetAllocationPack;

impl Pack for BudgetAllocationPack {
    fn name(&self) -> &'static str {
        "budget-allocation"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: BudgetAllocationInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_pack::GateError::invalid_input(format!("Invalid input: {}", e))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: BudgetAllocationInput = spec.inputs_as()?;
        input.validate()?;

        let solver = EfficiencySolver;
        let (output, report) = solver.solve_allocation(&input, spec)?;

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
        let output: BudgetAllocationOutput = plan.plan_as()?;
        // We need total budget for validation - use allocated + remaining
        let total_budget = output.total_allocated + output.budget_remaining;
        Ok(check_all_invariants(&output, total_budget))
    }

    fn evaluate_gate(
        &self,
        _plan: &ProposedPlan,
        invariant_results: &[InvariantResult],
    ) -> PromotionGate {
        default_gate_evaluation(invariant_results, self.invariants())
    }
}

fn calculate_confidence(output: &BudgetAllocationOutput, input: &BudgetAllocationInput) -> f64 {
    if output.allocations.is_empty() {
        return 0.0;
    }

    let mut confidence: f64 = 0.5;

    // Higher confidence if ROI is positive
    if output.portfolio_roi > 0.0 {
        confidence += 0.2;
    }

    // Higher confidence if budget well utilized
    let utilization = output.total_allocated / input.total_budget;
    if utilization >= 0.8 {
        confidence += 0.2;
    } else if utilization >= 0.5 {
        confidence += 0.1;
    }

    // Higher confidence if we funded multiple categories
    if output.allocations.len() >= 2 {
        confidence += 0.1;
    }

    confidence.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> BudgetAllocationInput {
        BudgetAllocationInput {
            total_budget: 100000.0,
            categories: vec![
                BudgetCategory {
                    id: "marketing".to_string(),
                    name: "Marketing".to_string(),
                    expected_roi: 0.20,
                    priority_weight: 2.0,
                    min_allocation: 10000.0,
                    max_allocation: 50000.0,
                },
                BudgetCategory {
                    id: "rnd".to_string(),
                    name: "R&D".to_string(),
                    expected_roi: 0.25,
                    priority_weight: 2.0,
                    min_allocation: 15000.0,
                    max_allocation: 60000.0,
                },
            ],
            constraints: AllocationConstraints {
                max_categories: None,
                min_roi_threshold: 0.05,
                allow_partial: false,
            },
        }
    }

    #[test]
    fn test_pack_name() {
        let pack = BudgetAllocationPack;
        assert_eq!(pack.name(), "budget-allocation");
        assert_eq!(pack.version(), "1.0.0");
    }

    #[test]
    fn test_validate_inputs() {
        let pack = BudgetAllocationPack;
        let input = create_test_input();
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_ok());
    }

    #[test]
    fn test_solve_basic() {
        let pack = BudgetAllocationPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-001", "test-tenant")
            .objective(ObjectiveSpec::maximize("roi"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        assert!(result.is_feasible());

        let output: BudgetAllocationOutput = result.plan.plan_as().unwrap();
        assert!(!output.allocations.is_empty());
    }

    #[test]
    fn test_check_invariants() {
        let pack = BudgetAllocationPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-002", "test-tenant")
            .objective(ObjectiveSpec::maximize("roi"))
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
        let pack = BudgetAllocationPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-003", "test-tenant")
            .objective(ObjectiveSpec::maximize("roi"))
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
        let pack = BudgetAllocationPack;
        let input = create_test_input();

        let spec1 = ProblemSpec::builder("test-a", "tenant")
            .objective(ObjectiveSpec::maximize("roi"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let spec2 = ProblemSpec::builder("test-b", "tenant")
            .objective(ObjectiveSpec::maximize("roi"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let result1 = pack.solve(&spec1).unwrap();
        let result2 = pack.solve(&spec2).unwrap();

        let output1: BudgetAllocationOutput = result1.plan.plan_as().unwrap();
        let output2: BudgetAllocationOutput = result2.plan.plan_as().unwrap();

        assert!((output1.total_allocated - output2.total_allocated).abs() < 0.01);
    }
}
