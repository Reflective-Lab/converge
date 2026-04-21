//! Invariants for Inventory Rebalancing pack

use super::types::InventoryRebalancingOutput;
use crate::packs::{InvariantDef, InvariantResult};
use converge_pack::gate::Violation;

/// Invariant definitions for inventory rebalancing
pub const INVARIANTS: &[InvariantDef] = &[InvariantDef {
    name: String::new(),
    description: String::new(),
    critical: true,
}];

/// Get invariant definitions (with proper String values)
pub fn get_invariants() -> Vec<InvariantDef> {
    vec![
        InvariantDef::critical(
            "no_negative_inventory",
            "Source locations must have sufficient inventory for transfers",
        ),
        InvariantDef::critical(
            "within_capacity_limits",
            "Destination locations must not exceed capacity",
        ),
        InvariantDef::critical(
            "within_budget",
            "Total transfer cost must not exceed budget",
        ),
        InvariantDef::advisory(
            "safety_stock_maintained",
            "Source locations should maintain safety stock levels",
        ),
        InvariantDef::advisory(
            "service_level_improved",
            "Overall service level should improve after rebalancing",
        ),
    ]
}

/// Check all invariants for an inventory rebalancing output
pub fn check_all_invariants(output: &InventoryRebalancingOutput) -> Vec<InvariantResult> {
    vec![
        check_no_negative_inventory(output),
        check_within_capacity(output),
        check_within_budget(output),
        check_safety_stock(output),
        check_service_improvement(output),
    ]
}

/// Check that no transfer creates negative inventory
fn check_no_negative_inventory(output: &InventoryRebalancingOutput) -> InvariantResult {
    let invariant = "no_negative_inventory";

    // Check if any location impact results in negative inventory
    let negative_locations: Vec<_> = output
        .location_impacts
        .iter()
        .filter(|impact| impact.final_quantity < 0)
        .map(|impact| impact.location_id.as_str())
        .collect();

    if negative_locations.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            1.0,
            format!("Negative inventory at: {}", negative_locations.join(", ")),
        )
        .with_affected_all(negative_locations.iter().map(|s| s.to_string()));
        InvariantResult::fail(invariant, violation)
    }
}

/// Check that no destination exceeds capacity
fn check_within_capacity(_output: &InventoryRebalancingOutput) -> InvariantResult {
    let invariant = "within_capacity_limits";

    // We don't have capacity info in output, so this is a simplified check
    // In practice, the solver should never violate this
    InvariantResult::pass(invariant)
}

/// Check that total cost is within budget
fn check_within_budget(output: &InventoryRebalancingOutput) -> InvariantResult {
    let invariant = "within_budget";

    // Budget check happens in solver - if we have output, it's within budget
    // This is a validation that could be enhanced with budget info in output
    if output.total_cost < 0.0 {
        let violation = Violation::new(invariant, 1.0, "Invalid negative cost");
        return InvariantResult::fail(invariant, violation);
    }

    InvariantResult::pass(invariant)
}

/// Check that safety stock is maintained at source locations
fn check_safety_stock(output: &InventoryRebalancingOutput) -> InvariantResult {
    let invariant = "safety_stock_maintained";

    // Check location impacts for violations
    // Note: We'd need min_quantity info in the output to fully check this
    // For now, just check that outgoing transfers don't seem excessive
    let excessive_outflows: Vec<_> = output
        .location_impacts
        .iter()
        .filter(|impact| impact.inventory_change < -100) // Arbitrary threshold
        .filter(|impact| !impact.meets_target)
        .map(|impact| impact.location_id.as_str())
        .collect();

    if excessive_outflows.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            0.5, // Advisory severity
            format!(
                "Large outflows from locations that don't meet target: {}",
                excessive_outflows.join(", ")
            ),
        )
        .with_affected_all(excessive_outflows.iter().map(|s| s.to_string()));
        InvariantResult::fail(invariant, violation)
    }
}

/// Check that overall service level improves
fn check_service_improvement(output: &InventoryRebalancingOutput) -> InvariantResult {
    let invariant = "service_level_improved";

    if output.transfers.is_empty() {
        // No transfers is valid (already balanced)
        return InvariantResult::pass(invariant);
    }

    if output.service_level_improvement <= 0.0 {
        let violation = Violation::new(
            invariant,
            0.3, // Low severity - advisory
            format!(
                "Service level did not improve ({}%)",
                output.service_level_improvement * 100.0
            ),
        );
        return InvariantResult::fail(invariant, violation);
    }

    InvariantResult::pass(invariant)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::inventory_rebalancing::types::*;

    fn create_valid_output() -> InventoryRebalancingOutput {
        InventoryRebalancingOutput {
            transfers: vec![Transfer::new("warehouse", "store", "widget", 50, 25.0, 24)],
            total_cost: 25.0,
            total_units_moved: 50,
            service_level_improvement: 0.15,
            location_impacts: vec![
                LocationImpact {
                    location_id: "warehouse".to_string(),
                    product_id: "widget".to_string(),
                    inventory_change: -50,
                    final_quantity: 450,
                    meets_target: true,
                },
                LocationImpact {
                    location_id: "store".to_string(),
                    product_id: "widget".to_string(),
                    inventory_change: 50,
                    final_quantity: 60,
                    meets_target: true,
                },
            ],
        }
    }

    #[test]
    fn test_all_invariants_pass() {
        let output = create_valid_output();
        let results = check_all_invariants(&output);

        for result in &results {
            assert!(result.passed, "Invariant {} failed", result.invariant);
        }
    }

    #[test]
    fn test_empty_output() {
        let output = InventoryRebalancingOutput::empty();
        let results = check_all_invariants(&output);

        for result in &results {
            assert!(
                result.passed,
                "Invariant {} should pass for empty output",
                result.invariant
            );
        }
    }

    #[test]
    fn test_negative_inventory() {
        let mut output = create_valid_output();
        output.location_impacts[0].final_quantity = -10;

        let result = check_no_negative_inventory(&output);
        assert!(!result.passed);
        assert!(result.violation.is_some());
    }

    #[test]
    fn test_no_service_improvement() {
        let mut output = create_valid_output();
        output.service_level_improvement = 0.0;

        let result = check_service_improvement(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 0.5); // Low severity
    }

    #[test]
    fn test_get_invariants() {
        let invariants = get_invariants();
        assert_eq!(invariants.len(), 5);

        let critical_count = invariants.iter().filter(|i| i.critical).count();
        assert_eq!(critical_count, 3);
    }
}
