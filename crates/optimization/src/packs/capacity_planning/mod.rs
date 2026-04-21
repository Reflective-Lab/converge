//! Capacity Planning Pack
//!
//! JTBD: "Plan resource capacity across teams/periods to meet demand forecasts."
//!
//! ## Problem
//!
//! Given:
//! - Demand forecasts by period, resource type, and required skill
//! - Available teams with their skills, capacity, and utilization limits
//! - Resource types with associated costs
//! - Planning constraints (budget, minimum fulfillment, etc.)
//!
//! Find:
//! - Optimal allocation of team resources to meet demand
//! - Utilization metrics for each team
//! - Fulfillment metrics for each period
//!
//! ## Solver
//!
//! Uses match-based allocation:
//! 1. Sort demands by priority (highest first)
//! 2. For each demand, find teams with matching skills and resource types
//! 3. Allocate capacity from matching teams, respecting utilization limits
//! 4. Track fulfillment and utilization metrics
//!
//! ## Invariants
//!
//! - `demand_met` (critical): Minimum fulfillment requirements must be met
//! - `capacity_not_exceeded` (critical): No team should exceed their maximum utilization
//! - `skills_matched` (critical): All assignments must match required skills
//! - `utilization_balanced` (advisory): Team utilization should be reasonably balanced
//! - `cost_within_budget` (advisory): Total cost should be within budget constraints

mod invariants;
mod solver;
mod types;

pub use invariants::*;
pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};
use converge_pack::{
    CONFIDENCE_STEP_MAJOR, CONFIDENCE_STEP_MEDIUM, CONFIDENCE_STEP_MINOR, CONFIDENCE_STEP_TINY,
};

/// Capacity Planning Pack
pub struct CapacityPlanningPack;

impl Pack for CapacityPlanningPack {
    fn name(&self) -> &'static str {
        "capacity-planning"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: CapacityPlanningInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_pack::GateError::invalid_input(format!("Invalid input: {}", e))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: CapacityPlanningInput = spec.inputs_as()?;
        input.validate()?;

        let solver = MatchAllocationSolver;
        let (output, report) = solver.solve_capacity(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));
        let confidence = calculate_confidence(&output, &input);

        let plan = ProposedPlan::from_payload(
            format!("plan-{}", spec.problem_id),
            self.name(),
            output.summary(),
            &output,
            confidence,
            trace,
        )?;

        Ok(PackSolveResult::new(plan, report))
    }

    fn check_invariants(&self, plan: &ProposedPlan) -> Result<Vec<InvariantResult>> {
        let output: CapacityPlanningOutput = plan.plan_as()?;
        Ok(check_all_invariants(&output))
    }

    fn evaluate_gate(
        &self,
        _plan: &ProposedPlan,
        invariant_results: &[InvariantResult],
    ) -> PromotionGate {
        default_gate_evaluation(invariant_results, self.invariants())
    }
}

