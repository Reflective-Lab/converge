//! Invariants for Backlog Prioritization pack

use super::types::BacklogPrioritizationOutput;
use crate::packs::{InvariantDef, InvariantResult};
use converge_pack::gate::Violation;

/// Get invariant definitions
pub fn get_invariants() -> Vec<InvariantDef> {
    vec![
        InvariantDef::critical("items_ranked", "All items must be ranked"),
        InvariantDef::critical(
            "rankings_sequential",
            "Rankings must be sequential starting at 1",
        ),
        InvariantDef::advisory(
            "capacity_utilized",
            "Capacity should be reasonably utilized",
        ),
        InvariantDef::advisory(
            "value_delivered",
            "Included items should deliver meaningful value",
        ),
    ]
}

/// Invariant definitions constant
pub const INVARIANTS: &[InvariantDef] = &[];

/// Check all invariants
pub fn check_all_invariants(output: &BacklogPrioritizationOutput) -> Vec<InvariantResult> {
    vec![
        check_items_ranked(output),
        check_rankings_sequential(output),
        check_capacity_utilized(output),
        check_value_delivered(output),
    ]
}

fn check_items_ranked(output: &BacklogPrioritizationOutput) -> InvariantResult {
    let invariant = "items_ranked";

    if !output.ranked_items.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(invariant, 1.0, "No items were ranked");
        InvariantResult::fail(invariant, violation)
    }
}

fn check_rankings_sequential(output: &BacklogPrioritizationOutput) -> InvariantResult {
    let invariant = "rankings_sequential";

    if output.ranked_items.is_empty() {
        return InvariantResult::pass(invariant);
    }

    for (i, item) in output.ranked_items.iter().enumerate() {
        if item.rank != i + 1 {
            let violation = Violation::new(
                invariant,
                1.0,
                format!(
                    "Item {} has rank {} but should be {}",
                    item.item_id,
                    item.rank,
                    i + 1
                ),
            );
            return InvariantResult::fail(invariant, violation);
        }
    }

    InvariantResult::pass(invariant)
}

fn check_capacity_utilized(output: &BacklogPrioritizationOutput) -> InvariantResult {
    let invariant = "capacity_utilized";

    if output.included_count == 0 && output.ranked_items.is_empty() {
        return InvariantResult::pass(invariant);
    }

    // Advisory: we should have included at least some items
    if output.included_count > 0 {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(invariant, 0.5, "No items fit within capacity");
        InvariantResult::fail(invariant, violation)
    }
}

fn check_value_delivered(output: &BacklogPrioritizationOutput) -> InvariantResult {
    let invariant = "value_delivered";

    if output.included_count == 0 {
        return InvariantResult::pass(invariant);
    }

    // Advisory: total value should be positive
    if output.total_value > 0.0 {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(invariant, 0.3, "Included items deliver no business value");
        InvariantResult::fail(invariant, violation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::backlog_prioritization::types::*;

    fn create_valid_output() -> BacklogPrioritizationOutput {
        BacklogPrioritizationOutput {
            ranked_items: vec![
                RankedItem {
                    item_id: "feat-1".to_string(),
                    item_title: "Feature 1".to_string(),
                    rank: 1,
                    wsjf_score: 36.0,
                    included_in_capacity: true,
                    cumulative_effort: 5,
                    ranking_reason: "WSJF: 36.0".to_string(),
                },
                RankedItem {
                    item_id: "feat-2".to_string(),
                    item_title: "Feature 2".to_string(),
                    rank: 2,
                    wsjf_score: 25.0,
                    included_in_capacity: true,
                    cumulative_effort: 8,
                    ranking_reason: "WSJF: 25.0".to_string(),
                },
            ],
            total_value: 120.0,
            total_effort: 8,
            included_count: 2,
            excluded_count: 0,
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
    fn test_empty_ranking_fails() {
        let output = BacklogPrioritizationOutput::empty("No items");
        let results = check_all_invariants(&output);

        let items_result = results
            .iter()
            .find(|r| r.invariant == "items_ranked")
            .unwrap();
        assert!(!items_result.passed);
    }

    #[test]
    fn test_invalid_ranking_sequence() {
        let mut output = create_valid_output();
        output.ranked_items[1].rank = 5; // Should be 2

        let result = check_rankings_sequential(&output);
        assert!(!result.passed);
    }

    #[test]
    fn test_no_capacity_used() {
        let mut output = create_valid_output();
        output.included_count = 0;
        for item in &mut output.ranked_items {
            item.included_in_capacity = false;
        }

        let result = check_capacity_utilized(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 1.0); // Advisory
    }
}
