//! Solver for Network Flow pack

use super::types::*;
use crate::Result;
use crate::gate::{ProblemSpec, ReplayEnvelope, SolverReport, StopReason};

/// Successive shortest path solver (simplified Bellman-Ford based)
pub struct SuccessiveShortestPathSolver;

impl SuccessiveShortestPathSolver {
    pub fn solve(
        &self,
        input: &NetworkFlowInput,
        spec: &ProblemSpec,
    ) -> Result<(NetworkFlowOutput, SolverReport)> {
        let n = input.nodes;
        let m = input.edges.len();

        // Build residual graph: for each original edge, we have forward + backward
        // residual_cap[2*i] = forward capacity, residual_cap[2*i+1] = backward capacity
        let mut residual_cap = vec![0.0_f64; 2 * m];
        let mut residual_cost = vec![0.0_f64; 2 * m];

        for (i, edge) in input.edges.iter().enumerate() {
            residual_cap[2 * i] = edge.capacity;
            residual_cost[2 * i] = edge.cost;
            residual_cap[2 * i + 1] = 0.0;
            residual_cost[2 * i + 1] = -edge.cost;
        }

        // Adjacency: for each node, list of residual edge indices
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, edge) in input.edges.iter().enumerate() {
            adj[edge.from].push(2 * i);
            adj[edge.to].push(2 * i + 1);
        }

        let mut total_flow = 0.0;
        let mut total_cost = 0.0;
        let max_iterations = 1000;
        let mut iterations = 0;

        while total_flow < input.demand && iterations < max_iterations {
            iterations += 1;

            // Bellman-Ford to find shortest path from source to sink
            let mut dist = vec![f64::MAX; n];
            let mut prev_edge: Vec<Option<usize>> = vec![None; n];
            dist[input.source] = 0.0;

            let mut updated = true;
            for _ in 0..n {
                if !updated {
                    break;
                }
                updated = false;
                for u in 0..n {
                    if dist[u] == f64::MAX {
                        continue;
                    }
                    for &eidx in &adj[u] {
                        if residual_cap[eidx] <= 0.0 {
                            continue;
                        }
                        let v = edge_to(eidx, &input.edges);
                        let new_dist = dist[u] + residual_cost[eidx];
                        if new_dist < dist[v] - 1e-9 {
                            dist[v] = new_dist;
                            prev_edge[v] = Some(eidx);
                            updated = true;
                        }
                    }
                }
            }

            if dist[input.sink] == f64::MAX {
                break; // No augmenting path
            }

            // Find bottleneck
            let mut bottleneck = input.demand - total_flow;
            let mut node = input.sink;
            while let Some(eidx) = prev_edge[node] {
                bottleneck = bottleneck.min(residual_cap[eidx]);
                node = edge_from(eidx, &input.edges);
            }

            // Augment flow
            node = input.sink;
            while let Some(eidx) = prev_edge[node] {
                residual_cap[eidx] -= bottleneck;
                residual_cap[eidx ^ 1] += bottleneck;
                total_cost += bottleneck * residual_cost[eidx];
                node = edge_from(eidx, &input.edges);
            }
            total_flow += bottleneck;
        }

        // Extract flows from residual capacities
        let flows: Vec<f64> = (0..m)
            .map(|i| input.edges[i].capacity - residual_cap[2 * i])
            .collect();

        let output = NetworkFlowOutput {
            flows,
            total_cost,
            total_flow,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = if total_flow >= input.demand - 1e-9 {
            SolverReport::optimal("successive-shortest-path-v1", total_cost, replay)
        } else {
            SolverReport::infeasible(
                "successive-shortest-path-v1",
                vec![],
                StopReason::NoFeasible,
                replay,
            )
        };

        Ok((output, report))
    }
}

fn edge_to(eidx: usize, edges: &[NetworkEdge]) -> usize {
    let original = eidx / 2;
    if eidx % 2 == 0 {
        edges[original].to
    } else {
        edges[original].from
    }
}

fn edge_from(eidx: usize, edges: &[NetworkEdge]) -> usize {
    let original = eidx / 2;
    if eidx % 2 == 0 {
        edges[original].from
    } else {
        edges[original].to
    }
}
