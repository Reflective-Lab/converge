---
source: llm
---

# Hopcroft-Karp Bipartite Matching

**Complexity:** O(E * sqrt(V))
**Converge module:** `converge_optimization::graph::matching::bipartite_matching`
**Reference:** Hopcroft, J. E.; Karp, R. M. (1973). "An n^(5/2) Algorithm for Maximum Matchings in Bipartite Graphs." *SIAM Journal on Computing*, 2(4), 225-231.

## Problem

Given a bipartite graph G = (L union R, E) with left vertices L, right vertices R, and edges E between them, find a maximum cardinality matching -- the largest set of edges with no shared vertices.

## Algorithm Steps

1. **Initialize** -- Start with empty matching M.
2. **BFS phase** -- From all free (unmatched) left vertices simultaneously, do BFS along alternating paths (unmatched edge, matched edge, unmatched edge, ...). Stop at the first layer that reaches a free right vertex.
3. **DFS phase** -- For each free left vertex, attempt to find an augmenting path via DFS along the BFS layers. Augment the matching along each path found.
4. **Repeat** -- Go to step 2 until no augmenting paths exist.

The key insight: finding all shortest augmenting paths simultaneously (BFS) before augmenting (DFS) gives the sqrt(V) bound on iterations.

## Worked Example 1: K(3,3) Complete Bipartite Graph

### Input

Left = {L0, L1, L2}, Right = {R0, R1, R2}. Every left connects to every right.

```
L0 -- R0, R1, R2
L1 -- R0, R1, R2
L2 -- R0, R1, R2
```

### Solution

Perfect matching exists. One solution: L0-R0, L1-R1, L2-R2. **Size = 3.**

## Worked Example 2: Greedy Fails

### Input

```
L0 -- R0, R1
L1 -- R0
```

### Step-by-step

A greedy algorithm might match L0-R0 first, leaving L1 with no available partner.

Hopcroft-Karp finds the augmenting path: L1-R0-L0-R1.
- Unmatch L0-R0
- Match L1-R0 and L0-R1

**Maximum matching size = 2** (both left vertices matched).

## Worked Example 3: Konig's Theorem

### Input (star graph)

```
L0 -- R0, R1, R2, R3
L1 -- R4
```

### Solution

Max matching = 2 (e.g., L0-R0, L1-R4).

Konig's theorem: in any bipartite graph, max matching = min vertex cover. Here, min vertex cover = {L0, L1} = 2.

## Converge Validation

```
cargo test -p converge-optimization --test reference_validation hopcroft_karp
```

All three cases (K3,3=3, augmenting=2, Konig=2) pass.
