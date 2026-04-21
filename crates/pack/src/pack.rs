//! Pack traits and core abstractions

use crate::gate::{
    GateError, GateResult, ProblemSpec, PromotionGate, ProposedPlan, SolverReport, Violation,
};
use serde::{Serialize, de::DeserializeOwned};

/// A domain pack for the solver gate
///
/// Packs define the contract for a specific optimization domain,
/// including input validation, solving, invariant checking, and
/// gate evaluation.
pub trait Pack: Send + Sync {
    /// Pack name (e.g., "meeting-scheduler")
    fn name(&self) -> &'static str;

    /// Pack version
    fn version(&self) -> &'static str;

    /// Validate and deserialize the input payload
    fn validate_inputs(&self, inputs: &serde_json::Value) -> GateResult<()>;

    /// Get invariant definitions for this pack
    fn invariants(&self) -> &[InvariantDef];

    /// Solve the problem and return a proposed plan
    fn solve(&self, spec: &ProblemSpec) -> GateResult<PackSolveResult>;

    /// Check invariants against a proposed plan
    fn check_invariants(&self, plan: &ProposedPlan) -> GateResult<Vec<InvariantResult>>;

    /// Evaluate promotion gate based on plan and invariant results
    fn evaluate_gate(
        &self,
        plan: &ProposedPlan,
        invariant_results: &[InvariantResult],
    ) -> PromotionGate;
}

/// A solver within a pack
///
/// Each pack can have multiple solvers (e.g., greedy, exact, heuristic).
pub trait PackSolver: Send + Sync {
    /// Solver identifier (e.g., "greedy-v1")
    fn id(&self) -> &'static str;

    /// Solve and produce plan payload + report
    fn solve(&self, spec: &ProblemSpec) -> GateResult<(serde_json::Value, SolverReport)>;

    /// Whether this solver guarantees optimality
    fn is_exact(&self) -> bool;
}

/// Result of pack solving
#[derive(Debug)]
pub struct PackSolveResult {
    /// The proposed plan
    pub plan: ProposedPlan,
    /// Solver reports (may have tried multiple solvers)
    pub reports: Vec<SolverReport>,
}

impl PackSolveResult {
    /// Create a new solve result
    pub fn new(plan: ProposedPlan, report: SolverReport) -> Self {
        Self {
            plan,
            reports: vec![report],
        }
    }

    /// Create with multiple reports
    pub fn with_reports(plan: ProposedPlan, reports: Vec<SolverReport>) -> Self {
        Self { plan, reports }
    }

    /// Get the primary (first) report
    pub fn primary_report(&self) -> Option<&SolverReport> {
        self.reports.first()
    }

    /// Check if any solver found a feasible solution
    pub fn is_feasible(&self) -> bool {
        self.reports.iter().any(|r| r.feasible)
    }
}

/// Definition of an invariant
#[derive(Debug, Clone)]
pub struct InvariantDef {
    /// Invariant name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Whether this is critical (blocks promotion if violated)
    pub critical: bool,
}

impl InvariantDef {
    /// Create a critical invariant (blocks promotion if violated)
    pub fn critical(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            critical: true,
        }
    }

    /// Create a non-critical (advisory) invariant
    pub fn advisory(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            critical: false,
        }
    }
}

/// Result of checking an invariant
#[derive(Debug, Clone)]
pub struct InvariantResult {
    /// Which invariant was checked
    pub invariant: String,
    /// Whether it passed
    pub passed: bool,
    /// Violation details if failed
    pub violation: Option<Violation>,
}

impl InvariantResult {
    /// Create a passing result
    pub fn pass(invariant: impl Into<String>) -> Self {
        Self {
            invariant: invariant.into(),
            passed: true,
            violation: None,
        }
    }

    /// Create a failing result
    pub fn fail(invariant: impl Into<String>, violation: Violation) -> Self {
        Self {
            invariant: invariant.into(),
            passed: false,
            violation: Some(violation),
        }
    }

    /// Check if this is a critical failure
    pub fn is_critical_failure(&self, invariants: &[InvariantDef]) -> bool {
        if self.passed {
            return false;
        }
        invariants
            .iter()
            .find(|i| i.name == self.invariant)
            .map(|i| i.critical)
            .unwrap_or(false)
    }
}

