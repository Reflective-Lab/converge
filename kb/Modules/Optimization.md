---
tags: [modules, optimization]
source: mixed
---
# Optimization Module

`converge-optimization` provides OR-Tools-backed solvers exposed as Suggestors
via the `SolverSuggestor` adapter.

## Available Solvers (11)

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

## Usage in a Formation

```rust
use converge_optimization::SolverSuggestor;

let solver = SolverSuggestor::budget_allocation(BudgetAllocationInput {
    total_budget: 1_000_000.0,
    categories: vec![...],
    constraints: vec![...],
});

engine.register_suggestor(solver);
```

## How It Works

1. Seeder writes problem input to `ContextKey::Seeds`
2. SolverSuggestor's `accepts()` checks if seeds contain its problem type
3. `execute()` runs OR-Tools solver (milliseconds for typical problems)
4. Proposes solution to `ContextKey::Strategies`
5. Other agents (policy gates, skeptics) can challenge the solution
6. If challenged, solver may re-run with tighter constraints

## Feature Flags

- Default: CP-SAT solver (pure Rust fallback)
- `sat` feature: Varisat SAT solver
- `ffi` feature: OR-Tools C++ FFI (full solver suite)

## Constraint Programming (CP) Module

Also provides a general CP solver for custom constraint problems:
- Variable declarations (int, bool, interval)
- Constraint expressions
- Objective functions (minimize/maximize)

See also: [[Architecture/Suggestor Contract]], [[Architecture/Formation Pattern]]
