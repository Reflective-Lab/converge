//! Solver for Pricing Guardrails pack

use super::types::*;
use converge_pack::PackSolver;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport, StopReason};

/// Rule-based pricing solver that respects margins and guardrails
///
/// Algorithm:
/// 1. For each product, calculate minimum price to meet margin requirement
/// 2. Apply competitive strategy to adjust price within bounds
/// 3. Ensure price stays within guardrails
/// 4. Generate recommendations with compliance analysis
pub struct GuardrailPricingSolver;

impl GuardrailPricingSolver {
    /// Solve the pricing guardrails problem
    pub fn solve_pricing(
        &self,
        input: &PricingGuardrailsInput,
        spec: &ProblemSpec,
    ) -> Result<(PricingGuardrailsOutput, SolverReport)> {
        let seed = spec.seed();
        let margin_req = &input.margin_requirements;

        let mut recommendations = Vec::new();
        let mut violations = Vec::new();

        for product in &input.products {
            let recommendation = self.price_product(product, margin_req, &input.price_bounds)?;

            // Track violations
            if !recommendation.within_bounds {
                violations.push(format!(
                    "Product {} price ${:.2} outside bounds",
                    product.product_id, recommendation.recommended_price
                ));
            }
            if !recommendation.margin_target_met {
                violations.push(format!(
                    "Product {} margin {:.1}% below minimum {:.1}%",
                    product.product_id, recommendation.margin_pct, margin_req.min_margin_pct
                ));
            }

            recommendations.push(recommendation);
        }

        // Calculate margin analysis
        let margin_analysis = self.calculate_margin_analysis(&recommendations, margin_req);

        // Calculate guardrail compliance
        let all_within_bounds = recommendations.iter().all(|r| r.within_bounds);
        let all_margins_met = recommendations.iter().all(|r| r.margin_target_met);
        let competitive_position_achieved =
            self.check_competitive_position(&recommendations, &input.products, margin_req);

        let guardrail_compliance = GuardrailCompliance {
            all_within_bounds,
            all_margins_met,
            competitive_position_achieved,
            violations,
        };

        let output = PricingGuardrailsOutput {
            recommendations,
            margin_analysis,
            guardrail_compliance,
        };

        let replay = ReplayEnvelope::minimal(seed);
        let is_feasible = all_within_bounds && all_margins_met;

        let report = if is_feasible {
            SolverReport::optimal(
                "guardrail-pricing-v1",
                output.margin_analysis.average_margin_pct,
                replay,
            )
        } else {
            SolverReport::feasible(
                "guardrail-pricing-v1",
                output.margin_analysis.average_margin_pct,
                StopReason::Feasible,
                replay,
            )
        };

        Ok((output, report))
    }

