//! Types for Bin Packing pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for bin packing optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinPackingInput {
    /// Capacity of each bin
    pub bin_capacity: f64,
    /// Size of each item
    pub items: Vec<f64>,
}

impl BinPackingInput {
    pub fn validate(&self) -> Result<()> {
        if !self.bin_capacity.is_finite() || self.bin_capacity <= 0.0 {
            return Err(converge_pack::GateError::invalid_input(
                "Bin capacity must be finite and positive",
            ));
        }
        if self.items.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one item required",
            ));
        }
        for (i, &size) in self.items.iter().enumerate() {
            if !size.is_finite() || size <= 0.0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Item {} must have finite positive size",
                    i
                )));
            }
            if size > self.bin_capacity {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Item {} (size {:.2}) exceeds bin capacity ({:.2})",
                    i, size, self.bin_capacity
                )));
            }
        }
        Ok(())
    }
}

/// Output for bin packing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinPackingOutput {
    /// Each bin contains a list of item indices
    pub bins: Vec<Vec<usize>>,
    /// Number of bins used
    pub bins_used: usize,
}

impl BinPackingOutput {
    pub fn summary(&self) -> String {
        format!("Packed all items into {} bins", self.bins_used)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_finite_capacity_and_items() {
        let mut input = BinPackingInput {
            bin_capacity: f64::NAN,
            items: vec![1.0],
        };

        assert!(input.validate().is_err());

        input.bin_capacity = 10.0;
        input.items = vec![f64::INFINITY];
        assert!(input.validate().is_err());
    }
}
