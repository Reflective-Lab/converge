//! Suggestor adapter -- bridges Pack trait to Converge Suggestor.
//!
//! Every domain pack becomes a first-class Suggestor, participatable
//! in the convergence loop via `PackSuggestor`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::Suggestor;
use crate::context::{Context, ContextKey};
use crate::effect::AgentEffect;
use crate::fact::Provenance;
use crate::fact::{FactFamilyId, FactPayload, PayloadError, PayloadVersion, ProposedFact};
use crate::gate::{GateError, GateResult, KernelTraceLink, ObjectiveSpec, ProblemSpec};
use crate::pack::Pack;

/// Typed input payload for generic [`PackSuggestor`] execution.
///
/// Domain-specific Suggestors should prefer domain-specific payloads. This
/// payload exists for generic `Pack` implementations whose schema is owned by
/// the pack and validated through `Pack::validate_inputs`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackInputPayload {
    pack: String,
    inputs: serde_json::Value,
}

impl PackInputPayload {
    /// Creates a typed generic pack input payload.
    #[must_use]
    pub fn new(pack: impl Into<String>, inputs: serde_json::Value) -> Self {
        Self {
            pack: pack.into(),
            inputs,
        }
    }

    /// Returns the target pack name.
    #[must_use]
    pub fn pack(&self) -> &str {
        &self.pack
    }

    /// Returns the pack-owned input value.
    #[must_use]
    pub fn inputs(&self) -> &serde_json::Value {
        &self.inputs
    }
}

impl FactPayload for PackInputPayload {
    const FAMILY: &'static str = "converge.pack.input";
    const VERSION: u16 = 1;

    fn validate(&self) -> Result<(), PayloadError> {
        if self.pack.trim().is_empty() {
            return Err(PayloadError::Invalid {
                family: FactFamilyId::from(Self::FAMILY),
                version: PayloadVersion::new(Self::VERSION),
                reason: "pack must not be empty".to_string(),
            });
        }
        Ok(())
    }
}

/// Typed output payload emitted by generic [`PackSuggestor`] execution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackPlanPayload {
    /// Unique plan identifier.
    pub plan_id: String,
    /// Pack that generated this plan.
    pub pack: String,
    /// Human-readable summary.
    pub summary: String,
    /// Pack-owned typed plan payload.
    pub plan: serde_json::Value,
    /// Calibrated confidence score in `[0.0, 1.0]`.
    pub confidence: f64,
    /// Link to kernel trace for replay/audit.
    pub trace_link: KernelTraceLink,
}

impl PackPlanPayload {
    /// Creates a payload from a pack [`crate::gate::ProposedPlan`].
    #[must_use]
    pub fn from_plan(plan: &crate::gate::ProposedPlan) -> Self {
        Self {
            plan_id: plan.plan_id.clone(),
            pack: plan.pack.clone(),
            summary: plan.summary.clone(),
            plan: plan.plan.clone(),
            confidence: plan.confidence(),
            trace_link: plan.trace_link.clone(),
        }
    }

    /// Deserialize the pack-owned plan payload to a typed struct.
    pub fn plan_as<T: DeserializeOwned>(&self) -> GateResult<T> {
        serde_json::from_value(self.plan.clone())
            .map_err(|err| GateError::invalid_input(format!("failed to parse plan: {err}")))
    }
}

impl FactPayload for PackPlanPayload {
    const FAMILY: &'static str = "converge.pack.plan";
    const VERSION: u16 = 1;

    fn validate(&self) -> Result<(), PayloadError> {
        if self.pack.trim().is_empty() {
            return Err(PayloadError::Invalid {
                family: FactFamilyId::from(Self::FAMILY),
                version: PayloadVersion::new(Self::VERSION),
                reason: "pack must not be empty".to_string(),
            });
        }
        if !self.confidence.is_finite() || !(0.0..=1.0).contains(&self.confidence) {
            return Err(PayloadError::Invalid {
                family: FactFamilyId::from(Self::FAMILY),
                version: PayloadVersion::new(Self::VERSION),
                reason: "confidence must be finite and in 0.0..=1.0".to_string(),
            });
        }
        Ok(())
    }
}

/// Wraps any Pack as a Converge Suggestor.
///
/// The adapter reads problem specifications from context (`input_key`),
/// runs the solver, and proposes the solution as a fact to `output_key`.
pub struct PackSuggestor<P: Pack> {
    pack: P,
    input_key: ContextKey,
    output_key: ContextKey,
}

impl<P: Pack> PackSuggestor<P> {
    /// Create a new `PackSuggestor` wrapping the given pack.
    pub fn new(pack: P, input_key: ContextKey, output_key: ContextKey) -> Self {
        Self {
            pack,
            input_key,
            output_key,
        }
    }
}

#[async_trait]
impl<P: Pack> Suggestor for PackSuggestor<P> {
    fn name(&self) -> &str {
        self.pack.name()
    }

