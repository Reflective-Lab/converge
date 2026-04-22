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

## Converge Validation

```
cargo test -p converge-optimization --test reference_validation dijkstra
```

All distances (7, 9, 11) confirmed.
