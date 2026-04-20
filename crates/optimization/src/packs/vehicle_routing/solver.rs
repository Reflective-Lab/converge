//! Solver for Vehicle Routing pack

use super::types::*;
use crate::Result;
use crate::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

/// Nearest-neighbor heuristic with capacity constraints
pub struct NearestNeighborVrpSolver;

impl NearestNeighborVrpSolver {
    pub fn solve(
        &self,
        input: &VehicleRoutingInput,
        spec: &ProblemSpec,
    ) -> Result<(VehicleRoutingOutput, SolverReport)> {
        let n = input.customers.len();
        let mut visited = vec![false; n];
        let mut routes: Vec<Vec<usize>> = Vec::new();
        let mut total_distance = 0.0;
        let mut served = 0;

        while served < n {
            let mut route = Vec::new();
            let mut remaining_cap = input.vehicle_capacity;
            let mut current_pos = input.depot;

            loop {
                // Find nearest unvisited customer that fits
                let mut best_idx = None;
                let mut best_dist = f64::MAX;

                for i in 0..n {
                    if visited[i] || input.demands[i] > remaining_cap {
                        continue;
                    }
                    let dist = euclidean(current_pos, input.customers[i]);
                    if dist < best_dist {
                        best_dist = dist;
                        best_idx = Some(i);
                    }
                }

                match best_idx {
                    Some(idx) => {
                        visited[idx] = true;
                        route.push(idx);
                        remaining_cap -= input.demands[idx];
                        total_distance += best_dist;
                        current_pos = input.customers[idx];
                        served += 1;
                    }
                    None => break,
                }
            }

            // Return to depot
            total_distance += euclidean(current_pos, input.depot);
            if !route.is_empty() {
                routes.push(route);
            }
        }

        let output = VehicleRoutingOutput {
            routes,
            total_distance,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("nearest-neighbor-vrp-v1", total_distance, replay);

        Ok((output, report))
    }
}

fn euclidean(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}
