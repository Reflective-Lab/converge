// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Provider selection via bipartite matching.
//!
//! Reads a [`ProviderRequest`] from context, matches required capabilities
//! against the registered backend pool using Hopcroft-Karp, and proposes a
//! [`ProviderAssignment`] to [`ContextKey::Strategies`].

use std::sync::Arc;

use async_trait::async_trait;
use converge_optimization::graph::matching::bipartite_matching;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use converge_provider::{
    Backend, BackendRequirements, CapabilityAssignment, ProviderAssignment, ProviderRequest,
};

// ── Suggestor ─────────────────────────────────────────────────────────────────

const REQUEST_PREFIX: &str = "provider-request:";
const ASSIGNMENT_PREFIX: &str = "provider-assignment:";
const MALFORMED_PREFIX: &str = "provider-request-error:";

/// Routes required capabilities to available backends via bipartite matching.
///
/// # Construction
///
/// ```rust,ignore
/// let backends: Vec<Arc<dyn Backend>> = vec![
///     Arc::new(AnthropicBackend::from_env()),
///     Arc::new(KongBackend::from_env()),
/// ];
///
/// engine.register_suggestor(ProviderSelectionSuggestor::new(backends));
/// ```
pub struct ProviderSelectionSuggestor {
    backends: Vec<Arc<dyn Backend>>,
}

impl ProviderSelectionSuggestor {
    pub fn new(backends: Vec<Arc<dyn Backend>>) -> Self {
        Self { backends }
    }
}

#[async_trait]
impl Suggestor for ProviderSelectionSuggestor {
    fn name(&self) -> &str {
        "ProviderSelectionSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|f| {
            f.id().as_str().starts_with(REQUEST_PREFIX)
                && match serde_json::from_str::<ProviderRequest>(f.content()) {
                    Ok(_) => !assignment_exists(ctx, request_id(f.id().as_str())),
                    Err(_) => !malformed_diagnostic_exists(ctx, f.id().as_str()),
                }
        })
    }

    async fn execute(&self, ctx: &dyn Context) -> AgentEffect {
        let mut proposals = Vec::new();

        for fact in ctx
            .get(ContextKey::Seeds)
            .iter()
            .filter(|f| f.id().as_str().starts_with(REQUEST_PREFIX))
        {
            match serde_json::from_str::<ProviderRequest>(fact.content()) {
                Ok(req) => {
                    if assignment_exists(ctx, request_id(fact.id().as_str())) {
                        continue;
                    }

                    let assignment = route(&req, &self.backends);
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Strategies,
                            format!("{}{}", ASSIGNMENT_PREFIX, assignment.request_id),
                            serde_json::to_string(&assignment).unwrap_or_default(),
                            self.name(),
                        )
                        .with_confidence(assignment.coverage_ratio),
                    );
                }
                Err(error) => {
                    if malformed_diagnostic_exists(ctx, fact.id().as_str()) {
                        continue;
                    }

                    let diagnostic = serde_json::json!({
                        "request_fact_id": fact.id(),
                        "message": "malformed provider request ignored",
                        "error": error.to_string(),
                    });
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Diagnostic,
                            malformed_diagnostic_id(fact.id().as_str()),
                            diagnostic.to_string(),
                            self.name(),
                        )
                        .with_confidence(1.0),
                    );
                }
            }
        }

        if proposals.is_empty() {
            AgentEffect::empty()
        } else {
            AgentEffect::with_proposals(proposals)
        }
    }
}

// ── Matching logic ────────────────────────────────────────────────────────────

fn route(req: &ProviderRequest, backends: &[Arc<dyn Backend>]) -> ProviderAssignment {
    if let Some(requirements) = &req.backend_requirements {
        return route_backend_requirements(req, requirements, backends);
    }

    // Left = required capability slots (index = position in req.required_capabilities).
    // Right = backends (index = position in `backends`).
    // Edge: backends[j].has_capability(req.required_capabilities[i]).
    let edges: Vec<(usize, usize)> = req
        .required_capabilities
        .iter()
        .enumerate()
        .flat_map(|(i, cap)| {
            let cap = cap.clone();
            backends
                .iter()
                .enumerate()
                .filter(move |(_, b)| b.has_capability(cap.clone()))
                .map(move |(j, _)| (i, j))
        })
        .collect();

    let matching = bipartite_matching(req.required_capabilities.len(), backends.len(), &edges)
        .unwrap_or_default();

    let mut covered = vec![false; req.required_capabilities.len()];
    let mut assignments = Vec::with_capacity(matching.size);

    for (cap_idx, backend_idx) in &matching.pairs {
        assignments.push(CapabilityAssignment {
            capability: req.required_capabilities[*cap_idx].clone(),
            backend_name: backends[*backend_idx].name().to_string(),
        });
        covered[*cap_idx] = true;
    }

    let unmatched = req
        .required_capabilities
        .iter()
        .enumerate()
        .filter(|(i, _)| !covered[*i])
        .map(|(_, c)| c.clone())
        .collect::<Vec<_>>();

    let coverage_ratio = if req.required_capabilities.is_empty() {
        1.0
    } else {
        matching.size as f64 / req.required_capabilities.len() as f64
    };

    ProviderAssignment {
        request_id: req.id.clone(),
        assignments,
        unmatched,
        coverage_ratio,
    }
}

