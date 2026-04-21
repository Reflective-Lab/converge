//! Vendor Shortlist Pack
//!
//! JTBD: "Shortlist vendors under risk/compliance constraints with explainability."
//!
//! ## Problem
//!
//! Given:
//! - Vendor profiles and scores
//! - Risk and compliance requirements
//! - Diversification rules
//!
//! Find:
//! - Shortlist of qualifying vendors ranked by composite score
//!
//! ## Solver
//!
//! Uses score-based ranking:
//! 1. Filter vendors by compliance and certifications
//! 2. Filter by minimum score and maximum risk
//! 3. Calculate composite score for each vendor
//! 4. Rank by composite score, take top N

mod invariants;
mod solver;
mod types;

pub use invariants::*;
pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

/// Vendor Shortlist Pack
pub struct VendorShortlistPack;

impl Pack for VendorShortlistPack {
    fn name(&self) -> &'static str {
        "vendor-shortlist"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: VendorShortlistInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_pack::GateError::invalid_input(format!("Invalid input: {}", e))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: VendorShortlistInput = spec.inputs_as()?;
        input.validate()?;

        let solver = ScoreRankingSolver;
        let (output, report) = solver.solve_shortlist(&input, spec)?;

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
        let output: VendorShortlistOutput = plan.plan_as()?;
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

fn calculate_confidence(output: &VendorShortlistOutput, input: &VendorShortlistInput) -> f64 {
    if output.shortlist.is_empty() {
        return 0.0;
    }

    let mut confidence: f64 = 0.5;

    // Higher confidence if we found multiple vendors
    if output.shortlist.len() >= 2 {
        confidence += 0.2;
    }

    // Higher confidence if shortlist is at capacity
    if output.shortlist.len() == input.requirements.max_vendors {
        confidence += 0.1;
    }

    // Higher confidence if average score is good
    if output.stats.average_score >= 80.0 {
        confidence += 0.2;
    } else if output.stats.average_score >= 70.0 {
        confidence += 0.1;
    }

    confidence.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> VendorShortlistInput {
        VendorShortlistInput {
            vendors: vec![
                Vendor {
                    id: "v1".to_string(),
                    name: "Acme Corp".to_string(),
                    score: 85.0,
                    risk_score: 20.0,
                    compliance_status: "compliant".to_string(),
                    certifications: vec!["ISO9001".to_string()],
                },
                Vendor {
                    id: "v2".to_string(),
                    name: "BetaCo".to_string(),
                    score: 75.0,
                    risk_score: 15.0,
                    compliance_status: "compliant".to_string(),
                    certifications: vec!["ISO9001".to_string()],
                },
            ],
            requirements: ShortlistRequirements {
                max_vendors: 3,
                min_score: 50.0,
                max_risk_score: 50.0,
                required_certifications: vec![],
            },
        }
    }

    #[test]
    fn test_pack_name() {
        let pack = VendorShortlistPack;
        assert_eq!(pack.name(), "vendor-shortlist");
        assert_eq!(pack.version(), "1.0.0");
    }

    #[test]
    fn test_validate_inputs() {
        let pack = VendorShortlistPack;
        let input = create_test_input();
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_ok());
    }

    #[test]
    fn test_solve_basic() {
        let pack = VendorShortlistPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-001", "test-tenant")
            .objective(ObjectiveSpec::maximize("score"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        assert!(result.is_feasible());

        let output: VendorShortlistOutput = result.plan.plan_as().unwrap();
        assert_eq!(output.shortlist.len(), 2);
    }

    #[test]
    fn test_check_invariants() {
        let pack = VendorShortlistPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-002", "test-tenant")
            .objective(ObjectiveSpec::maximize("score"))
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
        let pack = VendorShortlistPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-003", "test-tenant")
            .objective(ObjectiveSpec::maximize("score"))
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
        let pack = VendorShortlistPack;
        let input = create_test_input();

        let spec1 = ProblemSpec::builder("test-a", "tenant")
            .objective(ObjectiveSpec::maximize("score"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let spec2 = ProblemSpec::builder("test-b", "tenant")
            .objective(ObjectiveSpec::maximize("score"))
            .inputs(&input)
            .unwrap()
            .seed(99999)
            .build()
            .unwrap();

        let result1 = pack.solve(&spec1).unwrap();
        let result2 = pack.solve(&spec2).unwrap();

        let output1: VendorShortlistOutput = result1.plan.plan_as().unwrap();
        let output2: VendorShortlistOutput = result2.plan.plan_as().unwrap();

        assert_eq!(output1.shortlist.len(), output2.shortlist.len());
        for (a, b) in output1.shortlist.iter().zip(output2.shortlist.iter()) {
            assert_eq!(a.vendor_id, b.vendor_id);
        }
    }
}
