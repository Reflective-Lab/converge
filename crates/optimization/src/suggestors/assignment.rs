// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Optimal assignment via the Hungarian algorithm (O(n³)).
//!
//! Reads an [`AssignmentRequest`] from context, solves the linear-sum
//! assignment problem, and proposes an [`AssignmentPlan`] to
//! [`ContextKey::Strategies`].
//!
//! # Formation role
//!
//! Seed a request once; every downstream suggestor that needs to know who
//! does what reads the plan from `ContextKey::Strategies`. If cost estimates
//! change (e.g. a capacity suggestor updates constraints), re-seed with a new
//! request id — the suggestor reacts and the formation re-converges.

use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use serde::{Deserialize, Serialize};

use crate::assignment::{AssignmentProblem, hungarian};

// ── Request ───────────────────────────────────────────────────────────────────

/// Seed this under [`ContextKey::Seeds`] with id prefix `"assignment-request:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentRequest {
    /// Stable identifier for idempotency.
    pub id: String,
    /// Labels for the agents (rows). Length must equal `costs.len()`.
    pub agents: Vec<String>,
    /// Labels for the tasks (columns). Length must equal `costs[i].len()`.
    pub tasks: Vec<String>,
    /// Cost matrix: `costs[agent][task]`. Must be square (n×n).
    pub costs: Vec<Vec<i64>>,
}

// ── Plan (output) ─────────────────────────────────────────────────────────────

/// The optimal assignment produced by the suggestor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignmentPlan {
    pub request_id: String,
    /// `(agent_label, task_label)` pairs, one per matched agent.
    pub assignments: Vec<(String, String)>,
    pub total_cost: i64,
    /// `assignments.len() / agents.len()` — 1.0 means fully matched.
    pub utilization: f64,
}

// ── Suggestor ─────────────────────────────────────────────────────────────────

const REQUEST_PREFIX: &str = "assignment-request:";
const PLAN_PREFIX: &str = "assignment-plan:";
const ERROR_PREFIX: &str = "assignment-request-error:";

/// Solves a linear-sum assignment problem using the Hungarian algorithm.
///
/// Registers as a zero-configuration unit — no injected state required.
pub struct AssignmentSuggestor;

#[async_trait]
impl Suggestor for AssignmentSuggestor {
    fn name(&self) -> &str {
        "AssignmentSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("O(n³) Hungarian algorithm — n = agents = tasks; practical for n ≤ 500")
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|f| {
            f.id().as_str().starts_with(REQUEST_PREFIX)
                && match serde_json::from_str::<AssignmentRequest>(f.content()) {
                    Ok(_) => !plan_exists(ctx, req_id(f.id().as_str())),
                    Err(_) => !error_exists(ctx, f.id().as_str()),
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
            match serde_json::from_str::<AssignmentRequest>(fact.content()) {
                Ok(req) => {
                    if plan_exists(ctx, req_id(fact.id().as_str())) {
                        continue;
                    }
                    let plan = solve(&req);
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Strategies,
                            format!("{}{}", PLAN_PREFIX, plan.request_id),
                            serde_json::to_string(&plan).unwrap_or_default(),
                            self.name(),
                        )
                        .with_confidence(plan.utilization),
                    );
                }
                Err(e) => {
                    if error_exists(ctx, fact.id().as_str()) {
                        continue;
                    }
                    let diag = serde_json::json!({
                        "request_fact_id": fact.id(),
                        "message": "malformed assignment request",
                        "error": e.to_string(),
                    });
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Diagnostic,
                            format!("{}{}", ERROR_PREFIX, fact.id()),
                            diag.to_string(),
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

// ── Core logic ────────────────────────────────────────────────────────────────

fn solve(req: &AssignmentRequest) -> AssignmentPlan {
    if req.agents.is_empty() {
        return AssignmentPlan {
            request_id: req.id.clone(),
            assignments: vec![],
            total_cost: 0,
            utilization: 1.0,
        };
    }

    let problem = AssignmentProblem::from_costs(req.costs.clone());
    if problem.validate().is_err() {
        return AssignmentPlan {
            request_id: req.id.clone(),
            assignments: vec![],
            total_cost: 0,
            utilization: 0.0,
        };
    }

    match hungarian::solve(&problem) {
        Ok(sol) => {
            let assignments = sol
                .assignments
                .iter()
                .enumerate()
                .map(|(agent_idx, &task_idx)| {
                    (
                        req.agents.get(agent_idx).cloned().unwrap_or_default(),
                        req.tasks.get(task_idx).cloned().unwrap_or_default(),
                    )
                })
                .collect::<Vec<_>>();
            let n = assignments.len();
            AssignmentPlan {
                request_id: req.id.clone(),
                assignments,
                total_cost: sol.total_cost,
                utilization: n as f64 / req.agents.len() as f64,
            }
        }
        Err(_) => AssignmentPlan {
            request_id: req.id.clone(),
            assignments: vec![],
            total_cost: 0,
            utilization: 0.0,
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

    fn req_json(id: &str, costs: Vec<Vec<i64>>) -> String {
        let n = costs.len();
        serde_json::to_string(&AssignmentRequest {
            id: id.to_string(),
            agents: (0..n).map(|i| format!("agent-{i}")).collect(),
            tasks: (0..n).map(|i| format!("task-{i}")).collect(),
            costs,
        })
        .unwrap()
    }

    #[tokio::test]
    async fn textbook_3x3_finds_optimal_cost() {
        // Taha 3×3: optimal = 9
        let mut engine = Engine::new();
        engine.register_suggestor(AssignmentSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_input(
            ContextKey::Seeds,
            "assignment-request:r1",
            req_json("r1", vec![vec![9, 2, 7], vec![6, 4, 3], vec![5, 8, 1]]),
        )
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        let plans = result.context.get(ContextKey::Strategies);
        assert_eq!(plans.len(), 1);
        let plan: AssignmentPlan = serde_json::from_str(plans[0].content()).unwrap();
        assert_eq!(plan.total_cost, 9, "optimal cost = 9");
        assert_eq!(plan.assignments.len(), 3);
        assert!((plan.utilization - 1.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn result_is_idempotent() {
        let mut engine = Engine::new();
        engine.register_suggestor(AssignmentSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_input(
            ContextKey::Seeds,
            "assignment-request:r1",
            req_json("r1", vec![vec![9, 2, 7], vec![6, 4, 3], vec![5, 8, 1]]),
        )
        .unwrap();

        let first = engine.run(ctx).await.unwrap();
        let mut engine2 = Engine::new();
        engine2.register_suggestor(AssignmentSuggestor);
        let second = engine2.run(first.context.clone()).await.unwrap();
        assert_eq!(
            second.context.get(ContextKey::Strategies).len(),
            first.context.get(ContextKey::Strategies).len(),
        );
    }

    #[tokio::test]
    async fn malformed_request_emits_diagnostic() {
        let mut engine = Engine::new();
        engine.register_suggestor(AssignmentSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_input(ContextKey::Seeds, "assignment-request:bad", "{")
            .unwrap();

        let result = engine.run(ctx).await.unwrap();
        assert_eq!(result.context.get(ContextKey::Diagnostic).len(), 1);
        assert!(!result.context.has(ContextKey::Strategies));
    }
}
