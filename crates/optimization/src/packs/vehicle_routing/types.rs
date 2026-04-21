//! Types for Vehicle Routing pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for vehicle routing optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleRoutingInput {
    /// Depot location (x, y)
    pub depot: (f64, f64),
    /// Customer locations (x, y)
    pub customers: Vec<(f64, f64)>,
    /// Vehicle capacity (units)
    pub vehicle_capacity: usize,
    /// Demand at each customer
    pub demands: Vec<usize>,
}

impl VehicleRoutingInput {
    pub fn validate(&self) -> Result<()> {
        if self.customers.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one customer required",
            ));
        }
        if self.customers.len() != self.demands.len() {
            return Err(converge_pack::GateError::invalid_input(
                "Number of customers must match number of demands",
            ));
        }
        if self.vehicle_capacity == 0 {
            return Err(converge_pack::GateError::invalid_input(
                "Vehicle capacity must be positive",
            ));
        }
        for &d in &self.demands {
            if d > self.vehicle_capacity {
                return Err(converge_pack::GateError::invalid_input(
                    "Individual demand exceeds vehicle capacity",
                ));
            }
        }
        Ok(())
    }
}

/// Output for vehicle routing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleRoutingOutput {
    /// Routes: each route is a list of customer indices
    pub routes: Vec<Vec<usize>>,
    /// Total distance across all routes
    pub total_distance: f64,
}

impl VehicleRoutingOutput {
    pub fn summary(&self) -> String {
        format!(
            "{} routes serving all customers, total distance {:.2}",
            self.routes.len(),
            self.total_distance
        )
    }
}