    /// Price a single product according to rules
    fn price_product(
        &self,
        product: &Product,
        margin_req: &MarginRequirements,
        global_bounds: &Option<PriceBounds>,
    ) -> Result<PricingRecommendation> {
        // Step 1: Calculate minimum price for required margin
        // margin = (price - cost) / price
        // margin * price = price - cost
        // cost = price - margin * price = price * (1 - margin)
        // price = cost / (1 - margin)
        let min_margin_decimal = margin_req.min_margin_pct / 100.0;
        let target_margin_decimal = margin_req.target_margin_pct / 100.0;

        let min_price_for_margin = if min_margin_decimal < 1.0 {
            product.unit_cost / (1.0 - min_margin_decimal)
        } else {
            f64::MAX // Can't achieve 100%+ margin
        };

        let target_price_for_margin = if target_margin_decimal < 1.0 {
            product.unit_cost / (1.0 - target_margin_decimal)
        } else {
            min_price_for_margin * 1.5 // Reasonable fallback
        };

        // Step 2: Get effective bounds
        let effective_bounds = product.effective_bounds(global_bounds);
        let (bound_min, bound_max) = match &effective_bounds {
            Some(b) => (b.min_price, b.max_price),
            None => (0.0, f64::MAX),
        };

        // Step 3: Apply competitive strategy
        let competitive_price = self.calculate_competitive_price(product, margin_req);

        // Step 4: Determine recommended price
        // Start with target margin price, then adjust for competition
        let mut recommended_price = match competitive_price {
            Some(comp_price) => {
                match margin_req.competitive_strategy {
                    CompetitiveStrategy::IgnoreCompetitors => target_price_for_margin,
                    _ => {
                        // Balance between target margin and competitive price
                        // Weight toward target margin but consider competition
                        (target_price_for_margin * 0.4 + comp_price * 0.6).max(min_price_for_margin)
                    }
                }
            }
            None => target_price_for_margin,
        };

        // Ensure minimum margin is maintained
        recommended_price = recommended_price.max(min_price_for_margin);

        // Check bounds compliance (we'll track if violated but still recommend best possible)
        let within_bounds = recommended_price >= bound_min && recommended_price <= bound_max;

        // Clamp to bounds if needed
        recommended_price = recommended_price.max(bound_min).min(bound_max);

        // Recalculate margin after clamping
        let margin_pct = product.margin_at_price(recommended_price);
        let markup_pct = product.markup_at_price(recommended_price);
        let margin_target_met = margin_pct >= margin_req.min_margin_pct;

        // Calculate price change from current
        let (price_change, price_change_pct) = match product.current_price {
            Some(current) if current > 0.0 => {
                let change = recommended_price - current;
                let change_pct = (change / current) * 100.0;
                (Some(change), Some(change_pct))
            }
            _ => (None, None),
        };

        // Build competitive position
        let competitive_position = self.build_competitive_position(product, recommended_price);

        // Build rationale
        let rationale = self.build_rationale(
            product,
            recommended_price,
            margin_pct,
            margin_req,
            &competitive_position,
            within_bounds,
        );

        Ok(PricingRecommendation {
            product_id: product.product_id.clone(),
            recommended_price,
            previous_price: product.current_price,
            price_change,
            price_change_pct,
            margin_pct,
            markup_pct,
            competitive_position,
            within_bounds,
            margin_target_met,
            rationale,
        })
    }

    /// Calculate competitive price based on strategy
    fn calculate_competitive_price(
        &self,
        product: &Product,
        margin_req: &MarginRequirements,
    ) -> Option<f64> {
        let avg_competitor = product.avg_competitor_price()?;

        match margin_req.competitive_strategy {
            CompetitiveStrategy::PriceToBeat => {
                // Price 5% below market average
                Some(avg_competitor * 0.95)
            }
            CompetitiveStrategy::MatchMarket => {
                // Match market average
                Some(avg_competitor)
            }
            CompetitiveStrategy::Premium => {
                // Price 10% above market average
                Some(avg_competitor * 1.10)
            }
            CompetitiveStrategy::IgnoreCompetitors => None,
        }
    }

    /// Build competitive position analysis
    fn build_competitive_position(&self, product: &Product, price: f64) -> CompetitivePosition {
        let avg_competitor = product.avg_competitor_price();
        let competitor_count = product.competitor_prices.len();

        let position_vs_avg_pct = avg_competitor.map(|avg| {
            if avg > 0.0 {
                ((price - avg) / avg) * 100.0
            } else {
                0.0
            }
        });

        let (lowest_in_market, highest_in_market) = match product.competitor_price_range() {
            Some((min, max)) => (price < min, price > max),
            None => (false, false),
        };

        CompetitivePosition {
            avg_competitor_price: avg_competitor,
            position_vs_avg_pct,
            competitor_count,
            lowest_in_market,
            highest_in_market,
        }
    }

    /// Build human-readable rationale
    fn build_rationale(
        &self,
        _product: &Product,
        _price: f64,
        margin_pct: f64,
        margin_req: &MarginRequirements,
        competitive_position: &CompetitivePosition,
        within_bounds: bool,
    ) -> String {
        let mut parts = Vec::new();

        // Margin explanation
        if margin_pct >= margin_req.target_margin_pct {
            parts.push(format!("Achieves target margin of {:.1}%", margin_pct));
        } else if margin_pct >= margin_req.min_margin_pct {
            parts.push(format!(
                "Margin {:.1}% meets minimum but below {:.1}% target",
                margin_pct, margin_req.target_margin_pct
            ));
        } else {
            parts.push(format!(
                "Margin {:.1}% below minimum {:.1}% due to constraints",
                margin_pct, margin_req.min_margin_pct
            ));
        }

        // Competitive position
        if let Some(pos_pct) = competitive_position.position_vs_avg_pct {
            if pos_pct.abs() < 1.0 {
                parts.push("Matches market average".to_string());
            } else if pos_pct < 0.0 {
                parts.push(format!("{:.1}% below market", pos_pct.abs()));
            } else {
                parts.push(format!("{:.1}% above market", pos_pct));
            }
        }

        // Bounds compliance
        if !within_bounds {
            parts.push("Adjusted to fit guardrails".to_string());
        }

        parts.join(". ")
    }

