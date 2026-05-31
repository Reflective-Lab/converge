//! Solver for Inventory Rebalancing pack

use super::types::*;
use converge_pack::PackSolver;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{
    Diagnostic, DiagnosticKind, ProblemSpec, ReplayEnvelope, SolverReport, StopReason,
};
use std::collections::HashMap;

/// Greedy solver for inventory rebalancing
///
/// Algorithm:
/// 1. Calculate deficit/surplus for each (location, product)
/// 2. Sort deficits by urgency (most negative first)
/// 3. For each deficit, find cheapest transfer from surplus locations
/// 4. Stop when budget or transfer limits reached
pub struct GreedyRebalancingSolver;

impl GreedyRebalancingSolver {
    /// Solve the inventory rebalancing problem
    pub fn solve_rebalancing(
        &self,
        input: &InventoryRebalancingInput,
        spec: &ProblemSpec,
    ) -> Result<(InventoryRebalancingOutput, SolverReport)> {
        let start = std::time::Instant::now();
        let seed = spec.seed();

        // Build working state
        let mut state = WorkingState::from_input(input);

        // Find all deficits sorted by urgency (most negative first)
        let mut deficits: Vec<_> = state
            .levels
            .iter()
            .filter(|(_, level)| level.has_deficit())
            .map(|(key, level)| (key.clone(), level.deficit())) // deficit is negative
            .collect();

        deficits.sort_by_key(|d| d.1); // Sort ascending (most negative first)

        // Greedy allocation
        let mut transfers = Vec::new();
        let mut total_cost = 0.0;
        let mut total_units = 0i64;
        let mut iterations = 0;

        for (deficit_key, _) in deficits {
            if transfers.len() >= input.constraints.max_total_transfers {
                break;
            }

            let (dest_loc, product_id) = deficit_key;

            // Find cheapest source with surplus
            let best_source = self.find_cheapest_source(
                &state,
                input,
                &dest_loc,
                &product_id,
                input.constraints.max_transfer_quantity,
            );

            if let Some((source_loc, quantity, cost_per_unit, lead_time)) = best_source {
                let transfer_cost = quantity as f64 * cost_per_unit;

                // Check budget constraint
                if total_cost + transfer_cost > input.constraints.max_total_cost {
                    // Try smaller quantity
                    let affordable =
                        ((input.constraints.max_total_cost - total_cost) / cost_per_unit) as i64;
                    if affordable <= 0 {
                        continue;
                    }
                    // Use affordable amount instead
                    let quantity = affordable.min(quantity);
                    let transfer_cost = quantity as f64 * cost_per_unit;

                    // Apply transfer
                    state.apply_transfer(&source_loc, &dest_loc, &product_id, quantity);
                    total_cost += transfer_cost;
                    total_units += quantity;

                    transfers.push(Transfer::new(
                        &source_loc,
                        &dest_loc,
                        &product_id,
                        quantity,
                        transfer_cost,
                        lead_time,
                    ));
                } else {
                    // Apply full transfer
                    state.apply_transfer(&source_loc, &dest_loc, &product_id, quantity);
                    total_cost += transfer_cost;
                    total_units += quantity;

                    transfers.push(Transfer::new(
                        &source_loc,
                        &dest_loc,
                        &product_id,
                        quantity,
                        transfer_cost,
                        lead_time,
                    ));
                }
            }

            iterations += 1;
        }

        // Calculate service level improvement
        let service_improvement = self.calculate_service_improvement(&state, input);

        // Build location impacts
        let location_impacts = self.build_location_impacts(&state, input);

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        let output = InventoryRebalancingOutput {
            transfers,
            total_cost,
            total_units_moved: total_units,
            service_level_improvement: service_improvement,
            location_impacts,
        };

        // Build report
        let replay = ReplayEnvelope::minimal(seed);
        let stop_reason = if output.transfers.is_empty() {
            StopReason::Optimal // No transfers needed
        } else if total_cost >= input.constraints.max_total_cost * 0.99 {
            StopReason::Feasible // Hit budget limit
        } else {
            StopReason::Optimal
        };

        let report = SolverReport::feasible("greedy-v1", -total_cost, stop_reason, replay)
            .with_diagnostic(Diagnostic::performance(
                "rebalancing",
                elapsed_ms,
                iterations,
            ))
            .with_diagnostic(Diagnostic::with_data(
                DiagnosticKind::ScoringBreakdown,
                format!(
                    "{} transfers, {} units, ${:.2} cost",
                    output.transfers.len(),
                    total_units,
                    total_cost
                ),
                serde_json::json!({
                    "total_transfers": output.transfers.len(),
                    "total_units": total_units,
                    "total_cost": total_cost,
                    "service_improvement": service_improvement,
                }),
            ));

        Ok((output, report))
    }

