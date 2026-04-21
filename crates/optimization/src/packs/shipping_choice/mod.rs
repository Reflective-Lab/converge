//! Shipping Choice Pack
//!
//! JTBD: "Choose shipping method minimizing total cost subject to SLA."
//!
//! ## Problem
//!
//! Given:
//! - Order details (weight, dimensions, destination)
//! - Available carriers and rates
//! - SLA requirements
//!
//! Find:
//! - Optimal carrier/method selection minimizing cost while meeting SLA
//!
//! ## Solver
//!
//! Uses cost minimization:
//! 1. Filter carriers that can handle the order (hazmat check)
//! 2. Filter carriers that meet SLA requirements
//! 3. Sort by cost (ascending)
//! 4. Select cheapest option with tie-breaking

mod invariants;
mod solver;
mod types;

pub use invariants::*;
pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

/// Shipping Choice Pack
pub struct ShippingChoicePack;

impl Pack for ShippingChoicePack {
    fn name(&self) -> &'static str {
        "shipping-choice"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: ShippingChoiceInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_pack::GateError::invalid_input(format!("Invalid input: {}", e))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: ShippingChoiceInput = spec.inputs_as()?;
        input.validate()?;

        let solver = CostMinimizingSolver;
        let (output, report) = solver.solve_shipping(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));
        let confidence = calculate_confidence(&output);

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
        let output: ShippingChoiceOutput = plan.plan_as()?;
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

fn calculate_confidence(output: &ShippingChoiceOutput) -> f64 {
    if output.selected_carrier.is_none() {
        return 0.0;
    }

    let mut confidence: f64 = 0.6;

    if output.meets_sla {
        confidence += 0.3;
    }

    // Bonus if we have alternatives to compare against
    if !output.alternatives.is_empty() {
        confidence += 0.1;
    }

    confidence.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> ShippingChoiceInput {
        ShippingChoiceInput {
            order: OrderDetails {
                order_id: "ORD-001".to_string(),
                weight_kg: 2.5,
                dimensions_cm: [20.0, 15.0, 10.0],
                destination_zip: "10001".to_string(),
                is_hazmat: false,
            },
            carriers: vec![
                CarrierOption {
                    carrier_id: "ups".to_string(),
                    service_level: "ground".to_string(),
                    cost: 8.99,
                    estimated_days: 5,
                    supports_hazmat: false,
                },
                CarrierOption {
                    carrier_id: "fedex".to_string(),
                    service_level: "express".to_string(),
                    cost: 15.99,
                    estimated_days: 2,
                    supports_hazmat: true,
                },
            ],
            sla_days: 5,
        }
    }

    #[test]
    fn test_pack_name() {
        let pack = ShippingChoicePack;
        assert_eq!(pack.name(), "shipping-choice");
        assert_eq!(pack.version(), "1.0.0");
    }

    #[test]
    fn test_validate_inputs() {
        let pack = ShippingChoicePack;
        let input = create_test_input();
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_ok());
    }

    #[test]
    fn test_solve_basic() {
        let pack = ShippingChoicePack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-001", "test-tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        assert!(result.is_feasible());

        let output: ShippingChoiceOutput = result.plan.plan_as().unwrap();
        assert!(output.selected_carrier.is_some());
        assert_eq!(output.selected_carrier.as_deref(), Some("ups"));
    }

    #[test]
    fn test_check_invariants() {
        let pack = ShippingChoicePack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-002", "test-tenant")
            .objective(ObjectiveSpec::minimize("cost"))
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
        let pack = ShippingChoicePack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-003", "test-tenant")
            .objective(ObjectiveSpec::minimize("cost"))
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
        let pack = ShippingChoicePack;
        let input = create_test_input();

        let spec1 = ProblemSpec::builder("test-a", "tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let spec2 = ProblemSpec::builder("test-b", "tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let result1 = pack.solve(&spec1).unwrap();
        let result2 = pack.solve(&spec2).unwrap();

        let output1: ShippingChoiceOutput = result1.plan.plan_as().unwrap();
        let output2: ShippingChoiceOutput = result2.plan.plan_as().unwrap();

        assert_eq!(output1.selected_carrier, output2.selected_carrier);
        assert_eq!(output1.cost, output2.cost);
    }
}
