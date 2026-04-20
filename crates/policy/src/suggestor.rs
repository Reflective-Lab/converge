use std::sync::Arc;

use converge_core::FlowGateInput;
use converge_pack::{AgentEffect, Context, ContextKey, Suggestor, fact::ProposedFact};
use ed25519_dalek::VerifyingKey;

use crate::delegation;
use crate::engine::PolicyEngine;
use crate::types::DecideRequest;

const PROVENANCE: &str = "converge-policy";

// --- PolicyGateSuggestor ---

pub struct PolicyGateSuggestor {
    engine: Arc<PolicyEngine>,
    input_key: ContextKey,
    output_key: ContextKey,
}

impl PolicyGateSuggestor {
    #[must_use]
    pub fn new(engine: Arc<PolicyEngine>) -> Self {
        Self {
            engine,
            input_key: ContextKey::Seeds,
            output_key: ContextKey::Constraints,
        }
    }

    #[must_use]
    pub fn with_keys(
        engine: Arc<PolicyEngine>,
        input_key: ContextKey,
        output_key: ContextKey,
    ) -> Self {
        Self {
            engine,
            input_key,
            output_key,
        }
    }
}

#[async_trait::async_trait]
impl Suggestor for PolicyGateSuggestor {
    fn name(&self) -> &str {
        "policy-gate"
    }

    fn dependencies(&self) -> &[ContextKey] {
        std::slice::from_ref(&self.input_key)
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(self.input_key) && !ctx.has(self.output_key)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let facts = ctx.get(self.input_key);
        let Some(seed) = facts.first() else {
            return AgentEffect::empty();
        };

        let req: DecideRequest = match serde_json::from_str(&seed.content) {
            Ok(r) => r,
            Err(e) => {
                let diag = ProposedFact::new(
                    ContextKey::Diagnostic,
                    "policy-gate-error",
                    format!("failed to parse DecideRequest: {e}"),
                    PROVENANCE,
                );
                return AgentEffect::with_proposal(diag);
            }
        };

        match self.engine.evaluate(&req) {
            Ok(decision) => {
                let content = serde_json::to_string(&decision).unwrap_or_default();
                let proposal =
                    ProposedFact::new(self.output_key, "policy-decision", content, PROVENANCE);
                AgentEffect::with_proposal(proposal)
            }
            Err(e) => {
                let diag = ProposedFact::new(
                    ContextKey::Diagnostic,
                    "policy-gate-error",
                    format!("policy evaluation failed: {e}"),
                    PROVENANCE,
                );
                AgentEffect::with_proposal(diag)
            }
        }
    }
}

// --- DelegationVerifySuggestor ---

pub struct DelegationVerifySuggestor {
    verifying_key: VerifyingKey,
    input_key: ContextKey,
    output_key: ContextKey,
}

impl DelegationVerifySuggestor {
    #[must_use]
    pub fn new(verifying_key: VerifyingKey) -> Self {
        Self {
            verifying_key,
            input_key: ContextKey::Seeds,
            output_key: ContextKey::Constraints,
        }
    }

    #[must_use]
    pub fn with_keys(
        verifying_key: VerifyingKey,
        input_key: ContextKey,
        output_key: ContextKey,
    ) -> Self {
        Self {
            verifying_key,
            input_key,
            output_key,
        }
    }
}

#[async_trait::async_trait]
impl Suggestor for DelegationVerifySuggestor {
    fn name(&self) -> &str {
        "delegation-verify"
    }

    fn dependencies(&self) -> &[ContextKey] {
        std::slice::from_ref(&self.input_key)
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(self.input_key) && !ctx.has(self.output_key)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let facts = ctx.get(self.input_key);
        let Some(seed) = facts.first() else {
            return AgentEffect::empty();
        };

        let req: DecideRequest = match serde_json::from_str(&seed.content) {
            Ok(r) => r,
            Err(e) => {
                let diag = ProposedFact::new(
                    ContextKey::Diagnostic,
                    "delegation-verify-error",
                    format!("failed to parse DecideRequest: {e}"),
                    PROVENANCE,
                );
                return AgentEffect::with_proposal(diag);
            }
        };

        let Some(ref token_b64) = req.delegation_b64 else {
            let diag = ProposedFact::new(
                ContextKey::Diagnostic,
                "delegation-verify-error",
                "no delegation_b64 in request",
                PROVENANCE,
            );
            return AgentEffect::with_proposal(diag);
        };

        match delegation::verify(token_b64, &self.verifying_key, &req) {
            Ok(valid) => {
                let content = if valid {
                    r#"{"valid":true}"#.to_string()
                } else {
                    r#"{"valid":false,"reason":"constraints not met"}"#.to_string()
                };
                let proposal =
                    ProposedFact::new(self.output_key, "delegation-result", content, PROVENANCE);
                AgentEffect::with_proposal(proposal)
            }
            Err(e) => {
                let content = format!(r#"{{"valid":false,"reason":"{e}"}}"#);
                let proposal =
                    ProposedFact::new(self.output_key, "delegation-result", content, PROVENANCE);
                AgentEffect::with_proposal(proposal)
            }
        }
    }
}

// --- FlowGateSuggestor ---

pub struct FlowGateSuggestor {
    engine: Arc<PolicyEngine>,
    input_key: ContextKey,
    output_key: ContextKey,
}

impl FlowGateSuggestor {
    #[must_use]
    pub fn new(engine: Arc<PolicyEngine>) -> Self {
        Self {
            engine,
            input_key: ContextKey::Seeds,
            output_key: ContextKey::Constraints,
        }
    }

