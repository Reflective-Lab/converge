//! Dijkstra's shortest path algorithm
//!
//! Finds shortest paths from a source node to all other nodes
//! in a graph with non-negative edge weights.
//!
//! Time complexity: O((V + E) log V) using a binary heap.

use super::{Graph, NodeId, Path};
use crate::{Cost, Error, Result};
use petgraph::visit::EdgeRef;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

/// Find shortest path between two nodes, returning the full node sequence.
pub fn shortest_path<N, E>(
    graph: &Graph<N, E>,
    source: NodeId,
    target: NodeId,
    edge_cost: impl Fn(&E) -> Cost,
) -> Result<Option<Path>> {
    let (distances, predecessors) = dijkstra_with_paths(graph, source, edge_cost)?;

    match distances.get(&target) {
        Some(&cost) => Ok(Some(reconstruct_path(&predecessors, source, target, cost))),
        None => Ok(None),
    }
}

/// Run Dijkstra's algorithm from a source node
///
/// Returns a map from node to shortest distance from source.
pub fn dijkstra<N, E>(
    graph: &Graph<N, E>,
    source: NodeId,
    edge_cost: impl Fn(&E) -> Cost,
) -> Result<HashMap<NodeId, Cost>> {
    let mut distances: HashMap<NodeId, Cost> = HashMap::new();
    let mut heap: BinaryHeap<Reverse<(Cost, NodeId)>> = BinaryHeap::new();

    distances.insert(source, 0);
    heap.push(Reverse((0, source)));

    while let Some(Reverse((cost, node))) = heap.pop() {
        // Skip if we've found a better path
        if let Some(&best) = distances.get(&node) {
            if cost > best {
                continue;
            }
        }

        // Explore neighbors
        for edge in graph.edges(node) {
            let edge_weight = edge_cost(edge.weight());
            if edge_weight < 0 {
                return Err(Error::invalid_input(
                    "Dijkstra requires non-negative edge weights",
                ));
            }

            let next = edge.target();
            let next_cost = cost + edge_weight;

            let is_better = distances.get(&next).map_or(true, |&d| next_cost < d);

            if is_better {
                distances.insert(next, next_cost);
                heap.push(Reverse((next_cost, next)));
            }
        }
    }

    Ok(distances)
}

/// Find shortest paths from source to all nodes, returning predecessors
pub fn dijkstra_with_paths<N, E>(
    graph: &Graph<N, E>,
    source: NodeId,
    edge_cost: impl Fn(&E) -> Cost,
) -> Result<(HashMap<NodeId, Cost>, HashMap<NodeId, NodeId>)> {
    let mut distances: HashMap<NodeId, Cost> = HashMap::new();
    let mut predecessors: HashMap<NodeId, NodeId> = HashMap::new();
    let mut heap: BinaryHeap<Reverse<(Cost, NodeId)>> = BinaryHeap::new();

    distances.insert(source, 0);
    heap.push(Reverse((0, source)));

    while let Some(Reverse((cost, node))) = heap.pop() {
        if let Some(&best) = distances.get(&node) {
            if cost > best {
                continue;
            }
        }

        for edge in graph.edges(node) {
            let edge_weight = edge_cost(edge.weight());
            if edge_weight < 0 {
                return Err(Error::invalid_input(
                    "Dijkstra requires non-negative edge weights",
                ));
            }

            let next = edge.target();
            let next_cost = cost + edge_weight;

            let is_better = distances.get(&next).map_or(true, |&d| next_cost < d);

            if is_better {
                distances.insert(next, next_cost);
                predecessors.insert(next, node);
                heap.push(Reverse((next_cost, next)));
            }
        }
    }

    Ok((distances, predecessors))
}

/// Reconstruct path from predecessors map
pub fn reconstruct_path(
    predecessors: &HashMap<NodeId, NodeId>,
    source: NodeId,
    target: NodeId,
    total_cost: Cost,
) -> Path {
    let mut path = vec![target];
    let mut current = target;

    while current != source {
        if let Some(&pred) = predecessors.get(&current) {
            path.push(pred);
            current = pred;
        } else {
            // No path exists
            return Path::empty();
        }
    }

    path.reverse();
    Path {
        nodes: path,
        cost: total_cost,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use petgraph::graph::DiGraph;

    #[test]
    fn test_simple_dijkstra() {
        let mut graph: DiGraph<(), i64> = DiGraph::new();
        let a = graph.add_node(());
        let b = graph.add_node(());
        let c = graph.add_node(());

        graph.add_edge(a, b, 1);
        graph.add_edge(b, c, 2);
        graph.add_edge(a, c, 5);

        let distances = dijkstra(&graph, a, |&w| w).unwrap();

        assert_eq!(distances[&a], 0);
        assert_eq!(distances[&b], 1);
        assert_eq!(distances[&c], 3); // a->b->c = 1+2 < a->c = 5
    }

    #[test]
    fn test_unreachable() {
        let mut graph: DiGraph<(), i64> = DiGraph::new();
        let a = graph.add_node(());
        let b = graph.add_node(());

        // No edge from a to b
        let distances = dijkstra(&graph, a, |&w| w).unwrap();

        assert_eq!(distances.get(&a), Some(&0));
        assert_eq!(distances.get(&b), None);
    }
}
