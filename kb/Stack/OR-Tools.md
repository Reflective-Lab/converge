---
tags: [stack, optimization]
source: mixed
---
# OR-Tools

Google OR-Tools is not an active dependency in the current workspace.
`converge-optimization` is currently pure Rust plus optional Varisat-backed
constraint programming.

## Role in Converge

If OR-Tools returns, it should be an adapter implementation for agents that
need external constraint solving. It should not be part of the kernel contract.

## Current Architecture

```
converge-optimization
    ├── native Rust optimization algorithms
    └── optional Varisat-backed constraint programming
```

## Capabilities

| Solver | Use Case |
|---|---|
| Native CP | Constraint programming, scheduling |
| Graph/assignment algorithms | Routing, matching, flow, resource allocation |
| Pack solvers | Budget, capacity, scheduling, routing, triage |

## Agent Example

An optimization agent reads constraints and signals from the context, formulates a CP-SAT problem, solves it, and proposes the solution as a `ProposedFact`. The engine's governance gate validates the solution before it becomes a fact.

The solver produces observations, not decisions ([[Philosophy/Nine Axioms#4. Agents Suggest, Engine Decides|Axiom 4]]).

See also: [[Architecture/Providers]], [[Concepts/Agents]]