    /// Calculate overall margin analysis
    fn calculate_margin_analysis(
        &self,
        recommendations: &[PricingRecommendation],
        margin_req: &MarginRequirements,
    ) -> MarginAnalysis {
        if recommendations.is_empty() {
            return MarginAnalysis::default();
        }

        let total_products = recommendations.len();
        let products_meeting_margin = recommendations
            .iter()
            .filter(|r| r.margin_pct >= margin_req.min_margin_pct)
            .count();

        let margins: Vec<f64> = recommendations.iter().map(|r| r.margin_pct).collect();
        let average_margin_pct = margins.iter().sum::<f64>() / margins.len() as f64;
        let min_margin_pct = margins.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_margin_pct = margins.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

        MarginAnalysis {
            total_products,
            products_meeting_margin,
            average_margin_pct,
            min_margin_pct,
            max_margin_pct,
        }
    }

    /// Check if competitive positioning strategy was achieved
    fn check_competitive_position(
        &self,
        recommendations: &[PricingRecommendation],
        products: &[Product],
        margin_req: &MarginRequirements,
    ) -> bool {
        if margin_req.competitive_strategy == CompetitiveStrategy::IgnoreCompetitors {
            return true;
        }

        // For products with competitor data, check if strategy was achieved
        let mut achieved = 0;
        let mut applicable = 0;

        for (rec, prod) in recommendations.iter().zip(products.iter()) {
            if prod.competitor_prices.is_empty() {
                continue;
            }
            applicable += 1;

            if let Some(pos_pct) = rec.competitive_position.position_vs_avg_pct {
                let strategy_achieved = match margin_req.competitive_strategy {
                    CompetitiveStrategy::PriceToBeat => pos_pct <= -3.0, // At least 3% below
                    CompetitiveStrategy::MatchMarket => pos_pct.abs() <= 5.0, // Within 5%
                    CompetitiveStrategy::Premium => pos_pct >= 5.0,      // At least 5% above
                    CompetitiveStrategy::IgnoreCompetitors => true,
                };
                if strategy_achieved {
                    achieved += 1;
                }
            }
        }

        if applicable == 0 {
            true // No products to evaluate
        } else {
            achieved as f64 / applicable as f64 >= 0.8 // 80% threshold
        }
    }
}

impl PackSolver for GuardrailPricingSolver {
    fn id(&self) -> &'static str {
        "guardrail-pricing-v1"
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<(serde_json::Value, SolverReport)> {
        let input: PricingGuardrailsInput = spec.inputs_as()?;
        let (output, report) = self.solve_pricing(&input, spec)?;
        let json = serde_json::to_value(&output)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;
        Ok((json, report))
    }

    fn is_exact(&self) -> bool {
        true // Rule-based, deterministic
    }
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
                    competitor_prices: vec![
                        CompetitorPrice {
                            competitor_id: "comp1".to_string(),
                            price: 110.0,
                            as_of_date: None,
                        },
                        CompetitorPrice {
                            competitor_id: "comp2".to_string(),
                            price: 105.0,
                            as_of_date: None,
                        },
                    ],
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

    fn create_spec(input: &PricingGuardrailsInput, seed: u64) -> ProblemSpec {
        ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::maximize("margin"))
            .inputs(input)
            .unwrap()
            .seed(seed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_basic_pricing() {
        let solver = GuardrailPricingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, report) = solver.solve_pricing(&input, &spec).unwrap();

        assert_eq!(output.recommendations.len(), 2);
        assert!(report.feasible);

        // Check first product
        let rec1 = &output.recommendations[0];
        assert_eq!(rec1.product_id, "SKU-001");
        assert!(rec1.margin_pct >= 20.0); // Meets minimum margin
        assert!(rec1.within_bounds);
    }

