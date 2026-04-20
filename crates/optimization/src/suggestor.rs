//! Suggestor adapter — bridges optimization Pack trait to Converge Suggestor.
//!
//! Every optimization pack becomes a first-class Suggestor, participatable
//! in the convergence loop via `SolverSuggestor`.

use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};

use crate::gate::{ObjectiveSpec, ProblemSpec};
use crate::packs::Pack;

/// Wraps any optimization Pack as a Converge Suggestor.
///
/// The adapter reads problem specifications from context (`input_key`),
/// runs the solver, and proposes the solution as a fact to `output_key`.
pub struct SolverSuggestor<P: Pack> {
    pack: P,
    input_key: ContextKey,
    output_key: ContextKey,
}

impl<P: Pack> SolverSuggestor<P> {
    /// Create a new `SolverSuggestor` wrapping the given pack.
    pub fn new(pack: P, input_key: ContextKey, output_key: ContextKey) -> Self {
        Self {
            pack,
            input_key,
            output_key,
        }
    }
}

#[async_trait]
impl<P: Pack> Suggestor for SolverSuggestor<P> {
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

        let inputs: serde_json::Value = match serde_json::from_str(&seed_fact.content) {
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
                let confidence = result.plan.confidence;
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

// Constructor helpers for each built-in pack

impl SolverSuggestor<crate::packs::lead_routing::LeadRoutingPack> {
    /// Create a lead routing suggestor (Seeds → Strategies).
    pub fn lead_routing() -> Self {
        Self::new(
            crate::packs::lead_routing::LeadRoutingPack,
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }
}

impl SolverSuggestor<crate::packs::meeting_scheduler::MeetingSchedulerPack> {
    /// Create a meeting scheduler suggestor (Seeds → Strategies).
    pub fn meeting_scheduler() -> Self {
        Self::new(
            crate::packs::meeting_scheduler::MeetingSchedulerPack,
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }
}

impl SolverSuggestor<crate::packs::inventory_rebalancing::InventoryRebalancingPack> {
    /// Create an inventory rebalancing suggestor (Seeds → Strategies).
    pub fn inventory_rebalancing() -> Self {
        Self::new(
            crate::packs::inventory_rebalancing::InventoryRebalancingPack,
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }
}

impl SolverSuggestor<crate::packs::budget_allocation::BudgetAllocationPack> {
    /// Create a budget allocation suggestor (Seeds → Strategies).
    pub fn budget_allocation() -> Self {
        Self::new(
            crate::packs::budget_allocation::BudgetAllocationPack,
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }
}

impl SolverSuggestor<crate::packs::capacity_planning::CapacityPlanningPack> {
    /// Create a capacity planning suggestor (Seeds → Strategies).
    pub fn capacity_planning() -> Self {
        Self::new(
            crate::packs::capacity_planning::CapacityPlanningPack,
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }
}

impl SolverSuggestor<crate::packs::shipping_choice::ShippingChoicePack> {
    /// Create a shipping choice suggestor (Seeds → Strategies).
    pub fn shipping_choice() -> Self {
        Self::new(
            crate::packs::shipping_choice::ShippingChoicePack,
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }
}

impl SolverSuggestor<crate::packs::vendor_shortlist::VendorShortlistPack> {
    /// Create a vendor shortlist suggestor (Seeds → Strategies).
    pub fn vendor_shortlist() -> Self {
        Self::new(
            crate::packs::vendor_shortlist::VendorShortlistPack,
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }
}

impl SolverSuggestor<crate::packs::anomaly_triage::AnomalyTriagePack> {
    /// Create an anomaly triage suggestor (Seeds → Strategies).
    pub fn anomaly_triage() -> Self {
        Self::new(
            crate::packs::anomaly_triage::AnomalyTriagePack,
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }
}

impl SolverSuggestor<crate::packs::backlog_prioritization::BacklogPrioritizationPack> {
    /// Create a backlog prioritization suggestor (Seeds → Strategies).
    pub fn backlog_prioritization() -> Self {
        Self::new(
            crate::packs::backlog_prioritization::BacklogPrioritizationPack,
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }
}

impl SolverSuggestor<crate::packs::inventory_replenishment::InventoryReplenishmentPack> {
    /// Create an inventory replenishment suggestor (Seeds → Strategies).
    pub fn inventory_replenishment() -> Self {
        Self::new(
            crate::packs::inventory_replenishment::InventoryReplenishmentPack,
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }
}

impl SolverSuggestor<crate::packs::pricing_guardrails::PricingGuardrailsPack> {
    /// Create a pricing guardrails suggestor (Seeds → Strategies).
    pub fn pricing_guardrails() -> Self {
        Self::new(
            crate::packs::pricing_guardrails::PricingGuardrailsPack,
            ContextKey::Seeds,
            ContextKey::Strategies,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::lead_routing::{Lead, LeadRoutingInput, RoutingConfig, SalesRep};
    use converge_pack::Fact;
    use converge_pack::fact::kernel_authority;
    use std::collections::HashMap;

    struct MockContext {
        facts: HashMap<ContextKey, Vec<Fact>>,
    }

    impl MockContext {
        fn empty() -> Self {
            Self {
                facts: HashMap::new(),
            }
        }

        fn with_seed(content: &str) -> Self {
            let mut ctx = Self::empty();
            ctx.facts.insert(
                ContextKey::Seeds,
                vec![kernel_authority::new_fact(
                    ContextKey::Seeds,
                    "seed-1",
                    content,
                )],
            );
            ctx
        }

        fn with_seed_and_output(seed_content: &str) -> Self {
            let mut ctx = Self::with_seed(seed_content);
            ctx.facts.insert(
                ContextKey::Strategies,
                vec![kernel_authority::new_fact(
                    ContextKey::Strategies,
                    "existing",
                    "already solved",
                )],
            );
            ctx
        }
    }

    impl Context for MockContext {
        fn has(&self, key: ContextKey) -> bool {
            self.facts.get(&key).is_some_and(|v| !v.is_empty())
        }

        fn get(&self, key: ContextKey) -> &[Fact] {
            self.facts.get(&key).map_or(&[], Vec::as_slice)
        }
    }

    #[test]
    fn name_delegates_to_pack() {
        let s = SolverSuggestor::lead_routing();
        assert_eq!(s.name(), "lead-routing");
    }

    #[test]
    fn dependencies_returns_input_key() {
        let s = SolverSuggestor::lead_routing();
        assert_eq!(s.dependencies(), &[ContextKey::Seeds]);
    }

    #[test]
    fn accepts_true_when_input_present_and_no_output() {
        let s = SolverSuggestor::lead_routing();
        let ctx = MockContext::with_seed("{}");
        assert!(s.accepts(&ctx));
    }

    #[test]
    fn accepts_false_when_no_input() {
        let s = SolverSuggestor::lead_routing();
        let ctx = MockContext::empty();
        assert!(!s.accepts(&ctx));
    }

    #[test]
    fn accepts_false_when_output_already_present() {
        let s = SolverSuggestor::lead_routing();
        let ctx = MockContext::with_seed_and_output("{}");
        assert!(!s.accepts(&ctx));
    }

    #[tokio::test]
    async fn execute_with_valid_input_produces_proposal() {
        let s = SolverSuggestor::lead_routing();

        let input = LeadRoutingInput {
            leads: vec![Lead {
                id: "lead-1".to_string(),
                score: 85.0,
                territory: "west".to_string(),
                segment: "enterprise".to_string(),
                required_skills: vec!["cloud".to_string()],
                estimated_value: 100_000.0,
                priority: 1,
            }],
            reps: vec![SalesRep {
                id: "rep-1".to_string(),
                name: "Alice".to_string(),
                capacity: 10,
                current_load: 2,
                territories: vec!["west".to_string()],
                segments: vec!["enterprise".to_string()],
                skills: vec!["cloud".to_string()],
                performance_score: 90.0,
            }],
            config: RoutingConfig::default(),
        };

        let json = serde_json::to_string(&input).unwrap();
        let ctx = MockContext::with_seed(&json);

        let effect = s.execute(&ctx).await;
        assert!(!effect.is_empty());
        assert_eq!(effect.proposals.len(), 1);
        assert_eq!(effect.proposals[0].key, ContextKey::Strategies);
        assert_eq!(effect.proposals[0].id, "lead-routing-solution");
        assert!(effect.proposals[0].confidence > 0.0);
    }

    #[tokio::test]
    async fn execute_with_invalid_input_returns_empty() {
        let s = SolverSuggestor::lead_routing();
        let ctx = MockContext::with_seed("not valid json {{{");
        let effect = s.execute(&ctx).await;
        assert!(effect.is_empty());
    }
}
