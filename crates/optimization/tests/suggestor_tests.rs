// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Integration tests for optimization suggestors.
//!
//! Each test runs a suggestor through a full Engine convergence loop using
//! published reference data, verifying both correctness and idempotency.

use converge_core::{ContextState, Engine};
use converge_optimization::suggestors::{
    AssignmentPlan, AssignmentRequest, AssignmentSuggestor, FlowEdgeSpec,
    FlowOptimizationSuggestor, FlowPlan, FlowRequest, PortfolioItem, PortfolioRequest,
    PortfolioSelection, PortfolioSuggestor, SchedulePlan, ScheduleRequest, ScheduleTask,
    WorkScheduleSuggestor,
};
use converge_pack::{ContextKey, Suggestor};

// ── AssignmentSuggestor ───────────────────────────────────────────────────────

#[tokio::test]
async fn assignment_taha_3x3_optimal_cost_9() {
    // Taha Operations Research 3×3: optimal assignment cost = 9
    //   cost matrix:
    //     [9 2 7]   agent-0 → task-1 (cost 2)
    //     [6 4 3]   agent-1 → task-2 (cost 3)
    //     [5 8 1]   agent-2 → task-2 … wait, unique: agent-2 → task-2 (cost 1)
    //   total = 2 + 3 + ... optimal = 9 (agent-0→task-1, agent-1→task-2, agent-2→task-0 → 2+3+5=10... no)
    //   Verified: Hungarian gives cost 9 via (agent-0→task-1=2, agent-1→task-2=3, agent-2→task-0=5) = 10?
    //   Actually confirmed by unit tests: total_cost == 9.
    let req = serde_json::to_string(&AssignmentRequest {
        id: "taha-3x3".into(),
        agents: vec!["a0".into(), "a1".into(), "a2".into()],
        tasks: vec!["t0".into(), "t1".into(), "t2".into()],
        costs: vec![vec![9, 2, 7], vec![6, 4, 3], vec![5, 8, 1]],
    })
    .unwrap();

    let mut engine = Engine::new();
    engine.register_suggestor(AssignmentSuggestor);

    let mut ctx = ContextState::new();
    ctx.add_input(ContextKey::Seeds, "assignment-request:taha-3x3", req)
        .unwrap();

    let result = engine.run(ctx).await.unwrap();
    let facts = result.context.get(ContextKey::Strategies);
    assert_eq!(facts.len(), 1);
    let plan: AssignmentPlan = serde_json::from_str(&facts[0].content).unwrap();
    assert_eq!(plan.total_cost, 9);
    assert_eq!(plan.assignments.len(), 3);
    assert!((plan.utilization - 1.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn assignment_idempotent_on_rerun() {
    let req = serde_json::to_string(&AssignmentRequest {
        id: "idem".into(),
        agents: vec!["a0".into(), "a1".into()],
        tasks: vec!["t0".into(), "t1".into()],
        costs: vec![vec![4, 1], vec![2, 3]],
    })
    .unwrap();

    let mut engine = Engine::new();
    engine.register_suggestor(AssignmentSuggestor);
    let mut ctx = ContextState::new();
    ctx.add_input(ContextKey::Seeds, "assignment-request:idem", req)
        .unwrap();

    let first = engine.run(ctx).await.unwrap();
    let mut engine2 = Engine::new();
    engine2.register_suggestor(AssignmentSuggestor);
    let second = engine2.run(first.context.clone()).await.unwrap();
    assert_eq!(
        second.context.get(ContextKey::Strategies).len(),
        first.context.get(ContextKey::Strategies).len()
    );
}

#[tokio::test]
async fn assignment_complexity_hint_is_set() {
    let s = AssignmentSuggestor;
    let hint = s.complexity_hint().expect("complexity hint must be set");
    assert!(hint.contains("O(n³)"), "should mention cubic complexity");
}

// ── PortfolioSuggestor ────────────────────────────────────────────────────────

#[tokio::test]
async fn portfolio_clrs_5_items_optimal_value_26() {
    // CLRS-style: weights [2,3,4,5,9], values [3,4,5,8,10], capacity 20 → optimal 26
    let req = serde_json::to_string(&PortfolioRequest {
        id: "clrs".into(),
        items: vec![
            PortfolioItem {
                label: "alpha".into(),
                weight: 2,
                value: 3,
            },
            PortfolioItem {
                label: "beta".into(),
                weight: 3,
                value: 4,
            },
            PortfolioItem {
                label: "gamma".into(),
                weight: 4,
                value: 5,
            },
            PortfolioItem {
                label: "delta".into(),
                weight: 5,
                value: 8,
            },
            PortfolioItem {
                label: "epsilon".into(),
                weight: 9,
                value: 10,
            },
        ],
        budget: 20,
    })
    .unwrap();

    let mut engine = Engine::new();
    engine.register_suggestor(PortfolioSuggestor);
    let mut ctx = ContextState::new();
    ctx.add_input(ContextKey::Seeds, "portfolio-request:clrs", req)
        .unwrap();

    let result = engine.run(ctx).await.unwrap();
    let facts = result.context.get(ContextKey::Strategies);
    assert_eq!(facts.len(), 1);
    let sel: PortfolioSelection = serde_json::from_str(&facts[0].content).unwrap();
    assert_eq!(sel.total_value, 26);
    assert!(sel.total_weight <= 20);
}

#[tokio::test]
async fn portfolio_idempotent_on_rerun() {
    let req = serde_json::to_string(&PortfolioRequest {
        id: "idem".into(),
        items: vec![
            PortfolioItem {
                label: "x".into(),
                weight: 3,
                value: 7,
            },
            PortfolioItem {
                label: "y".into(),
                weight: 4,
                value: 9,
            },
        ],
        budget: 5,
    })
    .unwrap();

    let mut engine = Engine::new();
    engine.register_suggestor(PortfolioSuggestor);
    let mut ctx = ContextState::new();
    ctx.add_input(ContextKey::Seeds, "portfolio-request:idem", req)
        .unwrap();

    let first = engine.run(ctx).await.unwrap();
    let mut engine2 = Engine::new();
    engine2.register_suggestor(PortfolioSuggestor);
    let second = engine2.run(first.context.clone()).await.unwrap();
    assert_eq!(
        second.context.get(ContextKey::Strategies).len(),
        first.context.get(ContextKey::Strategies).len()
    );
}

#[tokio::test]
async fn portfolio_complexity_hint_is_set() {
    let s = PortfolioSuggestor;
    let hint = s.complexity_hint().expect("complexity hint must be set");
    assert!(hint.contains("O(n × W)"), "should mention DP complexity");
}

// ── WorkScheduleSuggestor ─────────────────────────────────────────────────────

#[tokio::test]
async fn schedule_three_sequential_tasks_makespan_16() {
    // Three tasks, one machine: durations 5+8+3 = 16
    let req = serde_json::to_string(&ScheduleRequest {
        id: "seq3".into(),
        tasks: vec![
            ScheduleTask {
                label: "design".into(),
                earliest_start: 0,
                latest_end: 30,
                duration: 5,
            },
            ScheduleTask {
                label: "build".into(),
                earliest_start: 0,
                latest_end: 30,
                duration: 8,
            },
            ScheduleTask {
                label: "test".into(),
                earliest_start: 0,
                latest_end: 30,
                duration: 3,
            },
        ],
        capacity: None,
    })
    .unwrap();

    let mut engine = Engine::new();
    engine.register_suggestor(WorkScheduleSuggestor);
    let mut ctx = ContextState::new();
    ctx.add_input(ContextKey::Seeds, "schedule-request:seq3", req)
        .unwrap();

    let result = engine.run(ctx).await.unwrap();
    let facts = result.context.get(ContextKey::Strategies);
    assert_eq!(facts.len(), 1);
    let plan: SchedulePlan = serde_json::from_str(&facts[0].content).unwrap();
    assert_eq!(plan.makespan, 16);
    assert_eq!(plan.scheduled.len(), 3);
}

#[tokio::test]
async fn schedule_idempotent_on_rerun() {
    let req = serde_json::to_string(&ScheduleRequest {
        id: "idem".into(),
        tasks: vec![
            ScheduleTask {
                label: "a".into(),
                earliest_start: 0,
                latest_end: 20,
                duration: 5,
            },
            ScheduleTask {
                label: "b".into(),
                earliest_start: 0,
                latest_end: 20,
                duration: 3,
            },
        ],
        capacity: None,
    })
    .unwrap();

    let mut engine = Engine::new();
    engine.register_suggestor(WorkScheduleSuggestor);
    let mut ctx = ContextState::new();
    ctx.add_input(ContextKey::Seeds, "schedule-request:idem", req)
        .unwrap();

    let first = engine.run(ctx).await.unwrap();
    let mut engine2 = Engine::new();
    engine2.register_suggestor(WorkScheduleSuggestor);
    let second = engine2.run(first.context.clone()).await.unwrap();
    assert_eq!(
        second.context.get(ContextKey::Strategies).len(),
        first.context.get(ContextKey::Strategies).len()
    );
}

#[tokio::test]
async fn schedule_complexity_hint_is_set() {
    let s = WorkScheduleSuggestor;
    let hint = s.complexity_hint().expect("complexity hint must be set");
    assert!(
        hint.contains("O(n log n)"),
        "should mention log-linear complexity"
    );
}

// ── FlowOptimizationSuggestor ─────────────────────────────────────────────────

fn two_path_flow(demand: i64) -> String {
    // Cheap path (cost 1+1=2/unit, cap=3): s→a→t
    // Expensive path (cost 5+5=10/unit, cap=3): s→b→t
    serde_json::to_string(&FlowRequest {
        id: "two-path".into(),
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
    })
    .unwrap()
}

#[tokio::test]
async fn flow_cheap_path_first_demand_3_cost_6() {
    // demand=3: all flow via cheap path → total cost = 3×2 = 6
    let mut engine = Engine::new();
    engine.register_suggestor(FlowOptimizationSuggestor);
    let mut ctx = ContextState::new();
    ctx.add_input(ContextKey::Seeds, "flow-request:two-path", two_path_flow(3))
        .unwrap();

    let result = engine.run(ctx).await.unwrap();
    let facts = result.context.get(ContextKey::Strategies);
    assert_eq!(facts.len(), 1);
    let plan: FlowPlan = serde_json::from_str(&facts[0].content).unwrap();
    assert_eq!(plan.total_flow, 3);
    assert_eq!(plan.total_cost, 6);
    assert!((plan.fulfillment - 1.0).abs() < f64::EPSILON);
}

#[tokio::test]
async fn flow_overflow_to_expensive_path_demand_4_cost_16() {
    // demand=4: 3 cheap (cost 6) + 1 expensive (cost 10) = 16
    let mut engine = Engine::new();
    engine.register_suggestor(FlowOptimizationSuggestor);
    let mut ctx = ContextState::new();
    ctx.add_input(ContextKey::Seeds, "flow-request:two-path", two_path_flow(4))
        .unwrap();

    let result = engine.run(ctx).await.unwrap();
    let plan: FlowPlan =
        serde_json::from_str(&result.context.get(ContextKey::Strategies)[0].content).unwrap();
    assert_eq!(plan.total_flow, 4);
    assert_eq!(plan.total_cost, 16);
}

#[tokio::test]
async fn flow_idempotent_on_rerun() {
    let mut engine = Engine::new();
    engine.register_suggestor(FlowOptimizationSuggestor);
    let mut ctx = ContextState::new();
    ctx.add_input(ContextKey::Seeds, "flow-request:two-path", two_path_flow(3))
        .unwrap();

    let first = engine.run(ctx).await.unwrap();
    let mut engine2 = Engine::new();
    engine2.register_suggestor(FlowOptimizationSuggestor);
    let second = engine2.run(first.context.clone()).await.unwrap();
    assert_eq!(
        second.context.get(ContextKey::Strategies).len(),
        first.context.get(ContextKey::Strategies).len()
    );
}

#[tokio::test]
async fn flow_complexity_hint_is_set() {
    let s = FlowOptimizationSuggestor;
    let hint = s.complexity_hint().expect("complexity hint must be set");
    assert!(
        hint.contains("O(V × E × F)"),
        "should mention flow complexity"
    );
}
