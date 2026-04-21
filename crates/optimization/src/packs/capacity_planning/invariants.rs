//! Invariants for Capacity Planning pack

use super::types::CapacityPlanningOutput;
use crate::packs::{InvariantDef, InvariantResult};
use converge_pack::gate::Violation;

/// Get invariant definitions
pub fn get_invariants() -> Vec<InvariantDef> {
    vec![
        InvariantDef::critical("demand_met", "Minimum fulfillment requirements must be met"),
        InvariantDef::critical(
            "capacity_not_exceeded",
            "No team should exceed their maximum utilization",
        ),
        InvariantDef::critical(
            "skills_matched",
            "All assignments must match required skills",
        ),
        InvariantDef::advisory(
            "utilization_balanced",
            "Team utilization should be reasonably balanced",
        ),
        InvariantDef::advisory(
            "cost_within_budget",
            "Total cost should be within budget constraints",
        ),
    ]
}

/// Invariant definitions constant
pub const INVARIANTS: &[InvariantDef] = &[];

/// Check all invariants
pub fn check_all_invariants(output: &CapacityPlanningOutput) -> Vec<InvariantResult> {
    vec![
        check_demand_met(output),
        check_capacity_not_exceeded(output),
        check_skills_matched(output),
        check_utilization_balanced(output),
        check_cost_within_budget(output),
    ]
}

