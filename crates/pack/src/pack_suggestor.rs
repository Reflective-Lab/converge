//! Suggestor adapter -- bridges Pack trait to Converge Suggestor.
//!
//! Every domain pack becomes a first-class Suggestor, participatable
//! in the convergence loop via `PackSuggestor`.

use async_trait::async_trait;

use crate::Suggestor;
use crate::context::{Context, ContextKey};
use crate::effect::AgentEffect;
use crate::fact::ProposedFact;
use crate::gate::{ObjectiveSpec, ProblemSpec};
use crate::pack::Pack;

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

        let inputs: serde_json::Value = match serde_json::from_str(seed_fact.content()) {
            Ok(v) => v,
            Err(_) => return AgentEffect::empty(),
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
                let content = serde_json::to_string(&result.plan).unwrap_or_default();
                let confidence = result.plan.confidence();
                let proposal = ProposedFact::new(
                    self.output_key,
                    format!("{}-solution", self.pack.name()),
                    content,
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

    #[test]
    fn pack_suggestor_constructed() {
        // Basic construction test -- full integration tests live in downstream crates
        struct DummyPack;
        impl Pack for DummyPack {
            fn name(&self) -> &'static str {
                "dummy"
            }
            fn version(&self) -> &'static str {
                "0.1.0"
            }
            fn validate_inputs(&self, _: &serde_json::Value) -> crate::gate::GateResult<()> {
                Ok(())
            }
            fn invariants(&self) -> &[crate::pack::InvariantDef] {
                &[]
            }
            fn solve(
                &self,
                _: &ProblemSpec,
            ) -> crate::gate::GateResult<crate::pack::PackSolveResult> {
                Err(crate::gate::GateError::invalid_input("not implemented"))
            }
            fn check_invariants(
                &self,
                _: &crate::gate::ProposedPlan,
            ) -> crate::gate::GateResult<Vec<crate::pack::InvariantResult>> {
                Ok(vec![])
            }
            fn evaluate_gate(
                &self,
                _: &crate::gate::ProposedPlan,
                _: &[crate::pack::InvariantResult],
            ) -> crate::gate::PromotionGate {
                crate::gate::PromotionGate::auto_promote("ok")
            }
        }

        let s = PackSuggestor::new(DummyPack, ContextKey::Seeds, ContextKey::Strategies);
        assert_eq!(s.name(), "dummy");
        assert_eq!(s.dependencies(), &[ContextKey::Seeds]);
    }
}
