---
source: llm
---

# Minimum-Cost Flow

**Complexity:** O(V * E * U) for successive shortest paths (U = max flow)
**Converge module:** `converge_optimization::graph::flow::min_cost_flow`
**Reference:** Ahuja, R. K.; Magnanti, T. L.; Orlin, J. B. (1993). *Network Flows: Theory, Algorithms, and Applications.* Prentice Hall.

## Problem

Given a directed graph where each edge has a capacity and a per-unit cost, find a flow of specified value D from source to sink that minimizes total cost while respecting capacity constraints.

## Algorithm Steps (Successive Shortest Paths)

1. **Initialize** -- Zero flow. Remaining demand = D.
2. **Find cheapest augmenting path** -- Use Bellman-Ford or SPFA to find the shortest (cheapest) path from source to sink in the residual graph, where edge weight = cost (forward) or -cost (reverse).
3. **Augment** -- Push as much flow as possible along this path (min of remaining demand and bottleneck capacity).
4. **Update** -- Reduce remaining demand. Update residual graph.
5. **Repeat** -- Until demand is satisfied or no path exists.

## Worked Example

### Input Network

```
0 --(cap=3, cost=1)--> 1 --(cap=3, cost=1)--> 3
0 --(cap=3, cost=5)--> 2 --(cap=3, cost=5)--> 3
```

Source = 0, Sink = 3, Demand = 4 units.

Two parallel paths:
- **Cheap path:** 0->1->3, cost per unit = 1+1 = 2, capacity = 3
- **Expensive path:** 0->2->3, cost per unit = 5+5 = 10, capacity = 3

### Step-by-step

**Iteration 1:** Shortest path = 0->1->3 (cost 2/unit). Bottleneck = min(3, 3) = 3. But demand = 4, so push 3 units. Cost so far = 3 x 2 = 6. Remaining demand = 1.

**Iteration 2:** Cheap path saturated. Shortest path = 0->2->3 (cost 10/unit). Push 1 unit. Cost += 1 x 10 = 10. Remaining demand = 0.

### Result

- **Total flow:** 4
- **Total cost:** 6 + 10 = **16**

The algorithm automatically routes as much flow as possible through the cheapest path before using the expensive one.

## Converge Validation

```
cargo test -p converge-optimization --test reference_validation min_cost_flow
```

Flow = 4, cost = 16 confirmed.
