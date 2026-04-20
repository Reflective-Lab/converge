//! Types for Network Flow pack

use crate::Result;
use serde::{Deserialize, Serialize};

/// A directed edge in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEdge {
    pub from: usize,
    pub to: usize,
    pub capacity: f64,
    pub cost: f64,
}

/// Input for min-cost network flow optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFlowInput {
    /// Number of nodes in the network
    pub nodes: usize,
    /// Directed edges with capacity and cost
    pub edges: Vec<NetworkEdge>,
    /// Source node index
    pub source: usize,
    /// Sink node index
    pub sink: usize,
    /// Total flow demand to push from source to sink
    pub demand: f64,
}

impl NetworkFlowInput {
    pub fn validate(&self) -> Result<()> {
        if self.nodes == 0 {
            return Err(crate::Error::invalid_input(
                "Network must have at least one node",
            ));
        }
        if self.edges.is_empty() {
            return Err(crate::Error::invalid_input(
                "Network must have at least one edge",
            ));
        }
        if self.source >= self.nodes {
            return Err(crate::Error::invalid_input(
                "Source node index out of bounds",
            ));
        }
        if self.sink >= self.nodes {
            return Err(crate::Error::invalid_input("Sink node index out of bounds"));
        }
        if self.source == self.sink {
            return Err(crate::Error::invalid_input("Source and sink must differ"));
        }
        if self.demand <= 0.0 {
            return Err(crate::Error::invalid_input("Demand must be positive"));
        }
        for edge in &self.edges {
            if edge.from >= self.nodes || edge.to >= self.nodes {
                return Err(crate::Error::invalid_input("Edge references invalid node"));
            }
            if edge.capacity < 0.0 {
                return Err(crate::Error::invalid_input(
                    "Edge capacity must be non-negative",
                ));
            }
        }
        Ok(())
    }
}

/// Output for network flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkFlowOutput {
    /// Flow on each edge (same order as input edges)
    pub flows: Vec<f64>,
    /// Total cost of the flow
    pub total_cost: f64,
    /// Total flow achieved
    pub total_flow: f64,
}

impl NetworkFlowOutput {
    pub fn summary(&self) -> String {
        format!(
            "Pushed {:.2} units of flow at total cost {:.2}",
            self.total_flow, self.total_cost
        )
    }
}
