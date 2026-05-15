// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Greedy single-vehicle routing with time windows.
//!
//! Reads a [`VrptwRequest`] from context and proposes a [`VrptwPlan`]. This is
//! the portable pure Rust baseline for Ferrox native CP-SAT VRPTW solving.

use async_trait::async_trait;
use converge_pack::ProvenanceSource;
use converge_pack::{
    AgentEffect, Context, ContextKey, DiagnosticPayload, FactPayload, ProposedFact, Suggestor,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

// -- Request -----------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VrptwCustomer {
    pub id: usize,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub window_open: i64,
    pub window_close: i64,
    pub service_time: i64,
}

impl VrptwCustomer {
    pub fn travel_to(&self, other: &Self) -> f64 {
        euclidean((self.x, self.y), (other.x, other.y))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VrptwDepot {
    pub x: f64,
    pub y: f64,
    pub ready_time: i64,
    pub due_time: i64,
}

impl VrptwDepot {
    pub fn travel_to_customer(&self, customer: &VrptwCustomer) -> f64 {
        euclidean((self.x, self.y), (customer.x, customer.y))
    }
}

/// Seed under [`ContextKey::Seeds`] with id prefix `"vrptw-request:"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VrptwRequest {
    pub id: String,
    pub depot: VrptwDepot,
    pub customers: Vec<VrptwCustomer>,
    #[serde(default = "default_time_limit")]
    pub time_limit_seconds: f64,
}

impl FactPayload for VrptwRequest {
    const FAMILY: &'static str = "converge.optimization.vrptw.request";
    const VERSION: u16 = 1;
}

fn default_time_limit() -> f64 {
    30.0
}

// -- Plan --------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteStop {
    pub customer_id: usize,
    pub customer_name: String,
    pub arrival: i64,
    pub departure: i64,
}

/// Written to [`ContextKey::Strategies`] with id prefix `"vrptw-plan-greedy:"`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VrptwPlan {
    pub request_id: String,
    pub route: Vec<RouteStop>,
    pub customers_total: usize,
    pub customers_visited: usize,
    pub total_distance: f64,
    pub return_time: i64,
    pub solver: String,
    pub status: String,
    pub wall_time_seconds: f64,
}

impl FactPayload for VrptwPlan {
    const FAMILY: &'static str = "converge.optimization.vrptw.plan";
    const VERSION: u16 = 1;
}

impl VrptwPlan {
    #[allow(clippy::cast_precision_loss)]
    pub fn visit_ratio(&self) -> f64 {
        if self.customers_total == 0 {
            return 0.0;
        }
        self.customers_visited as f64 / self.customers_total as f64
    }
}

// -- Suggestor ---------------------------------------------------------------

const REQUEST_PREFIX: &str = "vrptw-request:";
const PLAN_PREFIX: &str = "vrptw-plan-greedy:";
const ERROR_PREFIX: &str = "vrptw-request-error:";

/// Routes a single vehicle via nearest-neighbor search with time-window
/// feasibility checks.
pub struct NearestNeighborTimeWindowRoutingSuggestor;

#[async_trait]
impl Suggestor for NearestNeighborTimeWindowRoutingSuggestor {
    fn name(&self) -> &str {
        "NearestNeighborTimeWindowRoutingSuggestor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn complexity_hint(&self) -> Option<&'static str> {
        Some("O(n^2) nearest-neighbor with time-window feasibility")
    }