fn calculate_confidence(output: &CapacityPlanningOutput, input: &CapacityPlanningInput) -> f64 {
    if output.assignments.is_empty() {
        return 0.0;
    }

    let mut confidence: f64 = 0.4;

    // Higher confidence if fulfillment is high
    if output.summary.overall_fulfillment_ratio >= 0.95 {
        confidence += 0.3;
    } else if output.summary.overall_fulfillment_ratio >= 0.8 {
        confidence += CONFIDENCE_STEP_MAJOR;
    } else if output.summary.overall_fulfillment_ratio >= 0.6 {
        confidence += CONFIDENCE_STEP_MINOR;
    }

    // Higher confidence if no teams are over-utilized
    if output.summary.teams_over_capacity == 0 {
        confidence += CONFIDENCE_STEP_MEDIUM;
    }

    // Higher confidence if utilization is balanced
    if !output.team_utilization.is_empty() {
        let utils: Vec<f64> = output
            .team_utilization
            .iter()
            .filter(|t| t.total_capacity > 0.0)
            .map(|t| t.utilization_ratio)
            .collect();

        if !utils.is_empty() {
            let mean = utils.iter().sum::<f64>() / utils.len() as f64;
            let variance =
                utils.iter().map(|u| (u - mean).powi(2)).sum::<f64>() / utils.len() as f64;
            let std_dev = variance.sqrt();

            if std_dev < 0.15 {
                confidence += CONFIDENCE_STEP_MINOR;
            }
        }
    }

    // Higher confidence if within budget (if specified)
    if let Some(budget) = input.constraints.max_budget {
        if output.summary.total_cost <= budget {
            confidence += CONFIDENCE_STEP_TINY;
        }
    }

    confidence.min(1.0)
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

    #[test]
    fn test_pack_name() {
        let pack = CapacityPlanningPack;
        assert_eq!(pack.name(), "capacity-planning");
        assert_eq!(pack.version(), "1.0.0");
    }

    #[test]
    fn test_validate_inputs() {
        let pack = CapacityPlanningPack;
        let input = create_test_input();
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_ok());
    }

    #[test]
    fn test_validate_inputs_empty_demands() {
        let pack = CapacityPlanningPack;
        let mut input = create_test_input();
        input.demand_forecasts = vec![];
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_err());
    }

    #[test]
    fn test_solve_basic() {
        let pack = CapacityPlanningPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-001", "test-tenant")
            .objective(ObjectiveSpec::maximize("fulfillment"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        assert!(result.is_feasible());

        let output: CapacityPlanningOutput = result.plan.plan_as().unwrap();
        assert!(!output.assignments.is_empty());
        assert!(output.summary.overall_fulfillment_ratio > 0.0);
    }

    #[test]
    fn test_solve_with_skill_matching() {
        let pack = CapacityPlanningPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-002", "test-tenant")
            .objective(ObjectiveSpec::maximize("fulfillment"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: CapacityPlanningOutput = result.plan.plan_as().unwrap();

        // Verify skill matching worked
        let backend_assignments: Vec<_> = output
            .assignments
            .iter()
            .filter(|a| a.demand_id.contains("backend"))
            .collect();
        assert!(backend_assignments.iter().all(|a| a.team_id == "team-a"));

        let frontend_assignments: Vec<_> = output
            .assignments
            .iter()
            .filter(|a| a.demand_id.contains("frontend"))
            .collect();
        assert!(frontend_assignments.iter().all(|a| a.team_id == "team-b"));
    }

    #[test]
    fn test_check_invariants() {
        let pack = CapacityPlanningPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-003", "test-tenant")
            .objective(ObjectiveSpec::maximize("fulfillment"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();

        // With valid input and sufficient capacity, all should pass
        let all_pass = invariants.iter().all(|r| r.passed);
        assert!(all_pass);
    }

    #[test]
    fn test_gate_promotes() {
        let pack = CapacityPlanningPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-004", "test-tenant")
            .objective(ObjectiveSpec::maximize("fulfillment"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();
        let gate = pack.evaluate_gate(&result.plan, &invariants);

        assert!(gate.is_promoted());
    }

    #[test]
    fn test_determinism() {
        let pack = CapacityPlanningPack;
        let input = create_test_input();

        let spec1 = ProblemSpec::builder("test-a", "tenant")
            .objective(ObjectiveSpec::maximize("fulfillment"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let spec2 = ProblemSpec::builder("test-b", "tenant")
            .objective(ObjectiveSpec::maximize("fulfillment"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let result1 = pack.solve(&spec1).unwrap();
        let result2 = pack.solve(&spec2).unwrap();

        let output1: CapacityPlanningOutput = result1.plan.plan_as().unwrap();
        let output2: CapacityPlanningOutput = result2.plan.plan_as().unwrap();

        assert_eq!(output1.assignments.len(), output2.assignments.len());
        assert!((output1.summary.total_allocated - output2.summary.total_allocated).abs() < 0.01);
    }

    #[test]
    fn test_insufficient_capacity() {
        let pack = CapacityPlanningPack;
        let mut input = create_test_input();

        // Increase demand beyond available capacity
        input.demand_forecasts[0].demand_units = 500.0;
        input.constraints.min_overall_fulfillment = 0.95;

        let spec = ProblemSpec::builder("test-005", "test-tenant")
            .objective(ObjectiveSpec::maximize("fulfillment"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();

        // Should not be feasible because we can't meet 95% fulfillment
        assert!(!result.is_feasible());
    }

    #[test]
    fn test_utilization_metrics() {
        let pack = CapacityPlanningPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-006", "test-tenant")
            .objective(ObjectiveSpec::maximize("fulfillment"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: CapacityPlanningOutput = result.plan.plan_as().unwrap();

        // Should have utilization for both teams
        assert_eq!(output.team_utilization.len(), 2);

        // No team should be over-utilized with the test data
        assert!(output.team_utilization.iter().all(|t| !t.is_over_utilized));

        // Average utilization should be reasonable
        assert!(output.summary.average_utilization > 0.0);
        assert!(output.summary.average_utilization <= 1.0);
    }

    #[test]
    fn test_period_fulfillment() {
        let pack = CapacityPlanningPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-007", "test-tenant")
            .objective(ObjectiveSpec::maximize("fulfillment"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: CapacityPlanningOutput = result.plan.plan_as().unwrap();

        // Should have period fulfillment data
        assert!(!output.period_fulfillment.is_empty());

        let q1 = output
            .period_fulfillment
            .iter()
            .find(|p| p.period_id == "Q1-2024");
        assert!(q1.is_some());
        let q1 = q1.unwrap();

        // Should have high fulfillment with the test data
        assert!(q1.fulfillment_ratio > 0.8);
    }

    #[test]
    fn test_cost_calculation() {
        let pack = CapacityPlanningPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-008", "test-tenant")
            .objective(ObjectiveSpec::maximize("fulfillment"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: CapacityPlanningOutput = result.plan.plan_as().unwrap();

        // Total cost should equal sum of assignment costs
        let sum_costs: f64 = output.assignments.iter().map(|a| a.cost).sum();
        assert!((output.summary.total_cost - sum_costs).abs() < 0.01);

        // Each assignment cost should be units * cost_per_unit (100.0)
        for assignment in &output.assignments {
            assert!((assignment.cost - assignment.allocated_units * 100.0).abs() < 0.01);
        }
    }
}
