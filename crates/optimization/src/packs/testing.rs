//! Test harness for pack scenarios

use converge_pack::Pack;
use converge_pack::gate::{GateDecision, GateResult, ProblemSpec};

/// Test scenario for a pack
#[derive(Debug, Clone)]
pub struct TestScenario {
    /// Scenario name
    pub name: String,
    /// Description of what's being tested
    pub description: String,
    /// Problem specification for this scenario
    pub spec: ProblemSpec,
    /// Expected outcome
    pub expected: ExpectedOutcome,
}

impl TestScenario {
    /// Create a new test scenario
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        spec: ProblemSpec,
        expected: ExpectedOutcome,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            spec,
            expected,
        }
    }

    /// Create a feasibility test (expects solution)
    pub fn feasible(
        name: impl Into<String>,
        description: impl Into<String>,
        spec: ProblemSpec,
        min_confidence: f64,
    ) -> Self {
        Self::new(
            name,
            description,
            spec,
            ExpectedOutcome::Feasible {
                min_confidence,
                required_invariants: Vec::new(),
            },
        )
    }

    /// Create an infeasibility test (expects no solution)
    pub fn infeasible(
        name: impl Into<String>,
        description: impl Into<String>,
        spec: ProblemSpec,
        expected_violations: Vec<String>,
    ) -> Self {
        Self::new(
            name,
            description,
            spec,
            ExpectedOutcome::Infeasible {
                expected_violations,
            },
        )
    }
}

/// Expected outcome of a test scenario
#[derive(Debug, Clone)]
pub enum ExpectedOutcome {
    /// Should find a feasible solution
    Feasible {
        /// Minimum confidence score expected
        min_confidence: f64,
        /// Invariants that must pass
        required_invariants: Vec<String>,
    },
    /// Should report infeasible
    Infeasible {
        /// Expected constraint violations
        expected_violations: Vec<String>,
    },
    /// Should match specific gate decision
    GateDecision {
        /// Expected decision
        decision: GateDecision,
    },
    /// Should produce deterministic output
    Deterministic {
        /// Expected output hash or value
        expected_output: serde_json::Value,
    },
}

/// Result of running a test scenario
#[derive(Debug)]
pub struct ScenarioResult {
    /// Scenario name
    pub name: String,
    /// Whether the test passed
    pub passed: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Actual confidence (if applicable)
    pub actual_confidence: Option<f64>,
    /// Actual gate decision (if applicable)
    pub actual_decision: Option<GateDecision>,
    /// Duration in milliseconds
    pub duration_ms: f64,
}

impl ScenarioResult {
    /// Create a passing result
    pub fn pass(name: impl Into<String>, duration_ms: f64) -> Self {
        Self {
            name: name.into(),
            passed: true,
            error: None,
            actual_confidence: None,
            actual_decision: None,
            duration_ms,
        }
    }

    /// Create a failing result
    pub fn fail(name: impl Into<String>, error: impl Into<String>, duration_ms: f64) -> Self {
        Self {
            name: name.into(),
            passed: false,
            error: Some(error.into()),
            actual_confidence: None,
            actual_decision: None,
            duration_ms,
        }
    }

    /// Add actual confidence to result
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.actual_confidence = Some(confidence);
        self
    }

    /// Add actual decision to result
    pub fn with_decision(mut self, decision: GateDecision) -> Self {
        self.actual_decision = Some(decision);
        self
    }
}

