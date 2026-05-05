// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Portfolio selection via 0-1 Knapsack DP.
//!
//! Reads a [`PortfolioRequest`] from context — a set of candidate items each
//! with a weight (cost/effort) and value (benefit/ROI) — and proposes a
//! [`PortfolioSelection`] that maximises value within the given budget.
//!
//! # Formation role
//!
//! Downstream suggestors that plan execution or allocate resources read the
//! selected item labels from `ContextKey::Strategies`. A budget suggestor that
//! revises the available capacity re-seeds the request; the formation re-runs
//! and converges on the new optimum.

use async_trait::async_trait;
use converge_pack::{AgentEffect, Context, ContextKey, ProposedFact, Suggestor};
use serde::{Deserialize, Serialize};

use crate::knapsack::{self, KnapsackProblem};

// ── Request ───────────────────────────────────────────────────────────────────

/// A portfolio optimisation request. Seed under [`ContextKey::Seeds`] with id
/// prefix `"portfolio-request:"`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioRequest {
    /// Stable identifier for idempotency.
    pub id: String,
    /// Candidate initiatives, investments, or features to select from.
    pub items: Vec<PortfolioItem>,
    /// Total budget (capacity). Same unit as `PortfolioItem::weight`.
    pub budget: i64,
}

/// A single candidate item in the portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioItem {
    pub label: String,
    /// Resource consumption (cost, effort, capital, story points, …).
    pub weight: i64,
    /// Expected benefit (ROI, value, impact score, …).
    pub value: i64,
}

// ── Selection (output) ────────────────────────────────────────────────────────

/// The optimal portfolio selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioSelection {
    pub request_id: String,
    /// Labels of the selected items.
    pub selected: Vec<String>,
    pub total_value: i64,
    pub total_weight: i64,
    /// `total_weight / budget` — how much of the budget is consumed.
    pub utilization: f64,
}

// ── Suggestor ─────────────────────────────────────────────────────────────────

const REQUEST_PREFIX: &str = "portfolio-request:";
const SELECTION_PREFIX: &str = "portfolio-selection:";
const ERROR_PREFIX: &str = "portfolio-request-error:";

/// Selects an optimal portfolio of items under a budget constraint using 0-1
/// Knapsack dynamic programming.
pub struct PortfolioSuggestor;

