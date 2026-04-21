//! Core gate types: ProblemSpec and ProposedPlan

use serde::{Deserialize, Serialize};

use super::error::{GateError, GateResult};
use super::{
    ConstraintSpec, DeterminismSpec, KernelTraceLink, ObjectiveSpec, ProvenanceEnvelope,
    SolveBudgets,
};

/// Complete problem specification for the solver gate
///
/// This is the contract surface for optimization problems - pure, serializable,
/// and deterministic input that can be traced, replayed, and audited.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProblemSpec {
    /// Stable identifier for trace joins and audit
    pub problem_id: String,
    /// Tenant scope for multi-tenant flows
    pub tenant_scope: String,
    /// Objective to optimize (minimize/maximize)
    pub objective: ObjectiveSpec,
    /// Constraints that must be satisfied
    pub constraints: Vec<ConstraintSpec>,
    /// Typed payload per pack (schema-validated by the pack)
    pub inputs: serde_json::Value,
    /// Resource budgets for solving
    pub budgets: SolveBudgets,
    /// Determinism requirements
    pub determinism: DeterminismSpec,
    /// Provenance for audit trail
    pub provenance: ProvenanceEnvelope,
}

impl ProblemSpec {
    /// Create a new problem spec builder
    pub fn builder(
        problem_id: impl Into<String>,
        tenant_scope: impl Into<String>,
    ) -> ProblemSpecBuilder {
        ProblemSpecBuilder::new(problem_id.into(), tenant_scope.into())
    }

    /// Validate the problem spec (schema validation happens in pack)
    pub fn validate(&self) -> GateResult<()> {
        if self.problem_id.is_empty() {
            return Err(GateError::invalid_input("problem_id is required"));
        }
        if self.tenant_scope.is_empty() {
            return Err(GateError::invalid_input("tenant_scope is required"));
        }
        self.budgets.validate()?;
        Ok(())
    }

    /// Get the random seed from determinism spec
    pub fn seed(&self) -> u64 {
        self.determinism.seed
    }

    /// Generate a sub-seed for a specific phase
    pub fn sub_seed(&self, phase: &str) -> u64 {
        self.determinism.sub_seed(phase)
    }

    /// Parse inputs as a specific type
    pub fn inputs_as<T: for<'de> Deserialize<'de>>(&self) -> GateResult<T> {
        serde_json::from_value(self.inputs.clone())
            .map_err(|e| GateError::invalid_input(format!("failed to parse inputs: {}", e)))
    }
}

/// Builder for ProblemSpec
pub struct ProblemSpecBuilder {
    problem_id: String,
    tenant_scope: String,
    objective: Option<ObjectiveSpec>,
    constraints: Vec<ConstraintSpec>,
    inputs: serde_json::Value,
    budgets: SolveBudgets,
    determinism: DeterminismSpec,
    provenance: ProvenanceEnvelope,
}

impl ProblemSpecBuilder {
    /// Create a new builder with required fields
    pub fn new(problem_id: String, tenant_scope: String) -> Self {
        Self {
            problem_id,
            tenant_scope,
            objective: None,
            constraints: Vec::new(),
            inputs: serde_json::Value::Null,
            budgets: SolveBudgets::default(),
            determinism: DeterminismSpec::default(),
            provenance: ProvenanceEnvelope::default(),
        }
    }

    /// Set the objective
    pub fn objective(mut self, obj: ObjectiveSpec) -> Self {
        self.objective = Some(obj);
        self
    }

    /// Add a constraint
    pub fn constraint(mut self, c: ConstraintSpec) -> Self {
        self.constraints.push(c);
        self
    }

    /// Add multiple constraints
    pub fn constraints(mut self, cs: impl IntoIterator<Item = ConstraintSpec>) -> Self {
        self.constraints.extend(cs);
        self
    }

    /// Set inputs from a serializable type
    pub fn inputs<T: Serialize>(mut self, inputs: &T) -> GateResult<Self> {
        self.inputs =
            serde_json::to_value(inputs).map_err(|e| GateError::invalid_input(e.to_string()))?;
        Ok(self)
    }

    /// Set inputs from raw JSON value
    pub fn inputs_raw(mut self, inputs: serde_json::Value) -> Self {
        self.inputs = inputs;
        self
    }

    /// Set solve budgets
    pub fn budgets(mut self, budgets: SolveBudgets) -> Self {
        self.budgets = budgets;
        self
    }

    /// Set determinism spec
    pub fn determinism(mut self, det: DeterminismSpec) -> Self {
        self.determinism = det;
        self
    }

    /// Set random seed (convenience for common case)
    pub fn seed(mut self, seed: u64) -> Self {
        self.determinism.seed = seed;
        self
    }

    /// Set provenance
    pub fn provenance(mut self, prov: ProvenanceEnvelope) -> Self {
        self.provenance = prov;
        self
    }

