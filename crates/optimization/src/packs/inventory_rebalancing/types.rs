//! Types for Inventory Rebalancing pack

use converge_pack::PackSchema;
use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Input for inventory rebalancing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryRebalancingInput {
    /// Locations (warehouses, stores, etc.)
    pub locations: Vec<Location>,
    /// Products to consider
    pub products: Vec<Product>,
    /// Current inventory levels
    pub inventory: Vec<InventoryLevel>,
    /// Transfer costs between locations
    pub transfer_costs: Vec<TransferCost>,
    /// Constraints on rebalancing
    pub constraints: RebalancingConstraints,
}

impl InventoryRebalancingInput {
    /// Validate the input
    pub fn validate(&self) -> Result<()> {
        if self.locations.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "no locations provided",
            ));
        }
        if self.products.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "no products provided",
            ));
        }

        // Validate each location
        for loc in &self.locations {
            loc.validate()?;
        }

        // Validate each product
        for prod in &self.products {
            prod.validate()?;
        }

        // Validate inventory levels
        for inv in &self.inventory {
            inv.validate()?;
        }

        // Validate constraints
        self.constraints.validate()?;

        Ok(())
    }

    /// Get a location by ID
    pub fn get_location(&self, id: &str) -> Option<&Location> {
        self.locations.iter().find(|l| l.id == id)
    }

    /// Get a product by ID
    pub fn get_product(&self, id: &str) -> Option<&Product> {
        self.products.iter().find(|p| p.id == id)
    }

    /// Get inventory level for a location/product pair
    pub fn get_inventory(&self, location_id: &str, product_id: &str) -> Option<&InventoryLevel> {
        self.inventory
            .iter()
            .find(|i| i.location_id == location_id && i.product_id == product_id)
    }

    /// Get transfer cost between two locations
    pub fn get_transfer_cost(&self, from: &str, to: &str) -> Option<&TransferCost> {
        self.transfer_costs
            .iter()
            .find(|c| c.from_location == from && c.to_location == to)
    }

    /// Build a map of (location, product) -> inventory level
    pub fn inventory_map(&self) -> HashMap<(String, String), &InventoryLevel> {
        self.inventory
            .iter()
            .map(|i| ((i.location_id.clone(), i.product_id.clone()), i))
            .collect()
    }
}

impl PackSchema for InventoryRebalancingInput {
    fn validate(&self) -> Result<()> {
        InventoryRebalancingInput::validate(self)
    }
}

/// A location (warehouse, store, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    /// Unique location identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Total capacity
    pub capacity: i64,
    /// Type of location
    pub location_type: LocationType,
}

impl Location {
    /// Validate the location
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "location id is required",
            ));
        }
        if self.capacity <= 0 {
            return Err(converge_pack::GateError::invalid_input(
                "location capacity must be positive",
            ));
        }
        Ok(())
    }
}

/// Type of location
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationType {
    /// Main warehouse
    Warehouse,
    /// Retail store
    Store,
    /// Distribution center
    DistributionCenter,
}

/// A product
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Product {
    /// Unique product identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Weight per unit
    pub unit_weight: f64,
    /// Value per unit
    pub unit_value: f64,
}

impl Product {
    /// Validate the product
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "product id is required",
            ));
        }
        Ok(())
    }
}

/// Current inventory level at a location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryLevel {
    /// Location ID
    pub location_id: String,
    /// Product ID
    pub product_id: String,
    /// Current quantity on hand
    pub quantity: i64,
    /// Target (desired) quantity
    pub target_quantity: i64,
    /// Minimum quantity (safety stock)
    pub min_quantity: i64,
    /// Maximum quantity (capacity limit)
    pub max_quantity: i64,
}

impl InventoryLevel {
    /// Validate the inventory level
    pub fn validate(&self) -> Result<()> {
        if self.min_quantity > self.max_quantity {
            return Err(converge_pack::GateError::invalid_input(
                "min_quantity cannot exceed max_quantity",
            ));
        }
        if self.target_quantity < self.min_quantity || self.target_quantity > self.max_quantity {
            return Err(converge_pack::GateError::invalid_input(
                "target_quantity must be between min and max",
            ));
        }
        Ok(())
    }

    /// Calculate deficit (negative) or surplus (positive)
    pub fn deficit(&self) -> i64 {
        self.quantity - self.target_quantity
    }

    /// Check if this location has surplus
    pub fn has_surplus(&self) -> bool {
        self.quantity > self.target_quantity
    }

    /// Check if this location has deficit
    pub fn has_deficit(&self) -> bool {
        self.quantity < self.target_quantity
    }

    /// Get available surplus (respecting safety stock)
    pub fn available_surplus(&self) -> i64 {
        (self.quantity - self.min_quantity).max(0)
    }

    /// Get space available (respecting max capacity)
    pub fn available_space(&self) -> i64 {
        (self.max_quantity - self.quantity).max(0)
    }
}

/// Cost to transfer between locations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferCost {
    /// Source location ID
    pub from_location: String,
    /// Destination location ID
    pub to_location: String,
    /// Cost per unit transferred
    pub cost_per_unit: f64,
    /// Lead time in hours
    pub lead_time_hours: i64,
}

/// Constraints on rebalancing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalancingConstraints {
    /// Maximum number of transfers allowed
    pub max_total_transfers: usize,
    /// Maximum quantity per single transfer
    pub max_transfer_quantity: i64,
    /// Maximum total cost allowed
    pub max_total_cost: f64,
}