fn route_backend_requirements(
    req: &ProviderRequest,
    requirements: &BackendRequirements,
    backends: &[Arc<dyn Backend>],
) -> ProviderAssignment {
    let required_capabilities = if requirements.required_capabilities.is_empty() {
        req.required_capabilities.clone()
    } else {
        requirements.required_capabilities.clone()
    };

    let matched_backend = backends.iter().find(|backend| {
        backend.kind() == requirements.kind
            && required_capabilities
                .iter()
                .all(|capability| backend.has_capability(capability.clone()))
            && (!requirements.requires_replay || backend.supports_replay())
            && (!requirements.requires_offline || !backend.requires_network())
    });

    if let Some(backend) = matched_backend {
        let assignments = required_capabilities
            .iter()
            .cloned()
            .map(|capability| CapabilityAssignment {
                capability,
                backend_name: backend.name().to_string(),
            })
            .collect::<Vec<_>>();
        return ProviderAssignment {
            request_id: req.id.clone(),
            assignments,
            unmatched: Vec::new(),
            coverage_ratio: 1.0,
        };
    }

    let coverage_ratio = if required_capabilities.is_empty() {
        1.0
    } else {
        0.0
    };
    ProviderAssignment {
        request_id: req.id.clone(),
        assignments: Vec::new(),
        unmatched: required_capabilities,
        coverage_ratio,
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn request_id(fact_id: &str) -> &str {
    fact_id.trim_start_matches(REQUEST_PREFIX)
}

fn assignment_exists(ctx: &dyn Context, request_id: &str) -> bool {
    let assignment_id = format!("{}{}", ASSIGNMENT_PREFIX, request_id);
    ctx.get(ContextKey::Strategies)
        .iter()
        .any(|f| f.id().as_str() == assignment_id)
}

fn malformed_diagnostic_id(fact_id: &str) -> String {
    format!("{MALFORMED_PREFIX}{fact_id}")
}

fn malformed_diagnostic_exists(ctx: &dyn Context, fact_id: &str) -> bool {
    let diagnostic_id = malformed_diagnostic_id(fact_id);
    ctx.get(ContextKey::Diagnostic)
        .iter()
        .any(|fact| fact.id().as_str() == diagnostic_id)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{ContextState, Engine};
    use converge_provider::{BackendKind, Capability};

    struct MockBackend {
        name: &'static str,
        kind: BackendKind,
        capabilities: Vec<Capability>,
        supports_replay: bool,
        requires_network: bool,
    }

    impl Backend for MockBackend {
        fn name(&self) -> &str {
            self.name
        }
        fn kind(&self) -> BackendKind {
            self.kind.clone()
        }
        fn capabilities(&self) -> Vec<Capability> {
            self.capabilities.clone()
        }
        fn supports_replay(&self) -> bool {
            self.supports_replay
        }
        fn requires_network(&self) -> bool {
            self.requires_network
        }
    }

    fn backend(name: &'static str, caps: &[Capability]) -> Arc<dyn Backend> {
        backend_with(name, BackendKind::Llm, caps, false, true)
    }

    fn backend_with(
        name: &'static str,
        kind: BackendKind,
        caps: &[Capability],
        supports_replay: bool,
        requires_network: bool,
    ) -> Arc<dyn Backend> {
        Arc::new(MockBackend {
            name,
            kind,
            capabilities: caps.to_vec(),
            supports_replay,
            requires_network,
        })
    }

    fn request(id: &str, caps: &[Capability]) -> ProviderRequest {
        ProviderRequest {
            id: id.to_string(),
            required_capabilities: caps.to_vec(),
            backend_requirements: None,
        }
    }

    #[test]
    fn full_coverage_when_all_capabilities_available() {
        let pool = vec![
            backend("anthropic", &[Capability::Reasoning]),
            backend("kong", &[Capability::AccessControl]),
            backend("elastic", &[Capability::FullTextSearch]),
        ];
        let req = request(
            "req-1",
            &[
                Capability::Reasoning,
                Capability::AccessControl,
                Capability::FullTextSearch,
            ],
        );

        let assignment = route(&req, &pool);

        assert_eq!(assignment.assignments.len(), 3);
        assert!(assignment.unmatched.is_empty());
        assert!((assignment.coverage_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn partial_coverage_when_capability_missing() {
        let pool = vec![backend("anthropic", &[Capability::Reasoning])];
        let req = request("req-2", &[Capability::Reasoning, Capability::AccessControl]);

        let assignment = route(&req, &pool);

        assert_eq!(assignment.assignments.len(), 1);
        assert_eq!(assignment.unmatched, vec![Capability::AccessControl]);
        assert!((assignment.coverage_ratio - 0.5).abs() < f64::EPSILON);
    }

    #[test]
    fn no_double_booking_with_two_same_capability_slots() {
        let pool = vec![
            backend("anthropic", &[Capability::Reasoning]),
            backend("openai", &[Capability::Reasoning]),
        ];
        let req = request("req-3", &[Capability::Reasoning, Capability::Reasoning]);

        let assignment = route(&req, &pool);

        assert_eq!(assignment.assignments.len(), 2);
        let names: Vec<_> = assignment
            .assignments
            .iter()
            .map(|a| &a.backend_name)
            .collect();
        let unique: std::collections::HashSet<_> = names.iter().collect();
        assert_eq!(unique.len(), 2);
    }

    #[test]
    fn multi_capability_backend_can_only_fill_one_slot() {
        // One backend that has both capabilities but should only fill one slot.
        let pool = vec![backend(
            "all-in-one",
            &[Capability::Reasoning, Capability::AccessControl],
        )];
        let req = request("req-4", &[Capability::Reasoning, Capability::AccessControl]);

        let assignment = route(&req, &pool);

        // Only one slot filled — backend can't be double-booked.
        assert_eq!(assignment.assignments.len(), 1);
        assert_eq!(assignment.unmatched.len(), 1);
    }

    #[test]
    fn empty_pool_yields_zero_coverage() {
        let req = request("req-5", &[Capability::Reasoning]);
        let assignment = route(&req, &[]);
        assert_eq!(assignment.coverage_ratio, 0.0);
        assert_eq!(assignment.unmatched, vec![Capability::Reasoning]);
    }

    #[test]
    fn empty_request_yields_full_coverage() {
        let pool = vec![backend("anthropic", &[Capability::Reasoning])];
        let req = request("req-6", &[]);
        let assignment = route(&req, &pool);
        assert!((assignment.coverage_ratio - 1.0).abs() < f64::EPSILON);
        assert!(assignment.assignments.is_empty());
    }

    #[test]
    fn backend_requirements_select_one_backend_satisfying_role_constraints() {
        let pool = vec![
            backend("remote-llm", &[Capability::AccessControl]),
            backend_with(
                "local-policy",
                BackendKind::Policy,
                &[Capability::AccessControl],
                true,
                false,
            ),
        ];
        let req = ProviderRequest {
            id: "policy-role".to_string(),
            required_capabilities: vec![],
            backend_requirements: Some(
                BackendRequirements::access_policy()
                    .with_replay()
                    .with_offline(),
            ),
        };

        let assignment = route(&req, &pool);

        assert_eq!(assignment.assignments.len(), 1);
        assert_eq!(assignment.assignments[0].backend_name, "local-policy");
        assert!(assignment.unmatched.is_empty());
        assert!((assignment.coverage_ratio - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn repeated_routing_is_deterministic_for_equal_candidates() {
        let pool = vec![
            backend("reasoner-a", &[Capability::Reasoning]),
            backend("reasoner-b", &[Capability::Reasoning]),
            backend("policy-a", &[Capability::AccessControl]),
        ];
        let req = request(
            "req-7",
            &[
                Capability::Reasoning,
                Capability::Reasoning,
                Capability::AccessControl,
            ],
        );

        let first = route(&req, &pool);
        let second = route(&req, &pool);

        assert_eq!(first.assignments, second.assignments);
        assert_eq!(first.unmatched, second.unmatched);
        assert_eq!(first.coverage_ratio, second.coverage_ratio);
    }

    #[tokio::test]
    async fn malformed_request_emits_diagnostic_once() {
        let mut engine = Engine::new();
        engine.register_suggestor(ProviderSelectionSuggestor::new(vec![backend(
            "anthropic",
            &[Capability::Reasoning],
        )]));

        let mut ctx = ContextState::new();
        ctx.add_input(ContextKey::Seeds, "provider-request:broken", "{")
            .expect("seed should stage");

        let first = engine.run(ctx).await.expect("run should converge");
        let diagnostics = first.context.get(ContextKey::Diagnostic);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].id(),
            "provider-request-error:provider-request:broken"
        );
        assert!(!first.context.has(ContextKey::Strategies));

        let mut rerun_engine = Engine::new();
        rerun_engine.register_suggestor(ProviderSelectionSuggestor::new(vec![backend(
            "anthropic",
            &[Capability::Reasoning],
        )]));
        let second = rerun_engine
            .run(first.context.clone())
            .await
            .expect("rerun should converge");
        assert_eq!(second.context.get(ContextKey::Diagnostic).len(), 1);
    }
}
