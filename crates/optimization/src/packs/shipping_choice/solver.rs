//! Solver for Shipping Choice pack

use super::types::*;
use converge_pack::PackSolver;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport, StopReason};

/// Cost-minimizing solver for shipping choice
///
/// Algorithm:
/// 1. Filter carriers that can handle the order (hazmat check)
/// 2. Filter carriers that meet SLA requirements
/// 3. Sort by cost (ascending)
/// 4. Select cheapest option
pub struct CostMinimizingSolver;

impl CostMinimizingSolver {
    /// Solve the shipping choice problem
    pub fn solve_shipping(
        &self,
        input: &ShippingChoiceInput,
        spec: &ProblemSpec,
    ) -> Result<(ShippingChoiceOutput, SolverReport)> {
        let seed = spec.seed();

        // Filter to valid carriers
        let valid_carriers: Vec<&CarrierOption> = input
            .carriers
            .iter()
            .filter(|c| c.can_handle(&input.order))
            .collect();

        if valid_carriers.is_empty() {
            let output = ShippingChoiceOutput::no_carrier(if input.is_hazmat() {
                "No carriers support hazmat shipping"
            } else {
                "No carriers available"
            });
            let replay = ReplayEnvelope::minimal(seed);
            let report =
                SolverReport::infeasible("cost-min-v1", vec![], StopReason::NoFeasible, replay);
            return Ok((output, report));
        }

        // Separate carriers meeting SLA vs not
        let (meeting_sla, _not_meeting_sla): (Vec<_>, Vec<_>) = valid_carriers
            .iter()
            .partition(|c| c.estimated_days <= input.sla_days);

        // Sort by cost
        let mut candidates: Vec<_> = if !meeting_sla.is_empty() {
            meeting_sla
        } else {
            // Fall back to all carriers if none meet SLA
            valid_carriers.clone()
        };

        candidates.sort_by(|a, b| {
            a.cost
                .total_cmp(&b.cost)
                .then_with(|| a.carrier_id.cmp(&b.carrier_id))
                .then_with(|| a.service_level.cmp(&b.service_level))
        });

        // Apply tie-breaking for equal costs
        let tie_break = &spec.determinism.tie_break;

        // Group by cost and apply tie-breaking
        let mut best_cost = f64::INFINITY;
        let mut best_group: Vec<&&CarrierOption> = vec![];

        for carrier in &candidates {
            if (carrier.cost - best_cost).abs() < 0.01 {
                best_group.push(carrier);
            } else if carrier.cost < best_cost {
                best_cost = carrier.cost;
                best_group = vec![carrier];
            }
        }

        // Select from best group using tie-breaking
        let selected = if best_group.len() == 1 {
            *best_group[0]
        } else {
            // Sort by carrier_id for deterministic tie-breaking
            best_group.sort_by(|a, b| a.carrier_id.cmp(&b.carrier_id));
            tie_break
                .select_by(&best_group, seed, |a, b| a.carrier_id.cmp(&b.carrier_id))
                .map(|c| **c)
                .unwrap_or(best_group[0])
        };

        // Build alternatives list
        let alternatives: Vec<AlternativeCarrier> = candidates
            .iter()
            .filter(|c| c.carrier_id != selected.carrier_id)
            .take(3)
            .map(|c| AlternativeCarrier {
                carrier_id: c.carrier_id.clone(),
                service_level: c.service_level.clone(),
                cost: c.cost,
                reason_not_selected: if c.cost > selected.cost {
                    "Higher cost".to_string()
                } else if c.estimated_days > selected.estimated_days {
                    "Longer delivery time".to_string()
                } else {
                    "Tie-breaking".to_string()
                },
            })
            .collect();

        let meets_sla = selected.estimated_days <= input.sla_days;
        let selection_reason = if meets_sla {
            format!("Lowest cost carrier meeting {}-day SLA", input.sla_days)
        } else {
            format!(
                "Best available carrier (SLA not achievable, {} days vs {} required)",
                selected.estimated_days, input.sla_days
            )
        };

        let output = ShippingChoiceOutput {
            selected_carrier: Some(selected.carrier_id.clone()),
            selected_service: Some(selected.service_level.clone()),
            cost: selected.cost,
            estimated_days: selected.estimated_days,
            meets_sla,
            selection_reason,
            alternatives,
        };

        let replay = ReplayEnvelope::minimal(seed);
        let report = SolverReport::optimal("cost-min-v1", -selected.cost, replay); // Negative because we minimize

        Ok((output, report))
    }
}

