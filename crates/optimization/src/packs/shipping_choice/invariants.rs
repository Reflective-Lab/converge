//! Invariants for Shipping Choice pack

use super::types::ShippingChoiceOutput;
use crate::packs::{InvariantDef, InvariantResult};
use converge_pack::gate::Violation;

/// Get invariant definitions
pub fn get_invariants() -> Vec<InvariantDef> {
    vec![
        InvariantDef::critical(
            "carrier_selected",
            "A carrier must be selected for valid orders",
        ),
        InvariantDef::critical(
            "cost_positive",
            "Shipping cost must be positive when carrier selected",
        ),
        InvariantDef::advisory(
            "meets_sla",
            "Selected carrier should meet the SLA requirement",
        ),
        InvariantDef::advisory(
            "cost_reasonable",
            "Shipping cost should be reasonable relative to alternatives",
        ),
    ]
}

/// Invariant definitions constant
pub const INVARIANTS: &[InvariantDef] = &[];

/// Check all invariants
pub fn check_all_invariants(output: &ShippingChoiceOutput) -> Vec<InvariantResult> {
    vec![
        check_carrier_selected(output),
        check_cost_positive(output),
        check_meets_sla(output),
        check_cost_reasonable(output),
    ]
}

fn check_carrier_selected(output: &ShippingChoiceOutput) -> InvariantResult {
    let invariant = "carrier_selected";

    if output.selected_carrier.is_some() {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            1.0,
            format!("No carrier selected: {}", output.selection_reason),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_cost_positive(output: &ShippingChoiceOutput) -> InvariantResult {
    let invariant = "cost_positive";

    if output.selected_carrier.is_none() {
        return InvariantResult::pass(invariant);
    }

    if output.cost > 0.0 {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            1.0,
            format!("Invalid shipping cost: {}", output.cost),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_meets_sla(output: &ShippingChoiceOutput) -> InvariantResult {
    let invariant = "meets_sla";

    if output.selected_carrier.is_none() {
        return InvariantResult::pass(invariant);
    }

    if output.meets_sla {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            0.5,
            format!(
                "Selected carrier does not meet SLA ({} days)",
                output.estimated_days
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_cost_reasonable(output: &ShippingChoiceOutput) -> InvariantResult {
    let invariant = "cost_reasonable";

    if output.selected_carrier.is_none() || output.alternatives.is_empty() {
        return InvariantResult::pass(invariant);
    }

    // Check if selected cost is not significantly higher than cheapest alternative
    let min_alt_cost = output
        .alternatives
        .iter()
        .map(|a| a.cost)
        .fold(f64::INFINITY, f64::min);

    if output.cost <= min_alt_cost * 1.5 {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            0.3,
            format!(
                "Selected cost ${:.2} is significantly higher than alternative ${:.2}",
                output.cost, min_alt_cost
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::shipping_choice::types::*;

    fn create_valid_output() -> ShippingChoiceOutput {
        ShippingChoiceOutput {
            selected_carrier: Some("ups".to_string()),
            selected_service: Some("ground".to_string()),
            cost: 8.99,
            estimated_days: 5,
            meets_sla: true,
            selection_reason: "Lowest cost".to_string(),
            alternatives: vec![AlternativeCarrier {
                carrier_id: "fedex".to_string(),
                service_level: "express".to_string(),
                cost: 15.99,
                reason_not_selected: "Higher cost".to_string(),
            }],
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
    fn test_no_carrier_fails() {
        let output = ShippingChoiceOutput::no_carrier("No carriers available");
        let results = check_all_invariants(&output);

        let carrier_result = results
            .iter()
            .find(|r| r.invariant == "carrier_selected")
            .unwrap();
        assert!(!carrier_result.passed);
    }

    #[test]
    fn test_sla_not_met() {
        let mut output = create_valid_output();
        output.meets_sla = false;

        let result = check_meets_sla(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 1.0); // Advisory
    }
}
