//! Invariants for Pricing Guardrails pack

use super::types::PricingGuardrailsOutput;
use crate::packs::{InvariantDef, InvariantResult};
use converge_pack::gate::Violation;

/// Get invariant definitions
pub fn get_invariants() -> Vec<InvariantDef> {
    vec![
        InvariantDef::critical(
            "margin_maintained",
            "All products must maintain minimum margin",
        ),
        InvariantDef::critical(
            "within_bounds",
            "All prices must be within defined guardrails",
        ),
        InvariantDef::advisory(
            "competitive_position",
            "Pricing should achieve competitive positioning goals",
        ),
        InvariantDef::advisory("price_stability", "Price changes should be reasonable"),
    ]
}

/// Invariant definitions constant
pub const INVARIANTS: &[InvariantDef] = &[];

/// Check all invariants
pub fn check_all_invariants(output: &PricingGuardrailsOutput) -> Vec<InvariantResult> {
    vec![
        check_margin_maintained(output),
        check_within_bounds(output),
        check_competitive_position(output),
        check_price_stability(output),
    ]
}

fn check_margin_maintained(output: &PricingGuardrailsOutput) -> InvariantResult {
    let invariant = "margin_maintained";

    if output.recommendations.is_empty() {
        return InvariantResult::pass(invariant);
    }

    // Check if all products meet margin requirements
    if output.guardrail_compliance.all_margins_met {
        InvariantResult::pass(invariant)
    } else {
        // Find products that failed margin
        let failed_products: Vec<_> = output
            .recommendations
            .iter()
            .filter(|r| !r.margin_target_met)
            .map(|r| format!("{} ({:.1}%)", r.product_id, r.margin_pct))
            .collect();

        let violation = Violation::new(
            invariant,
            1.0,
            format!(
                "Products below minimum margin: {}",
                failed_products.join(", ")
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_within_bounds(output: &PricingGuardrailsOutput) -> InvariantResult {
    let invariant = "within_bounds";

    if output.recommendations.is_empty() {
        return InvariantResult::pass(invariant);
    }

    // Check if all prices are within bounds
    if output.guardrail_compliance.all_within_bounds {
        InvariantResult::pass(invariant)
    } else {
        // Find products outside bounds
        let out_of_bounds: Vec<_> = output
            .recommendations
            .iter()
            .filter(|r| !r.within_bounds)
            .map(|r| format!("{} (${:.2})", r.product_id, r.recommended_price))
            .collect();

        let violation = Violation::new(
            invariant,
            1.0,
            format!(
                "Products outside price bounds: {}",
                out_of_bounds.join(", ")
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_competitive_position(output: &PricingGuardrailsOutput) -> InvariantResult {
    let invariant = "competitive_position";

    if output.recommendations.is_empty() {
        return InvariantResult::pass(invariant);
    }

    // Check if competitive positioning was achieved
    if output.guardrail_compliance.competitive_position_achieved {
        InvariantResult::pass(invariant)
    } else {
        // Count products with competitor data that missed position target
        let products_with_competitor_data: Vec<_> = output
            .recommendations
            .iter()
            .filter(|r| r.competitive_position.competitor_count > 0)
            .collect();

        if products_with_competitor_data.is_empty() {
            // No products with competitor data, so position is N/A
            return InvariantResult::pass(invariant);
        }

        let violation = Violation::new(
            invariant,
            0.5,
            "Competitive positioning strategy not fully achieved",
        );
        InvariantResult::fail(invariant, violation)
    }
}

fn check_price_stability(output: &PricingGuardrailsOutput) -> InvariantResult {
    let invariant = "price_stability";

    if output.recommendations.is_empty() {
        return InvariantResult::pass(invariant);
    }

    // Check for extreme price changes (> 30% change)
    let extreme_changes: Vec<_> = output
        .recommendations
        .iter()
        .filter(|r| {
            r.price_change_pct
                .map(|pct| pct.abs() > 30.0)
                .unwrap_or(false)
        })
        .map(|r| {
            format!(
                "{} ({:+.1}%)",
                r.product_id,
                r.price_change_pct.unwrap_or(0.0)
            )
        })
        .collect();

    if extreme_changes.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            0.3,
            format!(
                "Large price changes detected: {}",
                extreme_changes.join(", ")
            ),
        );
        InvariantResult::fail(invariant, violation)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::pricing_guardrails::types::*;

    fn create_valid_output() -> PricingGuardrailsOutput {
        PricingGuardrailsOutput {
            recommendations: vec![
                PricingRecommendation {
                    product_id: "SKU-001".to_string(),
                    recommended_price: 100.0,
                    previous_price: Some(95.0),
                    price_change: Some(5.0),
                    price_change_pct: Some(5.26),
                    margin_pct: 25.0,
                    markup_pct: 33.3,
                    competitive_position: CompetitivePosition {
                        avg_competitor_price: Some(105.0),
                        position_vs_avg_pct: Some(-4.76),
                        competitor_count: 2,
                        lowest_in_market: true,
                        highest_in_market: false,
                    },
                    within_bounds: true,
                    margin_target_met: true,
                    rationale: "Achieves target margin".to_string(),
                },
                PricingRecommendation {
                    product_id: "SKU-002".to_string(),
                    recommended_price: 75.0,
                    previous_price: None,
                    price_change: None,
                    price_change_pct: None,
                    margin_pct: 30.0,
                    markup_pct: 42.9,
                    competitive_position: CompetitivePosition::default(),
                    within_bounds: true,
                    margin_target_met: true,
                    rationale: "New price set to target margin".to_string(),
                },
            ],
            margin_analysis: MarginAnalysis {
                total_products: 2,
                products_meeting_margin: 2,
                average_margin_pct: 27.5,
                min_margin_pct: 25.0,
                max_margin_pct: 30.0,
            },
            guardrail_compliance: GuardrailCompliance {
                all_within_bounds: true,
                all_margins_met: true,
                competitive_position_achieved: true,
                violations: vec![],
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
    fn test_margin_maintained_failure() {
        let mut output = create_valid_output();
        output.guardrail_compliance.all_margins_met = false;
        output.recommendations[0].margin_target_met = false;
        output.recommendations[0].margin_pct = 15.0;

        let result = check_margin_maintained(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity == 1.0); // Critical
    }

    #[test]
    fn test_within_bounds_failure() {
        let mut output = create_valid_output();
        output.guardrail_compliance.all_within_bounds = false;
        output.recommendations[0].within_bounds = false;

        let result = check_within_bounds(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity == 1.0); // Critical
    }

    #[test]
    fn test_competitive_position_failure() {
        let mut output = create_valid_output();
        output.guardrail_compliance.competitive_position_achieved = false;

        let result = check_competitive_position(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 1.0); // Advisory
    }

    #[test]
    fn test_price_stability_extreme_change() {
        let mut output = create_valid_output();
        output.recommendations[0].price_change_pct = Some(50.0); // 50% increase

        let result = check_price_stability(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 1.0); // Advisory
        assert!(
            result
                .violation
                .as_ref()
                .unwrap()
                .explanation
                .contains("Large price changes")
        );
    }

    #[test]
    fn test_price_stability_moderate_change() {
        let mut output = create_valid_output();
        output.recommendations[0].price_change_pct = Some(15.0); // 15% increase - acceptable

        let result = check_price_stability(&output);
        assert!(result.passed);
    }

    #[test]
    fn test_empty_recommendations() {
        let output = PricingGuardrailsOutput::no_valid_pricing("No products");
        let results = check_all_invariants(&output);

        // Most invariants should pass with empty data
        let margin_result = results
            .iter()
            .find(|r| r.invariant == "margin_maintained")
            .unwrap();
        assert!(margin_result.passed);

        let bounds_result = results
            .iter()
            .find(|r| r.invariant == "within_bounds")
            .unwrap();
        assert!(bounds_result.passed);
    }

    #[test]
    fn test_invariant_definitions() {
        let invariants = get_invariants();

        assert_eq!(invariants.len(), 4);

        // Check margin_maintained is critical
        let margin_inv = invariants
            .iter()
            .find(|i| i.name == "margin_maintained")
            .unwrap();
        assert!(margin_inv.critical);

        // Check within_bounds is critical
        let bounds_inv = invariants
            .iter()
            .find(|i| i.name == "within_bounds")
            .unwrap();
        assert!(bounds_inv.critical);

        // Check competitive_position is advisory
        let comp_inv = invariants
            .iter()
            .find(|i| i.name == "competitive_position")
            .unwrap();
        assert!(!comp_inv.critical);

        // Check price_stability is advisory
        let stability_inv = invariants
            .iter()
            .find(|i| i.name == "price_stability")
            .unwrap();
        assert!(!stability_inv.critical);
    }
}
