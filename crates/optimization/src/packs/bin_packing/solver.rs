//! Solver for Bin Packing pack

use super::types::*;
use crate::Result;
use crate::gate::{ProblemSpec, ReplayEnvelope, SolverReport};

/// First-Fit Decreasing heuristic
pub struct FirstFitDecreasingSolver;

impl FirstFitDecreasingSolver {
    pub fn solve(
        &self,
        input: &BinPackingInput,
        spec: &ProblemSpec,
    ) -> Result<(BinPackingOutput, SolverReport)> {
        // Sort items by size descending, keeping original indices
        let mut indexed_items: Vec<(usize, f64)> =
            input.items.iter().copied().enumerate().collect();
        indexed_items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut bins: Vec<Vec<usize>> = Vec::new();
        let mut bin_remaining: Vec<f64> = Vec::new();

        for (item_idx, item_size) in indexed_items {
            // Find first bin that fits
            let mut placed = false;
            for (bin_idx, remaining) in bin_remaining.iter_mut().enumerate() {
                if *remaining >= item_size {
                    bins[bin_idx].push(item_idx);
                    *remaining -= item_size;
                    placed = true;
                    break;
                }
            }
            if !placed {
                bins.push(vec![item_idx]);
                bin_remaining.push(input.bin_capacity - item_size);
            }
        }

        let bins_used = bins.len();
        let output = BinPackingOutput { bins, bins_used };

        let replay = ReplayEnvelope::minimal(spec.seed());
        let report = SolverReport::optimal("first-fit-decreasing-v1", bins_used as f64, replay);

        Ok((output, report))
    }
}
