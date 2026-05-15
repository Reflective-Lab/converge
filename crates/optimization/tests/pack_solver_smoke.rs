// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! End-to-end Pack::solve smoke tests for the three solver packs that
//! shipped without integration coverage in v3.7: Facility Location,
//! Network Flow, and Vehicle Routing.
//!
//! Each pack is exercised through the full Pack lifecycle: validate_inputs,
//! solve, and a structural check on the resulting ProposedPlan payload.

use converge_optimization::packs::Pack;
use converge_optimization::packs::facility_location::{
    Customer, Facility, FacilityLocationInput, FacilityLocationOutput, FacilityLocationPack,
};
use converge_optimization::packs::job_shop_scheduling::{
    Job, JobShopInput, JobShopOutput, JobShopSchedulingPack, Operation,
};
use converge_optimization::packs::network_flow::{
    NetworkEdge, NetworkFlowInput, NetworkFlowOutput, NetworkFlowPack,
};
use converge_optimization::packs::vehicle_routing::{
    VehicleRoutingInput, VehicleRoutingOutput, VehicleRoutingPack,
};
use converge_pack::gate::{ObjectiveSpec, ProblemSpec};
use serde_json::json;

fn spec_for<T: serde::Serialize>(id: &str, inputs: &T) -> ProblemSpec {
    ProblemSpec::builder(id, "tenant-test")
        .objective(ObjectiveSpec::minimize("cost"))
        .seed(42)
        .inputs(inputs)
        .expect("inputs serialize")
        .build()
        .expect("spec builds")
}

// ── Facility Location ────────────────────────────────────────────────────────

fn fl_two_two_input() -> FacilityLocationInput {
    // Two candidate facilities, two customers. Facility 0 is cheap to open
    // and cheap to ship from for both customers — the optimum is to open it
    // alone.
    FacilityLocationInput {
        facilities: vec![
            Facility {
                fixed_cost: 10.0,
                capacity: 100,
            },
            Facility {
                fixed_cost: 1000.0,
                capacity: 100,
            },
        ],
        customers: vec![
            Customer {
                demand: 1,
                transport_costs: vec![1.0, 50.0],
            },
            Customer {
                demand: 1,
                transport_costs: vec![2.0, 60.0],
            },
        ],
    }
}

#[test]
fn facility_location_validates_and_solves_textbook_instance() {
    let pack = FacilityLocationPack;
    let input = fl_two_two_input();

    pack.validate_inputs(&serde_json::to_value(&input).unwrap())
        .expect("input validates");

    let spec = spec_for("fl-textbook", &input);
    let result = pack.solve(&spec).expect("solve succeeds");

    let out: FacilityLocationOutput = result.plan.plan_as().expect("payload deserializes");

    assert_eq!(out.assignments.len(), 2, "every customer must be assigned");
    assert!(
        !out.open_facilities.is_empty(),
        "at least one facility must be opened"
    );
    // Facility 1 is dominated by facility 0 on every dimension; the greedy
    // solver should never open it alone.
    assert!(
        out.open_facilities.contains(&0),
        "facility 0 dominates and must be opened"
    );
    // Optimum: open facility 0, total = 10 (fixed) + 1 + 2 = 13.
    assert!(
        (out.total_cost - 13.0).abs() < 1e-6,
        "expected optimum 13.0, got {}",
        out.total_cost
    );
}

#[test]
fn facility_location_rejects_empty_facilities() {
    let pack = FacilityLocationPack;
    let bad = json!({ "facilities": [], "customers": [{ "demand": 1, "transport_costs": [] }] });
    let err = pack
        .validate_inputs(&bad)
        .expect_err("empty facilities is invalid");
    assert!(err.to_string().contains("facility"));
}

#[test]
fn facility_location_rejects_transport_cost_length_mismatch() {
    let pack = FacilityLocationPack;
    let bad = json!({
        "facilities": [{ "fixed_cost": 1.0, "capacity": 1 }, { "fixed_cost": 1.0, "capacity": 1 }],
        "customers": [{ "demand": 1, "transport_costs": [1.0] }]
    });
    let err = pack
        .validate_inputs(&bad)
        .expect_err("transport_costs length mismatch is invalid");
    assert!(err.to_string().contains("transport_costs"));
}

