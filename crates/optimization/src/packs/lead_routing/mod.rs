//! Lead Routing Pack
//!
//! JTBD: "Route sales leads to reps based on territory, expertise, and capacity."
//!
//! ## Problem
//!
//! Given:
//! - Incoming leads with scores, territories, segments, and required skills
//! - Sales reps with territories, expertise, and capacity constraints
//! - Routing configuration (territory requirements, load balancing preferences)
//!
//! Find:
//! - Optimal lead-to-rep assignment maximizing fit while respecting constraints
//!
//! ## Solver
//!
//! Uses score-based assignment:
//! 1. Sort leads by priority and score
//! 2. For each lead, calculate fit scores with all available reps
//! 3. Filter reps by territory requirement if configured
//! 4. Assign lead to best-scoring rep with available capacity
//! 5. Track rep utilization and provide detailed scoring rationale
//!
//! ## Invariants
//!
//! Critical:
//! - all_leads_assigned: All leads must be assigned to a rep
//! - capacity_not_exceeded: No rep should exceed their capacity
//! - territory_respected: Leads should be assigned to reps covering their territory (when required)
//!
//! Advisory:
//! - load_balanced: Lead assignments should be reasonably balanced across reps
//! - fit_score_acceptable: Average fit score should meet minimum threshold

mod invariants;
mod solver;
mod types;

pub use invariants::*;
pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

/// Lead Routing Pack
pub struct LeadRoutingPack;

impl Pack for LeadRoutingPack {
    fn name(&self) -> &'static str {
        "lead-routing"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: LeadRoutingInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_pack::GateError::invalid_input(format!("Invalid input: {}", e))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: LeadRoutingInput = spec.inputs_as()?;
        input.validate()?;

        let solver = ScoreBasedRoutingSolver;
        let (output, report) = solver.solve_routing(&input, spec)?;

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
        let output: LeadRoutingOutput = plan.plan_as()?;

        // We need the input for some invariant checks
        // For now, create a minimal input for invariant checking
        // In a full implementation, the input would be stored in the plan or spec
        let input = LeadRoutingInput {
            leads: output
                .assignments
                .iter()
                .map(|a| Lead {
                    id: a.lead_id.clone(),
                    score: 0.0,
                    territory: String::new(),
                    segment: String::new(),
                    required_skills: vec![],
                    estimated_value: 0.0,
                    priority: 5,
                })
                .chain(output.unassigned.iter().map(|u| Lead {
                    id: u.lead_id.clone(),
                    score: 0.0,
                    territory: String::new(),
                    segment: String::new(),
                    required_skills: vec![],
                    estimated_value: 0.0,
                    priority: 5,
                }))
                .collect(),
            reps: output
                .rep_utilization
                .iter()
                .map(|r| SalesRep {
                    id: r.rep_id.clone(),
                    name: r.rep_name.clone(),
                    capacity: r.capacity,
                    current_load: r.total_load - r.new_assignments,
                    territories: vec![],
                    segments: vec![],
                    skills: vec![],
                    performance_score: 50.0,
                })
                .collect(),
            config: RoutingConfig::default(),
        };

        Ok(check_all_invariants(&output, &input))
    }

    fn evaluate_gate(
        &self,
        _plan: &ProposedPlan,
        invariant_results: &[InvariantResult],
    ) -> PromotionGate {
        default_gate_evaluation(invariant_results, &get_invariants())
    }
}