/// Run a single test scenario against a pack
pub fn run_scenario(pack: &dyn Pack, scenario: &TestScenario) -> ScenarioResult {
    let start = std::time::Instant::now();

    // Solve the problem
    let solve_result = match pack.solve(&scenario.spec) {
        Ok(result) => result,
        Err(e) => {
            // Check if we expected infeasibility
            if let ExpectedOutcome::Infeasible { .. } = &scenario.expected {
                return ScenarioResult::pass(
                    &scenario.name,
                    start.elapsed().as_secs_f64() * 1000.0,
                );
            }
            return ScenarioResult::fail(
                &scenario.name,
                format!("Solve failed: {}", e),
                start.elapsed().as_secs_f64() * 1000.0,
            );
        }
    };

    // Check invariants
    let invariant_results = match pack.check_invariants(&solve_result.plan) {
        Ok(results) => results,
        Err(e) => {
            return ScenarioResult::fail(
                &scenario.name,
                format!("Invariant check failed: {}", e),
                start.elapsed().as_secs_f64() * 1000.0,
            );
        }
    };

    // Evaluate gate
    let gate = pack.evaluate_gate(&solve_result.plan, &invariant_results);
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    // Check against expected outcome
    match &scenario.expected {
        ExpectedOutcome::Feasible {
            min_confidence,
            required_invariants,
        } => {
            if solve_result.plan.confidence() < *min_confidence {
                return ScenarioResult::fail(
                    &scenario.name,
                    format!(
                        "Confidence too low: {} < {}",
                        solve_result.plan.confidence(),
                        min_confidence
                    ),
                    duration_ms,
                )
                .with_confidence(solve_result.plan.confidence());
            }

            // Check required invariants passed
            for required in required_invariants {
                let result = invariant_results.iter().find(|r| &r.invariant == required);
                match result {
                    Some(r) if !r.passed => {
                        return ScenarioResult::fail(
                            &scenario.name,
                            format!("Required invariant '{}' failed", required),
                            duration_ms,
                        );
                    }
                    None => {
                        return ScenarioResult::fail(
                            &scenario.name,
                            format!("Required invariant '{}' not found", required),
                            duration_ms,
                        );
                    }
                    _ => {}
                }
            }

            ScenarioResult::pass(&scenario.name, duration_ms)
                .with_confidence(solve_result.plan.confidence())
                .with_decision(gate.decision)
        }

        ExpectedOutcome::Infeasible {
            expected_violations,
        } => {
            // Should not have found a feasible solution
            if solve_result.is_feasible() && gate.is_promoted() {
                return ScenarioResult::fail(
                    &scenario.name,
                    "Expected infeasible but found solution",
                    duration_ms,
                );
            }

            // Check for expected violations
            for expected in expected_violations {
                let has_violation = invariant_results
                    .iter()
                    .any(|r| !r.passed && r.invariant == *expected);
                if !has_violation {
                    return ScenarioResult::fail(
                        &scenario.name,
                        format!("Expected violation '{}' not found", expected),
                        duration_ms,
                    );
                }
            }

            ScenarioResult::pass(&scenario.name, duration_ms).with_decision(gate.decision)
        }

        ExpectedOutcome::GateDecision { decision } => {
            if gate.decision != *decision {
                return ScenarioResult::fail(
                    &scenario.name,
                    format!("Expected {:?} but got {:?}", decision, gate.decision),
                    duration_ms,
                )
                .with_decision(gate.decision);
            }
            ScenarioResult::pass(&scenario.name, duration_ms).with_decision(gate.decision)
        }

        ExpectedOutcome::Deterministic { expected_output } => {
            if solve_result.plan.plan != *expected_output {
                return ScenarioResult::fail(
                    &scenario.name,
                    format!(
                        "Output mismatch: expected {:?}, got {:?}",
                        expected_output, solve_result.plan.plan
                    ),
                    duration_ms,
                );
            }
            ScenarioResult::pass(&scenario.name, duration_ms)
        }
    }
}

/// Run all scenarios for a pack
pub fn run_all_scenarios(pack: &dyn Pack, scenarios: &[TestScenario]) -> Vec<ScenarioResult> {
    scenarios.iter().map(|s| run_scenario(pack, s)).collect()
}

/// Summary of scenario results
#[derive(Debug)]
pub struct ScenarioSummary {
    /// Total scenarios run
    pub total: usize,
    /// Scenarios passed
    pub passed: usize,
    /// Scenarios failed
    pub failed: usize,
    /// Total duration in milliseconds
    pub total_duration_ms: f64,
}

impl ScenarioSummary {
    /// Create from results
    pub fn from_results(results: &[ScenarioResult]) -> Self {
        let passed = results.iter().filter(|r| r.passed).count();
        let total_duration_ms: f64 = results.iter().map(|r| r.duration_ms).sum();

        Self {
            total: results.len(),
            passed,
            failed: results.len() - passed,
            total_duration_ms,
        }
    }

    /// Check if all scenarios passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0
    }
}

/// Helper to create a minimal problem spec for testing
pub fn test_problem_spec(_pack_name: &str, inputs: serde_json::Value) -> GateResult<ProblemSpec> {
    use converge_pack::gate::ObjectiveSpec;

    ProblemSpec::builder(format!("test-{}", test_id()), "test-tenant")
        .objective(ObjectiveSpec::maximize("score"))
        .inputs_raw(inputs)
        .seed(42) // Fixed seed for determinism
        .build()
}

fn test_id() -> String {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{id:032x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scenario_result() {
        let pass = ScenarioResult::pass("test", 10.0);
        assert!(pass.passed);

        let fail = ScenarioResult::fail("test", "error", 10.0);
        assert!(!fail.passed);
        assert_eq!(fail.error, Some("error".to_string()));
    }

    #[test]
    fn test_scenario_summary() {
        let results = vec![
            ScenarioResult::pass("test1", 10.0),
            ScenarioResult::pass("test2", 20.0),
            ScenarioResult::fail("test3", "error", 5.0),
        ];

        let summary = ScenarioSummary::from_results(&results);
        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 1);
        assert!(!summary.all_passed());
    }

    #[test]
    fn test_test_problem_spec() {
        let spec = test_problem_spec("test-pack", serde_json::json!({"key": "value"})).unwrap();

        assert!(spec.problem_id.starts_with("test-"));
        assert_eq!(spec.tenant_scope, "test-tenant");
        assert_eq!(spec.seed(), 42);
    }
}
