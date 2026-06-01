// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Min-cost flow optimisation over a supply/demand network.
//!
//! Reads a [`FlowRequest`] from context — a directed graph with edge
//! capacities and costs, a source, a sink, and a required flow demand —
//! and proposes a [`FlowPlan`] with the cheapest routing of that flow.
//!
//! # Formation role
//!
//! Resource allocation suggestors (budget, capacity, workforce) produce demand
//! signals that are reflected in the flow request. The network models supply
//! chains, distribution networks, or internal resource routing. When any
//! upstream signal changes, a new request is seeded and the formation
//! re-converges on the updated routing.

use async_trait::async_trait;
use converge_pack::Provenance;
use converge_pack::ProvenanceSource;
use converge_pack::{
    AgentEffect, Context, ContextKey, DiagnosticPayload, FactPayload, ProposedFact, Suggestor,
};
use serde::{Deserialize, Serialize};

use crate::graph::flow::{FlowNetwork, MinCostFlowProblem, min_cost_flow};

// ── Request ───────────────────────────────────────────────────────────────────

/// Seed under [`ContextKey::Seeds`] with id prefix `"flow-request:"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowRequest {
    pub id: String,
    pub num_nodes: usize,
    pub edges: Vec<FlowEdgeSpec>,
    pub source: usize,
    pub sink: usize,
    /// Required flow volume from source to sink.
    pub demand: i64,
}

impl FactPayload for FlowRequest {
    const FAMILY: &'static str = "converge.optimization.flow.request";
    const VERSION: u16 = 1;
}

/// One directed edge in the flow network.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowEdgeSpec {
    pub from: usize,
    pub to: usize,
    pub capacity: i64,
    /// Per-unit routing cost.
    pub cost: i64,
    /// Optional human-readable label for the edge (supply route, link, etc.).
    pub label: Option<String>,
}

// ── Plan (output) ─────────────────────────────────────────────────────────────

/// The min-cost flow routing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowPlan {
    pub request_id: String,
    pub total_flow: i64,
    pub total_cost: i64,
    /// Flow on each edge, in the same order as the request's `edges` vec.
    pub edge_flows: Vec<i64>,
    /// `total_flow / demand` — 1.0 when demand is fully satisfied.
    pub fulfillment: f64,
    pub feasible: bool,
}

impl FactPayload for FlowPlan {
    const FAMILY: &'static str = "converge.optimization.flow.plan";
    const VERSION: u16 = 1;
}

// ── Suggestor ─────────────────────────────────────────────────────────────────

const REQUEST_PREFIX: &str = "flow-request:";
const PLAN_PREFIX: &str = "flow-plan:";
const ERROR_PREFIX: &str = "flow-request-error:";

/// Routes flow through a directed network at minimum cost (Successive Shortest
/// Paths with Bellman-Ford).
pub struct FlowOptimizationSuggestor;

