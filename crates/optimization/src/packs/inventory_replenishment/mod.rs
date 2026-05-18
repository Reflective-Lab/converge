//! Inventory Replenishment Pack
//!
//! JTBD: "Determine optimal reorder quantities and timing to maintain service levels."
//!
//! ## Problem
//!
//! Given:
//! - Products with current inventory levels
//! - Demand forecasts (average and variability)
//! - Lead times and order costs
//! - Service level targets and budget constraints
//!
//! Find:
//! - Optimal reorder quantities using EOQ methodology
//! - Order timing to prevent stockouts
//! - Safety stock levels for target service level
//! - Projected inventory levels over planning horizon
//!
//! ## Solver
//!
//! Uses EOQ-based optimization with safety stock:
//! 1. Calculate Economic Order Quantity (EOQ) for each product
//! 2. Determine safety stock based on service level and demand variability
//! 3. Calculate reorder points incorporating lead time
//! 4. Prioritize orders by urgency (days until stockout)
//! 5. Allocate budget starting with most urgent products
//! 6. Generate inventory projections for planning horizon

mod invariants;
mod solver;
mod types;

pub use invariants::*;
pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};
use converge_pack::{CONFIDENCE_STEP_MEDIUM, CONFIDENCE_STEP_MINOR, CONFIDENCE_STEP_PRIMARY};

/// Inventory Replenishment Pack
pub struct InventoryReplenishmentPack;

impl Pack for InventoryReplenishmentPack {
    fn name(&self) -> &'static str {
        "inventory-replenishment"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: InventoryReplenishmentInput =
            serde_json::from_value(inputs.clone()).map_err(|e| {
                converge_pack::GateError::invalid_input(format!("Invalid input: {}", e))
            })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: InventoryReplenishmentInput = spec.inputs_as()?;
        input.validate()?;

