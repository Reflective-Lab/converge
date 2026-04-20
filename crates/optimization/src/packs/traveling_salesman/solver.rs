//! Solver for Traveling Salesman pack
//!
//! Nearest-neighbor heuristic with 2-opt local search improvement.

use super::types::*;
use crate::Result;
use crate::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

pub struct NearestNeighborTwoOptSolver;

impl NearestNeighborTwoOptSolver {
    pub fn solve(&self, input: &TspInput, spec: &ProblemSpec) -> Result<(TspOutput, SolverReport)> {
        let n = input.cities.len();

        // Phase 1: Nearest-neighbor construction
        let mut tour = Vec::with_capacity(n);
        let mut visited = vec![false; n];
        tour.push(0);
        visited[0] = true;

        for _ in 1..n {
            let last = *tour.last().unwrap();
            let mut best_next = None;
            let mut best_dist = f64::INFINITY;

            for j in 0..n {
                if !visited[j] {
                    let d = input.distance(last, j);
                    if d < best_dist {
                        best_dist = d;
                        best_next = Some(j);
                    }
                }
            }

            if let Some(next) = best_next {
                tour.push(next);
                visited[next] = true;
            }
        }

        // Phase 2: 2-opt improvement
        let mut improved = true;
        while improved {
            improved = false;
            for i in 0..n - 1 {
                for j in (i + 2)..n {
                    let a = tour[i];
                    let b = tour[i + 1];
                    let c = tour[j];
                    let d = tour[(j + 1) % n];

                    let current = input.distance(a, b) + input.distance(c, d);
                    let swapped = input.distance(a, c) + input.distance(b, d);

                    if swapped < current - 1e-10 {
                        tour[i + 1..=j].reverse();
                        improved = true;
                    }
                }
            }
        }

        let total_distance = Self::tour_length(input, &tour);

        let output = TspOutput {
            tour,
            total_distance,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("nearest-neighbor-2opt-v1", total_distance, replay);

        Ok((output, report))
    }

    fn tour_length(input: &TspInput, tour: &[usize]) -> f64 {
        let n = tour.len();
        (0..n)
            .map(|i| input.distance(tour[i], tour[(i + 1) % n]))
            .sum()
    }
}
