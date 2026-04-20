//! Graph algorithms
//!
//! This module provides graph algorithms for optimization:
//!
//! - [`dijkstra`] - Shortest path algorithms
//! - [`flow`] - Max flow and min cost flow
//! - [`matching`] - Graph matching algorithms
//!
//! ## Graph Representation
//!
//! We use [`petgraph`] for the underlying graph structure, with
//! wrapper types for optimization-specific operations.
//!
//! ## Example: Max Flow
//!
//! ```rust,ignore
//! use converge_optimization::graph::flow::{FlowNetwork, max_flow};
//!
//! let mut net = FlowNetwork::new(4);
//! net.add_edge_with_capacity(0, 1, 10);
//! net.add_edge_with_capacity(0, 2, 10);
//! net.add_edge_with_capacity(1, 3, 10);
//! net.add_edge_with_capacity(2, 3, 10);
//!
//! let result = max_flow(&net, 0, 3).unwrap();
//! assert_eq!(result.max_flow, 20);
//! ```

pub mod dijkstra;
pub mod flow;
pub mod matching;

// Re-export main types
pub use flow::{
    FlowNetwork, MaxFlowResult, MinCostFlowProblem, MinCostFlowResult, max_flow, min_cost_flow,
};

use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use serde::{Deserialize, Serialize};

/// Node identifier
pub type NodeId = NodeIndex;

/// Edge identifier
pub type EdgeId = EdgeIndex;

/// A directed graph with node and edge weights
pub type Graph<N, E> = DiGraph<N, E>;

/// Edge with capacity and cost for flow problems
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct FlowEdge {
    /// Maximum flow capacity
    pub capacity: i64,
    /// Cost per unit flow
    pub cost: i64,
    /// Current flow (for solutions)
    pub flow: i64,
}

impl FlowEdge {
    /// Create a new flow edge
    pub fn new(capacity: i64, cost: i64) -> Self {
        Self {
            capacity,
            cost,
            flow: 0,
        }
    }

    /// Create an edge with only capacity (zero cost)
    pub fn with_capacity(capacity: i64) -> Self {
        Self::new(capacity, 0)
    }

    /// Remaining capacity
    pub fn residual(&self) -> i64 {
        self.capacity - self.flow
    }
}

/// Path in a graph
#[derive(Debug, Clone)]
pub struct Path {
    /// Nodes in the path (as indices)
    pub nodes: Vec<NodeId>,
    /// Total cost/distance of the path
    pub cost: i64,
}

impl Path {
    /// Create an empty path
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            cost: 0,
        }
    }

    /// Check if path is empty
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Number of nodes in path
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flow_edge() {
        let mut edge = FlowEdge::new(10, 5);
        assert_eq!(edge.residual(), 10);
        edge.flow = 3;
        assert_eq!(edge.residual(), 7);
    }
}
