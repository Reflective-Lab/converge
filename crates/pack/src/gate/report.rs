//! Solver execution report

use serde::{Deserialize, Serialize};

use super::{ReplayEnvelope, Violation};

/// Detailed solver execution report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverReport {
    /// Solver identifier
    pub solver_id: String,
    /// Whether a feasible solution was found
    pub feasible: bool,
    /// Objective value (if solution found)
    pub objective_value: Option<f64>,
    /// Constraint violations
    pub constraint_violations: Vec<Violation>,
    /// Diagnostic information
    pub diagnostics: Vec<Diagnostic>,
    /// Why the solver stopped
    pub stop_reason: StopReason,
    /// Replay information
    pub replay: ReplayEnvelope,
}

impl SolverReport {
    /// Create a feasible report with optimal solution
    pub fn optimal(
        solver_id: impl Into<String>,
        objective_value: f64,
        replay: ReplayEnvelope,
    ) -> Self {
        Self {
            solver_id: solver_id.into(),
            feasible: true,
            objective_value: Some(objective_value),
            constraint_violations: Vec::new(),
            diagnostics: Vec::new(),
            stop_reason: StopReason::Optimal,
            replay,
        }
    }

    /// Create a feasible report (may not be optimal)
    pub fn feasible(
        solver_id: impl Into<String>,
        objective_value: f64,
        stop_reason: StopReason,
        replay: ReplayEnvelope,
    ) -> Self {
        Self {
            solver_id: solver_id.into(),
            feasible: true,
            objective_value: Some(objective_value),
            constraint_violations: Vec::new(),
            diagnostics: Vec::new(),
            stop_reason,
            replay,
        }
    }

    /// Create an infeasible report
    pub fn infeasible(
        solver_id: impl Into<String>,
        violations: Vec<Violation>,
        stop_reason: StopReason,
        replay: ReplayEnvelope,
    ) -> Self {
        Self {
            solver_id: solver_id.into(),
            feasible: false,
            objective_value: None,
            constraint_violations: violations,
            diagnostics: Vec::new(),
            stop_reason,
            replay,
        }
    }

    /// Add a diagnostic
    pub fn with_diagnostic(mut self, diag: Diagnostic) -> Self {
        self.diagnostics.push(diag);
        self
    }

    /// Add multiple diagnostics
    pub fn with_diagnostics(mut self, diags: impl IntoIterator<Item = Diagnostic>) -> Self {
        self.diagnostics.extend(diags);
        self
    }

    /// Add a constraint violation
    pub fn with_violation(mut self, violation: Violation) -> Self {
        self.constraint_violations.push(violation);
        self
    }

    /// Check if solution is proven optimal
    pub fn is_optimal(&self) -> bool {
        self.feasible && self.stop_reason == StopReason::Optimal
    }

    /// Check if solver hit a budget limit
    pub fn hit_budget(&self) -> bool {
        matches!(
            self.stop_reason,
            StopReason::TimeBudgetExhausted
                | StopReason::IterationBudgetExhausted
                | StopReason::CandidateCapReached
        )
    }
}

/// Why the solver stopped
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Proven optimal solution found
    Optimal,
    /// Feasible solution found (may not be optimal)
    Feasible,
    /// Problem proven infeasible
    Infeasible,
    /// No feasible solution found (but not proven infeasible)
    NoFeasible,
    /// Time budget exhausted
    TimeBudgetExhausted,
    /// Iteration budget exhausted
    IterationBudgetExhausted,
    /// Candidate cap reached
    CandidateCapReached,
    /// User requested stop
    UserRequested,
    /// Solver encountered error
    SolverError,
    /// Insufficient data to solve
    DataInsufficient,
    /// Human decision required (multiple close options)
    HumanDecisionRequired,
}

impl StopReason {
    /// Check if this represents a successful solve
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Optimal | Self::Feasible)
    }

    /// Check if this represents a failure to find a solution
    pub fn is_failure(&self) -> bool {
        matches!(
            self,
            Self::Infeasible | Self::NoFeasible | Self::SolverError | Self::DataInsufficient
        )
    }

    /// Check if this represents a budget exhaustion
    pub fn is_budget_exhausted(&self) -> bool {
        matches!(
            self,
            Self::TimeBudgetExhausted | Self::IterationBudgetExhausted | Self::CandidateCapReached
        )
    }
}

/// Diagnostic information from solver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Diagnostic type
    pub kind: DiagnosticKind,
    /// Human-readable message
    pub message: String,
    /// Additional data
    pub data: serde_json::Value,
}

