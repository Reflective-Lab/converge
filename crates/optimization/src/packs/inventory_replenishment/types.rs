//! Types for Inventory Replenishment pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for inventory replenishment optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InventoryReplenishmentInput {
    /// Products to manage
    pub products: Vec<Product>,
    /// Replenishment constraints
    pub constraints: ReplenishmentConstraints,
}

impl InventoryReplenishmentInput {
    /// Validate the input
    pub fn validate(&self) -> Result<()> {
        if self.products.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one product is required",
            ));
        }
        if self.constraints.budget <= 0.0 {
            return Err(converge_pack::GateError::invalid_input(
                "Budget must be positive",
            ));
        }
        if self.constraints.target_service_level <= 0.0
            || self.constraints.target_service_level > 1.0
        {
            return Err(converge_pack::GateError::invalid_input(
                "Target service level must be between 0 and 1",
            ));
        }
        for product in &self.products {
            if product.current_inventory < 0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Product {} has negative inventory",
                    product.id
                )));
            }
            if product.lead_time_days < 0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Product {} has negative lead time",
                    product.id
                )));
            }
        }
        Ok(())
    }

    /// Get products that need replenishment (below reorder point)
    pub fn products_needing_reorder(&self) -> impl Iterator<Item = &Product> {
        self.products.iter().filter(|p| p.needs_reorder())
    }

    /// Get total estimated demand across all products
    pub fn total_demand(&self) -> f64 {
        self.products
            .iter()
            .map(|p| p.total_forecast_demand())
            .sum()
    }
}

/// A product with inventory and demand information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    /// Product identifier
    pub id: String,
    /// Product name
    pub name: String,
    /// Current inventory level (units)
    pub current_inventory: i64,
    /// Demand forecast (units per day)
    pub demand_forecast: DemandForecast,
    /// Lead time in days for replenishment
    pub lead_time_days: i64,
    /// Cost per unit to order
    pub unit_cost: f64,
    /// Fixed cost per order (ordering cost)
    pub ordering_cost: f64,
    /// Holding cost per unit per day
    pub holding_cost_per_day: f64,
    /// Stockout cost per unit (cost of not having stock)
    pub stockout_cost: f64,
}

impl Product {
    /// Calculate Economic Order Quantity (EOQ)
    /// EOQ = sqrt(2 * D * S / H)
    /// where D = annual demand, S = ordering cost, H = annual holding cost
    pub fn calculate_eoq(&self) -> f64 {
        let annual_demand = self.demand_forecast.average_daily * 365.0;
        let annual_holding_cost = self.holding_cost_per_day * 365.0;

        if annual_holding_cost <= 0.0 || annual_demand <= 0.0 {
            return 0.0;
        }

        ((2.0 * annual_demand * self.ordering_cost) / annual_holding_cost).sqrt()
    }

    /// Calculate safety stock based on service level
    /// Safety stock = Z * sigma * sqrt(L)
    /// where Z = service factor, sigma = demand std dev, L = lead time
    pub fn calculate_safety_stock(&self, service_level: f64) -> f64 {
        let z_score = self.service_level_to_z_score(service_level);
        let demand_std_dev = self.demand_forecast.std_deviation;
        let lead_time = self.lead_time_days as f64;

        z_score * demand_std_dev * lead_time.sqrt()
    }

    /// Convert service level to Z-score (approximate)
    fn service_level_to_z_score(&self, service_level: f64) -> f64 {
        // Common service levels and their Z-scores
        match service_level {
            s if s >= 0.99 => 2.33,
            s if s >= 0.98 => 2.05,
            s if s >= 0.97 => 1.88,
            s if s >= 0.95 => 1.65,
            s if s >= 0.90 => 1.28,
            s if s >= 0.85 => 1.04,
            s if s >= 0.80 => 0.84,
            _ => 0.5,
        }
    }

    /// Calculate reorder point
    /// ROP = (Average daily demand * Lead time) + Safety stock
    pub fn calculate_reorder_point(&self, service_level: f64) -> f64 {
        let demand_during_lead_time =
            self.demand_forecast.average_daily * self.lead_time_days as f64;
        let safety_stock = self.calculate_safety_stock(service_level);
        demand_during_lead_time + safety_stock
    }