// ── Network Flow ─────────────────────────────────────────────────────────────

#[test]
fn network_flow_solves_two_path_min_cost_instance() {
    // 4 nodes, two parallel paths from 0 → 3:
    //   0 → 1 (cap 5, cost 1)  → 3 (cap 5, cost 1)   path-A: 2 per unit
    //   0 → 2 (cap 5, cost 5)  → 3 (cap 5, cost 5)   path-B: 10 per unit
    // Demand 5: send everything via path-A. Optimal cost = 5 * 2 = 10.
    let input = NetworkFlowInput {
        nodes: 4,
        edges: vec![
            NetworkEdge {
                from: 0,
                to: 1,
                capacity: 5.0,
                cost: 1.0,
            },
            NetworkEdge {
                from: 1,
                to: 3,
                capacity: 5.0,
                cost: 1.0,
            },
            NetworkEdge {
                from: 0,
                to: 2,
                capacity: 5.0,
                cost: 5.0,
            },
            NetworkEdge {
                from: 2,
                to: 3,
                capacity: 5.0,
                cost: 5.0,
            },
        ],
        source: 0,
        sink: 3,
        demand: 5.0,
    };

    let pack = NetworkFlowPack;
    pack.validate_inputs(&serde_json::to_value(&input).unwrap())
        .expect("input validates");

    let spec = spec_for("nf-two-paths", &input);
    let result = pack.solve(&spec).expect("solve succeeds");

    let out: NetworkFlowOutput = result.plan.plan_as().expect("payload deserializes");

    assert_eq!(out.flows.len(), 4, "one flow value per edge");
    assert!(
        (out.total_flow - 5.0).abs() < 1e-6,
        "must satisfy full demand, got {}",
        out.total_flow
    );
    assert!(
        (out.total_cost - 10.0).abs() < 1e-3,
        "min-cost path totals 10, got {}",
        out.total_cost
    );
    // Sanity: cheap-path edges carry the flow.
    assert!(out.flows[0] > 0.0 && out.flows[1] > 0.0);
}

#[test]
fn network_flow_marks_infeasible_when_demand_exceeds_capacity() {
    // Single path 0 → 1, capacity 3, demand 10. Solver should still produce
    // an output but report less than full flow; the infeasible-path rounding
    // means total_flow < demand.
    let input = NetworkFlowInput {
        nodes: 2,
        edges: vec![NetworkEdge {
            from: 0,
            to: 1,
            capacity: 3.0,
            cost: 1.0,
        }],
        source: 0,
        sink: 1,
        demand: 10.0,
    };

    let pack = NetworkFlowPack;
    let spec = spec_for("nf-infeasible", &input);
    let result = pack
        .solve(&spec)
        .expect("solve returns Ok even on infeasible");
    let out: NetworkFlowOutput = result.plan.plan_as().unwrap();
    assert!(
        out.total_flow < input.demand,
        "infeasible run must under-deliver, got {}",
        out.total_flow
    );
}

#[test]
fn network_flow_rejects_source_equals_sink() {
    let pack = NetworkFlowPack;
    let bad = json!({
        "nodes": 2,
        "edges": [{ "from": 0, "to": 1, "capacity": 1.0, "cost": 1.0 }],
        "source": 0,
        "sink": 0,
        "demand": 1.0
    });
    let err = pack
        .validate_inputs(&bad)
        .expect_err("source == sink is invalid");
    assert!(err.to_string().to_lowercase().contains("source"));
}

// -- Job Shop Scheduling -----------------------------------------------------