#[async_trait]
impl Suggestor for PortfolioSuggestor {
    fn name(&self) -> &str {
        "PortfolioSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("O(n × W) 0-1 Knapsack DP — n = items, W = budget; avoid W > 10⁶")
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|f| {
            f.id().as_str().starts_with(REQUEST_PREFIX)
                && match serde_json::from_str::<PortfolioRequest>(f.content()) {
                    Ok(_) => !selection_exists(ctx, req_id(f.id().as_str())),
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
            match serde_json::from_str::<PortfolioRequest>(fact.content()) {
                Ok(req) => {
                    if selection_exists(ctx, req_id(fact.id().as_str())) {
                        continue;
                    }
                    let selection = solve(&req);
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Strategies,
                            format!("{}{}", SELECTION_PREFIX, selection.request_id),
                            serde_json::to_string(&selection).unwrap_or_default(),
                            self.name(),
                        )
                        .with_confidence(selection.utilization.min(1.0)),
                    );
                }
                Err(e) => {
                    if error_exists(ctx, fact.id().as_str()) {
                        continue;
                    }
                    let diag = serde_json::json!({
                        "request_fact_id": fact.id(),
                        "message": "malformed portfolio request",
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

fn solve(req: &PortfolioRequest) -> PortfolioSelection {
    if req.items.is_empty() {
        return PortfolioSelection {
            request_id: req.id.clone(),
            selected: vec![],
            total_value: 0,
            total_weight: 0,
            utilization: 0.0,
        };
    }

    let weights: Vec<i64> = req.items.iter().map(|i| i.weight).collect();
    let values: Vec<i64> = req.items.iter().map(|i| i.value).collect();

    let Ok(problem) = KnapsackProblem::new(weights, values, req.budget) else {
        return PortfolioSelection {
            request_id: req.id.clone(),
            selected: vec![],
            total_value: 0,
            total_weight: 0,
            utilization: 0.0,
        };
    };

    match knapsack::solve(&problem) {
        Ok(sol) => {
            let selected = sol
                .selected
                .iter()
                .filter_map(|&idx| req.items.get(idx).map(|i| i.label.clone()))
                .collect();
            let utilization = if req.budget > 0 {
                sol.total_weight as f64 / req.budget as f64
            } else {
                0.0
            };
            PortfolioSelection {
                request_id: req.id.clone(),
                selected,
                total_value: sol.total_value,
                total_weight: sol.total_weight,
                utilization,
            }
        }
        Err(_) => PortfolioSelection {
            request_id: req.id.clone(),
            selected: vec![],
            total_value: 0,
            total_weight: 0,
            utilization: 0.0,
        },
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn req_id(fact_id: &str) -> &str {
    fact_id.trim_start_matches(REQUEST_PREFIX)
}

fn selection_exists(ctx: &dyn Context, request_id: &str) -> bool {
    let id = format!("{}{}", SELECTION_PREFIX, request_id);
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

    fn req_json(id: &str, items: Vec<(&str, i64, i64)>, budget: i64) -> String {
        serde_json::to_string(&PortfolioRequest {
            id: id.to_string(),
            items: items
                .into_iter()
                .map(|(label, weight, value)| PortfolioItem {
                    label: label.to_string(),
                    weight,
                    value,
                })
                .collect(),
            budget,
        })
        .unwrap()
    }

    #[tokio::test]
    async fn five_item_clrs_variant() {
        // Weights [2,3,4,5,9], values [3,4,5,8,10], capacity 20 → optimal 26
        let mut engine = Engine::new();
        engine.register_suggestor(PortfolioSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_input(
            ContextKey::Seeds,
            "portfolio-request:r1",
            req_json(
                "r1",
                vec![
                    ("alpha", 2, 3),
                    ("beta", 3, 4),
                    ("gamma", 4, 5),
                    ("delta", 5, 8),
                    ("epsilon", 9, 10),
                ],
                20,
            ),
        )
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        let facts = result.context.get(ContextKey::Strategies);
        assert_eq!(facts.len(), 1);
        let sel: PortfolioSelection = serde_json::from_str(facts[0].content()).unwrap();
        assert_eq!(sel.total_value, 26, "optimal portfolio value = 26");
        assert!(sel.total_weight <= 20);
    }

    #[tokio::test]
    async fn result_is_idempotent() {
        let mut engine = Engine::new();
        engine.register_suggestor(PortfolioSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_input(
            ContextKey::Seeds,
            "portfolio-request:r1",
            req_json("r1", vec![("a", 2, 5), ("b", 3, 6), ("c", 4, 4)], 5),
        )
        .unwrap();

        let first = engine.run(ctx).await.unwrap();
        let mut engine2 = Engine::new();
        engine2.register_suggestor(PortfolioSuggestor);
        let second = engine2.run(first.context.clone()).await.unwrap();
        assert_eq!(
            second.context.get(ContextKey::Strategies).len(),
            first.context.get(ContextKey::Strategies).len(),
        );
    }

    #[tokio::test]
    async fn malformed_request_emits_diagnostic() {
        let mut engine = Engine::new();
        engine.register_suggestor(PortfolioSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_input(ContextKey::Seeds, "portfolio-request:bad", "not-json")
            .unwrap();

        let result = engine.run(ctx).await.unwrap();
        assert_eq!(result.context.get(ContextKey::Diagnostic).len(), 1);
        assert!(!result.context.has(ContextKey::Strategies));
    }
}
