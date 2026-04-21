//! Invariants for Inventory Replenishment pack

use super::types::{InventoryReplenishmentInput, InventoryReplenishmentOutput};
use crate::packs::{InvariantDef, InvariantResult};
use converge_pack::gate::Violation;

/// Get invariant definitions
pub fn get_invariants() -> Vec<InvariantDef> {
    vec![
        InvariantDef::critical(
            "service_level_met",
            "Projected service level must meet target",
        ),
        InvariantDef::critical(
            "budget_not_exceeded",
            "Total order cost must not exceed budget",
        ),
        InvariantDef::critical(
            "lead_time_respected",
            "Orders must account for lead time to prevent stockouts",
        ),
        InvariantDef::advisory(
            "eoq_reasonable",
            "Order quantities should be close to calculated EOQ",
        ),
        InvariantDef::advisory(
            "safety_stock_adequate",
            "Orders should maintain adequate safety stock levels",
        ),
    ]
}

/// Invariant definitions constant
pub const INVARIANTS: &[InvariantDef] = &[];

/// Check all invariants
pub fn check_all_invariants(
    output: &InventoryReplenishmentOutput,
    input: &InventoryReplenishmentInput,
) -> Vec<InvariantResult> {
    vec![
        check_service_level_met(output, input),
        check_budget_not_exceeded(output, input),
        check_lead_time_respected(output, input),
        check_eoq_reasonable(output),
        check_safety_stock_adequate(output),
    ]
}

