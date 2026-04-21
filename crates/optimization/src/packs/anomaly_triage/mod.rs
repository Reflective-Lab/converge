//! Anomaly Triage Pack
//!
//! JTBD: "Triages anomalies with stable thresholds and escalation rules."
//!
//! ## Problem
//!
//! Given:
//! - Detected anomalies with features (z-scores)
//! - Severity thresholds
//! - Escalation policies
//!
//! Find:
//! - Prioritized triage list with escalation recommendations
//!
//! ## Solver
//!
//! Uses threshold-based classification:
//! 1. Classify each anomaly by z-score threshold
//! 2. Sort by severity then by z-score
//! 3. Apply escalation policies
//! 4. Assign sequential priorities

mod invariants;
mod solver;
mod types;

pub use invariants::*;
pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

/// Anomaly Triage Pack
pub struct AnomalyTriagePack;

impl Pack for AnomalyTriagePack {
    fn name(&self) -> &'static str {
        "anomaly-triage"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: AnomalyTriageInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_pack::GateError::invalid_input(format!("Invalid input: {}", e))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: AnomalyTriageInput = spec.inputs_as()?;
        input.validate()?;

        let solver = ThresholdSolver;
        let (output, report) = solver.solve_triage(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));
        let confidence = calculate_confidence(&output);

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
        let output: AnomalyTriageOutput = plan.plan_as()?;
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

fn calculate_confidence(output: &AnomalyTriageOutput) -> f64 {
    if output.triaged.is_empty() {
        return 0.5; // Empty is valid but low confidence
    }

    let mut confidence: f64 = 0.6;

    // Higher confidence if all anomalies have recommendations
    let all_have_recommendations = output
        .triaged
        .iter()
        .all(|t| !t.recommended_actions.is_empty());
    if all_have_recommendations {
        confidence += 0.2;
    }

    // Higher confidence if critical items are escalated
    let critical_escalated = output
        .triaged
        .iter()
        .filter(|t| t.severity == "critical")
        .all(|t| t.escalate);
    if critical_escalated {
        confidence += 0.2;
    }

    confidence.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> AnomalyTriageInput {
        AnomalyTriageInput {
            anomalies: vec![
                Anomaly {
                    id: "a1".to_string(),
                    timestamp: 1700000000,
                    source: "api-server".to_string(),
                    z_score: 5.2,
                    features: serde_json::json!({}),
                },
                Anomaly {
                    id: "a2".to_string(),
                    timestamp: 1700000010,
                    source: "database".to_string(),
                    z_score: 2.5,
                    features: serde_json::json!({}),
                },
            ],
            thresholds: SeverityThresholds::default(),
            escalation_policies: vec![EscalationPolicy {
                severity_level: "critical".to_string(),
                auto_escalate: true,
                notify_channels: vec!["pagerduty".to_string()],
                response_sla_minutes: 15,
            }],
        }
    }

    #[test]
    fn test_pack_name() {
        let pack = AnomalyTriagePack;
        assert_eq!(pack.name(), "anomaly-triage");
        assert_eq!(pack.version(), "1.0.0");
    }

    #[test]
    fn test_validate_inputs() {
        let pack = AnomalyTriagePack;
        let input = create_test_input();
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_ok());
    }

    #[test]
    fn test_solve_basic() {
        let pack = AnomalyTriagePack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-001", "test-tenant")
            .objective(ObjectiveSpec::minimize("risk"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        assert!(result.is_feasible());

        let output: AnomalyTriageOutput = result.plan.plan_as().unwrap();
        assert_eq!(output.triaged.len(), 2);
    }

    #[test]
    fn test_check_invariants() {
        let pack = AnomalyTriagePack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-002", "test-tenant")
            .objective(ObjectiveSpec::minimize("risk"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();

        let all_pass = invariants.iter().all(|r| r.passed);
        assert!(all_pass);
    }

    #[test]
    fn test_gate_promotes() {
        let pack = AnomalyTriagePack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-003", "test-tenant")
            .objective(ObjectiveSpec::minimize("risk"))
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
        let pack = AnomalyTriagePack;
        let input = create_test_input();

        let spec1 = ProblemSpec::builder("test-a", "tenant")
            .objective(ObjectiveSpec::minimize("risk"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let spec2 = ProblemSpec::builder("test-b", "tenant")
            .objective(ObjectiveSpec::minimize("risk"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let result1 = pack.solve(&spec1).unwrap();
        let result2 = pack.solve(&spec2).unwrap();

        let output1: AnomalyTriageOutput = result1.plan.plan_as().unwrap();
        let output2: AnomalyTriageOutput = result2.plan.plan_as().unwrap();

        for (a, b) in output1.triaged.iter().zip(output2.triaged.iter()) {
            assert_eq!(a.anomaly_id, b.anomaly_id);
            assert_eq!(a.priority, b.priority);
        }
    }
}