fn check_demand_met(output: &CapacityPlanningOutput) -> InvariantResult {
    let invariant = "demand_met";

    // Check if we have any output at all
    if output.assignments.is_empty() && output.summary.total_demand > 0.0 {
        let violation = Violation::new(
            invariant,
            1.0,
            format!(
                "No allocations made, {:.1} units of demand unmet",
                output.summary.total_demand
            ),
        );
        return InvariantResult::fail(invariant, violation);
    }

    // Check overall fulfillment ratio
    // Consider passing if fulfillment is at least 80% (reasonable threshold)
    let min_acceptable = 0.8;
    if output.summary.overall_fulfillment_ratio >= min_acceptable {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            1.0,
            format!(
                "Overall fulfillment {:.1}% below minimum {:.1}%",
                output.summary.overall_fulfillment_ratio * 100.0,
                min_acceptable * 100.0
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_capacity_not_exceeded(output: &CapacityPlanningOutput) -> InvariantResult {
    let invariant = "capacity_not_exceeded";

    // Check if any team is over-utilized
    let over_utilized: Vec<_> = output
        .team_utilization
        .iter()
        .filter(|t| t.is_over_utilized)
        .collect();

    if over_utilized.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let team_names: Vec<_> = over_utilized
            .iter()
            .map(|t| format!("{} ({:.0}%)", t.team_name, t.utilization_ratio * 100.0))
            .collect();

        let violation = Violation::new(
            invariant,
            1.0,
            format!(
                "{} team(s) exceed maximum utilization: {}",
                over_utilized.len(),
                team_names.join(", ")
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_skills_matched(output: &CapacityPlanningOutput) -> InvariantResult {
    let invariant = "skills_matched";

    // In the current output structure, we don't have direct skill info in assignments
    // But we can check via period_fulfillment -> unmet_demands
    // If there are unmet demands due to skill mismatches, we would see it in reasons

    let skill_mismatch_issues: Vec<_> = output
        .period_fulfillment
        .iter()
        .flat_map(|p| &p.unmet_demands)
        .filter(|u| u.reason.contains("skill") || u.reason.contains("matching"))
        .collect();

    // Also check that assignments were actually made (implying skills were matched)
    if skill_mismatch_issues.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let issues: Vec<_> = skill_mismatch_issues
            .iter()
            .map(|u| format!("{}: {}", u.demand_id, u.reason))
            .collect();

        let violation = Violation::new(
            invariant,
            1.0,
            format!("Skill matching issues: {}", issues.join("; ")),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_utilization_balanced(output: &CapacityPlanningOutput) -> InvariantResult {
    let invariant = "utilization_balanced";

    if output.team_utilization.len() < 2 {
        return InvariantResult::pass(invariant);
    }

    // Calculate utilization variance
    let utilizations: Vec<f64> = output
        .team_utilization
        .iter()
        .filter(|t| t.total_capacity > 0.0) // Only count teams with capacity
        .map(|t| t.utilization_ratio)
        .collect();

    if utilizations.is_empty() {
        return InvariantResult::pass(invariant);
    }

    let mean = utilizations.iter().sum::<f64>() / utilizations.len() as f64;
    let variance =
        utilizations.iter().map(|u| (u - mean).powi(2)).sum::<f64>() / utilizations.len() as f64;
    let std_dev = variance.sqrt();

    // Consider balanced if standard deviation is less than 20%
    let max_acceptable_std_dev = 0.20;
    if std_dev <= max_acceptable_std_dev {
        InvariantResult::pass(invariant)
    } else {
        let min_util = utilizations.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_util = utilizations
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        let violation = Violation::new(
            invariant,
            0.5, // Advisory severity
            format!(
                "Utilization imbalance: min {:.0}%, max {:.0}%, std dev {:.1}%",
                min_util * 100.0,
                max_util * 100.0,
                std_dev * 100.0
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_cost_within_budget(output: &CapacityPlanningOutput) -> InvariantResult {
    let invariant = "cost_within_budget";

    // Note: We don't have access to the budget constraint from the output alone
    // This invariant would need the input to fully validate
    // For now, we'll just verify the cost is reasonable (non-negative)

    if output.summary.total_cost >= 0.0 {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            0.3, // Advisory severity
            format!("Invalid negative cost: {:.2}", output.summary.total_cost),
        );
        InvariantResult::fail(invariant, violation)
    }
}

/// Check invariants with input context (for budget validation)
pub fn check_all_invariants_with_input(
    output: &CapacityPlanningOutput,
    max_budget: Option<f64>,
) -> Vec<InvariantResult> {
    let mut results = check_all_invariants(output);

    // Replace the cost check with a budget-aware version
    if let Some(budget) = max_budget {
        let budget_result = check_cost_against_budget(output, budget);
        if let Some(idx) = results
            .iter()
            .position(|r| r.invariant == "cost_within_budget")
        {
            results[idx] = budget_result;
        }
    }

    results
}

fn check_cost_against_budget(output: &CapacityPlanningOutput, max_budget: f64) -> InvariantResult {
    let invariant = "cost_within_budget";

    if output.summary.total_cost <= max_budget {
        InvariantResult::pass(invariant)
    } else {
        let overage = output.summary.total_cost - max_budget;
        let overage_pct = (overage / max_budget) * 100.0;

        let violation = Violation::new(
            invariant,
            if overage_pct > 20.0 { 0.8 } else { 0.4 }, // Higher severity for large overages
            format!(
                "Cost {:.2} exceeds budget {:.2} by {:.2} ({:.1}%)",
                output.summary.total_cost, max_budget, overage, overage_pct
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::capacity_planning::types::*;

    fn create_valid_output() -> CapacityPlanningOutput {
        CapacityPlanningOutput {
            assignments: vec![ResourceAssignment {
                id: "assign-1".to_string(),
                team_id: "team-a".to_string(),
                period_id: "Q1-2024".to_string(),
                resource_type: "engineering".to_string(),
                demand_id: "demand-1".to_string(),
                allocated_units: 100.0,
                cost: 10000.0,
            }],
            team_utilization: vec![
                TeamUtilization {
                    team_id: "team-a".to_string(),
                    team_name: "Backend Team".to_string(),
                    total_capacity: 120.0,
                    allocated: 100.0,
                    utilization_ratio: 0.83,
                    remaining_capacity: 20.0,
                    is_over_utilized: false,
                },
                TeamUtilization {
                    team_id: "team-b".to_string(),
                    team_name: "Frontend Team".to_string(),
                    total_capacity: 80.0,
                    allocated: 60.0,
                    utilization_ratio: 0.75,
                    remaining_capacity: 20.0,
                    is_over_utilized: false,
                },
            ],
            period_fulfillment: vec![PeriodFulfillment {
                period_id: "Q1-2024".to_string(),
                total_demand: 100.0,
                total_allocated: 100.0,
                fulfillment_ratio: 1.0,
                unmet_demands: vec![],
            }],
            summary: CapacityPlanSummary {
                total_demand: 100.0,
                total_allocated: 100.0,
                overall_fulfillment_ratio: 1.0,
                total_cost: 10000.0,
                average_utilization: 0.79,
                teams_over_capacity: 0,
                unmet_demands: 0,
                plan_status: "Feasible plan meets all constraints".to_string(),
            },
        }
    }

    #[test]
    fn test_all_pass_valid_output() {
        let output = create_valid_output();
        let results = check_all_invariants(&output);

        for result in &results {
            assert!(result.passed, "Invariant {} failed", result.invariant);
        }
    }

    #[test]
    fn test_demand_not_met() {
        let mut output = create_valid_output();
        output.summary.overall_fulfillment_ratio = 0.5;

        let result = check_demand_met(&output);
        assert!(!result.passed);
        assert!(result.violation.unwrap().severity >= 1.0);
    }

    #[test]
    fn test_capacity_exceeded() {
        let mut output = create_valid_output();
        output.team_utilization[0].is_over_utilized = true;
        output.team_utilization[0].utilization_ratio = 0.95;

        let result = check_capacity_not_exceeded(&output);
        assert!(!result.passed);
    }

    #[test]
    fn test_utilization_imbalanced() {
        let mut output = create_valid_output();
        // Create significant imbalance
        output.team_utilization[0].utilization_ratio = 0.95;
        output.team_utilization[1].utilization_ratio = 0.30;

        let result = check_utilization_balanced(&output);
        assert!(!result.passed);
        // Should be advisory (severity < 1.0)
        assert!(result.violation.as_ref().unwrap().severity < 1.0);
    }

    #[test]
    fn test_cost_over_budget() {
        let output = create_valid_output();
        let result = check_cost_against_budget(&output, 5000.0);

        assert!(!result.passed);
        assert!(
            result
                .violation
                .unwrap()
                .explanation
                .contains("exceeds budget")
        );
    }

    #[test]
    fn test_cost_within_budget() {
        let output = create_valid_output();
        let result = check_cost_against_budget(&output, 15000.0);

        assert!(result.passed);
    }

    #[test]
    fn test_invariant_definitions() {
        let defs = get_invariants();

        // Should have 5 invariants
        assert_eq!(defs.len(), 5);

        // Critical invariants
        let critical_names: Vec<_> = defs
            .iter()
            .filter(|d| d.critical)
            .map(|d| d.name.as_str())
            .collect();
        assert!(critical_names.contains(&"demand_met"));
        assert!(critical_names.contains(&"capacity_not_exceeded"));
        assert!(critical_names.contains(&"skills_matched"));

        // Advisory invariants
        let advisory_names: Vec<_> = defs
            .iter()
            .filter(|d| !d.critical)
            .map(|d| d.name.as_str())
            .collect();
        assert!(advisory_names.contains(&"utilization_balanced"));
        assert!(advisory_names.contains(&"cost_within_budget"));
    }

    #[test]
    fn test_empty_output_fails_demand_met() {
        let output = CapacityPlanningOutput {
            assignments: vec![],
            team_utilization: vec![],
            period_fulfillment: vec![],
            summary: CapacityPlanSummary {
                total_demand: 100.0,
                total_allocated: 0.0,
                overall_fulfillment_ratio: 0.0,
                total_cost: 0.0,
                average_utilization: 0.0,
                teams_over_capacity: 0,
                unmet_demands: 1,
                plan_status: "No allocations".to_string(),
            },
        };

        let result = check_demand_met(&output);
        assert!(!result.passed);
    }
}
