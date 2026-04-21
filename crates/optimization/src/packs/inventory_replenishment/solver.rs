//! Solver for Inventory Replenishment pack

use super::types::*;
use converge_pack::PackSolver;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport, StopReason};

/// EOQ-based solver for inventory replenishment
///
/// Algorithm:
/// 1. Calculate EOQ and safety stock for each product
/// 2. Determine reorder points based on service level
/// 3. Prioritize orders by urgency (days until stockout)
/// 4. Allocate budget starting with most urgent
/// 5. Generate orders with timing and projected inventory
pub struct EoqSolver;

impl EoqSolver {
    /// Solve the inventory replenishment problem
    pub fn solve_replenishment(
        &self,
        input: &InventoryReplenishmentInput,
        spec: &ProblemSpec,
    ) -> Result<(InventoryReplenishmentOutput, SolverReport)> {
        let seed = spec.seed();
        let constraints = &input.constraints;

        // Calculate replenishment parameters for each product
        let mut candidates: Vec<ReplenishmentCandidate> = input
            .products
            .iter()
            .map(|p| self.calculate_candidate(p, constraints.target_service_level))
            .collect();

        // Sort by urgency (days until stockout, ascending)
        candidates.sort_by(|a, b| {
            a.days_until_stockout
                .partial_cmp(&b.days_until_stockout)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply tie-breaking for equal urgency
        let tie_break = &spec.determinism.tie_break;
        let mut sorted_candidates = Vec::new();
        let mut current_urgency = f64::NEG_INFINITY;
        let mut urgency_group: Vec<ReplenishmentCandidate> = vec![];

        for candidate in candidates {
            if (candidate.days_until_stockout - current_urgency).abs() < 0.01 {
                urgency_group.push(candidate);
            } else {
                if !urgency_group.is_empty() {
                    urgency_group.sort_by(|a, b| a.product.id.cmp(&b.product.id));
                    if let Some(selected) = tie_break
                        .select_by(&urgency_group, seed, |a, b| a.product.id.cmp(&b.product.id))
                    {
                        sorted_candidates.push(selected.clone());
                    } else {
                        sorted_candidates.extend(urgency_group.drain(..));
                    }
                }
                current_urgency = candidate.days_until_stockout;
                urgency_group = vec![candidate];
            }
        }
        // Don't forget the last group
        if !urgency_group.is_empty() {
            urgency_group.sort_by(|a, b| a.product.id.cmp(&b.product.id));
            if let Some(selected) =
                tie_break.select_by(&urgency_group, seed, |a, b| a.product.id.cmp(&b.product.id))
            {
                sorted_candidates.push(selected.clone());
            } else {
                sorted_candidates.extend(urgency_group.drain(..));
            }
        }

        // Allocate budget
        let mut remaining_budget = constraints.budget;
        let mut orders = Vec::new();
        let mut not_ordered = Vec::new();
        let mut total_cost = 0.0;
        let mut total_units: i64 = 0;

        for candidate in &sorted_candidates {
            // Check if we've hit max orders limit
            if let Some(max) = constraints.max_orders {
                if orders.len() >= max {
                    not_ordered.push(NotOrderedProduct {
                        product_id: candidate.product.id.clone(),
                        product_name: candidate.product.name.clone(),
                        reason: "Maximum order limit reached".to_string(),
                        current_inventory: candidate.product.current_inventory,
                        days_remaining: candidate.days_until_stockout,
                    });
                    continue;
                }
            }

            // Determine order quantity
            let mut order_qty = candidate.recommended_quantity;

            // Apply minimum order quantity if specified
            if let Some(min_qty) = constraints.min_order_quantity {
                if order_qty < min_qty && order_qty > 0 {
                    order_qty = min_qty;
                }
            }

            // Check if product needs ordering
            if !candidate.needs_order {
                not_ordered.push(NotOrderedProduct {
                    product_id: candidate.product.id.clone(),
                    product_name: candidate.product.name.clone(),
                    reason: format!(
                        "Sufficient inventory ({} days remaining)",
                        candidate.days_until_stockout as i64
                    ),
                    current_inventory: candidate.product.current_inventory,
                    days_remaining: candidate.days_until_stockout,
                });
                continue;
            }

            // Calculate cost
            let order_cost = candidate.product.total_order_cost(order_qty);

            // Check budget
            if order_cost > remaining_budget {
                // Try to order what we can afford
                let max_affordable_units = ((remaining_budget - candidate.product.ordering_cost)
                    / candidate.product.unit_cost)
                    .floor() as i64;

                if max_affordable_units > 0 {
                    let affordable_cost = candidate.product.total_order_cost(max_affordable_units);
                    if affordable_cost <= remaining_budget {
                        order_qty = max_affordable_units;
                    } else {
                        not_ordered.push(NotOrderedProduct {
                            product_id: candidate.product.id.clone(),
                            product_name: candidate.product.name.clone(),
                            reason: format!(
                                "Insufficient budget (need ${:.2}, have ${:.2})",
                                order_cost, remaining_budget
                            ),
                            current_inventory: candidate.product.current_inventory,
                            days_remaining: candidate.days_until_stockout,
                        });
                        continue;
                    }
                } else {
                    not_ordered.push(NotOrderedProduct {
                        product_id: candidate.product.id.clone(),
                        product_name: candidate.product.name.clone(),
                        reason: format!(
                            "Insufficient budget (need ${:.2}, have ${:.2})",
                            order_cost, remaining_budget
                        ),
                        current_inventory: candidate.product.current_inventory,
                        days_remaining: candidate.days_until_stockout,
                    });
                    continue;
                }
            }

            let final_cost = candidate.product.total_order_cost(order_qty);
            remaining_budget -= final_cost;
            total_cost += final_cost;
            total_units += order_qty;

            // Determine order timing
            let order_day = self.calculate_order_day(&candidate);
            let arrival_day = order_day + candidate.product.lead_time_days;

            orders.push(ReplenishmentOrder {
                product_id: candidate.product.id.clone(),
                product_name: candidate.product.name.clone(),
                quantity: order_qty,
                order_day,
                arrival_day,
                order_cost: final_cost,
                unit_cost: candidate.product.unit_cost,
                eoq: candidate.eoq,
                safety_stock: candidate.safety_stock,
                reorder_point: candidate.reorder_point,
                order_reason: self.generate_order_reason(&candidate),
            });
        }

        // Generate inventory projections
        let projections = self.generate_projections(&orders, &input.products, constraints);

        // Calculate projected service level
        let projected_service_level = self.calculate_projected_service_level(
            &orders,
            &input.products,
            constraints.target_service_level,
        );

        let budget_utilization = if constraints.budget > 0.0 {
            total_cost / constraints.budget
        } else {
            0.0
        };

        let output = InventoryReplenishmentOutput {
            orders,
            not_ordered,
            projections,
            stats: ReplenishmentStats {
                total_order_cost: total_cost,
                total_units_ordered: total_units,
                products_ordered: sorted_candidates
                    .iter()
                    .filter(|c| c.needs_order)
                    .count()
                    .min(input.products.len()),
                products_skipped: input.products.len()
                    - sorted_candidates
                        .iter()
                        .filter(|c| c.needs_order)
                        .count()
                        .min(input.products.len()),
                budget_utilization,
                projected_service_level,
                reason: if total_units > 0 {
                    format!(
                        "EOQ-based replenishment plan for {} products",
                        input.products.len()
                    )
                } else {
                    "No replenishment needed".to_string()
                },
            },
        };

        // Update stats based on actual orders
        let mut final_output = output;
        final_output.stats.products_ordered = final_output.orders.len();
        final_output.stats.products_skipped = final_output.not_ordered.len();

        let replay = ReplayEnvelope::minimal(seed);
        let report = if !final_output.orders.is_empty() {
            // Objective: minimize total cost while meeting service level
            SolverReport::optimal("eoq-v1", -total_cost, replay)
        } else if input.products.iter().all(|p| !p.needs_reorder()) {
            // No orders needed - this is feasible, not infeasible
            SolverReport::feasible("eoq-v1", 0.0, StopReason::Feasible, replay)
        } else {
            SolverReport::infeasible("eoq-v1", vec![], StopReason::NoFeasible, replay)
        };

        Ok((final_output, report))
    }

    fn calculate_candidate(&self, product: &Product, service_level: f64) -> ReplenishmentCandidate {
        let eoq = product.calculate_eoq();
        let safety_stock = product.calculate_safety_stock(service_level);
        let reorder_point = product.calculate_reorder_point(service_level);
        let days_until_stockout = product.days_of_inventory();

        // Determine if ordering is needed
        let needs_order = (product.current_inventory as f64) < reorder_point;

        // Recommended quantity is EOQ, but ensure it brings us above reorder point + safety stock
        let target_level = reorder_point + eoq;
        let quantity_needed = (target_level - product.current_inventory as f64).max(0.0);
        let recommended_quantity = if needs_order {
            eoq.max(quantity_needed).ceil() as i64
        } else {
            0
        };

        ReplenishmentCandidate {
            product: product.clone(),
            eoq,
            safety_stock,
            reorder_point,
            days_until_stockout,
            needs_order,
            recommended_quantity,
        }
    }

    fn calculate_order_day(&self, candidate: &ReplenishmentCandidate) -> i64 {
        // Order immediately if below reorder point
        if candidate.product.current_inventory as f64 <= candidate.reorder_point {
            return 0;
        }

        // Calculate when inventory will hit reorder point
        let inventory_above_rop =
            candidate.product.current_inventory as f64 - candidate.reorder_point;
        let days_until_rop = if candidate.product.demand_forecast.average_daily > 0.0 {
            (inventory_above_rop / candidate.product.demand_forecast.average_daily).floor() as i64
        } else {
            0
        };

        days_until_rop.max(0)
    }

    fn generate_order_reason(&self, candidate: &ReplenishmentCandidate) -> String {
        if candidate.days_until_stockout < candidate.product.lead_time_days as f64 {
            format!(
                "Urgent: stockout risk in {:.1} days, lead time is {} days",
                candidate.days_until_stockout, candidate.product.lead_time_days
            )
        } else if candidate.product.current_inventory as f64 <= candidate.reorder_point {
            format!(
                "Below reorder point ({:.0} units), current inventory: {}",
                candidate.reorder_point, candidate.product.current_inventory
            )
        } else {
            format!(
                "Proactive replenishment, {:.1} days of inventory remaining",
                candidate.days_until_stockout
            )
        }
    }

    fn generate_projections(
        &self,
        orders: &[ReplenishmentOrder],
        products: &[Product],
        constraints: &ReplenishmentConstraints,
    ) -> Vec<InventoryProjection> {
        let mut projections = Vec::new();

        for product in products {
            let order = orders.iter().find(|o| o.product_id == product.id);
            let current_inventory = product.current_inventory as f64;

            // Generate projections for key days
            let key_days = vec![0, 7, 14, 21, constraints.planning_horizon_days];

            for &day in &key_days {
                if day > constraints.planning_horizon_days {
                    break;
                }

                // Consume demand
                let demand = product.demand_forecast.average_daily * day as f64;
                let mut projected = (current_inventory - demand).max(0.0);

                // Add order if it arrives by this day
                let order_arriving = if let Some(o) = order {
                    if o.arrival_day <= day {
                        projected += o.quantity as f64;
                        o.arrival_day == day
                    } else {
                        false
                    }
                } else {
                    false
                };

                // Calculate stockout probability
                let safety_stock = product.calculate_safety_stock(constraints.target_service_level);
                let stockout_prob = if projected <= 0.0 {
                    1.0
                } else if projected < safety_stock {
                    (safety_stock - projected) / safety_stock
                } else {
                    0.0
                };

                projections.push(InventoryProjection {
                    product_id: product.id.clone(),
                    day,
                    projected_inventory: projected.max(0.0) as i64,
                    stockout_probability: stockout_prob.min(1.0),
                    order_arriving,
                });
            }
        }

        projections
    }

    fn calculate_projected_service_level(
        &self,
        orders: &[ReplenishmentOrder],
        products: &[Product],
        target_service_level: f64,
    ) -> f64 {
        if products.is_empty() {
            return 0.0;
        }

        let mut total_fill_rate = 0.0;

        for product in products {
            let order = orders.iter().find(|o| o.product_id == product.id);
            let total_demand = product.total_forecast_demand();

            if total_demand <= 0.0 {
                total_fill_rate += 1.0;
                continue;
            }

            // Estimate fill rate based on inventory + orders vs demand
            let available =
                product.current_inventory as f64 + order.map_or(0, |o| o.quantity) as f64;
            let fill_rate = (available / total_demand).min(1.0);
            total_fill_rate += fill_rate;
        }

        let avg_fill_rate = total_fill_rate / products.len() as f64;

        // Weight the result towards target if we're close
        if avg_fill_rate >= target_service_level {
            avg_fill_rate
        } else {
            avg_fill_rate
        }
    }
}

/// Internal candidate structure for replenishment calculation
#[derive(Debug, Clone)]
struct ReplenishmentCandidate {
    product: Product,
    eoq: f64,
    safety_stock: f64,
    reorder_point: f64,
    days_until_stockout: f64,
    needs_order: bool,
    recommended_quantity: i64,
}

impl PackSolver for EoqSolver {
    fn id(&self) -> &'static str {
        "eoq-v1"
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<(serde_json::Value, SolverReport)> {
        let input: InventoryReplenishmentInput = spec.inputs_as()?;
        let (output, report) = self.solve_replenishment(&input, spec)?;
        let json = serde_json::to_value(&output)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;
        Ok((json, report))
    }

    fn is_exact(&self) -> bool {
        false // EOQ is an approximation
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_product(id: &str, inventory: i64, demand: f64) -> Product {
        Product {
            id: id.to_string(),
            name: format!("Product {}", id),
            current_inventory: inventory,
            demand_forecast: DemandForecast {
                average_daily: demand,
                std_deviation: demand * 0.2,
                forecast_days: 30,
            },
            lead_time_days: 7,
            unit_cost: 10.0,
            ordering_cost: 50.0,
            holding_cost_per_day: 0.02,
            stockout_cost: 25.0,
        }
    }

    fn create_test_input() -> InventoryReplenishmentInput {
        InventoryReplenishmentInput {
            products: vec![
                create_test_product("p1", 50, 10.0), // Low inventory, high demand
                create_test_product("p2", 200, 5.0), // Good inventory
                create_test_product("p3", 20, 15.0), // Critical - very low
            ],
            constraints: ReplenishmentConstraints {
                budget: 10000.0,
                target_service_level: 0.95,
                planning_horizon_days: 30,
                max_orders: None,
                min_order_quantity: None,
            },
        }
    }

    fn create_spec(input: &InventoryReplenishmentInput, seed: u64) -> ProblemSpec {
        ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(input)
            .unwrap()
            .seed(seed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_basic_replenishment() {
        let solver = EoqSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, report) = solver.solve_replenishment(&input, &spec).unwrap();

        assert!(!output.orders.is_empty());
        assert!(report.feasible);
    }

    #[test]
    fn test_prioritizes_urgent() {
        let solver = EoqSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_replenishment(&input, &spec).unwrap();

        // p3 has only 20 units with 15/day demand = 1.3 days
        // Should be ordered first
        if !output.orders.is_empty() {
            let first_order = &output.orders[0];
            assert_eq!(first_order.product_id, "p3");
        }
    }

    #[test]
    fn test_respects_budget() {
        let solver = EoqSolver;
        let mut input = create_test_input();
        input.constraints.budget = 500.0; // Very limited budget

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_replenishment(&input, &spec).unwrap();

        assert!(output.stats.total_order_cost <= 500.0);
    }

    #[test]
    fn test_no_order_when_sufficient() {
        let solver = EoqSolver;
        let input = InventoryReplenishmentInput {
            products: vec![create_test_product("p1", 1000, 5.0)], // Very high inventory
            constraints: ReplenishmentConstraints::default(),
        };

        let spec = create_spec(&input, 42);
        let (output, report) = solver.solve_replenishment(&input, &spec).unwrap();

        // With 1000 units and 5/day demand, we have 200 days of inventory
        // Should not need to order
        assert!(output.orders.is_empty() || output.not_ordered.len() > 0);
        assert!(report.feasible);
    }

    #[test]
    fn test_max_orders_limit() {
        let solver = EoqSolver;
        let mut input = create_test_input();
        input.constraints.max_orders = Some(1);

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_replenishment(&input, &spec).unwrap();

        assert!(output.orders.len() <= 1);
    }

    #[test]
    fn test_determinism() {
        let solver = EoqSolver;
        let input = create_test_input();

        let spec1 = create_spec(&input, 12345);
        let spec2 = create_spec(&input, 12345);

        let (output1, _) = solver.solve_replenishment(&input, &spec1).unwrap();
        let (output2, _) = solver.solve_replenishment(&input, &spec2).unwrap();

        assert_eq!(output1.orders.len(), output2.orders.len());
        for (a, b) in output1.orders.iter().zip(output2.orders.iter()) {
            assert_eq!(a.product_id, b.product_id);
            assert_eq!(a.quantity, b.quantity);
        }
    }

    #[test]
    fn test_projections_generated() {
        let solver = EoqSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_replenishment(&input, &spec).unwrap();

        assert!(!output.projections.is_empty());
        // Should have projections for each product at key days
        let p1_projections: Vec<_> = output
            .projections
            .iter()
            .filter(|p| p.product_id == "p1")
            .collect();
        assert!(!p1_projections.is_empty());
    }

    #[test]
    fn test_service_level_calculation() {
        let solver = EoqSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_replenishment(&input, &spec).unwrap();

        assert!(output.stats.projected_service_level >= 0.0);
        assert!(output.stats.projected_service_level <= 1.0);
    }
}