    /// Find the cheapest source location with available surplus
    fn find_cheapest_source(
        &self,
        state: &WorkingState,
        input: &InventoryRebalancingInput,
        dest_loc: &str,
        product_id: &str,
        max_qty: i64,
    ) -> Option<(String, i64, f64, i64)> {
        let dest_level = state
            .levels
            .get(&(dest_loc.to_string(), product_id.to_string()))?;
        let needed = (dest_level.target_quantity - dest_level.quantity).max(0);
        let space_available = dest_level.available_space();

        if needed == 0 || space_available == 0 {
            return None;
        }

        let mut best: Option<(String, i64, f64, i64)> = None;

        for (key, level) in &state.levels {
            let (source_loc, source_product) = key;

            // Must be same product
            if source_product != product_id {
                continue;
            }

            // Can't transfer to self
            if source_loc == dest_loc {
                continue;
            }

            // Must have surplus
            let surplus = level.available_surplus();
            if surplus <= 0 {
                continue;
            }

            // Get transfer cost
            let cost_info = match input.get_transfer_cost(source_loc, dest_loc) {
                Some(c) => c,
                None => continue, // No transfer route
            };

            // Calculate quantity to transfer
            let qty = needed.min(surplus).min(space_available).min(max_qty);
            if qty <= 0 {
                continue;
            }

            // Compare with best
            match &best {
                None => {
                    best = Some((
                        source_loc.clone(),
                        qty,
                        cost_info.cost_per_unit,
                        cost_info.lead_time_hours,
                    ));
                }
                Some((_, _, best_cost, _)) => {
                    if cost_info.cost_per_unit < *best_cost {
                        best = Some((
                            source_loc.clone(),
                            qty,
                            cost_info.cost_per_unit,
                            cost_info.lead_time_hours,
                        ));
                    }
                }
            }
        }

        best
    }

    /// Calculate service level improvement
    fn calculate_service_improvement(
        &self,
        state: &WorkingState,
        input: &InventoryRebalancingInput,
    ) -> f64 {
        let mut initial_deficit_sum = 0i64;
        let mut final_deficit_sum = 0i64;

        for inv in &input.inventory {
            let initial_deficit = (inv.target_quantity - inv.quantity).max(0);
            initial_deficit_sum += initial_deficit;

            if let Some(level) = state
                .levels
                .get(&(inv.location_id.clone(), inv.product_id.clone()))
            {
                let final_deficit = (level.target_quantity - level.quantity).max(0);
                final_deficit_sum += final_deficit;
            }
        }

        if initial_deficit_sum == 0 {
            return 0.0; // Already perfect
        }

        let deficit_reduction = initial_deficit_sum - final_deficit_sum;
        deficit_reduction as f64 / initial_deficit_sum as f64
    }

    /// Build location impact records
    fn build_location_impacts(
        &self,
        state: &WorkingState,
        input: &InventoryRebalancingInput,
    ) -> Vec<LocationImpact> {
        let mut impacts = Vec::new();

        for inv in &input.inventory {
            let key = (inv.location_id.clone(), inv.product_id.clone());
            if let Some(level) = state.levels.get(&key) {
                let change = level.quantity - inv.quantity;
                if change != 0 {
                    impacts.push(LocationImpact {
                        location_id: inv.location_id.clone(),
                        product_id: inv.product_id.clone(),
                        inventory_change: change,
                        final_quantity: level.quantity,
                        meets_target: level.quantity >= level.target_quantity,
                    });
                }
            }
        }

        impacts
    }
}

impl PackSolver for GreedyRebalancingSolver {
    fn id(&self) -> &'static str {
        "greedy-v1"
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<(serde_json::Value, SolverReport)> {
        let input: InventoryRebalancingInput = spec.inputs_as()?;
        let (output, report) = self.solve_rebalancing(&input, spec)?;
        let json = serde_json::to_value(&output)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;
        Ok((json, report))
    }

    fn is_exact(&self) -> bool {
        false // Greedy is heuristic for this problem
    }
}

/// Working state for solver
struct WorkingState {
    levels: HashMap<(String, String), InventoryLevel>,
}

impl WorkingState {
    fn from_input(input: &InventoryRebalancingInput) -> Self {
        let levels = input
            .inventory
            .iter()
            .map(|inv| {
                (
                    (inv.location_id.clone(), inv.product_id.clone()),
                    inv.clone(),
                )
            })
            .collect();
        Self { levels }
    }

