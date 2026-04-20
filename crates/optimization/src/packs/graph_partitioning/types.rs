//! Types for Graph Partitioning pack

use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPartitioningInput {
    pub num_nodes: usize,
    pub edges: Vec<Edge>,
    pub num_partitions: usize,
}

impl GraphPartitioningInput {
    pub fn validate(&self) -> Result<()> {
        if self.num_nodes == 0 {
            return Err(crate::Error::invalid_input("At least one node required"));
        }
        if self.num_partitions == 0 {
            return Err(crate::Error::invalid_input(
                "At least one partition required",
            ));
        }
        if self.num_partitions > self.num_nodes {
            return Err(crate::Error::invalid_input("More partitions than nodes"));
        }
        for (i, edge) in self.edges.iter().enumerate() {
            if edge.from >= self.num_nodes || edge.to >= self.num_nodes {
                return Err(crate::Error::invalid_input(format!(
                    "Edge {} references node outside range [0, {})",
                    i, self.num_nodes
                )));
            }
            if !edge.weight.is_finite() || edge.weight < 0.0 {
                return Err(crate::Error::invalid_input(format!(
                    "Edge {} has invalid weight {}",
                    i, edge.weight
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPartitioningOutput {
    pub partition: Vec<usize>,
    pub cut_weight: f64,
    pub balance: f64,
}

impl GraphPartitioningOutput {
    pub fn summary(&self) -> String {
        format!(
            "Partitioned {} nodes with cut weight {:.2} and balance {:.2}",
            self.partition.len(),
            self.cut_weight,
            self.balance
        )
    }
}
