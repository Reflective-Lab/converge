//! Types for Shipping Choice pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for shipping choice optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShippingChoiceInput {
    /// Order details
    pub order: OrderDetails,
    /// Available carrier options
    pub carriers: Vec<CarrierOption>,
    /// Required delivery within this many days
    pub sla_days: i64,
}

impl ShippingChoiceInput {
    /// Validate the input
    pub fn validate(&self) -> Result<()> {
        if self.carriers.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one carrier is required",
            ));
        }
        if self.sla_days <= 0 {
            return Err(converge_pack::GateError::invalid_input(
                "SLA days must be positive",
            ));
        }
        if self.order.weight_kg < 0.0 {
            return Err(converge_pack::GateError::invalid_input(
                "Weight cannot be negative",
            ));
        }
        Ok(())
    }

    /// Get carriers that meet the SLA requirement
    pub fn carriers_meeting_sla(&self) -> impl Iterator<Item = &CarrierOption> {
        self.carriers
            .iter()
            .filter(|c| c.estimated_days <= self.sla_days)
    }

    /// Check if order is hazmat
    pub fn is_hazmat(&self) -> bool {
        self.order.is_hazmat
    }
}

/// Order details
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderDetails {
    /// Order identifier
    pub order_id: String,
    /// Weight in kilograms
    pub weight_kg: f64,
    /// Dimensions in centimeters [length, width, height]
    pub dimensions_cm: [f64; 3],
    /// Destination zip/postal code
    pub destination_zip: String,
    /// Whether order contains hazardous materials
    pub is_hazmat: bool,
}

impl OrderDetails {
    /// Calculate dimensional weight (standard formula)
    pub fn dimensional_weight(&self) -> f64 {
        let volume = self.dimensions_cm[0] * self.dimensions_cm[1] * self.dimensions_cm[2];
        // Standard dim factor: 5000 for cm/kg
        volume / 5000.0
    }

    /// Get billable weight (max of actual and dimensional)
    pub fn billable_weight(&self) -> f64 {
        self.weight_kg.max(self.dimensional_weight())
    }
}

/// Carrier option with pricing and capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CarrierOption {
    /// Carrier identifier
    pub carrier_id: String,
    /// Service level (e.g., "ground", "express", "overnight")
    pub service_level: String,
    /// Shipping cost
    pub cost: f64,
    /// Estimated delivery days
    pub estimated_days: i64,
    /// Whether carrier supports hazmat shipping
    pub supports_hazmat: bool,
}

impl CarrierOption {
    /// Check if this carrier can handle the given order
    pub fn can_handle(&self, order: &OrderDetails) -> bool {
        if order.is_hazmat && !self.supports_hazmat {
            return false;
        }
        true
    }

    /// Calculate cost efficiency (lower is better)
    pub fn cost_per_day(&self) -> f64 {
        if self.estimated_days == 0 {
            self.cost
        } else {
            self.cost / self.estimated_days as f64
        }
    }
}

/// Output for shipping choice optimization
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShippingChoiceOutput {
    /// Selected carrier ID
    pub selected_carrier: Option<String>,
    /// Selected service level
    pub selected_service: Option<String>,
    /// Total shipping cost
    pub cost: f64,
    /// Estimated delivery days
    pub estimated_days: i64,
    /// Whether selection meets SLA
    pub meets_sla: bool,
    /// Reason for selection
    pub selection_reason: String,
    /// Alternatives considered
    pub alternatives: Vec<AlternativeCarrier>,
}

impl ShippingChoiceOutput {
    /// Create output when no carrier is available
    pub fn no_carrier(reason: &str) -> Self {
        Self {
            selected_carrier: None,
            selected_service: None,
            cost: 0.0,
            estimated_days: 0,
            meets_sla: false,
            selection_reason: reason.to_string(),
            alternatives: vec![],
        }
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        match &self.selected_carrier {
            Some(carrier) => format!(
                "Selected {} ({}) for ${:.2}, {} days",
                carrier,
                self.selected_service.as_deref().unwrap_or("unknown"),
                self.cost,
                self.estimated_days
            ),
            None => format!("No carrier selected: {}", self.selection_reason),
        }
    }
}

/// Alternative carrier option with comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeCarrier {
    /// Carrier ID
    pub carrier_id: String,
    /// Service level
    pub service_level: String,
    /// Cost
    pub cost: f64,
    /// Why it wasn't selected
    pub reason_not_selected: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimensional_weight() {
        let order = OrderDetails {
            order_id: "test".to_string(),
            weight_kg: 5.0,
            dimensions_cm: [30.0, 20.0, 10.0],
            destination_zip: "12345".to_string(),
            is_hazmat: false,
        };

        // 30*20*10 = 6000 / 5000 = 1.2 kg dimensional
        assert!((order.dimensional_weight() - 1.2).abs() < 0.01);
        // Actual weight is higher, so billable = actual
        assert!((order.billable_weight() - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_carrier_can_handle_hazmat() {
        let carrier = CarrierOption {
            carrier_id: "ups".to_string(),
            service_level: "ground".to_string(),
            cost: 10.0,
            estimated_days: 5,
            supports_hazmat: false,
        };

        let normal_order = OrderDetails {
            is_hazmat: false,
            ..Default::default()
        };

        let hazmat_order = OrderDetails {
            is_hazmat: true,
            ..Default::default()
        };

        assert!(carrier.can_handle(&normal_order));
        assert!(!carrier.can_handle(&hazmat_order));
    }

    #[test]
    fn test_input_validation() {
        let mut input = ShippingChoiceInput {
            order: OrderDetails::default(),
            carriers: vec![CarrierOption {
                carrier_id: "test".to_string(),
                service_level: "ground".to_string(),
                cost: 10.0,
                estimated_days: 5,
                supports_hazmat: false,
            }],
            sla_days: 7,
        };

        assert!(input.validate().is_ok());

        input.sla_days = 0;
        assert!(input.validate().is_err());
    }
}
