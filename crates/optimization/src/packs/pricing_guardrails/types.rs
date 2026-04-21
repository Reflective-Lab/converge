//! Types for Pricing Guardrails pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for pricing guardrails optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PricingGuardrailsInput {
    /// Products to price
    pub products: Vec<Product>,
    /// Margin requirements
    pub margin_requirements: MarginRequirements,
    /// Global price bounds (optional)
    pub price_bounds: Option<PriceBounds>,
}

impl PricingGuardrailsInput {
    /// Validate the input
    pub fn validate(&self) -> Result<()> {
        if self.products.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one product is required",
            ));
        }

        for product in &self.products {
            if product.unit_cost < 0.0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Product {} has negative unit cost",
                    product.product_id
                )));
            }
            if let Some(ref bounds) = product.price_bounds {
                if bounds.min_price < 0.0 {
                    return Err(converge_pack::GateError::invalid_input(format!(
                        "Product {} has negative min price",
                        product.product_id
                    )));
                }
                if bounds.min_price > bounds.max_price {
                    return Err(converge_pack::GateError::invalid_input(format!(
                        "Product {} has min price > max price",
                        product.product_id
                    )));
                }
            }
        }

        if self.margin_requirements.min_margin_pct < 0.0 {
            return Err(converge_pack::GateError::invalid_input(
                "Minimum margin cannot be negative",
            ));
        }
        if self.margin_requirements.min_margin_pct > 100.0 {
            return Err(converge_pack::GateError::invalid_input(
                "Minimum margin cannot exceed 100%",
            ));
        }

        Ok(())
    }

    /// Get products with competitor pricing data
    pub fn products_with_competitor_data(&self) -> impl Iterator<Item = &Product> {
        self.products
            .iter()
            .filter(|p| !p.competitor_prices.is_empty())
    }
}

/// A product to price
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Product {
    /// Product identifier
    pub product_id: String,
    /// Product name
    pub name: String,
    /// Unit cost (COGS)
    pub unit_cost: f64,
    /// Current price (if any)
    pub current_price: Option<f64>,
    /// Product-specific price bounds (overrides global)
    pub price_bounds: Option<PriceBounds>,
    /// Competitor prices for this product
    pub competitor_prices: Vec<CompetitorPrice>,
    /// Product category (for category-level rules)
    pub category: Option<String>,
}

impl Product {
    /// Calculate margin at a given price
    pub fn margin_at_price(&self, price: f64) -> f64 {
        if price <= 0.0 {
            return 0.0;
        }
        ((price - self.unit_cost) / price) * 100.0
    }

    /// Calculate markup at a given price
    pub fn markup_at_price(&self, price: f64) -> f64 {
        if self.unit_cost <= 0.0 {
            return 0.0;
        }
        ((price - self.unit_cost) / self.unit_cost) * 100.0
    }

    /// Get effective price bounds (product-specific or global)
    pub fn effective_bounds(&self, global_bounds: &Option<PriceBounds>) -> Option<PriceBounds> {
        self.price_bounds.clone().or_else(|| global_bounds.clone())
    }

    /// Get average competitor price
    pub fn avg_competitor_price(&self) -> Option<f64> {
        if self.competitor_prices.is_empty() {
            return None;
        }
        let sum: f64 = self.competitor_prices.iter().map(|c| c.price).sum();
        Some(sum / self.competitor_prices.len() as f64)
    }

    /// Get min and max competitor prices
    pub fn competitor_price_range(&self) -> Option<(f64, f64)> {
        if self.competitor_prices.is_empty() {
            return None;
        }
        let min = self
            .competitor_prices
            .iter()
            .map(|c| c.price)
            .fold(f64::INFINITY, f64::min);
        let max = self
            .competitor_prices
            .iter()
            .map(|c| c.price)
            .fold(f64::NEG_INFINITY, f64::max);
        Some((min, max))
    }
}