impl Diagnostic {
    /// Create a generic diagnostic
    pub fn new(kind: DiagnosticKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            data: serde_json::Value::Null,
        }
    }

    /// Create a generic diagnostic with data
    pub fn with_data(
        kind: DiagnosticKind,
        message: impl Into<String>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            data,
        }
    }

    /// Create a scoring breakdown diagnostic
    pub fn scoring_breakdown(breakdown: serde_json::Value) -> Self {
        Self {
            kind: DiagnosticKind::ScoringBreakdown,
            message: "Objective scoring breakdown".to_string(),
            data: breakdown,
        }
    }

    /// Create a tie-break rationale diagnostic
    pub fn tie_break_rationale(message: impl Into<String>, candidates: Vec<String>) -> Self {
        Self {
            kind: DiagnosticKind::TieBreakRationale,
            message: message.into(),
            data: serde_json::json!({ "candidates": candidates }),
        }
    }

    /// Create a pruning diagnostic
    pub fn pruning(message: impl Into<String>, pruned_count: usize) -> Self {
        Self {
            kind: DiagnosticKind::Pruning,
            message: message.into(),
            data: serde_json::json!({ "pruned_count": pruned_count }),
        }
    }

    /// Create a performance diagnostic
    pub fn performance(phase: impl Into<String>, elapsed_ms: f64, iterations: usize) -> Self {
        Self {
            kind: DiagnosticKind::Performance,
            message: format!(
                "{}: {:.2}ms, {} iterations",
                phase.into(),
                elapsed_ms,
                iterations
            ),
            data: serde_json::json!({
                "elapsed_ms": elapsed_ms,
                "iterations": iterations
            }),
        }
    }

    /// Create a constraint analysis diagnostic
    pub fn constraint_analysis(constraint: impl Into<String>, slack: f64, binding: bool) -> Self {
        Self {
            kind: DiagnosticKind::ConstraintAnalysis,
            message: format!(
                "Constraint '{}': slack={:.2}, {}",
                constraint.into(),
                slack,
                if binding { "binding" } else { "non-binding" }
            ),
            data: serde_json::json!({
                "slack": slack,
                "binding": binding
            }),
        }
    }
}

/// Types of diagnostics
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticKind {
    /// Breakdown of objective scoring
    ScoringBreakdown,
    /// Why a particular tie-break decision was made
    TieBreakRationale,
    /// Information about pruned branches/candidates
    Pruning,
    /// Performance metrics
    Performance,
    /// Constraint analysis
    ConstraintAnalysis,
    /// Why a candidate was rejected
    CandidateRejection,
    /// Custom diagnostic
    Custom,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimal_report() {
        let replay = ReplayEnvelope::minimal(42);
        let report = SolverReport::optimal("hungarian-v1", 100.0, replay);

        assert!(report.feasible);
        assert!(report.is_optimal());
        assert!(!report.hit_budget());
        assert_eq!(report.objective_value, Some(100.0));
    }

    #[test]
    fn test_infeasible_report() {
        let replay = ReplayEnvelope::minimal(42);
        let violation = Violation::new("capacity", 1.0, "exceeded limit");
        let report =
            SolverReport::infeasible("solver-v1", vec![violation], StopReason::Infeasible, replay);

        assert!(!report.feasible);
        assert!(!report.is_optimal());
        assert_eq!(report.constraint_violations.len(), 1);
    }

    #[test]
    fn test_budget_exhausted() {
        let replay = ReplayEnvelope::minimal(42);
        let report =
            SolverReport::feasible("solver-v1", 150.0, StopReason::TimeBudgetExhausted, replay);

        assert!(report.feasible);
        assert!(!report.is_optimal());
        assert!(report.hit_budget());
    }

    #[test]
    fn test_stop_reason_classification() {
        assert!(StopReason::Optimal.is_success());
        assert!(StopReason::Feasible.is_success());
        assert!(!StopReason::Infeasible.is_success());

        assert!(StopReason::Infeasible.is_failure());
        assert!(StopReason::NoFeasible.is_failure());
        assert!(!StopReason::Optimal.is_failure());

        assert!(StopReason::TimeBudgetExhausted.is_budget_exhausted());
        assert!(!StopReason::Optimal.is_budget_exhausted());
    }

    #[test]
    fn test_diagnostics() {
        let diag = Diagnostic::scoring_breakdown(serde_json::json!({"cost": 10, "penalty": 5}));
        assert_eq!(diag.kind, DiagnosticKind::ScoringBreakdown);

        let diag2 =
            Diagnostic::tie_break_rationale("chose by id", vec!["a".to_string(), "b".to_string()]);
        assert_eq!(diag2.kind, DiagnosticKind::TieBreakRationale);
    }

    #[test]
    fn test_report_with_diagnostics() {
        let replay = ReplayEnvelope::minimal(42);
        let report = SolverReport::optimal("solver", 50.0, replay)
            .with_diagnostic(Diagnostic::performance("phase1", 10.5, 100))
            .with_diagnostic(Diagnostic::pruning("removed infeasible", 25));

        assert_eq!(report.diagnostics.len(), 2);
    }
}
