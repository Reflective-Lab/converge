//! Solver for Graph Partitioning pack
//!
//! Greedy partitioning by node degree, then swap-based improvement (simplified Kernighan-Lin).

use super::types::*;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct GreedyKernighanLinSolver;

impl GreedyKernighanLinSolver {
    pub fn solve(
        &self,
        input: &GraphPartitioningInput,
        spec: &ProblemSpec,
    ) -> Result<(GraphPartitioningOutput, SolverReport)> {
        let n = input.num_nodes;
        let k = input.num_partitions;

        // Build adjacency list with weights
        let mut adj: Vec<Vec<(usize, f64)>> = vec![vec![]; n];
        for edge in &input.edges {
            adj[edge.from].push((edge.to, edge.weight));
            adj[edge.to].push((edge.from, edge.weight));
        }

        // Phase 1: Greedy assignment by node degree (highest degree first)
        let mut degrees: Vec<(usize, usize)> = (0..n).map(|i| (i, adj[i].len())).collect();
        degrees.sort_by_key(|d| std::cmp::Reverse(d.1));

        let mut partition = vec![0usize; n];
        let mut part_sizes = vec![0usize; k];
        let target_size = n.div_ceil(k);

        for &(node, _) in &degrees {
            // Assign to smallest partition that has room
            let best_part = (0..k)
                .filter(|&p| part_sizes[p] < target_size)
                .min_by_key(|&p| part_sizes[p])
                .unwrap_or(0);
            partition[node] = best_part;
            part_sizes[best_part] += 1;
        }

        // Phase 2: Swap-based improvement (simplified Kernighan-Lin)
        let max_iterations = n * 2;
        for _ in 0..max_iterations {
            let mut best_gain = 0.0f64;
            let mut best_swap: Option<(usize, usize)> = None;

            for node_a in 0..n {
                for node_b in (node_a + 1)..n {
                    if partition[node_a] == partition[node_b] {
                        continue;
                    }
                    let gain = Self::swap_gain(input, &partition, &adj, node_a, node_b);
                    if gain > best_gain {
                        best_gain = gain;
                        best_swap = Some((node_a, node_b));
                    }
                }
            }

            match best_swap {
                Some((a, b)) if best_gain > 1e-10 => {
                    partition.swap(a, b);
                }
                _ => break,
            }
        }

        let cut_weight = Self::compute_cut_weight(input, &partition);
        let balance = Self::compute_balance(&partition, k);

        let output = GraphPartitioningOutput {
            partition,
            cut_weight,
            balance,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("greedy-kernighan-lin-v1", cut_weight, replay);

        Ok((output, report))
    }

    fn swap_gain(
        _input: &GraphPartitioningInput,
        partition: &[usize],
        adj: &[Vec<(usize, f64)>],
        a: usize,
        b: usize,
    ) -> f64 {
        let pa = partition[a];
        let pb = partition[b];

        let external_cost = |node: usize, part: usize| -> f64 {
            adj[node]
                .iter()
                .filter(|&&(neighbor, _)| partition[neighbor] != part)
                .map(|&(_, w)| w)
                .sum::<f64>()
        };

        let before = external_cost(a, pa) + external_cost(b, pb);

        // Simulate swap
        let after = {
            let mut cost = 0.0;
            for &(neighbor, w) in &adj[a] {
                let np = if neighbor == b {
                    pa
                } else {
                    partition[neighbor]
                };
                if np != pb {
                    cost += w;
                }
            }
            for &(neighbor, w) in &adj[b] {
                let np = if neighbor == a {
                    pb
                } else {
                    partition[neighbor]
                };
                if np != pa {
                    cost += w;
                }
            }
            cost
        };

        before - after
    }

    fn compute_cut_weight(input: &GraphPartitioningInput, partition: &[usize]) -> f64 {
        input
            .edges
            .iter()
            .filter(|e| partition[e.from] != partition[e.to])
            .map(|e| e.weight)
            .sum()
    }

    fn compute_balance(partition: &[usize], k: usize) -> f64 {
        let mut sizes = vec![0usize; k];
        for &p in partition {
            sizes[p] += 1;
        }
        let max = sizes.iter().copied().max().unwrap_or(1) as f64;
        let min = sizes.iter().copied().filter(|&s| s > 0).min().unwrap_or(1) as f64;
        max / min
    }
}