/// Competitor price data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitorPrice {
    /// Competitor identifier
    pub competitor_id: String,
    /// Competitor's price for this product
    pub price: f64,
    /// Price as of date (optional)
    pub as_of_date: Option<String>,
}

/// Price bounds for guardrails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceBounds {
    /// Minimum allowed price
    pub min_price: f64,
    /// Maximum allowed price
    pub max_price: f64,
}

impl Default for PriceBounds {
    fn default() -> Self {
        Self {
            min_price: 0.0,
            max_price: f64::MAX,
        }
    }
}

/// Margin requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarginRequirements {
    /// Minimum margin percentage (e.g., 20.0 for 20%)
    pub min_margin_pct: f64,
    /// Target margin percentage (optimal)
    pub target_margin_pct: f64,
    /// Competitive position strategy
    pub competitive_strategy: CompetitiveStrategy,
}

impl Default for MarginRequirements {
    fn default() -> Self {
        Self {
            min_margin_pct: 10.0,
            target_margin_pct: 25.0,
            competitive_strategy: CompetitiveStrategy::MatchMarket,
        }
    }
}

/// Competitive positioning strategy
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum CompetitiveStrategy {
    /// Price below market average
    PriceToBeat,
    /// Match market average
    #[default]
    MatchMarket,
    /// Price above market (premium positioning)
    Premium,
    /// Ignore competitor pricing
    IgnoreCompetitors,
}

/// Output for pricing guardrails optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PricingGuardrailsOutput {
    /// Pricing recommendations per product
    pub recommendations: Vec<PricingRecommendation>,
    /// Overall margin analysis
    pub margin_analysis: MarginAnalysis,
    /// Guardrail compliance summary
    pub guardrail_compliance: GuardrailCompliance,
}

impl PricingGuardrailsOutput {
    /// Create output when no valid pricing exists
    pub fn no_valid_pricing(reason: &str) -> Self {
        Self {
            recommendations: vec![],
            margin_analysis: MarginAnalysis {
                total_products: 0,
                products_meeting_margin: 0,
                average_margin_pct: 0.0,
                min_margin_pct: 0.0,
                max_margin_pct: 0.0,
            },
            guardrail_compliance: GuardrailCompliance {
                all_within_bounds: false,
                all_margins_met: false,
                competitive_position_achieved: false,
                violations: vec![reason.to_string()],
            },
        }
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        let compliant = self.guardrail_compliance.all_within_bounds
            && self.guardrail_compliance.all_margins_met;
        format!(
            "Priced {} products, avg margin {:.1}%, {}",
            self.recommendations.len(),
            self.margin_analysis.average_margin_pct,
            if compliant {
                "all guardrails passed"
            } else {
                "some guardrails violated"
            }
        )
    }
}

/// Pricing recommendation for a single product
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingRecommendation {
    /// Product identifier
    pub product_id: String,
    /// Recommended price
    pub recommended_price: f64,
    /// Previous price (if any)
    pub previous_price: Option<f64>,
    /// Price change from previous
    pub price_change: Option<f64>,
    /// Price change percentage
    pub price_change_pct: Option<f64>,
    /// Margin at recommended price
    pub margin_pct: f64,
    /// Markup at recommended price
    pub markup_pct: f64,
    /// Position relative to competitors
    pub competitive_position: CompetitivePosition,
    /// Whether this price is within bounds
    pub within_bounds: bool,
    /// Whether margin target is met
    pub margin_target_met: bool,
    /// Pricing rationale
    pub rationale: String,
}

/// Position relative to competitor prices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompetitivePosition {
    /// Average competitor price (if available)
    pub avg_competitor_price: Option<f64>,
    /// Position relative to average (-10 = 10% below, +10 = 10% above)
    pub position_vs_avg_pct: Option<f64>,
    /// Number of competitors priced
    pub competitor_count: usize,
    /// Is price below all competitors?
    pub lowest_in_market: bool,
    /// Is price above all competitors?
    pub highest_in_market: bool,
}

