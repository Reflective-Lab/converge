---
source: llm
---

# Dijkstra's Shortest Path

**Complexity:** O((V + E) log V) with binary heap
**Converge module:** `converge_optimization::graph::dijkstra`
**Reference:** Dijkstra, E. W. (1959). "A note on two problems in connexion with graphs." *Numerische Mathematik*, 1, 269-271.

## Problem

Given a weighted directed graph with non-negative edge weights, find the shortest path from a source vertex to all other vertices.

## Algorithm Steps

1. **Initialize** -- Set dist[source] = 0, dist[v] = infinity for all other v. Insert source into priority queue.
2. **Extract minimum** -- Remove vertex u with smallest dist[u] from the queue.
3. **Relax neighbors** -- For each neighbor v of u: if dist[u] + weight(u,v) < dist[v], update dist[v] and insert v into queue.
4. **Repeat** -- Continue until queue is empty.
5. **Result** -- dist[v] contains shortest distance from source to v.

## Worked Example (Sedgewick, "Algorithms")

### Input Graph

```
0 --(7)--> 1 --(10)--> 3
0 --(9)--> 2 --(1)---> 1
                --(2)---> 3
```

Edges: 0->1 (7), 0->2 (9), 1->3 (10), 2->1 (1), 2->3 (2).

### Step-by-step from source=0

| Step | Extract | dist[0] | dist[1] | dist[2] | dist[3] |
|------|---------|---------|---------|---------|---------|
| Init |    -    |    0    |   inf   |   inf   |   inf   |
|  1   |    0    |    0    |    7    |    9    |   inf   |
|  2   |    1    |    0    |    7    |    9    |   17    |
|  3   |    2    |    0    |    7    |    9    |   11    |
|  4   |    3    |    0    |    7    |    9    |   11    |

**Step 1:** Extract 0 (dist=0). Relax: 0->1 gives 7, 0->2 gives 9.
**Step 2:** Extract 1 (dist=7). Relax: 1->3 gives 7+10=17.
**Step 3:** Extract 2 (dist=9). Relax: 2->1 gives 9+1=10 (no improvement over 7), 2->3 gives 9+2=11 (improves 17->11).
**Step 4:** Extract 3 (dist=11). No outgoing edges.

### Result

| Destination | Distance | Path |
|-------------|----------|------|
| 0 -> 1      |    7     | 0 -> 1 |
| 0 -> 2      |    9     | 0 -> 2 |
| 0 -> 3      |   11     | 0 -> 2 -> 3 |

Note: The path 0->1->3 costs 17, but 0->2->3 costs only 11.

## Why it matters for agents

**Business decision:** What is the cheapest path from where we are to where we want to be — through a graph of options, dependencies, or service hops. Unlike flow (which routes volume) or assignment (which matches sets), Dijkstra answers a single-query path question.

Typical decisions: routing a customer support escalation through a service tier graph, finding the fastest capability chain for a multi-step agent task, resolving the cheapest dependency path in a project plan.

**Formation arc — FormationAssemblySuggestor**

`FormationAssemblySuggestor` (the built-in formation self-assembly suggestor) builds a graph where nodes are available agent roles and edges represent "agent A can hand off to agent B at a certain transition cost" based on capability overlap scores. Dijkstra finds the cheapest end-to-end chain from the entry role to the resolution role.

For a customer intake formation with 6 possible agent roles:

```
Graph:
  intake(0) --(2)--> triage(1) --(5)--> specialist(3)
  intake(0) --(9)--> generalist(2) --(1)--> triage(1)
                                  --(2)--> specialist(3)
```

Dijkstra from `intake` finds:
- intake → triage → specialist: cost 2 + 5 = **7**
- intake → generalist → specialist: cost 9 + 2 = **11**
- intake → generalist → triage → specialist: cost 9 + 1 + 5 = **15**

The optimal chain costs 7. The formation assembles along that path and proposes the role assignments to Strategies. Agents not on the optimal path are not activated.

**Why the math matters:** Without Dijkstra, a formation would have to evaluate all possible chains (exponential in the number of roles). With it, the cheapest formation assembles in O((V + E) log V) regardless of how many roles exist — the formation self-selects its own minimal structure.

## Converge Validation

```
cargo test -p converge-optimization --test reference_validation dijkstra
```

All distances (7, 9, 11) confirmed.
