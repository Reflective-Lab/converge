//! Types for Facility Location pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// A candidate facility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Facility {
    /// Fixed cost to open this facility
    pub fixed_cost: f64,
    /// Maximum capacity
    pub capacity: usize,
}

/// A customer with demand and transport costs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    /// Units of demand
    pub demand: usize,
    /// Transport cost to each facility (indexed by facility)
    pub transport_costs: Vec<f64>,
}

/// Input for facility location optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacilityLocationInput {
    /// Candidate facilities
    pub facilities: Vec<Facility>,
    /// Customers to serve
    pub customers: Vec<Customer>,
}

impl FacilityLocationInput {
    pub fn validate(&self) -> Result<()> {
        if self.facilities.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one facility required",
            ));
        }
        if self.customers.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one customer required",
            ));
        }
        let num_facilities = self.facilities.len();
        for (i, customer) in self.customers.iter().enumerate() {
            if customer.transport_costs.len() != num_facilities {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Customer {} transport_costs length ({}) must match number of facilities ({})",
                    i,
                    customer.transport_costs.len(),
                    num_facilities
                )));
            }
        }
        Ok(())
    }
}

/// Output for facility location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacilityLocationOutput {
    /// Indices of facilities that are opened
    pub open_facilities: Vec<usize>,
    /// Assignment of each customer to a facility index
    pub assignments: Vec<usize>,
    /// Total cost (fixed + transport)
    pub total_cost: f64,
}

impl FacilityLocationOutput {
    pub fn summary(&self) -> String {
        format!(
            "Opened {} facilities, assigned {} customers, total cost {:.2}",
            self.open_facilities.len(),
            self.assignments.len(),
            self.total_cost
        )
    }
}
