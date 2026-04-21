//! Invariants for Budget Allocation pack

use super::types::BudgetAllocationOutput;
use crate::packs::{InvariantDef, InvariantResult};
use converge_pack::gate::Violation;

/// Get invariant definitions
pub fn get_invariants() -> Vec<InvariantDef> {
    vec![
        InvariantDef::critical(
            "budget_not_exceeded",
            "Total allocation must not exceed budget",
        ),
        InvariantDef::critical(
            "allocations_non_negative",
            "All allocations must be non-negative",
        ),
        InvariantDef::advisory(
            "positive_roi",
            "Portfolio should have positive expected ROI",
        ),
        InvariantDef::advisory(
            "budget_utilized",
            "Significant portion of budget should be utilized",
        ),
    ]
}

/// Invariant definitions constant
pub const INVARIANTS: &[InvariantDef] = &[];

/// Check all invariants
pub fn check_all_invariants(
    output: &BudgetAllocationOutput,
    total_budget: f64,
) -> Vec<InvariantResult> {
    vec![
        check_budget_not_exceeded(output, total_budget),
        check_allocations_non_negative(output),
        check_positive_roi(output),
        check_budget_utilized(output, total_budget),
    ]
}

fn check_budget_not_exceeded(
    output: &BudgetAllocationOutput,
    total_budget: f64,
) -> InvariantResult {
    let invariant = "budget_not_exceeded";

    if output.total_allocated <= total_budget * 1.001 {
        // Allow tiny floating point errors
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            1.0,
            format!(
                "Allocated ${:.2} exceeds budget ${:.2}",
                output.total_allocated, total_budget
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_allocations_non_negative(output: &BudgetAllocationOutput) -> InvariantResult {
    let invariant = "allocations_non_negative";

    let negative: Vec<_> = output
        .allocations
        .iter()
        .filter(|a| a.amount < 0.0)
        .collect();

    if negative.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let ids: Vec<_> = negative.iter().map(|a| a.category_id.as_str()).collect();
        let violation = Violation::new(
            invariant,
            1.0,
            format!("Negative allocations for: {}", ids.join(", ")),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_positive_roi(output: &BudgetAllocationOutput) -> InvariantResult {
    let invariant = "positive_roi";

    if output.allocations.is_empty() {
        return InvariantResult::pass(invariant);
    }

    if output.portfolio_roi > 0.0 {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            0.5,
            format!("Portfolio ROI is {:.1}%", output.portfolio_roi * 100.0),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_budget_utilized(output: &BudgetAllocationOutput, total_budget: f64) -> InvariantResult {
    let invariant = "budget_utilized";

    if total_budget == 0.0 {
        return InvariantResult::pass(invariant);
    }

    let utilization = output.total_allocated / total_budget;

    if utilization >= 0.5 {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            0.3,
            format!("Only {:.1}% of budget utilized", utilization * 100.0),
        );
        InvariantResult::fail(invariant, violation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::budget_allocation::types::*;

    fn create_valid_output() -> BudgetAllocationOutput {
        BudgetAllocationOutput {
            allocations: vec![
                CategoryAllocation {
                    category_id: "marketing".to_string(),
                    category_name: "Marketing".to_string(),
                    amount: 40000.0,
                    percentage: 40.0,
                    expected_return: 8000.0,
                    reason: "High efficiency".to_string(),
                },
                CategoryAllocation {
                    category_id: "rnd".to_string(),
                    category_name: "R&D".to_string(),
                    amount: 50000.0,
                    percentage: 50.0,
                    expected_return: 15000.0,
                    reason: "High ROI".to_string(),
                },
            ],
            total_allocated: 90000.0,
            total_expected_return: 23000.0,
            budget_remaining: 10000.0,
            portfolio_roi: 0.256,
        }
    }

    #[test]
    fn test_all_pass_valid_output() {
        let output = create_valid_output();
        let results = check_all_invariants(&output, 100000.0);

        for result in &results {
            assert!(result.passed, "Invariant {} failed", result.invariant);
        }
    }

    #[test]
    fn test_budget_exceeded() {
        let mut output = create_valid_output();
        output.total_allocated = 110000.0;

        let result = check_budget_not_exceeded(&output, 100000.0);
        assert!(!result.passed);
    }

    #[test]
    fn test_negative_allocation() {
        let mut output = create_valid_output();
        output.allocations[0].amount = -5000.0;

        let result = check_allocations_non_negative(&output);
        assert!(!result.passed);
    }

    #[test]
    fn test_low_utilization() {
        let mut output = create_valid_output();
        output.total_allocated = 20000.0;

        let result = check_budget_utilized(&output, 100000.0);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 1.0); // Advisory
    }
}