impl PackSolver for CostMinimizingSolver {
    fn id(&self) -> &'static str {
        "cost-min-v1"
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<(serde_json::Value, SolverReport)> {
        let input: ShippingChoiceInput = spec.inputs_as()?;
        let (output, report) = self.solve_shipping(&input, spec)?;
        let json = serde_json::to_value(&output)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;
        Ok((json, report))
    }

    fn is_exact(&self) -> bool {
        true // We enumerate all options
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> ShippingChoiceInput {
        ShippingChoiceInput {
            order: OrderDetails {
                order_id: "ORD-001".to_string(),
                weight_kg: 2.5,
                dimensions_cm: [20.0, 15.0, 10.0],
                destination_zip: "10001".to_string(),
                is_hazmat: false,
            },
            carriers: vec![
                CarrierOption {
                    carrier_id: "ups".to_string(),
                    service_level: "ground".to_string(),
                    cost: 8.99,
                    estimated_days: 5,
                    supports_hazmat: false,
                },
                CarrierOption {
                    carrier_id: "fedex".to_string(),
                    service_level: "express".to_string(),
                    cost: 15.99,
                    estimated_days: 2,
                    supports_hazmat: true,
                },
                CarrierOption {
                    carrier_id: "usps".to_string(),
                    service_level: "priority".to_string(),
                    cost: 9.99,
                    estimated_days: 3,
                    supports_hazmat: false,
                },
            ],
            sla_days: 5,
        }
    }

    fn create_spec(input: &ShippingChoiceInput, seed: u64) -> ProblemSpec {
        ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(input)
            .unwrap()
            .seed(seed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_selects_cheapest() {
        let solver = CostMinimizingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, report) = solver.solve_shipping(&input, &spec).unwrap();

        assert_eq!(output.selected_carrier.as_deref(), Some("ups"));
        assert!((output.cost - 8.99).abs() < 0.01);
        assert!(output.meets_sla);
        assert!(report.feasible);
    }

    #[test]
    fn test_respects_sla() {
        let solver = CostMinimizingSolver;
        let mut input = create_test_input();
        input.sla_days = 2; // Tight SLA

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_shipping(&input, &spec).unwrap();

        // Only FedEx meets 2-day SLA
        assert_eq!(output.selected_carrier.as_deref(), Some("fedex"));
        assert!(output.meets_sla);
    }

    #[test]
    fn test_hazmat_filtering() {
        let solver = CostMinimizingSolver;
        let mut input = create_test_input();
        input.order.is_hazmat = true;

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_shipping(&input, &spec).unwrap();

        // Only FedEx supports hazmat
        assert_eq!(output.selected_carrier.as_deref(), Some("fedex"));
    }

    #[test]
    fn test_no_carrier_available() {
        let solver = CostMinimizingSolver;
        let input = ShippingChoiceInput {
            order: OrderDetails {
                is_hazmat: true,
                ..Default::default()
            },
            carriers: vec![CarrierOption {
                carrier_id: "basic".to_string(),
                service_level: "ground".to_string(),
                cost: 5.0,
                estimated_days: 7,
                supports_hazmat: false, // Can't handle hazmat
            }],
            sla_days: 5,
        };

        let spec = create_spec(&input, 42);
        let (output, report) = solver.solve_shipping(&input, &spec).unwrap();

        assert!(output.selected_carrier.is_none());
        assert!(!report.feasible);
    }

    #[test]
    fn test_determinism() {
        let solver = CostMinimizingSolver;
        let input = create_test_input();

        let spec1 = create_spec(&input, 12345);
        let spec2 = create_spec(&input, 12345);

        let (output1, _) = solver.solve_shipping(&input, &spec1).unwrap();
        let (output2, _) = solver.solve_shipping(&input, &spec2).unwrap();

        assert_eq!(output1.selected_carrier, output2.selected_carrier);
        assert_eq!(output1.cost, output2.cost);
    }
}