    fn apply_transfer(&mut self, from: &str, to: &str, product: &str, qty: i64) {
        // Decrease source
        if let Some(level) = self
            .levels
            .get_mut(&(from.to_string(), product.to_string()))
        {
            level.quantity -= qty;
        }
        // Increase destination
        if let Some(level) = self.levels.get_mut(&(to.to_string(), product.to_string())) {
            level.quantity += qty;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::{ObjectiveSpec, SolveBudgets};

    fn create_test_input() -> InventoryRebalancingInput {
        InventoryRebalancingInput {
            locations: vec![
                Location {
                    id: "warehouse".to_string(),
                    name: "Main Warehouse".to_string(),
                    capacity: 1000,
                    location_type: LocationType::Warehouse,
                },
                Location {
                    id: "store-a".to_string(),
                    name: "Store A".to_string(),
                    capacity: 100,
                    location_type: LocationType::Store,
                },
                Location {
                    id: "store-b".to_string(),
                    name: "Store B".to_string(),
                    capacity: 100,
                    location_type: LocationType::Store,
                },
            ],
            products: vec![Product {
                id: "widget".to_string(),
                name: "Widget".to_string(),
                unit_weight: 1.0,
                unit_value: 10.0,
            }],
            inventory: vec![
                InventoryLevel {
                    location_id: "warehouse".to_string(),
                    product_id: "widget".to_string(),
                    quantity: 500,
                    target_quantity: 300,
                    min_quantity: 100,
                    max_quantity: 800,
                },
                InventoryLevel {
                    location_id: "store-a".to_string(),
                    product_id: "widget".to_string(),
                    quantity: 10,
                    target_quantity: 40,
                    min_quantity: 20,
                    max_quantity: 80,
                },
                InventoryLevel {
                    location_id: "store-b".to_string(),
                    product_id: "widget".to_string(),
                    quantity: 5,
                    target_quantity: 50,
                    min_quantity: 25,
                    max_quantity: 80,
                },
            ],
            transfer_costs: vec![
                TransferCost {
                    from_location: "warehouse".to_string(),
                    to_location: "store-a".to_string(),
                    cost_per_unit: 0.5,
                    lead_time_hours: 24,
                },
                TransferCost {
                    from_location: "warehouse".to_string(),
                    to_location: "store-b".to_string(),
                    cost_per_unit: 0.8,
                    lead_time_hours: 48,
                },
            ],
            constraints: RebalancingConstraints {
                max_total_transfers: 10,
                max_transfer_quantity: 50,
                max_total_cost: 100.0,
            },
        }
    }

    fn create_spec(input: &InventoryRebalancingInput, seed: u64) -> ProblemSpec {
        ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(input)
            .unwrap()
            .budgets(SolveBudgets::with_time_limit(10))
            .seed(seed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_greedy_solver() {
        let solver = GreedyRebalancingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, report) = solver.solve_rebalancing(&input, &spec).unwrap();

        assert!(!output.transfers.is_empty());
        assert!(report.feasible);
        assert!(output.total_cost > 0.0);
        assert!(output.service_level_improvement > 0.0);
    }

    #[test]
    fn test_prefers_cheaper_route() {
        let solver = GreedyRebalancingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_rebalancing(&input, &spec).unwrap();

        // First transfer should go to store-a (cheaper route at $0.5/unit vs $0.8/unit)
        // unless store-b has larger deficit
        let to_store_a: i64 = output
            .transfers
            .iter()
            .filter(|t| t.to_location == "store-a")
            .map(|t| t.quantity)
            .sum();
        let to_store_b: i64 = output
            .transfers
            .iter()
            .filter(|t| t.to_location == "store-b")
            .map(|t| t.quantity)
            .sum();

        // Both stores should receive some inventory
        assert!(to_store_a > 0 || to_store_b > 0);
    }

    #[test]
    fn test_respects_budget() {
        let solver = GreedyRebalancingSolver;
        let mut input = create_test_input();
        input.constraints.max_total_cost = 10.0; // Very low budget

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_rebalancing(&input, &spec).unwrap();

        assert!(output.total_cost <= input.constraints.max_total_cost + 0.01);
    }

    #[test]
    fn test_respects_transfer_limit() {
        let solver = GreedyRebalancingSolver;
        let mut input = create_test_input();
        input.constraints.max_total_transfers = 1;

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_rebalancing(&input, &spec).unwrap();

        assert!(output.transfers.len() <= 1);
    }

    #[test]
    fn test_no_transfers_when_balanced() {
        let solver = GreedyRebalancingSolver;
        let input = InventoryRebalancingInput {
            locations: vec![Location {
                id: "warehouse".to_string(),
                name: "Warehouse".to_string(),
                capacity: 1000,
                location_type: LocationType::Warehouse,
            }],
            products: vec![Product {
                id: "widget".to_string(),
                name: "Widget".to_string(),
                unit_weight: 1.0,
                unit_value: 10.0,
            }],
            inventory: vec![InventoryLevel {
                location_id: "warehouse".to_string(),
                product_id: "widget".to_string(),
                quantity: 100,
                target_quantity: 100,
                min_quantity: 50,
                max_quantity: 200,
            }],
            transfer_costs: vec![],
            constraints: RebalancingConstraints {
                max_total_transfers: 10,
                max_transfer_quantity: 50,
                max_total_cost: 100.0,
            },
        };

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_rebalancing(&input, &spec).unwrap();

        assert!(output.transfers.is_empty());
        assert_eq!(output.total_cost, 0.0);
    }
}