impl Default for CompetitivePosition {
    fn default() -> Self {
        Self {
            avg_competitor_price: None,
            position_vs_avg_pct: None,
            competitor_count: 0,
            lowest_in_market: false,
            highest_in_market: false,
        }
    }
}

/// Overall margin analysis
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarginAnalysis {
    /// Total products priced
    pub total_products: usize,
    /// Products meeting minimum margin
    pub products_meeting_margin: usize,
    /// Average margin across all products
    pub average_margin_pct: f64,
    /// Minimum margin in recommendations
    pub min_margin_pct: f64,
    /// Maximum margin in recommendations
    pub max_margin_pct: f64,
}

/// Guardrail compliance summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GuardrailCompliance {
    /// All prices within bounds
    pub all_within_bounds: bool,
    /// All minimum margins met
    pub all_margins_met: bool,
    /// Competitive position achieved per strategy
    pub competitive_position_achieved: bool,
    /// List of violations (if any)
    pub violations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_product(id: &str, cost: f64, competitor_price: Option<f64>) -> Product {
        let competitor_prices = competitor_price
            .map(|p| {
                vec![CompetitorPrice {
                    competitor_id: "comp1".to_string(),
                    price: p,
                    as_of_date: None,
                }]
            })
            .unwrap_or_default();

        Product {
            product_id: id.to_string(),
            name: format!("Product {}", id),
            unit_cost: cost,
            current_price: None,
            price_bounds: None,
            competitor_prices,
            category: None,
        }
    }

    #[test]
    fn test_margin_calculation() {
        let product = create_test_product("p1", 80.0, None);
        // Price $100, cost $80 -> margin = (100-80)/100 = 20%
        assert!((product.margin_at_price(100.0) - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_markup_calculation() {
        let product = create_test_product("p1", 80.0, None);
        // Price $100, cost $80 -> markup = (100-80)/80 = 25%
        assert!((product.markup_at_price(100.0) - 25.0).abs() < 0.01);
    }

    #[test]
    fn test_competitor_price_average() {
        let mut product = create_test_product("p1", 80.0, Some(100.0));
        product.competitor_prices.push(CompetitorPrice {
            competitor_id: "comp2".to_string(),
            price: 120.0,
            as_of_date: None,
        });

        let avg = product.avg_competitor_price().unwrap();
        assert!((avg - 110.0).abs() < 0.01);
    }

    #[test]
    fn test_competitor_price_range() {
        let mut product = create_test_product("p1", 80.0, Some(100.0));
        product.competitor_prices.push(CompetitorPrice {
            competitor_id: "comp2".to_string(),
            price: 120.0,
            as_of_date: None,
        });

        let (min, max) = product.competitor_price_range().unwrap();
        assert!((min - 100.0).abs() < 0.01);
        assert!((max - 120.0).abs() < 0.01);
    }

    #[test]
    fn test_input_validation() {
        let mut input = PricingGuardrailsInput {
            products: vec![create_test_product("p1", 80.0, None)],
            margin_requirements: MarginRequirements::default(),
            price_bounds: None,
        };

        assert!(input.validate().is_ok());

        // Test negative cost
        input.products[0].unit_cost = -10.0;
        assert!(input.validate().is_err());
        input.products[0].unit_cost = 80.0;

        // Test invalid margin requirement
        input.margin_requirements.min_margin_pct = 150.0;
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_price_bounds_validation() {
        let input = PricingGuardrailsInput {
            products: vec![Product {
                product_id: "p1".to_string(),
                name: "Product 1".to_string(),
                unit_cost: 80.0,
                current_price: None,
                price_bounds: Some(PriceBounds {
                    min_price: 120.0,
                    max_price: 100.0, // Invalid: min > max
                }),
                competitor_prices: vec![],
                category: None,
            }],
            margin_requirements: MarginRequirements::default(),
            price_bounds: None,
        };

        assert!(input.validate().is_err());
    }
}