impl RebalancingConstraints {
    /// Validate constraints
    pub fn validate(&self) -> Result<()> {
        if self.max_total_cost < 0.0 {
            return Err(converge_pack::GateError::invalid_input(
                "max_total_cost cannot be negative",
            ));
        }
        Ok(())
    }
}

/// Output transfer plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryRebalancingOutput {
    /// Planned transfers
    pub transfers: Vec<Transfer>,
    /// Total cost of all transfers
    pub total_cost: f64,
    /// Total units moved
    pub total_units_moved: i64,
    /// Estimated service level improvement
    pub service_level_improvement: f64,
    /// Per-location impact
    pub location_impacts: Vec<LocationImpact>,
}

impl InventoryRebalancingOutput {
    /// Create an empty output (no transfers)
    pub fn empty() -> Self {
        Self {
            transfers: Vec::new(),
            total_cost: 0.0,
            total_units_moved: 0,
            service_level_improvement: 0.0,
            location_impacts: Vec::new(),
        }
    }

    /// Create a summary string
    pub fn summary(&self) -> String {
        if self.transfers.is_empty() {
            return "No transfers needed - inventory is balanced".to_string();
        }
        format!(
            "{} transfers moving {} units for ${:.2} total cost, {:.1}% service improvement",
            self.transfers.len(),
            self.total_units_moved,
            self.total_cost,
            self.service_level_improvement * 100.0
        )
    }
}

impl PackSchema for InventoryRebalancingOutput {
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

/// A single transfer in the plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transfer {
    /// Source location ID
    pub from_location: String,
    /// Destination location ID
    pub to_location: String,
    /// Product ID
    pub product_id: String,
    /// Quantity to transfer
    pub quantity: i64,
    /// Transfer cost
    pub cost: f64,
    /// Expected arrival time (hours from now)
    pub expected_arrival_hours: i64,
}

impl Transfer {
    /// Create a new transfer
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        product: impl Into<String>,
        quantity: i64,
        cost: f64,
        arrival_hours: i64,
    ) -> Self {
        Self {
            from_location: from.into(),
            to_location: to.into(),
            product_id: product.into(),
            quantity,
            cost,
            expected_arrival_hours: arrival_hours,
        }
    }
}

/// Impact on a specific location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationImpact {
    /// Location ID
    pub location_id: String,
    /// Product ID
    pub product_id: String,
    /// Inventory change (negative = outgoing, positive = incoming)
    pub inventory_change: i64,
    /// Final quantity after transfers
    pub final_quantity: i64,
    /// Whether target will be met
    pub meets_target: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inventory_level_deficit() {
        let inv = InventoryLevel {
            location_id: "loc".to_string(),
            product_id: "prod".to_string(),
            quantity: 30,
            target_quantity: 50,
            min_quantity: 10,
            max_quantity: 100,
        };

        assert!(inv.has_deficit());
        assert!(!inv.has_surplus());
        assert_eq!(inv.deficit(), -20);
        assert_eq!(inv.available_surplus(), 20); // 30 - 10
        assert_eq!(inv.available_space(), 70); // 100 - 30
    }

    #[test]
    fn test_inventory_level_surplus() {
        let inv = InventoryLevel {
            location_id: "loc".to_string(),
            product_id: "prod".to_string(),
            quantity: 80,
            target_quantity: 50,
            min_quantity: 10,
            max_quantity: 100,
        };

        assert!(!inv.has_deficit());
        assert!(inv.has_surplus());
        assert_eq!(inv.deficit(), 30);
        assert_eq!(inv.available_surplus(), 70); // 80 - 10
        assert_eq!(inv.available_space(), 20); // 100 - 80
    }

    #[test]
    fn test_location_validation() {
        let valid = Location {
            id: "loc-1".to_string(),
            name: "Warehouse".to_string(),
            capacity: 1000,
            location_type: LocationType::Warehouse,
        };
        assert!(valid.validate().is_ok());

        let no_id = Location {
            id: "".to_string(),
            name: "Warehouse".to_string(),
            capacity: 1000,
            location_type: LocationType::Warehouse,
        };
        assert!(no_id.validate().is_err());
    }

    #[test]
    fn test_inventory_level_validation() {
        let valid = InventoryLevel {
            location_id: "loc".to_string(),
            product_id: "prod".to_string(),
            quantity: 50,
            target_quantity: 50,
            min_quantity: 10,
            max_quantity: 100,
        };
        assert!(valid.validate().is_ok());

        let invalid_minmax = InventoryLevel {
            location_id: "loc".to_string(),
            product_id: "prod".to_string(),
            quantity: 50,
            target_quantity: 50,
            min_quantity: 100,
            max_quantity: 10, // min > max
        };
        assert!(invalid_minmax.validate().is_err());
    }

    #[test]
    fn test_output_summary() {
        let output = InventoryRebalancingOutput {
            transfers: vec![Transfer::new("a", "b", "prod", 50, 25.0, 24)],
            total_cost: 25.0,
            total_units_moved: 50,
            service_level_improvement: 0.15,
            location_impacts: vec![],
        };

        let summary = output.summary();
        assert!(summary.contains("1 transfers"));
        assert!(summary.contains("50 units"));
        assert!(summary.contains("$25.00"));
    }
}
