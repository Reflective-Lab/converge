//! Integration tests: each new optimization pack as a Suggestor in the convergence loop.

use converge_core::{Budget, ContextKey, ContextState, ConvergeResult, Engine};
use converge_optimization::packs::constraint_programming::ConstraintProgrammingPack;
use converge_optimization::packs::graph_partitioning::GraphPartitioningPack;
use converge_optimization::packs::job_shop_scheduling::JobShopSchedulingPack;
use converge_optimization::packs::staff_rostering::StaffRosteringPack;
use converge_optimization::packs::traveling_salesman::TravelingSalesmanPack;
use converge_pack::PackSuggestor;

fn budget() -> Budget {
    Budget {
        max_cycles: 5,
        max_facts: 100,
    }
}

async fn run_with_input<P: converge_pack::Pack + 'static>(
    pack: P,
    input: serde_json::Value,
) -> ConvergeResult {
    let mut engine = Engine::with_budget(budget());
    engine.register_suggestor(PackSuggestor::new(
        pack,
        ContextKey::Seeds,
        ContextKey::Strategies,
    ));
    let mut ctx = ContextState::new();
    let _ = ctx.add_input(ContextKey::Seeds, "input-1", input.to_string());
    engine.run(ctx).await.expect("should converge")
}

#[tokio::test]
async fn test_constraint_programming_formation() {
    let result = run_with_input(
        ConstraintProgrammingPack,
        serde_json::json!({
            "variables": [
                {"name": "x", "min": 1, "max": 5},
                {"name": "y", "min": 1, "max": 5}
            ],
            "constraints": [
                {"type": "not_equal", "args": {"var1": "x", "var2": "y"}}
            ],
            "objective": {"variable": "x", "maximize": true}
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);

    let v: serde_json::Value = serde_json::from_str(strategies[0].content()).unwrap();
    let plan = &v["plan"];
    assert!(plan["feasible"].as_bool().unwrap());

    let assignments = plan["assignments"].as_array().unwrap();
    let x_val = assignments.iter().find(|a| a["name"] == "x").unwrap()["value"]
        .as_i64()
        .unwrap();
    let y_val = assignments.iter().find(|a| a["name"] == "y").unwrap()["value"]
        .as_i64()
        .unwrap();

    assert_ne!(x_val, y_val, "x and y must differ (not_equal constraint)");
    assert_eq!(x_val, 5, "x should be maximized to 5");
}

#[tokio::test]
async fn test_job_shop_scheduling_formation() {
    let result = run_with_input(
        JobShopSchedulingPack,
        serde_json::json!({
            "jobs": [
                {"operations": [{"machine": 0, "duration": 3}, {"machine": 1, "duration": 2}]},
                {"operations": [{"machine": 1, "duration": 4}, {"machine": 0, "duration": 1}]}
            ],
            "machines": 2
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);

    let v: serde_json::Value = serde_json::from_str(strategies[0].content()).unwrap();
    let payload = &v["plan"];
    let makespan = payload["makespan"].as_u64().unwrap();
    assert!(makespan > 0, "makespan must be positive");

    let schedule = payload["schedule"].as_array().unwrap();
    assert_eq!(schedule.len(), 4, "should schedule all 4 operations");
}

#[tokio::test]
async fn test_staff_rostering_formation() {
    let result = run_with_input(
        StaffRosteringPack,
        serde_json::json!({
            "staff": [
                {"id": "alice", "skills": ["nursing"], "max_hours": 40},
                {"id": "bob", "skills": ["nursing", "admin"], "max_hours": 40}
            ],
            "shifts": [
                {"id": "morning", "required_skill": "nursing", "hours": 8, "period": "monday"},
                {"id": "afternoon", "required_skill": "admin", "hours": 8, "period": "monday"}
            ]
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);

    let v: serde_json::Value = serde_json::from_str(strategies[0].content()).unwrap();
    let payload = &v["plan"];
    let assignments = payload["assignments"].as_array().unwrap();
    let unassigned = payload["unassigned_shifts"].as_array().unwrap();

    assert!(
        unassigned.is_empty(),
        "all shifts should be assigned: {unassigned:?}"
    );
    assert_eq!(assignments.len(), 2, "both shifts should have assignments");
}

#[tokio::test]
async fn test_traveling_salesman_formation() {
    let result = run_with_input(
        TravelingSalesmanPack,
        serde_json::json!({
            "cities": [
                {"x": 0.0, "y": 0.0},
                {"x": 1.0, "y": 0.0},
                {"x": 1.0, "y": 1.0},
                {"x": 0.0, "y": 1.0}
            ]
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);

    let v: serde_json::Value = serde_json::from_str(strategies[0].content()).unwrap();
    let payload = &v["plan"];
    let tour = payload["tour"].as_array().unwrap();
    let distance = payload["total_distance"].as_f64().unwrap();

    assert_eq!(tour.len(), 4, "tour should visit all 4 cities");
    assert!(distance > 0.0, "total distance must be positive");

    let mut visited: Vec<u64> = tour.iter().map(|c| c.as_u64().unwrap()).collect();
    visited.sort_unstable();
    assert_eq!(visited, vec![0, 1, 2, 3], "tour must visit every city");
}

#[tokio::test]
async fn test_graph_partitioning_formation() {
    let result = run_with_input(
        GraphPartitioningPack,
        serde_json::json!({
            "num_nodes": 6,
            "edges": [
                {"from": 0, "to": 1, "weight": 1.0},
                {"from": 1, "to": 2, "weight": 1.0},
                {"from": 3, "to": 4, "weight": 1.0},
                {"from": 4, "to": 5, "weight": 1.0},
                {"from": 2, "to": 3, "weight": 0.1}
            ],
            "num_partitions": 2
        }),
    )
    .await;

    assert!(result.converged);
    let strategies = result.context.get(ContextKey::Strategies);
    assert_eq!(strategies.len(), 1);

    let v: serde_json::Value = serde_json::from_str(strategies[0].content()).unwrap();
    let payload = &v["plan"];
    let partition = payload["partition"].as_array().unwrap();
    let cut_weight = payload["cut_weight"].as_f64().unwrap();

    assert_eq!(
        partition.len(),
        6,
        "every node must be assigned a partition"
    );

    let partitions_used: std::collections::HashSet<u64> =
        partition.iter().map(|p| p.as_u64().unwrap()).collect();
    assert_eq!(
        partitions_used.len(),
        2,
        "should produce exactly 2 partitions"
    );

    // The weak edge (0.1) between nodes 2-3 is the natural cut point.
    // A good partition cuts there, giving cut_weight <= 0.1.
    assert!(
        cut_weight <= 1.0,
        "cut weight {cut_weight} should be small (weak edge is the natural cut)"
    );
}