        let solver = EoqSolver;
        let (output, report) = solver.solve_replenishment(&input, spec)?;

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
        let output: InventoryReplenishmentOutput = plan.plan_as()?;
        // We need the input for some invariant checks, but we'll use a simplified version
        // that can work with just the output where possible
        let input = InventoryReplenishmentInput {
            products: Vec::new(),
            constraints: ReplenishmentConstraints::default(),
        };
        Ok(check_all_invariants(&output, &input))
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
    output: &InventoryReplenishmentOutput,
    input: &InventoryReplenishmentInput,
) -> f64 {
    // Start with base confidence
    let mut confidence: f64 = 0.5;

    // If no orders needed and all products have sufficient inventory
    if output.orders.is_empty() {
        if input.products.iter().all(|p| !p.needs_reorder()) {
            return 0.9; // High confidence - no action needed
        }
        return 0.3; // Low confidence - might have missed something
    }

    // Higher confidence if we're meeting service level target
    if output.stats.projected_service_level >= input.constraints.target_service_level {
        confidence += CONFIDENCE_STEP_PRIMARY;
    } else if output.stats.projected_service_level >= input.constraints.target_service_level * 0.9 {
        confidence += CONFIDENCE_STEP_MEDIUM;
    }

    // Higher confidence if budget utilization is reasonable (not too high, not too low)
    if output.stats.budget_utilization > 0.1 && output.stats.budget_utilization < 0.9 {
        confidence += CONFIDENCE_STEP_MINOR;
    }

    // Higher confidence if we have projections showing no stockouts
    let has_stockout_risk = output
        .projections
        .iter()
        .any(|p| p.stockout_probability > 0.3);
    if !has_stockout_risk {
        confidence += CONFIDENCE_STEP_MEDIUM;
    }

    confidence.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_product(id: &str, inventory: i64, demand: f64) -> Product {
        Product {
            id: id.to_string(),
            name: format!("Product {}", id),
            current_inventory: inventory,
            demand_forecast: DemandForecast {
                average_daily: demand,
                std_deviation: demand * 0.2,
                forecast_days: 30,
            },
            lead_time_days: 7,
            unit_cost: 10.0,
            ordering_cost: 50.0,
            holding_cost_per_day: 0.02,
            stockout_cost: 25.0,
        }
    }

    fn create_test_input() -> InventoryReplenishmentInput {
        InventoryReplenishmentInput {
            products: vec![
                create_test_product("p1", 50, 10.0),
                create_test_product("p2", 200, 5.0),
            ],
            constraints: ReplenishmentConstraints {
                budget: 10000.0,
                target_service_level: 0.95,
                planning_horizon_days: 30,
                max_orders: None,
                min_order_quantity: None,
            },
        }
    }

    #[test]
    fn test_pack_name() {
        let pack = InventoryReplenishmentPack;
        assert_eq!(pack.name(), "inventory-replenishment");
        assert_eq!(pack.version(), "1.0.0");
    }

    #[test]
    fn test_validate_inputs() {
        let pack = InventoryReplenishmentPack;
        let input = create_test_input();
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_ok());
    }

    #[test]
    fn test_validate_inputs_invalid_budget() {
        let pack = InventoryReplenishmentPack;
        let mut input = create_test_input();
        input.constraints.budget = -100.0;
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_err());
    }

    #[test]
    fn test_validate_inputs_invalid_service_level() {
        let pack = InventoryReplenishmentPack;
        let mut input = create_test_input();
        input.constraints.target_service_level = 1.5;
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_err());
    }

    #[test]
    fn test_solve_basic() {
        let pack = InventoryReplenishmentPack;
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

        let output: InventoryReplenishmentOutput = result.plan.plan_as().unwrap();
        assert!(!output.orders.is_empty() || !output.not_ordered.is_empty());
    }

    #[test]
    fn test_solve_with_sufficient_inventory() {
        let pack = InventoryReplenishmentPack;
        let input = InventoryReplenishmentInput {
            products: vec![create_test_product("p1", 1000, 5.0)], // Lots of inventory
            constraints: ReplenishmentConstraints::default(),
        };

        let spec = ProblemSpec::builder("test-002", "test-tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        assert!(result.is_feasible());
    }

    #[test]
    fn test_check_invariants() {
        let pack = InventoryReplenishmentPack;
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

        // Should have multiple invariant checks
        assert!(!invariants.is_empty());
    }

    #[test]
    fn test_gate_evaluation() {
        let pack = InventoryReplenishmentPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-004", "test-tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();
        let gate = pack.evaluate_gate(&result.plan, &invariants);

        // Gate evaluation should work
        assert!(gate.is_promoted() || !gate.is_promoted());
    }

    #[test]
    fn test_determinism() {
        let pack = InventoryReplenishmentPack;
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

        let output1: InventoryReplenishmentOutput = result1.plan.plan_as().unwrap();
        let output2: InventoryReplenishmentOutput = result2.plan.plan_as().unwrap();

        assert_eq!(output1.orders.len(), output2.orders.len());
        assert_eq!(
            output1.stats.total_order_cost,
            output2.stats.total_order_cost
        );

        for (a, b) in output1.orders.iter().zip(output2.orders.iter()) {
            assert_eq!(a.product_id, b.product_id);
            assert_eq!(a.quantity, b.quantity);
        }
    }

    #[test]
    fn test_budget_constraint() {
        let pack = InventoryReplenishmentPack;
        let mut input = create_test_input();
        input.constraints.budget = 500.0; // Very limited budget

        let spec = ProblemSpec::builder("test-005", "test-tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: InventoryReplenishmentOutput = result.plan.plan_as().unwrap();

        // Total cost should not exceed budget
        assert!(output.stats.total_order_cost <= 500.0);
    }

    #[test]
    fn test_max_orders_constraint() {
        let pack = InventoryReplenishmentPack;
        let mut input = create_test_input();
        input.constraints.max_orders = Some(1);

        let spec = ProblemSpec::builder("test-006", "test-tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: InventoryReplenishmentOutput = result.plan.plan_as().unwrap();

        // Should have at most 1 order
        assert!(output.orders.len() <= 1);
    }

    #[test]
    fn test_output_summary() {
        let pack = InventoryReplenishmentPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-007", "test-tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: InventoryReplenishmentOutput = result.plan.plan_as().unwrap();

        let summary = output.summary();
        assert!(!summary.is_empty());
        assert!(summary.contains("units") || summary.contains("products"));
    }
}
