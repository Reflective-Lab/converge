//! Solver for Capacity Planning pack

use super::types::*;
use converge_pack::PackSolver;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport, StopReason};
use std::collections::HashMap;

/// Match-based allocation solver for capacity planning
///
/// Algorithm:
/// 1. Sort demands by priority (highest first)
/// 2. For each demand, find teams with matching skills and resource types
/// 3. Allocate capacity from matching teams, respecting utilization limits
/// 4. Track fulfillment and utilization metrics
pub struct MatchAllocationSolver;

impl MatchAllocationSolver {
    /// Solve the capacity planning problem
    pub fn solve_capacity(
        &self,
        input: &CapacityPlanningInput,
        spec: &ProblemSpec,
    ) -> Result<(CapacityPlanningOutput, SolverReport)> {
        let seed = spec.seed();

        // Build resource type cost lookup
        let resource_costs: HashMap<&str, f64> = input
            .resource_types
            .iter()
            .map(|r| (r.id.as_str(), r.cost_per_unit))
            .collect();

        // Track remaining capacity per team
        let mut team_remaining: HashMap<String, f64> = input
            .teams
            .iter()
            .map(|t| (t.id.clone(), t.effective_capacity()))
            .collect();

        // Track allocations per team
        let mut team_allocated: HashMap<String, f64> =
            input.teams.iter().map(|t| (t.id.clone(), 0.0)).collect();

        // Sort demands by priority
        let mut sorted_demands: Vec<&DemandForecast> = input.demand_forecasts.iter().collect();
        sorted_demands.sort_by(|a, b| {
            a.priority.cmp(&b.priority).then_with(|| {
                // Tie-break by demand_units (higher demand first)
                b.demand_units
                    .partial_cmp(&a.demand_units)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        let tie_break = &spec.determinism.tie_break;

        let mut assignments = Vec::new();
        let mut period_allocations: HashMap<String, Vec<(String, f64, f64)>> = HashMap::new(); // period -> [(demand_id, requested, allocated)]
        let mut assignment_id = 0;

        // Process each demand
        for demand in sorted_demands {
            let mut remaining_demand = demand.demand_units;
            let demand_id = format!("demand-{}-{}", demand.period_id, demand.required_skill);

            // Find matching teams
            let mut matching_teams: Vec<&Team> = input
                .teams
                .iter()
                .filter(|t| {
                    // Must have the required skill (if strict matching)
                    let skill_match = if input.constraints.strict_skill_matching {
                        t.has_skill(&demand.required_skill)
                    } else {
                        true
                    };

                    // Must provide the required resource type
                    let resource_match = t.provides_resource_type(&demand.resource_type);

                    // Must have remaining capacity
                    let has_capacity = team_remaining.get(&t.id).map_or(false, |&c| c > 0.0);

                    skill_match && resource_match && has_capacity
                })
                .collect();

            // Sort by remaining capacity (prefer teams with more capacity for load balancing)
            matching_teams.sort_by(|a, b| {
                let cap_a = team_remaining.get(&a.id).unwrap_or(&0.0);
                let cap_b = team_remaining.get(&b.id).unwrap_or(&0.0);
                cap_b
                    .partial_cmp(cap_a)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Apply deterministic tie-breaking for teams with similar capacity
            if matching_teams.len() > 1 {
                let first_cap = team_remaining.get(&matching_teams[0].id).unwrap_or(&0.0);
                let similar_cap: Vec<&Team> = matching_teams
                    .iter()
                    .filter(|t| {
                        let cap = team_remaining.get(&t.id).unwrap_or(&0.0);
                        (cap - first_cap).abs() < 0.01
                    })
                    .copied()
                    .collect();

                if similar_cap.len() > 1 {
                    // Sort by ID for deterministic selection
                    let mut sorted = similar_cap.clone();
                    sorted.sort_by(|a, b| a.id.cmp(&b.id));
                    if let Some(selected) =
                        tie_break.select_by(&sorted, seed, |a, b| a.id.cmp(&b.id))
                    {
                        // Move selected to front
                        matching_teams.retain(|t| t.id != selected.id);
                        matching_teams.insert(0, selected);
                    }
                }
            }

            let mut allocated_for_demand = 0.0;

            // Allocate from matching teams
            for team in matching_teams {
                if remaining_demand <= 0.0 {
                    break;
                }

                let available = team_remaining.get(&team.id).unwrap_or(&0.0);
                let allocate = remaining_demand.min(*available);

                if allocate > 0.0 {
                    let cost_per_unit = resource_costs
                        .get(demand.resource_type.as_str())
                        .unwrap_or(&0.0);
                    let cost = allocate * cost_per_unit;

                    assignments.push(ResourceAssignment {
                        id: format!("assign-{}", assignment_id),
                        team_id: team.id.clone(),
                        period_id: demand.period_id.clone(),
                        resource_type: demand.resource_type.clone(),
                        demand_id: demand_id.clone(),
                        allocated_units: allocate,
                        cost,
                    });

                    assignment_id += 1;
                    remaining_demand -= allocate;
                    allocated_for_demand += allocate;

                    // Update tracking
                    if let Some(rem) = team_remaining.get_mut(&team.id) {
                        *rem -= allocate;
                    }
                    if let Some(alloc) = team_allocated.get_mut(&team.id) {
                        *alloc += allocate;
                    }
                }
            }

            // Track period allocations
            period_allocations
                .entry(demand.period_id.clone())
                .or_default()
                .push((demand_id.clone(), demand.demand_units, allocated_for_demand));
        }

        // Build team utilization metrics
        let team_utilization: Vec<TeamUtilization> = input
            .teams
            .iter()
            .map(|t| {
                let allocated = *team_allocated.get(&t.id).unwrap_or(&0.0);
                let utilization_ratio = t.utilization(allocated);
                TeamUtilization {
                    team_id: t.id.clone(),
                    team_name: t.name.clone(),
                    total_capacity: t.available_capacity,
                    allocated,
                    utilization_ratio,
                    remaining_capacity: t.available_capacity - allocated,
                    is_over_utilized: utilization_ratio > t.max_utilization,
                }
            })
            .collect();

        // Build period fulfillment metrics
        let period_fulfillment: Vec<PeriodFulfillment> = {
            let mut periods: Vec<String> = period_allocations.keys().cloned().collect();
            periods.sort();
            periods
                .into_iter()
                .map(|period_id| {
                    let demands = period_allocations.get(&period_id).unwrap();
                    let total_demand: f64 = demands.iter().map(|(_, req, _)| req).sum();
                    let total_allocated: f64 = demands.iter().map(|(_, _, alloc)| alloc).sum();
                    let fulfillment_ratio = if total_demand > 0.0 {
                        total_allocated / total_demand
                    } else {
                        1.0
                    };

                    let unmet_demands: Vec<UnmetDemand> = demands
                        .iter()
                        .filter(|(_, req, alloc)| alloc < req)
                        .map(|(demand_id, requested, allocated)| {
                            // Find the original demand to get the skill
                            let skill = input
                                .demand_forecasts
                                .iter()
                                .find(|d| {
                                    format!("demand-{}-{}", d.period_id, d.required_skill)
                                        == *demand_id
                                })
                                .map(|d| d.required_skill.clone())
                                .unwrap_or_default();

                            UnmetDemand {
                                demand_id: demand_id.clone(),
                                required_skill: skill,
                                requested: *requested,
                                allocated: *allocated,
                                shortfall: requested - allocated,
                                reason: "Insufficient matching capacity".to_string(),
                            }
                        })
                        .collect();

                    PeriodFulfillment {
                        period_id,
                        total_demand,
                        total_allocated,
                        fulfillment_ratio,
                        unmet_demands,
                    }
                })
                .collect()
        };

        // Calculate summary statistics
        let total_demand: f64 = input.demand_forecasts.iter().map(|d| d.demand_units).sum();
        let total_allocated: f64 = assignments.iter().map(|a| a.allocated_units).sum();
        let total_cost: f64 = assignments.iter().map(|a| a.cost).sum();
        let overall_fulfillment_ratio = if total_demand > 0.0 {
            total_allocated / total_demand
        } else {
            1.0
        };
        let average_utilization = if !team_utilization.is_empty() {
            team_utilization
                .iter()
                .map(|t| t.utilization_ratio)
                .sum::<f64>()
                / team_utilization.len() as f64
        } else {
            0.0
        };
        let teams_over_capacity = team_utilization
            .iter()
            .filter(|t| t.is_over_utilized)
            .count();
        let unmet_demands_count = period_fulfillment
            .iter()
            .map(|p| p.unmet_demands.len())
            .sum();

        let plan_status = if overall_fulfillment_ratio >= input.constraints.min_overall_fulfillment
            && teams_over_capacity == 0
        {
            "Feasible plan meets all constraints".to_string()
        } else if overall_fulfillment_ratio < input.constraints.min_overall_fulfillment {
            format!(
                "Fulfillment {:.1}% below minimum {:.1}%",
                overall_fulfillment_ratio * 100.0,
                input.constraints.min_overall_fulfillment * 100.0
            )
        } else {
            format!("{} teams exceed maximum utilization", teams_over_capacity)
        };

        let summary = CapacityPlanSummary {
            total_demand,
            total_allocated,
            overall_fulfillment_ratio,
            total_cost,
            average_utilization,
            teams_over_capacity,
            unmet_demands: unmet_demands_count,
            plan_status,
        };

        let output = CapacityPlanningOutput {
            assignments,
            team_utilization,
            period_fulfillment,
            summary,
        };

        // Determine solver report status
        let replay = ReplayEnvelope::minimal(seed);
        let report = if overall_fulfillment_ratio >= input.constraints.min_overall_fulfillment {
            SolverReport::feasible(
                "match-alloc-v1",
                overall_fulfillment_ratio,
                StopReason::Feasible,
                replay,
            )
        } else {
            SolverReport::infeasible("match-alloc-v1", vec![], StopReason::NoFeasible, replay)
        };

        Ok((output, report))
    }
}

impl PackSolver for MatchAllocationSolver {
    fn id(&self) -> &'static str {
        "match-alloc-v1"
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<(serde_json::Value, SolverReport)> {
        let input: CapacityPlanningInput = spec.inputs_as()?;
        let (output, report) = self.solve_capacity(&input, spec)?;
        let json = serde_json::to_value(&output)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;
        Ok((json, report))
    }

    fn is_exact(&self) -> bool {
        false // Greedy algorithm, not guaranteed optimal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> CapacityPlanningInput {
        CapacityPlanningInput {
            demand_forecasts: vec![
                DemandForecast {
                    period_id: "Q1-2024".to_string(),
                    resource_type: "engineering".to_string(),
                    required_skill: "backend".to_string(),
                    demand_units: 100.0,
                    priority: 1,
                    min_fulfillment_ratio: 0.8,
                },
                DemandForecast {
                    period_id: "Q1-2024".to_string(),
                    resource_type: "engineering".to_string(),
                    required_skill: "frontend".to_string(),
                    demand_units: 50.0,
                    priority: 2,
                    min_fulfillment_ratio: 0.7,
                },
            ],
            resource_types: vec![ResourceType {
                id: "engineering".to_string(),
                name: "Engineering Hours".to_string(),
                unit: "hours".to_string(),
                cost_per_unit: 100.0,
            }],
            teams: vec![
                Team {
                    id: "team-a".to_string(),
                    name: "Backend Team".to_string(),
                    skills: vec!["backend".to_string()],
                    resource_types: vec!["engineering".to_string()],
                    available_capacity: 120.0,
                    max_utilization: 0.85,
                    headcount: 6,
                },
                Team {
                    id: "team-b".to_string(),
                    name: "Frontend Team".to_string(),
                    skills: vec!["frontend".to_string()],
                    resource_types: vec!["engineering".to_string()],
                    available_capacity: 80.0,
                    max_utilization: 0.85,
                    headcount: 4,
                },
            ],
            constraints: PlanningConstraints {
                target_utilization: 0.75,
                max_budget: Some(20000.0),
                min_overall_fulfillment: 0.8,
                allow_cross_team: false,
                strict_skill_matching: true,
            },
        }
    }

    fn create_spec(input: &CapacityPlanningInput, seed: u64) -> ProblemSpec {
        ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::maximize("fulfillment"))
            .inputs(input)
            .unwrap()
            .seed(seed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_basic_allocation() {
        let solver = MatchAllocationSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, report) = solver.solve_capacity(&input, &spec).unwrap();

        assert!(report.feasible);
        assert!(!output.assignments.is_empty());
        assert!(output.summary.overall_fulfillment_ratio > 0.0);
    }

    #[test]
    fn test_skill_matching() {
        let solver = MatchAllocationSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_capacity(&input, &spec).unwrap();

        // Backend demand should be allocated to team-a (backend team)
        let backend_assignments: Vec<_> = output
            .assignments
            .iter()
            .filter(|a| a.demand_id.contains("backend"))
            .collect();
        assert!(!backend_assignments.is_empty());
        assert!(backend_assignments.iter().all(|a| a.team_id == "team-a"));

        // Frontend demand should be allocated to team-b (frontend team)
        let frontend_assignments: Vec<_> = output
            .assignments
            .iter()
            .filter(|a| a.demand_id.contains("frontend"))
            .collect();
        assert!(!frontend_assignments.is_empty());
        assert!(frontend_assignments.iter().all(|a| a.team_id == "team-b"));
    }

    #[test]
    fn test_respects_utilization_limits() {
        let solver = MatchAllocationSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_capacity(&input, &spec).unwrap();

        // Team A: capacity 120, max_util 0.85 -> effective 102
        // Backend demand is 100, should fit within effective capacity
        let team_a_util = output
            .team_utilization
            .iter()
            .find(|t| t.team_id == "team-a")
            .unwrap();
        assert!(!team_a_util.is_over_utilized);
    }

    #[test]
    fn test_insufficient_capacity() {
        let solver = MatchAllocationSolver;
        let mut input = create_test_input();
        // Increase demand beyond capacity
        input.demand_forecasts[0].demand_units = 500.0;
        input.constraints.min_overall_fulfillment = 0.95;

        let spec = create_spec(&input, 42);
        let (output, report) = solver.solve_capacity(&input, &spec).unwrap();

        // Should be infeasible because we can't meet 95% of 550 units with ~170 effective capacity
        assert!(!report.feasible);
        assert!(output.summary.overall_fulfillment_ratio < 0.95);
    }

    #[test]
    fn test_cost_calculation() {
        let solver = MatchAllocationSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_capacity(&input, &spec).unwrap();

        // Cost should be allocated_units * cost_per_unit
        for assignment in &output.assignments {
            assert!((assignment.cost - assignment.allocated_units * 100.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_determinism() {
        let solver = MatchAllocationSolver;
        let input = create_test_input();

        let spec1 = create_spec(&input, 12345);
        let spec2 = create_spec(&input, 12345);

        let (output1, _) = solver.solve_capacity(&input, &spec1).unwrap();
        let (output2, _) = solver.solve_capacity(&input, &spec2).unwrap();

        assert_eq!(output1.assignments.len(), output2.assignments.len());
        assert!((output1.summary.total_allocated - output2.summary.total_allocated).abs() < 0.01);
    }

    #[test]
    fn test_priority_ordering() {
        let solver = MatchAllocationSolver;
        let mut input = create_test_input();

        // Add a third demand with same skill but lower priority
        input.demand_forecasts.push(DemandForecast {
            period_id: "Q1-2024".to_string(),
            resource_type: "engineering".to_string(),
            required_skill: "backend".to_string(),
            demand_units: 50.0,
            priority: 3,
            min_fulfillment_ratio: 0.5,
        });

        // Reduce backend team capacity to force prioritization
        input.teams[0].available_capacity = 100.0;
        input.teams[0].max_utilization = 1.0;

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_capacity(&input, &spec).unwrap();

        // Higher priority backend demand (100 units, priority 1) should be fully allocated
        // before lower priority (50 units, priority 3)
        let total_backend: f64 = output
            .assignments
            .iter()
            .filter(|a| a.demand_id.contains("backend"))
            .map(|a| a.allocated_units)
            .sum();

        // With 100 capacity and 150 total backend demand, priority 1 should get full 100
        assert!(total_backend >= 100.0);
    }

    #[test]
    fn test_period_fulfillment_tracking() {
        let solver = MatchAllocationSolver;
        let mut input = create_test_input();

        // Add a second period
        input.demand_forecasts.push(DemandForecast {
            period_id: "Q2-2024".to_string(),
            resource_type: "engineering".to_string(),
            required_skill: "backend".to_string(),
            demand_units: 30.0,
            priority: 1,
            min_fulfillment_ratio: 0.8,
        });

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_capacity(&input, &spec).unwrap();

        // Should have fulfillment records for both periods
        assert!(output.period_fulfillment.len() >= 2);

        let q1 = output
            .period_fulfillment
            .iter()
            .find(|p| p.period_id == "Q1-2024");
        let q2 = output
            .period_fulfillment
            .iter()
            .find(|p| p.period_id == "Q2-2024");

        assert!(q1.is_some());
        assert!(q2.is_some());
    }
}
