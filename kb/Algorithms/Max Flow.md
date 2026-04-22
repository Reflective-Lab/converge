---
source: llm
---

# Maximum Flow (Ford-Fulkerson / Edmonds-Karp)

**Complexity:** O(VE^2) for Edmonds-Karp (BFS-based Ford-Fulkerson)
**Converge module:** `converge_optimization::graph::flow::max_flow`
**Reference:** Ford, L. R.; Fulkerson, D. R. (1956). "Maximal flow through a network." *Canadian Journal of Mathematics*, 8, 399-404.

## Problem

Given a directed graph where each edge has a capacity, find the maximum flow from source s to sink t such that flow on each edge does not exceed its capacity and flow is conserved at every intermediate node.

## Algorithm Steps (Edmonds-Karp variant)

1. **Initialize** -- Set flow on all edges to 0.
2. **Find augmenting path** -- Use BFS to find a path from s to t in the residual graph (edges with remaining capacity > 0).
3. **Augment** -- Find the bottleneck (minimum residual capacity along the path). Push that much flow along the path.
4. **Update residual graph** -- Decrease forward capacity, increase reverse capacity.
5. **Repeat** -- Until no augmenting path exists.

## Max-Flow Min-Cut Theorem

The maximum flow from s to t equals the minimum cut capacity separating s from t. This is one of the fundamental theorems of combinatorial optimization.

## Worked Example (Sedgewick)

### Input Network

```
s(0) --(10)--> 1 --(10)--> t(3)
s(0) --(10)--> 2 --(10)--> t(3)
  1  --(1)-->  2
```

4 nodes. Source = 0, Sink = 3.

### Step-by-step

**Iteration 1:** BFS finds path s->1->t. Bottleneck = min(10, 10) = 10. Push 10.
**Iteration 2:** BFS finds path s->2->t. Bottleneck = min(10, 10) = 10. Push 10.
**Iteration 3:** BFS finds no augmenting path (s->1 saturated, s->2 saturated).

The cross-edge 1->2 (capacity 1) is immaterial because both parallel paths are already saturated at the source.

### Result

**Max flow = 20.**

Min cut: {s} vs {1, 2, t}. Cut edges: s->1 (10) + s->2 (10) = 20. Confirms max-flow = min-cut.

## Converge Validation

```
cargo test -p converge-optimization --test reference_validation max_flow
```

Max flow = 20 confirmed.