    fn accepts(&self, ctx: &dyn Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|f| {
            f.id().as_str().starts_with(REQUEST_PREFIX)
                && match f.payload::<VrptwRequest>() {
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
            match fact.payload::<VrptwRequest>() {
                Some(req) => {
                    if plan_exists(ctx, req_id(fact.id().as_str())) {
                        continue;
                    }
                    let plan = solve_nearest_neighbor_time_windows(req);
                    let confidence = (plan.visit_ratio() * 0.60).min(0.60);
                    proposals.push(
                        ProposedFact::new(
                            ContextKey::Strategies,
                            format!("{}{}", PLAN_PREFIX, plan.request_id),
                            plan.clone(),
                            self.name().to_string(),
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
                                    "malformed vrptw request '{}': expected {} v{} payload",
                                    fact.id(),
                                    VrptwRequest::FAMILY,
                                    VrptwRequest::VERSION
                                ),
                            ),
                            self.name().to_string(),
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

    fn provenance(&self) -> &'static str {
        super::CONVERGE_OPTIMIZATION_PROVENANCE.as_str()
    }
}

// -- Core logic --------------------------------------------------------------

pub fn solve_nearest_neighbor_time_windows(req: &VrptwRequest) -> VrptwPlan {
    let t0 = Instant::now();
    let n = req.customers.len();
    let mut visited = vec![false; n];
    let mut route = Vec::new();

    let mut cur_x = req.depot.x;
    let mut cur_y = req.depot.y;
    let mut cur_t = req.depot.ready_time;
    let mut total_distance = 0.0_f64;

    loop {
        let best = (0..n)
            .filter(|&i| !visited[i])
            .filter_map(|i| {
                let customer = &req.customers[i];
                if customer.service_time < 0 || customer.window_close < customer.window_open {
                    return None;
                }

                let distance = euclidean((cur_x, cur_y), (customer.x, customer.y));
                #[allow(clippy::cast_possible_truncation)]
                let travel_time = distance.ceil() as i64;
                let arrival = cur_t + travel_time;
                let departure = arrival.max(customer.window_open) + customer.service_time;
                let return_distance =
                    euclidean((customer.x, customer.y), (req.depot.x, req.depot.y));
                #[allow(clippy::cast_possible_truncation)]
                let return_time = return_distance.ceil() as i64;

                if arrival <= customer.window_close && departure + return_time <= req.depot.due_time
                {
                    Some((i, distance, arrival, departure))
                } else {
                    None
                }
            })
            .min_by(|a, b| a.1.total_cmp(&b.1));

        match best {
            Some((i, distance, arrival, departure)) => {
                let customer = &req.customers[i];
                visited[i] = true;
                total_distance += distance;
                cur_x = customer.x;
                cur_y = customer.y;
                cur_t = departure;
                route.push(RouteStop {
                    customer_id: customer.id,
                    customer_name: customer.name.clone(),
                    arrival: arrival.max(customer.window_open),
                    departure,
                });
            }
            None => break,
        }
    }

    let return_distance = euclidean((cur_x, cur_y), (req.depot.x, req.depot.y));
    total_distance += return_distance;
    #[allow(clippy::cast_possible_truncation)]
    let return_time = cur_t + return_distance.ceil() as i64;
    let customers_visited = route.len();
    let status = if n == 0 || customers_visited > 0 {
        "feasible"
    } else {
        "infeasible"
    };

    VrptwPlan {
        request_id: req.id.clone(),
        route,
        customers_total: n,
        customers_visited,
        total_distance,
        return_time,
        solver: "nearest-neighbor-tw".to_string(),
        status: status.to_string(),
        wall_time_seconds: t0.elapsed().as_secs_f64(),
    }
}

fn euclidean(a: (f64, f64), b: (f64, f64)) -> f64 {
    let dx = a.0 - b.0;
    let dy = a.1 - b.1;
    (dx * dx + dy * dy).sqrt()
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{ContextState, Engine};
    use converge_pack::TextPayload;
    use proptest::prelude::*;
    use std::collections::BTreeSet;

    fn depot(due_time: i64) -> VrptwDepot {
        VrptwDepot {
            x: 0.0,
            y: 0.0,
            ready_time: 0,
            due_time,
        }
    }

    fn customer(id: usize, x: f64, open: i64, close: i64) -> VrptwCustomer {
        VrptwCustomer {
            id,
            name: format!("customer-{id}"),
            x,
            y: 0.0,
            window_open: open,
            window_close: close,
            service_time: 1,
        }
    }

    fn req(customers: Vec<VrptwCustomer>, due_time: i64) -> VrptwRequest {
        VrptwRequest {
            id: "route-1".to_string(),
            depot: depot(due_time),
            customers,
            time_limit_seconds: 1.0,
        }
    }

    #[tokio::test]
    async fn suggestor_emits_time_window_route() {
        let request = req(
            vec![customer(1, 1.0, 0, 100), customer(2, 2.0, 0, 100)],
            1000,
        );

        let mut engine = Engine::new();
        engine.register_suggestor(NearestNeighborTimeWindowRoutingSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "vrptw-request:route-1",
            request,
            "test",
        ))
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        let facts = result.context.get(ContextKey::Strategies);
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].id().as_str(), "vrptw-plan-greedy:route-1");
        let plan = facts[0].require_payload::<VrptwPlan>().unwrap();
        assert_eq!(plan.customers_visited, 2);
    }

    #[tokio::test]
    async fn malformed_request_emits_diagnostic() {
        let mut engine = Engine::new();
        engine.register_suggestor(NearestNeighborTimeWindowRoutingSuggestor);

        let mut ctx = ContextState::new();
        ctx.add_proposal(ProposedFact::new(
            ContextKey::Seeds,
            "vrptw-request:bad",
            TextPayload::new("not a vrptw request"),
            "test",
        ))
        .unwrap();

        let result = engine.run(ctx).await.unwrap();
        assert!(result.context.get(ContextKey::Strategies).is_empty());
        assert_eq!(result.context.get(ContextKey::Diagnostic).len(), 1);
    }

    #[test]
    fn unreachable_customer_is_not_visited() {
        let plan =
            solve_nearest_neighbor_time_windows(&req(vec![customer(1, 1000.0, 0, 1)], 10_000));
        assert_eq!(plan.customers_visited, 0);
        assert_eq!(plan.status, "infeasible");
    }

    proptest! {
        #[test]
        fn route_has_unique_feasible_stops(customer_count in 0usize..30) {
            let customers: Vec<_> = (0..customer_count)
                .map(|i| customer(i, (i + 1) as f64, 0, 10_000))
                .collect();
            let request = req(customers.clone(), 100_000);
            let plan = solve_nearest_neighbor_time_windows(&request);

            let mut seen = BTreeSet::new();
            for stop in &plan.route {
                prop_assert!(seen.insert(stop.customer_id));
                let customer = customers
                    .iter()
                    .find(|customer| customer.id == stop.customer_id)
                    .expect("route stop must refer to a known customer");
                prop_assert!(stop.arrival >= customer.window_open);
                prop_assert!(stop.arrival <= customer.window_close);
                prop_assert_eq!(stop.departure, stop.arrival + customer.service_time);
            }

            prop_assert!(plan.customers_visited <= plan.customers_total);
            prop_assert!(plan.return_time <= request.depot.due_time);
        }
    }
}
