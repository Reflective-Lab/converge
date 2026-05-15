---
tags: [modules, optimization]
source: mixed
---
# Optimization Module

`converge-optimization` provides native optimization packs exposed as
`Suggestor`s through `converge-pack::PackSuggestor`. It is pure Rust by
default, with optional Varisat-backed constraint programming through the
`sat` feature.

## Representative Solver Packs

| Problem | ContextKey Read | ContextKey Written |
|---------|----------------|-------------------|
| LeadRouting | Seeds (lead data) | Strategies |
| MeetingScheduler | Seeds (availability) | Strategies |
| BudgetAllocation | Seeds (constraints, categories) | Strategies |
| CapacityPlanning | Seeds (demand forecast) | Strategies |
| InventoryReplenishment | Seeds (stock levels) | Strategies |
| InventoryRebalancing | Seeds (warehouse data) | Strategies |
| AnomalyTriage | Seeds (alerts) | Strategies |
| PricingGuardrails | Seeds (market data) | Strategies |
| ShippingChoice | Seeds (shipment requests) | Strategies |
| VendorShortlist | Seeds (vendor profiles) | Strategies |
| BacklogPrioritization | Seeds (backlog items) | Strategies |

## Portable Suggestor Baselines

`converge-optimization` also exposes pure Rust baseline Suggestors directly
from `converge_optimization::suggestors`:

| Suggestor | Seed prefix | Plan prefix | Role |
|-----------|-------------|-------------|------|
| GreedySchedulerSuggestor | `scheduling-request:*` | `scheduling-plan-greedy:*` | Fast EDF task scheduling with skills and time windows |
| NearestNeighborTimeWindowRoutingSuggestor | `vrptw-request:*` | `vrptw-plan-greedy:*` | Fast single-vehicle time-window routing baseline |
| AssignmentSuggestor | `assignment-request:*` | `assignment-plan:*` | Exact Hungarian assignment |
| FlowOptimizationSuggestor | `flow-request:*` | `flow-plan:*` | Pure Rust min-cost flow baseline |
| WorkScheduleSuggestor | `schedule-request:*` | `schedule-plan:*` | Simple disjunctive/cumulative work scheduling |

Native Ferrox Suggestors should use the same seed shape when they are intended
to compete with these baselines, but they must write distinct solver-prefixed
plan facts so formations can compare candidates by confidence and policy.

## Usage in a Formation

```rust
use converge_pack::{ContextKey, PackSuggestor};
use converge_optimization::packs::budget_allocation::BudgetAllocationPack;

let solver = PackSuggestor::new(
    BudgetAllocationPack,
    ContextKey::Seeds,
    ContextKey::Strategies,
);
engine.register_suggestor(solver);
```

## How It Works

1. Seeder writes problem input to `ContextKey::Seeds`
2. `PackSuggestor` checks whether seeds contain its problem type
3. `execute()` runs the native solver
4. Proposes solution to `ContextKey::Strategies`
5. Other agents (policy gates, skeptics) can challenge the solution
6. If challenged, solver may re-run with tighter constraints

## Feature Flags

- Default: native Rust solvers
- `sat` feature: Varisat SAT solver
- `full` feature: all native optimization features

## Constraint Programming (CP) Module

Also provides a general CP solver for custom constraint problems:
- Variable declarations (int, bool, interval)
- Constraint expressions
- Objective functions (minimize/maximize)

See also: [[Architecture/Suggestor Contract]], [[Architecture/Formation Pattern]]