    #[must_use]
    pub fn with_keys(
        engine: Arc<PolicyEngine>,
        input_key: ContextKey,
        output_key: ContextKey,
    ) -> Self {
        Self {
            engine,
            input_key,
            output_key,
        }
    }
}

#[async_trait::async_trait]
impl Suggestor for FlowGateSuggestor {
    fn name(&self) -> &str {
        "flow-gate"
    }

    fn dependencies(&self) -> &[ContextKey] {
        std::slice::from_ref(&self.input_key)
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.has(self.input_key) && !ctx.has(self.output_key)
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let facts = ctx.get(self.input_key);
        let Some(seed) = facts.first() else {
            return AgentEffect::empty();
        };

        let input: FlowGateInput = match serde_json::from_str(&seed.content) {
            Ok(i) => i,
            Err(e) => {
                let diag = ProposedFact::new(
                    ContextKey::Diagnostic,
                    "flow-gate-error",
                    format!("failed to parse FlowGateInput: {e}"),
                    PROVENANCE,
                );
                return AgentEffect::with_proposal(diag);
            }
        };

        match self.engine.evaluate_flow(&input) {
            Ok(decision) => {
                let content = serde_json::to_string(&decision).unwrap_or_default();
                let proposal =
                    ProposedFact::new(self.output_key, "flow-gate-decision", content, PROVENANCE);
                AgentEffect::with_proposal(proposal)
            }
            Err(e) => {
                let diag = ProposedFact::new(
                    ContextKey::Diagnostic,
                    "flow-gate-error",
                    format!("flow gate evaluation failed: {e}"),
                    PROVENANCE,
                );
                AgentEffect::with_proposal(diag)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::fact::Fact;
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
    fn policy_gate_name() {
        let engine = Arc::new(
            PolicyEngine::from_policy_str("permit(principal, action, resource);").unwrap(),
        );
        let s = PolicyGateSuggestor::new(engine);
        assert_eq!(s.name(), "policy-gate");
    }

    #[test]
    fn policy_gate_dependencies() {
        let engine = Arc::new(
            PolicyEngine::from_policy_str("permit(principal, action, resource);").unwrap(),
        );
        let s = PolicyGateSuggestor::new(engine);
        assert_eq!(s.dependencies(), &[ContextKey::Seeds]);
    }

    #[test]
    fn policy_gate_rejects_empty_context() {
        let engine = Arc::new(
            PolicyEngine::from_policy_str("permit(principal, action, resource);").unwrap(),
        );
        let s = PolicyGateSuggestor::new(engine);
        let ctx = MockContext::empty();
        assert!(!s.accepts(&ctx));
    }

    #[test]
    fn delegation_verify_name() {
        let key = ed25519_dalek::SigningKey::from_bytes(&[42u8; 32]).verifying_key();
        let s = DelegationVerifySuggestor::new(key);
        assert_eq!(s.name(), "delegation-verify");
    }

    #[test]
    fn flow_gate_name() {
        let engine = Arc::new(
            PolicyEngine::from_policy_str("permit(principal, action, resource);").unwrap(),
        );
        let s = FlowGateSuggestor::new(engine);
        assert_eq!(s.name(), "flow-gate");
    }

    #[test]
    fn flow_gate_rejects_empty_context() {
        let engine = Arc::new(
            PolicyEngine::from_policy_str("permit(principal, action, resource);").unwrap(),
        );
        let s = FlowGateSuggestor::new(engine);
        let ctx = MockContext::empty();
        assert!(!s.accepts(&ctx));
    }
}