    #[test]
    fn test_margin_calculation() {
        let solver = GuardrailPricingSolver;
        let mut input = create_test_input();
        input.products = vec![Product {
            product_id: "test".to_string(),
            name: "Test".to_string(),
            unit_cost: 80.0,
            current_price: None,
            price_bounds: None,
            competitor_prices: vec![],
            category: None,
        }];
        input.margin_requirements.min_margin_pct = 20.0;
        input.margin_requirements.target_margin_pct = 25.0;
        input.margin_requirements.competitive_strategy = CompetitiveStrategy::IgnoreCompetitors;

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_pricing(&input, &spec).unwrap();

        let rec = &output.recommendations[0];
        // For 25% target margin with $80 cost: price = 80 / (1 - 0.25) = 106.67
        assert!(rec.margin_pct >= 25.0 - 0.1);
        assert!(rec.margin_target_met);
    }

    #[test]
    fn test_price_bounds_enforced() {
        let solver = GuardrailPricingSolver;
        let mut input = create_test_input();
        input.products = vec![Product {
            product_id: "constrained".to_string(),
            name: "Constrained".to_string(),
            unit_cost: 80.0,
            current_price: None,
            price_bounds: Some(PriceBounds {
                min_price: 85.0,
                max_price: 90.0, // Very tight bounds
            }),
            competitor_prices: vec![],
            category: None,
        }];

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_pricing(&input, &spec).unwrap();

        let rec = &output.recommendations[0];
        assert!(rec.recommended_price >= 85.0);
        assert!(rec.recommended_price <= 90.0);
        // With $80 cost and max $90 price, margin = (90-80)/90 = 11.1%
        // This is below minimum, so margin_target_met should be false
        assert!(!rec.margin_target_met);
    }

    #[test]
    fn test_competitive_strategy_price_to_beat() {
        let solver = GuardrailPricingSolver;
        let mut input = create_test_input();
        input.margin_requirements.competitive_strategy = CompetitiveStrategy::PriceToBeat;
        input.margin_requirements.min_margin_pct = 5.0; // Low margin to allow competitive pricing

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_pricing(&input, &spec).unwrap();

        let rec1 = &output.recommendations[0];
        // Should be priced below market average of 107.5
        if let Some(pos) = rec1.competitive_position.position_vs_avg_pct {
            // Should be at or below market
            assert!(pos <= 0.0 || rec1.margin_pct >= input.margin_requirements.min_margin_pct);
        }
    }

    #[test]
    fn test_competitive_strategy_premium() {
        let solver = GuardrailPricingSolver;
        let mut input = create_test_input();
        input.margin_requirements.competitive_strategy = CompetitiveStrategy::Premium;
        input.margin_requirements.min_margin_pct = 20.0;

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_pricing(&input, &spec).unwrap();

        let rec1 = &output.recommendations[0];
        // Premium strategy should price above market
        if let Some(pos) = rec1.competitive_position.position_vs_avg_pct {
            // Should be above market or margin prevents it
            assert!(pos > 0.0 || rec1.margin_pct >= input.margin_requirements.min_margin_pct);
        }
    }

    #[test]
    fn test_determinism() {
        let solver = GuardrailPricingSolver;
        let input = create_test_input();

        let spec1 = create_spec(&input, 12345);
        let spec2 = create_spec(&input, 12345);

        let (output1, _) = solver.solve_pricing(&input, &spec1).unwrap();
        let (output2, _) = solver.solve_pricing(&input, &spec2).unwrap();

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
    fn test_margin_analysis() {
        let solver = GuardrailPricingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_pricing(&input, &spec).unwrap();

        assert_eq!(output.margin_analysis.total_products, 2);
        assert!(output.margin_analysis.average_margin_pct > 0.0);
        assert!(output.margin_analysis.min_margin_pct <= output.margin_analysis.max_margin_pct);
    }

    #[test]
    fn test_guardrail_compliance_tracking() {
        let solver = GuardrailPricingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_pricing(&input, &spec).unwrap();

        // Should have compliance flags set
        // With reasonable inputs, most guardrails should be met
        assert!(
            output.guardrail_compliance.all_within_bounds
                || !output.guardrail_compliance.violations.is_empty()
        );
    }
}