    /// Build the problem spec
    pub fn build(self) -> GateResult<ProblemSpec> {
        let spec = ProblemSpec {
            problem_id: self.problem_id,
            tenant_scope: self.tenant_scope,
            objective: self
                .objective
                .ok_or_else(|| GateError::invalid_input("objective is required"))?,
            constraints: self.constraints,
            inputs: self.inputs,
            budgets: self.budgets,
            determinism: self.determinism,
            provenance: self.provenance,
        };
        spec.validate()?;
        Ok(spec)
    }
}

/// Proposed plan from solver
///
/// This is always a proposal (never authority) - promotion happens only
/// at the PromotionGate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedPlan {
    /// Unique plan identifier
    pub plan_id: String,
    /// Pack that generated this plan
    pub pack: String,
    /// Human-readable summary
    pub summary: String,
    /// Typed plan payload (pack-specific)
    pub plan: serde_json::Value,
    /// Calibrated confidence score (0.0 - 1.0). Always clamped at construction.
    confidence: f64,
    /// Link to kernel trace for replay/audit
    pub trace_link: KernelTraceLink,
}

impl ProposedPlan {
    /// Create a new proposed plan
    pub fn new(
        plan_id: impl Into<String>,
        pack: impl Into<String>,
        summary: impl Into<String>,
        plan: serde_json::Value,
        confidence: f64,
        trace_link: KernelTraceLink,
    ) -> Self {
        Self {
            plan_id: plan_id.into(),
            pack: pack.into(),
            summary: summary.into(),
            plan,
            confidence: confidence.clamp(0.0, 1.0),
            trace_link,
        }
    }

    /// Create a plan from a serializable payload
    pub fn from_payload<T: Serialize>(
        plan_id: impl Into<String>,
        pack: impl Into<String>,
        summary: impl Into<String>,
        payload: &T,
        confidence: f64,
        trace_link: KernelTraceLink,
    ) -> GateResult<Self> {
        let plan =
            serde_json::to_value(payload).map_err(|e| GateError::invalid_input(e.to_string()))?;
        Ok(Self::new(
            plan_id, pack, summary, plan, confidence, trace_link,
        ))
    }

    /// Returns the confidence score, always in [0.0, 1.0].
    #[must_use]
    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    /// Deserialize plan payload to typed struct
    pub fn plan_as<T: for<'de> Deserialize<'de>>(&self) -> GateResult<T> {
        serde_json::from_value(self.plan.clone())
            .map_err(|e| GateError::invalid_input(format!("failed to parse plan: {}", e)))
    }

    /// Check if plan has high confidence (>= 0.8)
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.8
    }

    /// Check if plan has low confidence (< 0.5)
    pub fn is_low_confidence(&self) -> bool {
        self.confidence < 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_problem_spec_builder() {
        let spec = ProblemSpec::builder("prob-001", "tenant-abc")
            .objective(ObjectiveSpec::minimize("cost"))
            .seed(42)
            .build()
            .unwrap();

        assert_eq!(spec.problem_id, "prob-001");
        assert_eq!(spec.tenant_scope, "tenant-abc");
        assert_eq!(spec.seed(), 42);
    }

    #[test]
    fn test_problem_spec_validation() {
        let result = ProblemSpec::builder("", "tenant")
            .objective(ObjectiveSpec::minimize("x"))
            .build();
        assert!(result.is_err());

        let result = ProblemSpec::builder("id", "")
            .objective(ObjectiveSpec::minimize("x"))
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_problem_spec_with_inputs() {
        #[derive(Serialize, Deserialize, Debug, PartialEq)]
        struct TestInput {
            value: i32,
        }

        let input = TestInput { value: 42 };
        let spec = ProblemSpec::builder("prob-001", "tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .inputs(&input)
            .unwrap()
            .build()
            .unwrap();

        let parsed: TestInput = spec.inputs_as().unwrap();
        assert_eq!(parsed, input);
    }

    #[test]
    fn test_proposed_plan() {
        let trace = KernelTraceLink::audit_only("trace-001");
        let plan = ProposedPlan::new(
            "plan-001",
            "meeting-scheduler",
            "Selected slot A at 10am",
            serde_json::json!({"slot": "A"}),
            0.95,
            trace,
        );

        assert_eq!(plan.plan_id, "plan-001");
        assert!(plan.is_high_confidence());
        assert!(!plan.is_low_confidence());
    }

    #[test]
    fn test_confidence_clamped() {
        let trace = KernelTraceLink::default();
        let plan = ProposedPlan::new("p", "pack", "s", serde_json::Value::Null, 1.5, trace);
        assert_eq!(plan.confidence(), 1.0);

        let trace = KernelTraceLink::default();
        let plan = ProposedPlan::new("p", "pack", "s", serde_json::Value::Null, -0.5, trace);
        assert_eq!(plan.confidence(), 0.0);
    }

    #[test]
    fn test_serde_roundtrip() {
        let spec = ProblemSpec::builder("prob-001", "tenant")
            .objective(ObjectiveSpec::minimize("cost"))
            .seed(123)
            .build()
            .unwrap();

        let json = serde_json::to_string(&spec).unwrap();
        let restored: ProblemSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(restored.problem_id, spec.problem_id);
        assert_eq!(restored.seed(), 123);
    }
}
