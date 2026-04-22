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

## Why it matters for agents

**Business decision:** What is the maximum throughput our network can sustain, and where is the bottleneck. Max flow answers both simultaneously — the max-flow min-cut theorem tells you not just the ceiling but exactly which edges are saturating it.

Typical decisions: can our distribution network handle peak demand, what is the maximum concurrent caseload a service tier can support, which team is the bottleneck in a delivery pipeline.

**Formation arc — capacity sufficiency check**

Before committing to a demand plan, a formation checks whether the logistics network can physically deliver it. A capacity suggestor seeds a max-flow problem modeling the full supply chain:

```
Seeds ← "flow-request:capacity-check-peak"
  demand: 200      ← peak weekly units
  edges:
    (factory → hub-A, cap=80)
    (factory → hub-B, cap=70)
    (hub-A   → region-1, cap=50)
    (hub-A   → region-2, cap=40)
    (hub-B   → region-2, cap=30)
    (hub-B   → region-3, cap=60)
```

`FlowOptimizationSuggestor` (which uses max-flow internally for feasibility checks) finds:

```
Strategies ← "flow-plan:capacity-check-peak"
  total_flow: 150         ← only 150 of 200 units can be delivered
  fulfillment: 0.75       ← confidence = 0.75
  feasible: true          ← network is feasible, just capacity-limited
```

The low confidence (0.75) signals to a downstream suggestor that demand cannot be met. The min-cut (which edges are saturated) is embedded in the `edge_flows` — any edge at 100% capacity is a cut edge. A response suggestor can propose adding capacity on the bottleneck edges and re-seed a new flow request. The formation converges when fulfillment reaches 1.0 or the plan is accepted at reduced scale.

**Why the math matters:** A planner checking each edge manually cannot see that adding capacity to hub-A alone does nothing if hub-B is also saturated. Max-flow sees the global constraint simultaneously.

## Converge Validation

```
cargo test -p converge-optimization --test reference_validation max_flow
```

Max flow = 20 confirmed.
