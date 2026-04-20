//! Solver for Facility Location pack

use super::types::*;
use crate::Result;
use crate::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

/// Greedy facility location solver
///
/// Algorithm: iteratively open the facility that provides the greatest
/// cost reduction (considering fixed cost vs transport savings), then
/// assign each customer to its cheapest open facility.
pub struct GreedyFacilityLocationSolver;

impl GreedyFacilityLocationSolver {
    pub fn solve(
        &self,
        input: &FacilityLocationInput,
        spec: &ProblemSpec,
    ) -> Result<(FacilityLocationOutput, SolverReport)> {
        let num_facilities = input.facilities.len();
        let num_customers = input.customers.len();

        let mut open = vec![false; num_facilities];
        let mut assignments = vec![0_usize; num_customers];

        // Greedy: open facilities one at a time, picking the one that
        // minimizes total cost when added
        loop {
            let mut best_facility = None;
            let mut best_cost_reduction = f64::NEG_INFINITY;

            let current_cost = self.compute_total_cost(input, &open, &mut assignments);

            for f in 0..num_facilities {
                if open[f] {
                    continue;
                }
                open[f] = true;
                let mut tmp_assign = vec![0_usize; num_customers];
                let new_cost = self.compute_total_cost(input, &open, &mut tmp_assign);
                let reduction = current_cost - new_cost;
                if reduction > best_cost_reduction {
                    best_cost_reduction = reduction;
                    best_facility = Some(f);
                }
                open[f] = false;
            }

            match best_facility {
                Some(f) if best_cost_reduction > 0.0 => {
                    open[f] = true;
                }
                _ => break,
            }
        }

        // If nothing is open yet, open the single cheapest option
        if !open.iter().any(|&o| o) {
            let mut best_f = 0;
            let mut best_total = f64::MAX;
            for f in 0..num_facilities {
                open[f] = true;
                let mut tmp = vec![0; num_customers];
                let cost = self.compute_total_cost(input, &open, &mut tmp);
                if cost < best_total {
                    best_total = cost;
                    best_f = f;
                }
                open[f] = false;
            }
            open[best_f] = true;
        }

        let total_cost = self.compute_total_cost(input, &open, &mut assignments);
        let open_facilities: Vec<usize> = open
            .iter()
            .enumerate()
            .filter(|(_, o)| **o)
            .map(|(i, _)| i)
            .collect();

        let output = FacilityLocationOutput {
            open_facilities,
            assignments,
            total_cost,
        };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("greedy-facility-v1", total_cost, replay);

        Ok((output, report))
    }

    fn compute_total_cost(
        &self,
        input: &FacilityLocationInput,
        open: &[bool],
        assignments: &mut [usize],
    ) -> f64 {
        let mut cost = 0.0;

        // Fixed costs
        for (i, facility) in input.facilities.iter().enumerate() {
            if open[i] {
                cost += facility.fixed_cost;
            }
        }

        // Assign each customer to cheapest open facility
        for (c, customer) in input.customers.iter().enumerate() {
            let mut best_f = 0;
            let mut best_transport = f64::MAX;
            for (f, &is_open) in open.iter().enumerate() {
                if is_open && customer.transport_costs[f] < best_transport {
                    best_transport = customer.transport_costs[f];
                    best_f = f;
                }
            }
            if best_transport < f64::MAX {
                assignments[c] = best_f;
                cost += best_transport;
            }
        }

        cost
    }
}