    /// Check if product needs reorder based on current inventory
    pub fn needs_reorder(&self) -> bool {
        // Conservative check: reorder if inventory is below average lead time demand
        let demand_during_lead_time =
            self.demand_forecast.average_daily * self.lead_time_days as f64;
        (self.current_inventory as f64) < demand_during_lead_time * 1.5
    }

    /// Calculate total forecast demand for planning horizon
    pub fn total_forecast_demand(&self) -> f64 {
        self.demand_forecast.average_daily * self.demand_forecast.forecast_days as f64
    }

    /// Calculate days of inventory remaining
    pub fn days_of_inventory(&self) -> f64 {
        if self.demand_forecast.average_daily <= 0.0 {
            return f64::INFINITY;
        }
        self.current_inventory as f64 / self.demand_forecast.average_daily
    }

    /// Calculate total order cost for a given quantity
    pub fn total_order_cost(&self, quantity: i64) -> f64 {
        self.ordering_cost + (self.unit_cost * quantity as f64)
    }
}

/// Demand forecast for a product
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DemandForecast {
    /// Average daily demand (units)
    pub average_daily: f64,
    /// Standard deviation of daily demand
    pub std_deviation: f64,
    /// Number of days in forecast horizon
    pub forecast_days: i64,
}

impl DemandForecast {
    /// Create a new demand forecast
    pub fn new(average_daily: f64, std_deviation: f64, forecast_days: i64) -> Self {
        Self {
            average_daily,
            std_deviation,
            forecast_days,
        }
    }

    /// Calculate coefficient of variation
    pub fn coefficient_of_variation(&self) -> f64 {
        if self.average_daily <= 0.0 {
            return 0.0;
        }
        self.std_deviation / self.average_daily
    }
}

/// Constraints for replenishment planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplenishmentConstraints {
    /// Total budget for replenishment orders
    pub budget: f64,
    /// Target service level (0.0 to 1.0, e.g., 0.95 for 95%)
    pub target_service_level: f64,
    /// Planning horizon in days
    pub planning_horizon_days: i64,
    /// Maximum number of orders allowed
    pub max_orders: Option<usize>,
    /// Minimum order quantity (if any)
    pub min_order_quantity: Option<i64>,
}

impl Default for ReplenishmentConstraints {
    fn default() -> Self {
        Self {
            budget: 100000.0,
            target_service_level: 0.95,
            planning_horizon_days: 30,
            max_orders: None,
            min_order_quantity: None,
        }
    }
}

/// Output for inventory replenishment optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InventoryReplenishmentOutput {
    /// Recommended replenishment orders
    pub orders: Vec<ReplenishmentOrder>,
    /// Products not ordered with reasons
    pub not_ordered: Vec<NotOrderedProduct>,
    /// Projected inventory levels
    pub projections: Vec<InventoryProjection>,
    /// Summary statistics
    pub stats: ReplenishmentStats,
}

impl InventoryReplenishmentOutput {
    /// Create empty output when no orders needed
    pub fn no_orders(reason: &str) -> Self {
        Self {
            orders: vec![],
            not_ordered: vec![],
            projections: vec![],
            stats: ReplenishmentStats {
                total_order_cost: 0.0,
                total_units_ordered: 0,
                products_ordered: 0,
                products_skipped: 0,
                budget_utilization: 0.0,
                projected_service_level: 0.0,
                reason: reason.to_string(),
            },
        }
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        format!(
            "Ordered {} units across {} products, total cost ${:.2}, projected service level {:.1}%",
            self.stats.total_units_ordered,
            self.stats.products_ordered,
            self.stats.total_order_cost,
            self.stats.projected_service_level * 100.0
        )
    }

    /// Get total order cost
    pub fn total_cost(&self) -> f64 {
        self.stats.total_order_cost
    }
}

/// A replenishment order recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplenishmentOrder {
    /// Product identifier
    pub product_id: String,
    /// Product name
    pub product_name: String,
    /// Quantity to order
    pub quantity: i64,
    /// Order timing (days from now)
    pub order_day: i64,
    /// Expected arrival day
    pub arrival_day: i64,
    /// Order cost (ordering + unit costs)
    pub order_cost: f64,
    /// Unit cost
    pub unit_cost: f64,
    /// Calculated EOQ for reference
    pub eoq: f64,
    /// Safety stock level
    pub safety_stock: f64,
    /// Reorder point
    pub reorder_point: f64,
    /// Reason for this order
    pub order_reason: String,
}