fn check_service_level_met(
    output: &InventoryReplenishmentOutput,
    input: &InventoryReplenishmentInput,
) -> InvariantResult {
    let invariant = "service_level_met";

    // If no orders needed and no products need replenishment, this passes
    if output.orders.is_empty() && input.products.iter().all(|p| !p.needs_reorder()) {
        return InvariantResult::pass(invariant);
    }

    let target = input.constraints.target_service_level;
    let projected = output.stats.projected_service_level;

    // Allow some tolerance (within 5% of target)
    if projected >= target * 0.95 {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            1.0,
            format!(
                "Projected service level {:.1}% below target {:.1}%",
                projected * 100.0,
                target * 100.0
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_budget_not_exceeded(
    output: &InventoryReplenishmentOutput,
    input: &InventoryReplenishmentInput,
) -> InvariantResult {
    let invariant = "budget_not_exceeded";

    let budget = input.constraints.budget;
    let total_cost = output.stats.total_order_cost;

    if total_cost <= budget {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            1.0,
            format!(
                "Total order cost ${:.2} exceeds budget ${:.2}",
                total_cost, budget
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_lead_time_respected(
    output: &InventoryReplenishmentOutput,
    input: &InventoryReplenishmentInput,
) -> InvariantResult {
    let invariant = "lead_time_respected";

    for order in &output.orders {
        // Find the corresponding product
        if let Some(product) = input.products.iter().find(|p| p.id == order.product_id) {
            let days_of_inventory = product.days_of_inventory();
            let lead_time = product.lead_time_days as f64;

            // Check if we're ordering in time for lead time
            // Allow ordering even if we're already in a stockout situation
            let arrival_day = order.order_day + product.lead_time_days;

            // If current inventory won't last until arrival, this is a problem
            // unless we're already ordering immediately (order_day = 0)
            if order.order_day > 0 && days_of_inventory < (order.order_day as f64 + lead_time) {
                let violation = Violation::new(
                    invariant,
                    1.0,
                    format!(
                        "Product {} will stockout before order arrives (order day: {}, arrival day: {}, days of inventory: {:.1})",
                        order.product_id, order.order_day, arrival_day, days_of_inventory
                    ),
                );
                return InvariantResult::fail(invariant, violation);
            }
        }
    }

    InvariantResult::pass(invariant)
}

fn check_eoq_reasonable(output: &InventoryReplenishmentOutput) -> InvariantResult {
    let invariant = "eoq_reasonable";

    if output.orders.is_empty() {
        return InvariantResult::pass(invariant);
    }

    let mut deviations = Vec::new();

    for order in &output.orders {
        if order.eoq > 0.0 {
            let ratio = order.quantity as f64 / order.eoq;
            // Allow 50% to 200% of EOQ
            if ratio < 0.5 || ratio > 2.0 {
                deviations.push(format!(
                    "{}: ordered {} vs EOQ {:.0} ({:.0}%)",
                    order.product_id,
                    order.quantity,
                    order.eoq,
                    ratio * 100.0
                ));
            }
        }
    }

    if deviations.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            0.3, // Advisory
            format!(
                "Order quantities deviate significantly from EOQ: {}",
                deviations.join(", ")
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_safety_stock_adequate(output: &InventoryReplenishmentOutput) -> InvariantResult {
    let invariant = "safety_stock_adequate";

    if output.orders.is_empty() {
        return InvariantResult::pass(invariant);
    }

    // Check projections for any high stockout probability
    let high_risk_projections: Vec<_> = output
        .projections
        .iter()
        .filter(|p| p.stockout_probability > 0.5)
        .collect();

    if high_risk_projections.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let products: Vec<_> = high_risk_projections
            .iter()
            .map(|p| format!("{} on day {}", p.product_id, p.day))
            .take(3)
            .collect();

        let violation = Violation::new(
            invariant,
            0.5, // Advisory but more serious
            format!("High stockout risk detected for: {}", products.join(", ")),
        );
        InvariantResult::fail(invariant, violation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::inventory_replenishment::types::*;

    fn create_test_input() -> InventoryReplenishmentInput {
        InventoryReplenishmentInput {
            products: vec![Product {
                id: "p1".to_string(),
                name: "Product 1".to_string(),
                current_inventory: 100,
                demand_forecast: DemandForecast {
                    average_daily: 10.0,
                    std_deviation: 2.0,
                    forecast_days: 30,
                },
                lead_time_days: 7,
                unit_cost: 10.0,
                ordering_cost: 50.0,
                holding_cost_per_day: 0.02,
                stockout_cost: 25.0,
            }],
            constraints: ReplenishmentConstraints {
                budget: 5000.0,
                target_service_level: 0.95,
                planning_horizon_days: 30,
                max_orders: None,
                min_order_quantity: None,
            },
        }
    }

    fn create_valid_output() -> InventoryReplenishmentOutput {
        InventoryReplenishmentOutput {
            orders: vec![ReplenishmentOrder {
                product_id: "p1".to_string(),
                product_name: "Product 1".to_string(),
                quantity: 200,
                order_day: 0,
                arrival_day: 7,
                order_cost: 2050.0,
                unit_cost: 10.0,
                eoq: 220.0,
                safety_stock: 10.0,
                reorder_point: 80.0,
                order_reason: "Below reorder point".to_string(),
            }],
            not_ordered: vec![],
            projections: vec![
                InventoryProjection {
                    product_id: "p1".to_string(),
                    day: 0,
                    projected_inventory: 100,
                    stockout_probability: 0.0,
                    order_arriving: false,
                },
                InventoryProjection {
                    product_id: "p1".to_string(),
                    day: 7,
                    projected_inventory: 230,
                    stockout_probability: 0.0,
                    order_arriving: true,
                },
            ],
            stats: ReplenishmentStats {
                total_order_cost: 2050.0,
                total_units_ordered: 200,
                products_ordered: 1,
                products_skipped: 0,
                budget_utilization: 0.41,
                projected_service_level: 0.96,
                reason: "EOQ-based replenishment".to_string(),
            },
        }
    }

    #[test]
    fn test_all_pass_valid_output() {
        let input = create_test_input();
        let output = create_valid_output();
        let results = check_all_invariants(&output, &input);

        for result in &results {
            assert!(
                result.passed,
                "Invariant {} failed: {:?}",
                result.invariant, result.violation
            );
        }
    }

    #[test]
    fn test_budget_exceeded_fails() {
        let input = create_test_input();
        let mut output = create_valid_output();
        output.stats.total_order_cost = 10000.0; // Exceeds 5000 budget

        let result = check_budget_not_exceeded(&output, &input);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity >= 1.0);
    }

    #[test]
    fn test_service_level_not_met() {
        let input = create_test_input();
        let mut output = create_valid_output();
        output.stats.projected_service_level = 0.80; // Below 0.95 target

        let result = check_service_level_met(&output, &input);
        assert!(!result.passed);
    }

    #[test]
    fn test_eoq_deviation_warning() {
        let _input = create_test_input();
        let mut output = create_valid_output();
        output.orders[0].quantity = 50; // Very low compared to EOQ of 220
        output.orders[0].eoq = 220.0;

        let result = check_eoq_reasonable(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 1.0); // Advisory
    }

    #[test]
    fn test_high_stockout_risk_warning() {
        let _input = create_test_input();
        let mut output = create_valid_output();
        output.projections.push(InventoryProjection {
            product_id: "p1".to_string(),
            day: 14,
            projected_inventory: 5,
            stockout_probability: 0.8,
            order_arriving: false,
        });

        let result = check_safety_stock_adequate(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 1.0); // Advisory
    }

    #[test]
    fn test_empty_orders_passes() {
        let mut input = create_test_input();
        // Set high inventory so no reorder needed
        input.products[0].current_inventory = 1000;

        let output = InventoryReplenishmentOutput::no_orders("Sufficient inventory");

        let results = check_all_invariants(&output, &input);

        // Should pass since no orders needed and sufficient inventory
        let service_result = results
            .iter()
            .find(|r| r.invariant == "service_level_met")
            .unwrap();
        assert!(service_result.passed);
    }
}
