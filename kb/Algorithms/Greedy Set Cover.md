---
source: llm
---

# Greedy Set Cover

**Complexity:** O(m * n) where m = number of sets, n = universe size
**Approximation ratio:** O(ln n) -- at most H(n) times optimal, where H(n) is the n-th harmonic number
**Converge module:** `converge_optimization::setcover`
**Reference:** Chvatal, V. (1979). "A greedy heuristic for the set-covering problem." *Mathematics of Operations Research*, 4(3), 233-235.

## Problem

Given a universe U = {1, 2, ..., n} and a collection of sets S1, S2, ..., Sm each with a cost, find a minimum-cost sub-collection that covers every element in U.

Set cover is NP-hard. The greedy algorithm achieves the best possible polynomial-time approximation ratio (unless P=NP).

## Algorithm Steps

1. **Initialize** -- Uncovered = U. Selected = empty.
2. **Pick best set** -- Choose the set with the lowest cost-effectiveness ratio: cost(S) / |S intersect Uncovered|.
3. **Update** -- Add the chosen set to Selected. Remove its elements from Uncovered.
4. **Repeat** -- Until Uncovered is empty.

## Worked Example

### Input

Universe U = {0, 1, 2, 3, 4}

| Set | Elements   | Cost |
|-----|------------|------|
| S0  | {0, 1, 2}  |  3   |
| S1  | {2, 3}     |  2   |
| S2  | {3, 4}     |  2   |
| S3  | {0,1,2,3,4}|  6   |

### Step-by-step

**Iteration 1:** Cost-effectiveness ratios:
- S0: 3/3 = 1.0 per element
- S1: 2/2 = 1.0 per element
- S2: 2/2 = 1.0 per element
- S3: 6/5 = 1.2 per element

Pick S0 (cost 1.0, covers most elements among ties). Uncovered = {3, 4}.

**Iteration 2:** Cost-effectiveness for remaining:
- S1: covers {3} -> 2/1 = 2.0
- S2: covers {3, 4} -> 2/2 = 1.0
- S3: covers {3, 4} -> 6/2 = 3.0

Pick S2 (cost 1.0/elem). Uncovered = {}.

### Result

Selected: {S0, S2}. **Total cost = 3 + 2 = 5.**

This happens to be the optimal solution. The greedy guarantee says cost <= H(5) x OPT = (1 + 1/2 + 1/3 + 1/4 + 1/5) x 5 = 2.283 x 5 = 11.4, so cost <= 11.

## Converge Validation

```
cargo test -p converge-optimization --test reference_validation set_cover
```

Greedy cost = 5 (optimal) confirmed.