fn calculate_confidence(output: &LeadRoutingOutput, _input: &LeadRoutingInput) -> f64 {
    if output.assignments.is_empty() {
        return 0.0;
    }

    let mut confidence = 0.5;

    // Higher confidence if all leads assigned
    if output.unassigned.is_empty() {
        confidence += 0.25;
    } else {
        // Partial credit based on assignment rate
        let assignment_rate = output.stats.assigned_leads as f64 / output.stats.total_leads as f64;
        confidence += 0.25 * assignment_rate;
    }

    // Higher confidence if fit score is good
    if output.stats.average_fit_score >= 70.0 {
        confidence += 0.15;
    } else if output.stats.average_fit_score >= 50.0 {
        confidence += 0.1;
    }

    // Higher confidence if load is balanced (check utilization variance)
    if output.rep_utilization.len() >= 2 {
        let utilizations: Vec<f64> = output
            .rep_utilization
            .iter()
            .map(|u| u.utilization_pct)
            .collect();
        let avg: f64 = utilizations.iter().sum::<f64>() / utilizations.len() as f64;
        let variance: f64 =
            utilizations.iter().map(|u| (u - avg).powi(2)).sum::<f64>() / utilizations.len() as f64;
        let std_dev = variance.sqrt();

        if std_dev <= 15.0 {
            confidence += 0.1;
        }
    } else {
        confidence += 0.05;
    }

    confidence.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> LeadRoutingInput {
        LeadRoutingInput {
            leads: vec![
                Lead {
                    id: "lead-1".to_string(),
                    score: 85.0,
                    territory: "west".to_string(),
                    segment: "enterprise".to_string(),
                    required_skills: vec!["cloud".to_string()],
                    estimated_value: 100000.0,
                    priority: 1,
                },
                Lead {
                    id: "lead-2".to_string(),
                    score: 70.0,
                    territory: "east".to_string(),
                    segment: "smb".to_string(),
                    required_skills: vec![],
                    estimated_value: 25000.0,
                    priority: 3,
                },
            ],
            reps: vec![
                SalesRep {
                    id: "rep-1".to_string(),
                    name: "Alice Johnson".to_string(),
                    capacity: 10,
                    current_load: 5,
                    territories: vec!["west".to_string()],
                    segments: vec!["enterprise".to_string()],
                    skills: vec!["cloud".to_string()],
                    performance_score: 92.0,
                },
                SalesRep {
                    id: "rep-2".to_string(),
                    name: "Bob Smith".to_string(),
                    capacity: 8,
                    current_load: 3,
                    territories: vec!["east".to_string()],
                    segments: vec!["smb".to_string()],
                    skills: vec!["demos".to_string()],
                    performance_score: 78.0,
                },
            ],
            config: RoutingConfig::default(),
        }
    }

    #[test]
    fn test_pack_name() {
        let pack = LeadRoutingPack;
        assert_eq!(pack.name(), "lead-routing");
        assert_eq!(pack.version(), "1.0.0");
    }

    #[test]
    fn test_validate_inputs() {
        let pack = LeadRoutingPack;
        let input = create_test_input();
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_ok());
    }

    #[test]
    fn test_validate_inputs_empty_leads() {
        let pack = LeadRoutingPack;
        let input = LeadRoutingInput {
            leads: vec![],
            reps: vec![SalesRep {
                id: "rep-1".to_string(),
                name: "Test".to_string(),
                capacity: 10,
                current_load: 0,
                territories: vec![],
                segments: vec![],
                skills: vec![],
                performance_score: 50.0,
            }],
            config: RoutingConfig::default(),
        };
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_err());
    }

    #[test]
    fn test_solve_basic() {
        let pack = LeadRoutingPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-001", "test-tenant")
            .objective(ObjectiveSpec::maximize("conversion"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        assert!(result.is_feasible());

        let output: LeadRoutingOutput = result.plan.plan_as().unwrap();
        assert_eq!(output.stats.total_leads, 2);
        assert_eq!(output.stats.assigned_leads, 2);
        assert!(output.unassigned.is_empty());
    }

    #[test]
    fn test_solve_with_territory_routing() {
        let pack = LeadRoutingPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-002", "test-tenant")
            .objective(ObjectiveSpec::maximize("conversion"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: LeadRoutingOutput = result.plan.plan_as().unwrap();

        // Check that leads are routed to appropriate territory reps
        for assignment in &output.assignments {
            if assignment.lead_id == "lead-1" {
                // West territory lead should go to rep-1 (west territory)
                assert_eq!(assignment.rep_id, "rep-1");
            } else if assignment.lead_id == "lead-2" {
                // East territory lead should go to rep-2 (east territory)
                assert_eq!(assignment.rep_id, "rep-2");
            }
        }
    }

    #[test]
    fn test_check_invariants() {
        let pack = LeadRoutingPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-003", "test-tenant")
            .objective(ObjectiveSpec::maximize("conversion"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();

        // All invariants should pass for a valid solution
        let critical_pass = invariants
            .iter()
            .filter(|r| {
                r.invariant == "all_leads_assigned" || r.invariant == "capacity_not_exceeded"
            })
            .all(|r| r.passed);
        assert!(critical_pass);
    }

    #[test]
    fn test_gate_promotes() {
        let pack = LeadRoutingPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-004", "test-tenant")
            .objective(ObjectiveSpec::maximize("conversion"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();
        let gate = pack.evaluate_gate(&result.plan, &invariants);

        // Should promote or require review (not reject)
        assert!(!gate.is_rejected());
    }

    #[test]
    fn test_determinism() {
        let pack = LeadRoutingPack;
        let input = create_test_input();

        let spec1 = ProblemSpec::builder("test-a", "tenant")
            .objective(ObjectiveSpec::maximize("conversion"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let spec2 = ProblemSpec::builder("test-b", "tenant")
            .objective(ObjectiveSpec::maximize("conversion"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let result1 = pack.solve(&spec1).unwrap();
        let result2 = pack.solve(&spec2).unwrap();

        let output1: LeadRoutingOutput = result1.plan.plan_as().unwrap();
        let output2: LeadRoutingOutput = result2.plan.plan_as().unwrap();

        assert_eq!(output1.assignments.len(), output2.assignments.len());
        for (a1, a2) in output1.assignments.iter().zip(output2.assignments.iter()) {
            assert_eq!(a1.lead_id, a2.lead_id);
            assert_eq!(a1.rep_id, a2.rep_id);
        }
    }

    #[test]
    fn test_capacity_constraint() {
        let pack = LeadRoutingPack;
        let mut input = create_test_input();

        // Add many more leads than capacity
        for i in 0..20 {
            input.leads.push(Lead {
                id: format!("lead-extra-{}", i),
                score: 60.0,
                territory: "west".to_string(),
                segment: "enterprise".to_string(),
                required_skills: vec![],
                estimated_value: 30000.0,
                priority: 5,
            });
        }

        let spec = ProblemSpec::builder("test-005", "test-tenant")
            .objective(ObjectiveSpec::maximize("conversion"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: LeadRoutingOutput = result.plan.plan_as().unwrap();

        // Check that capacity is respected
        for util in &output.rep_utilization {
            assert!(
                util.total_load <= util.capacity,
                "Rep {} has load {} but capacity {}",
                util.rep_name,
                util.total_load,
                util.capacity
            );
        }

        // Some leads should be unassigned
        assert!(!output.unassigned.is_empty());
    }

    #[test]
    fn test_scoring_rationale_included() {
        let pack = LeadRoutingPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-006", "test-tenant")
            .objective(ObjectiveSpec::maximize("conversion"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: LeadRoutingOutput = result.plan.plan_as().unwrap();

        for assignment in &output.assignments {
            assert!(assignment.fit_score > 0.0);
            assert!(!assignment.scoring_rationale.explanation.is_empty());
        }
    }

    #[test]
    fn test_rep_utilization_output() {
        let pack = LeadRoutingPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-007", "test-tenant")
            .objective(ObjectiveSpec::maximize("conversion"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: LeadRoutingOutput = result.plan.plan_as().unwrap();

        // Should have utilization for reps that received assignments
        assert!(!output.rep_utilization.is_empty());

        for util in &output.rep_utilization {
            assert!(util.new_assignments > 0);
            assert!(util.utilization_pct >= 0.0);
            assert!(util.utilization_pct <= 100.0);
        }
    }
}
