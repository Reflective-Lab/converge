//! Pricing Guardrails Pack
//!
//! JTBD: "Set pricing within guardrails ensuring margin targets and competitive position"
//!
//! ## Problem
//!
//! Given:
//! - Products with cost data
//! - Competitor prices for market positioning
//! - Margin requirements (minimum and target)
//! - Price bounds (guardrails)
//!
//! Find:
//! - Recommended prices that respect margins and guardrails
//! - Margin analysis and compliance reporting
//!
//! ## Solver
//!
//! Uses rule-based pricing:
//! 1. Calculate minimum price to meet margin requirement
//! 2. Apply competitive strategy to adjust price
//! 3. Enforce price bounds (guardrails)
//! 4. Generate recommendations with compliance analysis

mod invariants;
mod solver;
mod types;

pub use invariants::*;
pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

/// Pricing Guardrails Pack
pub struct PricingGuardrailsPack;

impl Pack for PricingGuardrailsPack {
    fn name(&self) -> &'static str {
        "pricing-guardrails"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: PricingGuardrailsInput =
            serde_json::from_value(inputs.clone()).map_err(|e| {
                converge_pack::GateError::invalid_input(format!("Invalid input: {}", e))
            })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: PricingGuardrailsInput = spec.inputs_as()?;
        input.validate()?;

        let solver = GuardrailPricingSolver;
        let (output, report) = solver.solve_pricing(&input, spec)?;

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
        let output: PricingGuardrailsOutput = plan.plan_as()?;
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

fn calculate_confidence(output: &PricingGuardrailsOutput) -> f64 {
    if output.recommendations.is_empty() {
        return 0.0;
    }

    let mut confidence: f64 = 0.5;

    // Bonus for all margins met
    if output.guardrail_compliance.all_margins_met {
        confidence += 0.2;
    }

    // Bonus for all within bounds
    if output.guardrail_compliance.all_within_bounds {
        confidence += 0.2;
    }

    // Bonus for competitive position achieved
    if output.guardrail_compliance.competitive_position_achieved {
        confidence += 0.1;
    }

    confidence.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> PricingGuardrailsInput {
        PricingGuardrailsInput {
            products: vec![
                Product {
                    product_id: "SKU-001".to_string(),
                    name: "Widget A".to_string(),
                    unit_cost: 80.0,
                    current_price: Some(100.0),
                    price_bounds: Some(PriceBounds {
                        min_price: 90.0,
                        max_price: 150.0,
                    }),
                    competitor_prices: vec![CompetitorPrice {
                        competitor_id: "comp1".to_string(),
                        price: 110.0,
                        as_of_date: None,
                    }],
                    category: Some("widgets".to_string()),
                },
                Product {
                    product_id: "SKU-002".to_string(),
                    name: "Widget B".to_string(),
                    unit_cost: 50.0,
                    current_price: None,
                    price_bounds: None,
                    competitor_prices: vec![],
                    category: Some("widgets".to_string()),
                },
            ],
            margin_requirements: MarginRequirements {
                min_margin_pct: 20.0,
                target_margin_pct: 30.0,
                competitive_strategy: CompetitiveStrategy::MatchMarket,
            },
            price_bounds: Some(PriceBounds {
                min_price: 10.0,
                max_price: 1000.0,
            }),
        }
    }

    #[test]
    fn test_pack_name() {
        let pack = PricingGuardrailsPack;
        assert_eq!(pack.name(), "pricing-guardrails");
        assert_eq!(pack.version(), "1.0.0");
    }

    #[test]
    fn test_validate_inputs() {
        let pack = PricingGuardrailsPack;
        let input = create_test_input();
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_ok());
    }