/// Schema trait for typed pack inputs/outputs
///
/// Implement this for your pack's input and output types to get
/// automatic validation and JSON conversion.
pub trait PackSchema: Sized + Serialize + DeserializeOwned {
    /// Validate the schema
    fn validate(&self) -> GateResult<()>;

    /// Convert to JSON value
    fn to_json(&self) -> GateResult<serde_json::Value> {
        serde_json::to_value(self).map_err(|e| GateError::invalid_input(e.to_string()))
    }

    /// Parse from JSON value
    fn from_json(value: &serde_json::Value) -> GateResult<Self> {
        serde_json::from_value(value.clone()).map_err(|e| GateError::invalid_input(e.to_string()))
    }
}

/// Helper to evaluate gate based on invariant results
pub fn default_gate_evaluation(
    invariant_results: &[InvariantResult],
    invariant_defs: &[InvariantDef],
) -> PromotionGate {
    // Check for critical failures
    let critical_failures: Vec<_> = invariant_results
        .iter()
        .filter(|r| r.is_critical_failure(invariant_defs))
        .collect();

    if !critical_failures.is_empty() {
        let failed_names: Vec<_> = critical_failures
            .iter()
            .map(|r| r.invariant.as_str())
            .collect();
        return PromotionGate::reject(format!(
            "Critical invariant(s) violated: {}",
            failed_names.join(", ")
        ));
    }

    // Check for any failures (advisory)
    let advisory_failures: Vec<_> = invariant_results.iter().filter(|r| !r.passed).collect();

    if !advisory_failures.is_empty() {
        let failed_names: Vec<_> = advisory_failures
            .iter()
            .map(|r| r.invariant.as_str())
            .collect();
        return PromotionGate::requires_review(
            failed_names.iter().map(|s| s.to_string()).collect(),
            format!(
                "Advisory invariant(s) violated: {}",
                failed_names.join(", ")
            ),
        );
    }

    // All passed
    PromotionGate::auto_promote("All invariants passed")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invariant_def() {
        let critical = InvariantDef::critical("cap", "capacity check");
        assert!(critical.critical);

        let advisory = InvariantDef::advisory("pref", "preference check");
        assert!(!advisory.critical);
    }

    #[test]
    fn test_invariant_result() {
        let pass = InvariantResult::pass("test");
        assert!(pass.passed);
        assert!(pass.violation.is_none());

        let fail = InvariantResult::fail("test", Violation::new("test", 1.0, "failed"));
        assert!(!fail.passed);
        assert!(fail.violation.is_some());
    }

    #[test]
    fn test_critical_failure_detection() {
        let invariants = vec![
            InvariantDef::critical("critical_one", "must pass"),
            InvariantDef::advisory("advisory_one", "nice to have"),
        ];

        let critical_fail = InvariantResult::fail(
            "critical_one",
            Violation::new("critical_one", 1.0, "failed"),
        );
        assert!(critical_fail.is_critical_failure(&invariants));

        let advisory_fail = InvariantResult::fail(
            "advisory_one",
            Violation::new("advisory_one", 0.5, "failed"),
        );
        assert!(!advisory_fail.is_critical_failure(&invariants));
    }

    #[test]
    fn test_default_gate_evaluation() {
        let invariants = vec![
            InvariantDef::critical("must_pass", "critical"),
            InvariantDef::advisory("nice_to_have", "advisory"),
        ];

        // All pass
        let results = vec![
            InvariantResult::pass("must_pass"),
            InvariantResult::pass("nice_to_have"),
        ];
        let gate = default_gate_evaluation(&results, &invariants);
        assert!(gate.is_promoted());

        // Critical failure
        let results = vec![
            InvariantResult::fail("must_pass", Violation::new("must_pass", 1.0, "failed")),
            InvariantResult::pass("nice_to_have"),
        ];
        let gate = default_gate_evaluation(&results, &invariants);
        assert!(gate.is_rejected());

        // Advisory failure only
        let results = vec![
            InvariantResult::pass("must_pass"),
            InvariantResult::fail(
                "nice_to_have",
                Violation::new("nice_to_have", 0.5, "failed"),
            ),
        ];
        let gate = default_gate_evaluation(&results, &invariants);
        assert!(gate.requires_escalation());
    }
}
