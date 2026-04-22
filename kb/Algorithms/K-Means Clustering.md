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

## Why it matters for agents

**Business decision:** What natural groups exist in this population. Clustering is the exploratory step that precedes targeted action — you cannot tailor a message, price, or service to "all customers" but you can do so meaningfully to "the three segments that naturally emerge from the data."

Typical decisions: customer segmentation before personalized outreach, market segment discovery for product positioning, grouping support tickets by issue type for routing, clustering sales opportunities by deal size and industry.

**Formation arc — customer segmentation → targeted strategy**

A marketing formation receives customer data across two dimensions: average order value (AOV) and purchase frequency. It clusters customers into 3 segments, then a downstream suggestor assigns a different retention strategy to each segment.

```
Seeds ← "cluster-request:customers-q3"
  k: 3
  records:
    [low-AOV, low-freq]   × 200   ← "occasional"
    [high-AOV, low-freq]  × 80    ← "high-value rare"
    [mid-AOV, high-freq]  × 120   ← "loyal regulars"
```

A `ClusteringSuggestor` runs K-means and writes:

```
Strategies ← "cluster-plan:customers-q3"
  clusters:
    0: centroid=(25, 1.2),  size=200   ← occasional
    1: centroid=(180, 0.8), size=80    ← high-value rare
    2: centroid=(65, 6.5),  size=120   ← loyal regulars
```

Three downstream strategy suggestors each read from Strategies, each checking for their respective segment:
- Segment 0 → "win-back email series"
- Segment 1 → "white-glove outreach from account team"
- Segment 2 → "loyalty reward program"

The formation converges when all three strategies are proposed and the assignment plan is complete.

**Why the math matters:** Without clustering, any downstream suggestor has to apply one strategy to all 400 customers. Personalization at segment level consistently outperforms by 20–40% in marketing response rates. The algorithm makes the segments discoverable rather than requiring a human analyst to define them first.

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation kmeans
```

Correct cluster assignments and centroid positions confirmed.
