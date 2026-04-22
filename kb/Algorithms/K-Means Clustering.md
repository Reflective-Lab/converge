---
source: llm
---

# K-Means Clustering

**Complexity:** O(n * k * d * I) where n = points, k = clusters, d = dimensions, I = iterations
**Converge module:** `converge_analytics::packs::segmentation::KMeansSolver`
**Reference:** Lloyd, S. P. (1982). "Least squares quantization in PCM." *IEEE Transactions on Information Theory*, 28(2), 129-137.

## Algorithm Steps

1. **Initialize** -- Select k initial centroids (random or seeded).
2. **Assign** -- Assign each point to the nearest centroid (Euclidean distance).
3. **Update** -- Recompute each centroid as the mean of its assigned points.
4. **Repeat** -- Steps 2-3 until assignments don't change or max iterations reached.

## Convergence

K-means always converges (the objective function decreases monotonically), but may converge to a local minimum. Well-separated clusters are found reliably regardless of initialization.

## Worked Example

### Input

6 points in 2D, k=2:

| Point | x    | y    |
|-------|------|------|
|  P0   | 0.0  | 0.0  |
|  P1   | 1.0  | 0.0  |
|  P2   | 0.0  | 1.0  |
|  P3   | 10.0 | 10.0 |
|  P4   | 11.0 | 10.0 |
|  P5   | 10.0 | 11.0 |

### Step-by-step (assuming sensible initialization)

**After convergence:**

Cluster A = {P0, P1, P2}
- Centroid = ((0+1+0)/3, (0+0+1)/3) = **(0.333, 0.333)**

Cluster B = {P3, P4, P5}
- Centroid = ((10+11+10)/3, (10+10+11)/3) = **(10.333, 10.333)**

The clusters are separated by distance ~14.1 (Euclidean), while within-cluster distances are ~1.0. K-means finds this trivially.

### Result

| Point | Cluster |
|-------|---------|
| P0    |    A    |
| P1    |    A    |
| P2    |    A    |
| P3    |    B    |
| P4    |    B    |
| P5    |    B    |

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation kmeans
```

Correct cluster assignments and centroid positions confirmed.