    /// The wrapped pack's name doubles as its canonical provenance string.
    /// External packs that need a separate provenance (e.g., a crate-level
    /// `ProvenanceSource`) should wrap themselves in an outer Suggestor and
    /// override `provenance()` there instead of relying on this default.
    fn provenance(&self) -> Provenance {
        Provenance::new(self.pack.name())
    }

    fn dependencies(&self) -> &[ContextKey] {
        std::slice::from_ref(&self.input_key)
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(self.input_key) && !ctx.has(self.output_key)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let facts = ctx.get(self.input_key);
        let Some(seed_fact) = facts.first() else {
            return AgentEffect::empty();
        };

        let inputs = match seed_fact.payload::<PackInputPayload>() {
            Some(payload) if payload.pack() == self.pack.name() => payload.inputs().clone(),
            None => return AgentEffect::empty(),
            Some(_) => return AgentEffect::empty(),
        };

        let spec = match ProblemSpec::builder(format!("{}-converge", self.pack.name()), "converge")
            .objective(ObjectiveSpec::maximize("default"))
            .inputs_raw(inputs)
            .build()
        {
            Ok(s) => s,
            Err(_) => return AgentEffect::empty(),
        };

        match self.pack.solve(&spec) {
            Ok(result) => {
                let confidence = result.plan.confidence();
                let proposal = ProposedFact::new(
                    self.output_key,
                    format!("{}-solution", self.pack.name()),
                    PackPlanPayload::from_plan(&result.plan),
                    format!("solver:{}", self.pack.name()),
                )
                .with_confidence(confidence);
                AgentEffect::with_proposal(proposal)
            }
            Err(_) => AgentEffect::empty(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fact::{
        ContextFact, FactActor, FactActorKind, FactLocalTrace, FactPromotionRecord, FactTraceLink,
        FactValidationSummary, TextPayload,
    };
    use crate::gate::{
        GateError, GateResult, KernelTraceLink, PromotionGate, ProposedPlan, ReplayEnvelope,
        SolverReport,
    };
    use crate::pack::{InvariantDef, InvariantResult, PackSolveResult};
    use crate::types::{ContentHash, Timestamp};
    use std::collections::HashMap;

    /// Pack double whose behaviour is configured per test.
    struct ConfigurablePack {
        name: &'static str,
        outcome: PackOutcome,
    }

    #[derive(Clone)]
    enum PackOutcome {
        /// Returns a successful solve with this confidence.
        Solved(f64),
        /// Returns a Pack::solve error.
        Errored,
    }

    impl Pack for ConfigurablePack {
        fn name(&self) -> &'static str {
            self.name
        }
        fn version(&self) -> &'static str {
            "0.1.0"
        }
        fn validate_inputs(&self, _: &serde_json::Value) -> GateResult<()> {
            Ok(())
        }
        fn invariants(&self) -> &[InvariantDef] {
            &[]
        }
        fn solve(&self, spec: &ProblemSpec) -> GateResult<PackSolveResult> {
            match self.outcome {
                PackOutcome::Errored => Err(GateError::invalid_input("intentional test failure")),
                PackOutcome::Solved(conf) => {
                    let plan = ProposedPlan::from_payload(
                        format!("plan-{}", spec.problem_id),
                        self.name,
                        "solved",
                        &serde_json::json!({"value": 42}),
                        conf,
                        KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id)),
                    )
                    .expect("payload");
                    let report = SolverReport::optimal(
                        format!("{}-v1", self.name),
                        0.0,
                        ReplayEnvelope::minimal(spec.seed()),
                    );
                    Ok(PackSolveResult::new(plan, report))
                }
            }
        }
        fn check_invariants(&self, _: &ProposedPlan) -> GateResult<Vec<InvariantResult>> {
            Ok(vec![])
        }
        fn evaluate_gate(&self, _: &ProposedPlan, _: &[InvariantResult]) -> PromotionGate {
            PromotionGate::auto_promote("ok")
        }
    }

    /// Minimal Context implementation for unit-testing the adapter.
    struct MockContext {
        facts: HashMap<ContextKey, Vec<ContextFact>>,
    }

    impl MockContext {
        fn empty() -> Self {
            Self {
                facts: HashMap::new(),
            }
        }
        fn with_pack_input(pack: &str, value: serde_json::Value) -> Self {
            let mut ctx = Self::empty();
            let record = FactPromotionRecord::new_projection(
                "projection-test",
                ContentHash::zero(),
                FactActor::new_projection("test", FactActorKind::System),
                FactValidationSummary::default(),
                Vec::new(),
                FactTraceLink::Local(FactLocalTrace::new_projection("trace", "span", None, true)),
                Timestamp::epoch(),
            );
            let fact = ContextFact::new_projection(
                ContextKey::Seeds,
                "seed-1",
                PackInputPayload::new(pack, value),
                record,
                Timestamp::epoch(),
            );
            ctx.facts.insert(ContextKey::Seeds, vec![fact]);
            ctx
        }
        fn with_text_seed(content: &str) -> Self {
            let mut ctx = Self::empty();
            let record = FactPromotionRecord::new_projection(
                "projection-test",
                ContentHash::zero(),
                FactActor::new_projection("test", FactActorKind::System),
                FactValidationSummary::default(),
                Vec::new(),
                FactTraceLink::Local(FactLocalTrace::new_projection("trace", "span", None, true)),
                Timestamp::epoch(),
            );
            let fact = ContextFact::new_projection(
                ContextKey::Seeds,
                "seed-1",
                TextPayload::new(content),
                record,
                Timestamp::epoch(),
            );
            ctx.facts.insert(ContextKey::Seeds, vec![fact]);
            ctx
        }
        fn with_existing_output(self) -> Self {
            // Mark the output key as present by inserting a placeholder fact.
            let mut me = self;
            me.facts.insert(
                ContextKey::Strategies,
                vec![ContextFact::new_projection(
                    ContextKey::Strategies,
                    "strat-1",
                    TextPayload::new("{}"),
                    FactPromotionRecord::new_projection(
                        "projection-test",
                        ContentHash::zero(),
                        FactActor::new_projection("test", FactActorKind::System),
                        FactValidationSummary::default(),
                        Vec::new(),
                        FactTraceLink::Local(FactLocalTrace::new_projection(
                            "trace", "span", None, true,
                        )),
                        Timestamp::epoch(),
                    ),
                    Timestamp::epoch(),
                )],
            );
            me
        }
    }

    impl Context for MockContext {
        fn has(&self, key: ContextKey) -> bool {
            self.facts.get(&key).is_some_and(|v| !v.is_empty())
        }
        fn get(&self, key: ContextKey) -> &[ContextFact] {
            self.facts.get(&key).map_or(&[], Vec::as_slice)
        }
    }

    fn solver(outcome: PackOutcome) -> PackSuggestor<ConfigurablePack> {
        PackSuggestor::new(
            ConfigurablePack {
                name: "test-pack",
                outcome,
            },
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }

    #[test]
    fn pack_suggestor_constructed() {
        let s = solver(PackOutcome::Solved(0.9));
        assert_eq!(s.name(), "test-pack");
        assert_eq!(s.dependencies(), &[ContextKey::Seeds]);
    }

    #[test]
    fn accepts_when_input_present_and_output_missing() {
        let s = solver(PackOutcome::Solved(0.9));
        let ctx = MockContext::with_pack_input("test-pack", serde_json::json!({"x": 1}));
        assert!(s.accepts(&ctx));
    }

    #[test]
    fn rejects_when_input_missing() {
        let s = solver(PackOutcome::Solved(0.9));
        let ctx = MockContext::empty();
        assert!(!s.accepts(&ctx));
    }

    #[test]
    fn rejects_when_output_already_present() {
        let s = solver(PackOutcome::Solved(0.9));
        let ctx = MockContext::with_pack_input("test-pack", serde_json::json!({"x": 1}))
            .with_existing_output();
        assert!(!s.accepts(&ctx));
    }

    #[tokio::test]
    async fn execute_with_empty_context_returns_empty_effect() {
        let s = solver(PackOutcome::Solved(0.9));
        let ctx = MockContext::empty();
        let effect = s.execute(&ctx).await;
        assert_eq!(effect.proposals().len(), 0);
    }

    #[tokio::test]
    async fn execute_with_invalid_json_seed_returns_empty_effect() {
        let s = solver(PackOutcome::Solved(0.9));
        let ctx = MockContext::with_text_seed("not a typed pack input");
        let effect = s.execute(&ctx).await;
        assert_eq!(effect.proposals().len(), 0);
    }

    #[tokio::test]
    async fn execute_with_wrong_pack_input_returns_empty_effect() {
        let s = solver(PackOutcome::Solved(0.9));
        let ctx = MockContext::with_pack_input("other-pack", serde_json::json!({"x": 1}));
        let effect = s.execute(&ctx).await;
        assert_eq!(effect.proposals().len(), 0);
    }

    #[tokio::test]
    async fn execute_with_pack_solve_error_returns_empty_effect() {
        let s = solver(PackOutcome::Errored);
        let ctx = MockContext::with_pack_input("test-pack", serde_json::json!({"x": 1}));
        let effect = s.execute(&ctx).await;
        assert_eq!(effect.proposals().len(), 0);
    }

    #[tokio::test]
    async fn execute_with_successful_solve_emits_proposal_with_carried_confidence() {
        let s = solver(PackOutcome::Solved(0.42));
        let ctx = MockContext::with_pack_input("test-pack", serde_json::json!({"x": 1}));
        let effect = s.execute(&ctx).await;
        assert_eq!(effect.proposals().len(), 1);
        let proposal = &effect.proposals()[0];
        assert_eq!(proposal.key(), ContextKey::Strategies);
        let payload = proposal
            .require_payload::<PackPlanPayload>()
            .expect("PackSuggestor should emit typed pack plan payload");
        assert_eq!(payload.pack, "test-pack");
        assert_eq!(payload.plan["value"], 42);
        assert!(
            (proposal.confidence() - 0.42).abs() < 1e-6,
            "confidence must propagate from plan, got {}",
            proposal.confidence()
        );
    }
}