#[async_trait]
impl Suggestor for FlowOptimizationSuggestor {
    fn name(&self) -> &str {
        "FlowOptimizationSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some(
            "O(V × E × F) successive shortest paths — V = nodes, E = edges, F = total flow; practical for V ≤ 1000",
        )
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|f| {
            f.id().as_str().starts_with(REQUEST_PREFIX)
                && match f.payload::<FlowRequest>() {
                    Some(_) => !plan_exists(ctx, req_id(f.id().as_str())),
                    None => !error_exists(ctx, f.id().as_str()),
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
            match fact.payload::<FlowRequest>() {
                Some(req) => {
                    if plan_exists(ctx, req_id(fact.id().as_str())) {
                        continue;
                    }
                    let plan = solve(req);
                    let confidence = plan.fulfillment;
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Strategies,
                            format!("{}{}", PLAN_PREFIX, plan.request_id),
                            plan.clone(),
                            self.provenance(),
                        )
                        .with_confidence(confidence),
                    );
                }
                None => {
                    if error_exists(ctx, fact.id().as_str()) {
                        continue;
                    }
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Diagnostic,
                            format!("{}{}", ERROR_PREFIX, fact.id()),
                            DiagnosticPayload::new(
                                self.name(),
                                format!(
                                    "malformed flow request '{}': expected {} v{} payload",
                                    fact.id(),
                                    FlowRequest::FAMILY,
                                    FlowRequest::VERSION
                                ),
                            ),
                            self.provenance(),
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

    fn provenance(&self) -> Provenance {
        crate::suggestors::CONVERGE_OPTIMIZATION_PROVENANCE.provenance()
    }
}

// ── Core logic ────────────────────────────────────────────────────────────────

fn solve(req: &FlowRequest) -> FlowPlan {
    if req.edges.is_empty() || req.demand == 0 {
        return FlowPlan {
            request_id: req.id.clone(),
            total_flow: 0,
            total_cost: 0,
            edge_flows: vec![],
            fulfillment: 1.0,
            feasible: true,
        };
    }

    let mut net = FlowNetwork::new(req.num_nodes);
    for edge in &req.edges {
        net.add_edge(edge.from, edge.to, edge.capacity, edge.cost);
    }

    let Ok(problem) = MinCostFlowProblem::source_sink(net, req.source, req.sink, req.demand) else {
        return FlowPlan {
            request_id: req.id.clone(),
            total_flow: 0,
            total_cost: 0,
            edge_flows: vec![0; req.edges.len()],
            fulfillment: 0.0,
            feasible: false,
        };
    };

    match min_cost_flow(&problem) {
        Ok(sol) => {
            let fulfillment = if req.demand > 0 {
                (sol.flow as f64 / req.demand as f64).min(1.0)
            } else {
                1.0
            };
            FlowPlan {
                request_id: req.id.clone(),
                total_flow: sol.flow,
                total_cost: sol.cost,
                edge_flows: sol.edge_flows,
                fulfillment,
                feasible: true,
            }
        }
        Err(_) => FlowPlan {
            request_id: req.id.clone(),
            total_flow: 0,
            total_cost: 0,
            edge_flows: vec![0; req.edges.len()],
            fulfillment: 0.0,
            feasible: false,
        },
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn req_id(fact_id: &str) -> &str {
    fact_id.trim_start_matches(REQUEST_PREFIX)
}

fn plan_exists(ctx: &dyn Context, request_id: &str) -> bool {
    let id = format!("{}{}", PLAN_PREFIX, request_id);
    ctx.get(ContextKey::Strategies)
        .iter()
        .any(|f| f.id().as_str() == id)
}

fn error_exists(ctx: &dyn Context, fact_id: &str) -> bool {
    let id = format!("{}{}", ERROR_PREFIX, fact_id);
    ctx.get(ContextKey::Diagnostic)
        .iter()
        .any(|f| f.id().as_str() == id)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{ContextState, Engine};
    use converge_pack::TextPayload;

    fn two_path_request(demand: i64) -> FlowRequest {
        // Cheap path (cost=2/unit, cap=3): s→a→t
        // Expensive path (cost=10/unit, cap=3): s→b→t
        FlowRequest {
            id: "r1".into(),
            num_nodes: 4,
            edges: vec![
                FlowEdgeSpec {
                    from: 0,
                    to: 1,
                    capacity: 3,
                    cost: 1,
                    label: Some("s→a".into()),
                },
                FlowEdgeSpec {
                    from: 1,
                    to: 3,
                    capacity: 3,
                    cost: 1,
                    label: Some("a→t".into()),
                },
                FlowEdgeSpec {
                    from: 0,
                    to: 2,
                    capacity: 3,
                    cost: 5,
                    label: Some("s→b".into()),
                },
                FlowEdgeSpec {
                    from: 2,
                    to: 3,
                    capacity: 3,
                    cost: 5,
                    label: Some("b→t".into()),
                },
            ],
            source: 0,
            sink: 3,
            demand,
        }
    }

    #[tokio::test]
    async fn cheap_path_used_first() {
        // demand=3: all flow via cheap path, cost = 3×2 = 6
        let mut engine = Engine::new();
        engine.register_suggestor(FlowOptimizationSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "flow-request:r1",
            two_path_request(3),
            converge_pack::ProvenanceSource::provenance(
                crate::suggestors::CONVERGE_OPTIMIZATION_PROVENANCE,
            ),
        ))
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        let facts = result.context.get(ContextKey::Strategies);
        assert_eq!(facts.len(), 1);
        let plan = facts[0].require_payload::<FlowPlan>().unwrap();
        assert_eq!(plan.total_flow, 3);
        assert_eq!(plan.total_cost, 6);
        assert!((plan.fulfillment - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn overflow_uses_expensive_path() {
        // demand=4: 3 cheap (cost 6) + 1 expensive (cost 10) = 16
        let mut engine = Engine::new();
        engine.register_suggestor(FlowOptimizationSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "flow-request:r1",
            two_path_request(4),
            converge_pack::ProvenanceSource::provenance(
                crate::suggestors::CONVERGE_OPTIMIZATION_PROVENANCE,
            ),
        ))
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        let plan = result.context.get(ContextKey::Strategies)[0]
            .require_payload::<FlowPlan>()
            .unwrap();
        assert_eq!(plan.total_flow, 4);
        assert_eq!(plan.total_cost, 16, "3×2 + 1×10 = 16");
    }

    #[tokio::test]
    async fn result_is_idempotent() {
        let mut engine = Engine::new();
        engine.register_suggestor(FlowOptimizationSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "flow-request:r1",
            two_path_request(3),
            converge_pack::ProvenanceSource::provenance(
                crate::suggestors::CONVERGE_OPTIMIZATION_PROVENANCE,
            ),
        ))
        .unwrap();

        let first = engine.run(ctx).await.unwrap();
        let mut engine2 = Engine::new();
        engine2.register_suggestor(FlowOptimizationSuggestor);
        let second = engine2.run(first.context.clone()).await.unwrap();
        assert_eq!(
            second.context.get(ContextKey::Strategies).len(),
            first.context.get(ContextKey::Strategies).len(),
        );
    }

    #[tokio::test]
    async fn malformed_request_emits_diagnostic() {
        let mut engine = Engine::new();
        engine.register_suggestor(FlowOptimizationSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "flow-request:bad",
            TextPayload::new("not a flow request"),
            converge_pack::ProvenanceSource::provenance(
                crate::suggestors::CONVERGE_OPTIMIZATION_PROVENANCE,
            ),
        ))
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        assert_eq!(result.context.get(ContextKey::Diagnostic).len(), 1);
        assert!(!result.context.has(ContextKey::Strategies));
    }
}