/// A product that was not ordered with reason
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotOrderedProduct {
    /// Product identifier
    pub product_id: String,
    /// Product name
    pub product_name: String,
    /// Reason not ordered
    pub reason: String,
    /// Current inventory level
    pub current_inventory: i64,
    /// Days of inventory remaining
    pub days_remaining: f64,
}

/// Projected inventory level at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryProjection {
    /// Product identifier
    pub product_id: String,
    /// Day in planning horizon
    pub day: i64,
    /// Projected inventory level
    pub projected_inventory: i64,
    /// Projected stockout probability
    pub stockout_probability: f64,
    /// Whether order is arriving this day
    pub order_arriving: bool,
}

/// Summary statistics for replenishment plan
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReplenishmentStats {
    /// Total cost of all orders
    pub total_order_cost: f64,
    /// Total units ordered
    pub total_units_ordered: i64,
    /// Number of products ordered
    pub products_ordered: usize,
    /// Number of products skipped
    pub products_skipped: usize,
    /// Budget utilization percentage
    pub budget_utilization: f64,
    /// Projected overall service level
    pub projected_service_level: f64,
    /// Additional notes/reason
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_product(id: &str, inventory: i64, demand: f64) -> Product {
        Product {
            id: id.to_string(),
            name: format!("Product {}", id),
            current_inventory: inventory,
            demand_forecast: DemandForecast {
                average_daily: demand,
                std_deviation: demand * 0.2,
                forecast_days: 30,
            },
            lead_time_days: 7,
            unit_cost: 10.0,
            ordering_cost: 50.0,
            holding_cost_per_day: 0.02,
            stockout_cost: 25.0,
        }
    }

    #[test]
    fn test_eoq_calculation() {
        let product = create_test_product("p1", 100, 10.0);
        let eoq = product.calculate_eoq();
        // EOQ = sqrt(2 * 3650 * 50 / 7.3) = sqrt(50000) ~ 223
        assert!(eoq > 200.0 && eoq < 250.0);
    }

    #[test]
    fn test_safety_stock_calculation() {
        let product = create_test_product("p1", 100, 10.0);
        let safety_stock = product.calculate_safety_stock(0.95);
        // Z=1.65, sigma=2.0, sqrt(7) ~ 2.65
        // Safety stock = 1.65 * 2.0 * 2.65 ~ 8.7
        assert!(safety_stock > 5.0 && safety_stock < 15.0);
    }

    #[test]
    fn test_reorder_point_calculation() {
        let product = create_test_product("p1", 100, 10.0);
        let rop = product.calculate_reorder_point(0.95);
        // Demand during lead time = 10 * 7 = 70
        // Safety stock ~ 8.7
        // ROP ~ 78.7
        assert!(rop > 70.0 && rop < 90.0);
    }

    #[test]
    fn test_needs_reorder() {
        let low_inventory = create_test_product("p1", 50, 10.0);
        let high_inventory = create_test_product("p2", 500, 10.0);

        assert!(low_inventory.needs_reorder()); // 50 < 70 * 1.5 = 105
        assert!(!high_inventory.needs_reorder()); // 500 > 105
    }

    #[test]
    fn test_days_of_inventory() {
        let product = create_test_product("p1", 100, 10.0);
        let days = product.days_of_inventory();
        assert!((days - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_input_validation() {
        let mut input = InventoryReplenishmentInput {
            products: vec![create_test_product("p1", 100, 10.0)],
            constraints: ReplenishmentConstraints::default(),
        };

        assert!(input.validate().is_ok());

        input.constraints.budget = -100.0;
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_input_validation_service_level() {
        let mut input = InventoryReplenishmentInput {
            products: vec![create_test_product("p1", 100, 10.0)],
            constraints: ReplenishmentConstraints::default(),
        };

        input.constraints.target_service_level = 1.5;
        assert!(input.validate().is_err());

        input.constraints.target_service_level = 0.0;
        assert!(input.validate().is_err());
    }

    #[test]
    fn test_coefficient_of_variation() {
        let forecast = DemandForecast::new(10.0, 2.0, 30);
        assert!((forecast.coefficient_of_variation() - 0.2).abs() < 0.01);
    }
}
