//! Inventory Rebalancing Pack
//!
//! Plans inventory transfers to optimize service levels while minimizing costs.
//!
//! ## Problem
//!
//! Given:
//! - Locations with current inventory levels
//! - Target levels and safety stock requirements
//! - Transfer costs between locations
//! - Budget and transfer constraints
//!
//! Find:
//! - Set of transfers that improve service levels within budget
//!
//! ## Solver
//!
//! Uses greedy cost/service optimization:
//! 1. Calculate deficit/surplus for each (location, product)
//! 2. Sort deficits by urgency (most negative first)
//! 3. For each deficit, find cheapest transfer from surplus locations
//! 4. Stop when budget or transfer limits reached

mod invariants;
mod solver;
mod types;

pub use invariants::*;
pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

/// Inventory Rebalancing Pack
pub struct InventoryRebalancingPack;

impl Pack for InventoryRebalancingPack {
    fn name(&self) -> &'static str {
        "inventory-rebalancing"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: InventoryRebalancingInput =
            serde_json::from_value(inputs.clone()).map_err(|e| {
                converge_pack::GateError::invalid_input(format!("Invalid input: {}", e))
            })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: InventoryRebalancingInput = spec.inputs_as()?;
        input.validate()?;

        let solver = GreedyRebalancingSolver;
        let (output, report) = solver.solve_rebalancing(&input, spec)?;

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
        let output: InventoryRebalancingOutput = plan.plan_as()?;
        Ok(check_all_invariants(&output))
    }

    fn evaluate_gate(
        &self,
        plan: &ProposedPlan,
        invariant_results: &[InvariantResult],
    ) -> PromotionGate {
        // Check for critical financial threshold
        if let Ok(output) = plan.plan_as::<InventoryRebalancingOutput>() {
            // If cost is very high relative to improvement, require review
            if output.total_cost > 0.0 && output.service_level_improvement <= 0.0 {
                return PromotionGate::reject("Cost incurred with no service improvement");
            }
        }

        default_gate_evaluation(invariant_results, self.invariants())
    }
}

/// Calculate confidence score based on output quality
fn calculate_confidence(
    output: &InventoryRebalancingOutput,
    input: &InventoryRebalancingInput,
) -> f64 {
    if output.transfers.is_empty() {
        // No transfers might be correct (already balanced)
        return 0.6;
    }

    let mut confidence = 0.5;

    // Bonus for positive service improvement
    if output.service_level_improvement > 0.0 {
        confidence += 0.2_f64.min(output.service_level_improvement * 0.1);
    }

    // Bonus for staying well under budget
    if input.constraints.max_total_cost > 0.0 {
        let budget_usage = output.total_cost / input.constraints.max_total_cost;
        if budget_usage < 0.8 {
            confidence += 0.1;
        }
    }

    // Bonus for using fewer transfers than limit
    if input.constraints.max_total_transfers > 0 {
        let transfer_usage =
            output.transfers.len() as f64 / input.constraints.max_total_transfers as f64;
        if transfer_usage < 0.8 {
            confidence += 0.1;
        }
    }

    confidence.min(1.0_f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::{ObjectiveSpec, SolveBudgets};

    fn create_test_input() -> InventoryRebalancingInput {
        InventoryRebalancingInput {
            locations: vec![
                Location {
                    id: "warehouse-1".to_string(),
                    name: "Main Warehouse".to_string(),
                    capacity: 1000,
                    location_type: LocationType::Warehouse,
                },
                Location {
                    id: "store-1".to_string(),
                    name: "Store A".to_string(),
                    capacity: 100,
                    location_type: LocationType::Store,
                },
            ],
            products: vec![Product {
                id: "sku-001".to_string(),
                name: "Widget".to_string(),
                unit_weight: 1.0,
                unit_value: 10.0,
            }],
            inventory: vec![
                InventoryLevel {
                    location_id: "warehouse-1".to_string(),
                    product_id: "sku-001".to_string(),
                    quantity: 500,
                    target_quantity: 200,
                    min_quantity: 50,
                    max_quantity: 800,
                },
                InventoryLevel {
                    location_id: "store-1".to_string(),
                    product_id: "sku-001".to_string(),
                    quantity: 10,
                    target_quantity: 50,
                    min_quantity: 20,
                    max_quantity: 80,
                },
            ],
            transfer_costs: vec![TransferCost {
                from_location: "warehouse-1".to_string(),
                to_location: "store-1".to_string(),
                cost_per_unit: 0.5,
                lead_time_hours: 24,
            }],
            constraints: RebalancingConstraints {
                max_total_transfers: 10,
                max_transfer_quantity: 100,
                max_total_cost: 100.0,
            },
        }
    }

    #[test]
    fn test_pack_name() {
        let pack = InventoryRebalancingPack;
        assert_eq!(pack.name(), "inventory-rebalancing");
    }

    #[test]
    fn test_validate_inputs() {
        let pack = InventoryRebalancingPack;
        let input = create_test_input();
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_ok());
    }

    #[test]
    fn test_solve_basic() {
        let pack = InventoryRebalancingPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-001", "test-tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(&input)
            .unwrap()
            .budgets(SolveBudgets::with_time_limit(10))
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        assert!(result.is_feasible());

        let output: InventoryRebalancingOutput = result.plan.plan_as().unwrap();
        // Should transfer from warehouse to store
        assert!(!output.transfers.is_empty());

        // Check transfer direction
        let transfer = &output.transfers[0];
        assert_eq!(transfer.from_location, "warehouse-1");
        assert_eq!(transfer.to_location, "store-1");
    }

    #[test]
    fn test_check_invariants() {
        let pack = InventoryRebalancingPack;
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

        // All critical invariants should pass
        let critical_pass = invariants
            .iter()
            .filter(|r| {
                r.invariant == "no_negative_inventory"
                    || r.invariant == "within_capacity_limits"
                    || r.invariant == "within_budget"
            })
            .all(|r| r.passed);
        assert!(critical_pass);
    }

    #[test]
    fn test_evaluate_gate() {
        let pack = InventoryRebalancingPack;
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
    fn test_no_transfers_needed() {
        let pack = InventoryRebalancingPack;

        // Create input where everything is already at target
        let input = InventoryRebalancingInput {
            locations: vec![Location {
                id: "warehouse-1".to_string(),
                name: "Warehouse".to_string(),
                capacity: 1000,
                location_type: LocationType::Warehouse,
            }],
            products: vec![Product {
                id: "sku-001".to_string(),
                name: "Widget".to_string(),
                unit_weight: 1.0,
                unit_value: 10.0,
            }],
            inventory: vec![InventoryLevel {
                location_id: "warehouse-1".to_string(),
                product_id: "sku-001".to_string(),
                quantity: 100,
                target_quantity: 100, // Already at target
                min_quantity: 50,
                max_quantity: 200,
            }],
            transfer_costs: vec![],
            constraints: RebalancingConstraints {
                max_total_transfers: 10,
                max_transfer_quantity: 100,
                max_total_cost: 100.0,
            },
        };

        let spec = ProblemSpec::builder("test-004", "test-tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: InventoryRebalancingOutput = result.plan.plan_as().unwrap();

        // No transfers needed
        assert!(output.transfers.is_empty());
    }
}