#[test]
fn job_shop_pack_schedules_precedence_and_machines() {
    let input = JobShopInput {
        machines: 2,
        jobs: vec![
            Job {
                operations: vec![
                    Operation {
                        machine: 0,
                        duration: 3,
                    },
                    Operation {
                        machine: 1,
                        duration: 2,
                    },
                ],
            },
            Job {
                operations: vec![
                    Operation {
                        machine: 1,
                        duration: 4,
                    },
                    Operation {
                        machine: 0,
                        duration: 1,
                    },
                ],
            },
        ],
    };

    let pack = JobShopSchedulingPack;
    pack.validate_inputs(&serde_json::to_value(&input).unwrap())
        .expect("input validates");

    let spec = spec_for("jsp-two-two", &input);
    let result = pack.solve(&spec).expect("solve succeeds");
    let out: JobShopOutput = result.plan.plan_as().expect("payload deserializes");

    let total_ops: usize = input.jobs.iter().map(|job| job.operations.len()).sum();
    assert_eq!(out.schedule.len(), total_ops);
    assert!(out.makespan > 0);

    for job_idx in 0..input.jobs.len() {
        let mut ops: Vec<_> = out
            .schedule
            .iter()
            .filter(|scheduled| scheduled.job == job_idx)
            .collect();
        ops.sort_by_key(|scheduled| scheduled.operation);
        for pair in ops.windows(2) {
            let prev = pair[0];
            let next = pair[1];
            let prev_duration = input.jobs[prev.job].operations[prev.operation].duration;
            assert!(
                prev.start + prev_duration <= next.start,
                "job precedence must be respected"
            );
        }
    }
}

#[test]
fn job_shop_rejects_invalid_machine_reference() {
    let pack = JobShopSchedulingPack;
    let bad = json!({
        "machines": 1,
        "jobs": [{
            "operations": [{ "machine": 99, "duration": 1 }]
        }]
    });
    let err = pack
        .validate_inputs(&bad)
        .expect_err("invalid machine reference is rejected");
    assert!(err.to_string().to_lowercase().contains("machine"));
}

// ── Vehicle Routing ──────────────────────────────────────────────────────────

#[test]
fn vehicle_routing_serves_all_customers_within_capacity() {
    // Depot at origin; four customers at unit-distance corners of a square.
    // Each demand = 1. Vehicle capacity = 2 ⇒ at least 2 routes.
    let input = VehicleRoutingInput {
        depot: (0.0, 0.0),
        customers: vec![(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)],
        vehicle_capacity: 2,
        demands: vec![1, 1, 1, 1],
    };

    let pack = VehicleRoutingPack;
    pack.validate_inputs(&serde_json::to_value(&input).unwrap())
        .expect("input validates");

    let spec = spec_for("vrp-square", &input);
    let result = pack.solve(&spec).expect("solve succeeds");
    let out: VehicleRoutingOutput = result.plan.plan_as().expect("payload deserializes");

    let visited: Vec<usize> = out.routes.iter().flatten().copied().collect();
    let mut seen = visited.clone();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), 4, "every customer visited exactly once");
    assert!(
        out.routes.len() >= 2,
        "with capacity 2 and 4 unit demands, need ≥ 2 routes"
    );
    // Each route must respect capacity.
    for route in &out.routes {
        let load: usize = route.iter().map(|&i| input.demands[i]).sum();
        assert!(
            load <= input.vehicle_capacity,
            "route load {} exceeds capacity {}",
            load,
            input.vehicle_capacity
        );
    }
    assert!(out.total_distance > 0.0, "non-trivial routing distance");
}

#[test]
fn vehicle_routing_rejects_demand_above_capacity() {
    let pack = VehicleRoutingPack;
    let bad = json!({
        "depot": [0.0, 0.0],
        "customers": [[1.0, 0.0]],
        "vehicle_capacity": 1,
        "demands": [5]
    });
    let err = pack
        .validate_inputs(&bad)
        .expect_err("per-customer demand above capacity is invalid");
    assert!(err.to_string().to_lowercase().contains("capacity"));
}

#[test]
fn vehicle_routing_rejects_customer_demand_length_mismatch() {
    let pack = VehicleRoutingPack;
    let bad = json!({
        "depot": [0.0, 0.0],
        "customers": [[1.0, 0.0], [0.0, 1.0]],
        "vehicle_capacity": 5,
        "demands": [1]
    });
    let err = pack
        .validate_inputs(&bad)
        .expect_err("customer/demand length mismatch is invalid");
    assert!(err.to_string().to_lowercase().contains("demand"));
}