    #[test]
    fn test_validate_inputs_rejects_invalid() {
        let pack = PricingGuardrailsPack;
        let mut input = create_test_input();
        input.products[0].unit_cost = -10.0;
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_err());
    }

    #[test]
    fn test_solve_basic() {
        let pack = PricingGuardrailsPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-001", "test-tenant")
            .objective(ObjectiveSpec::maximize("margin"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        assert!(result.is_feasible());

        let output: PricingGuardrailsOutput = result.plan.plan_as().unwrap();
        assert_eq!(output.recommendations.len(), 2);
        assert!(output.margin_analysis.average_margin_pct > 0.0);
    }

    #[test]
    fn test_check_invariants() {
        let pack = PricingGuardrailsPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-002", "test-tenant")
            .objective(ObjectiveSpec::maximize("margin"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();

        // Should have all 4 invariants checked
        assert_eq!(invariants.len(), 4);
    }

    #[test]
    fn test_gate_promotes_valid() {
        let pack = PricingGuardrailsPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-003", "test-tenant")
            .objective(ObjectiveSpec::maximize("margin"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();
        let gate = pack.evaluate_gate(&result.plan, &invariants);

        // With valid input, should either promote or require review
        assert!(!gate.is_rejected());
    }

    #[test]
    fn test_determinism() {
        let pack = PricingGuardrailsPack;
        let input = create_test_input();

        let spec1 = ProblemSpec::builder("test-a", "tenant")
            .objective(ObjectiveSpec::maximize("margin"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let spec2 = ProblemSpec::builder("test-b", "tenant")
            .objective(ObjectiveSpec::maximize("margin"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let result1 = pack.solve(&spec1).unwrap();
        let result2 = pack.solve(&spec2).unwrap();

        let output1: PricingGuardrailsOutput = result1.plan.plan_as().unwrap();
        let output2: PricingGuardrailsOutput = result2.plan.plan_as().unwrap();

        assert_eq!(output1.recommendations.len(), output2.recommendations.len());
        for (r1, r2) in output1
            .recommendations
            .iter()
            .zip(output2.recommendations.iter())
        {
            assert_eq!(r1.product_id, r2.product_id);
            assert!((r1.recommended_price - r2.recommended_price).abs() < 0.01);
        }
    }

    #[test]
    fn test_margin_enforcement() {
        let pack = PricingGuardrailsPack;
        let mut input = create_test_input();
        input.margin_requirements.min_margin_pct = 30.0;
        input.margin_requirements.target_margin_pct = 35.0;
        input.margin_requirements.competitive_strategy = CompetitiveStrategy::IgnoreCompetitors;

        let spec = ProblemSpec::builder("test-margin", "test-tenant")
            .objective(ObjectiveSpec::maximize("margin"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: PricingGuardrailsOutput = result.plan.plan_as().unwrap();

        // All products should meet minimum margin (or be constrained by bounds)
        for rec in &output.recommendations {
            // Either meets target or is constrained by bounds
            assert!(rec.margin_pct >= 30.0 || !rec.within_bounds);
        }
    }

    #[test]
    fn test_price_bounds_enforcement() {
        let pack = PricingGuardrailsPack;
        let mut input = create_test_input();
        input.products = vec![Product {
            product_id: "bounded".to_string(),
            name: "Bounded Product".to_string(),
            unit_cost: 80.0,
            current_price: None,
            price_bounds: Some(PriceBounds {
                min_price: 95.0,
                max_price: 105.0,
            }),
            competitor_prices: vec![],
            category: None,
        }];

        let spec = ProblemSpec::builder("test-bounds", "test-tenant")
            .objective(ObjectiveSpec::maximize("margin"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: PricingGuardrailsOutput = result.plan.plan_as().unwrap();

        let rec = &output.recommendations[0];
        assert!(rec.recommended_price >= 95.0);
        assert!(rec.recommended_price <= 105.0);
    }

    #[test]
    fn test_competitive_pricing() {
        let pack = PricingGuardrailsPack;
        let mut input = create_test_input();
        input.margin_requirements.competitive_strategy = CompetitiveStrategy::PriceToBeat;
        input.margin_requirements.min_margin_pct = 10.0; // Lower margin to allow competitive pricing

        let spec = ProblemSpec::builder("test-competitive", "test-tenant")
            .objective(ObjectiveSpec::maximize("margin"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: PricingGuardrailsOutput = result.plan.plan_as().unwrap();

        // First product has competitor data
        let rec = &output.recommendations[0];
        assert!(rec.competitive_position.competitor_count > 0);
        assert!(rec.competitive_position.avg_competitor_price.is_some());
    }

    #[test]
    fn test_calculate_confidence() {
        // Full compliance should give high confidence
        let output = PricingGuardrailsOutput {
            recommendations: vec![PricingRecommendation {
                product_id: "test".to_string(),
                recommended_price: 100.0,
                previous_price: None,
                price_change: None,
                price_change_pct: None,
                margin_pct: 25.0,
                markup_pct: 33.0,
                competitive_position: CompetitivePosition::default(),
                within_bounds: true,
                margin_target_met: true,
                rationale: "Test".to_string(),
            }],
            margin_analysis: MarginAnalysis::default(),
            guardrail_compliance: GuardrailCompliance {
                all_within_bounds: true,
                all_margins_met: true,
                competitive_position_achieved: true,
                violations: vec![],
            },
        };

        let confidence = calculate_confidence(&output);
        assert!(confidence >= 0.9);

        // Empty recommendations should give 0 confidence
        let empty_output = PricingGuardrailsOutput::no_valid_pricing("No products");
        let empty_confidence = calculate_confidence(&empty_output);
        assert_eq!(empty_confidence, 0.0);
    }
}
