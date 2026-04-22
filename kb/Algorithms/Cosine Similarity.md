---
source: llm
---

# Cosine Similarity

**Complexity:** O(n^2 * d) for all pairs, where n = items, d = dimensions
**Converge module:** `converge_analytics::packs::similarity::PairwiseSimilaritySolver`
**Reference:** Hand-computable from the definition.

## Formula

```
cos(A, B) = (A . B) / (||A|| * ||B||)
```

Where A . B is the dot product and ||A|| is the Euclidean norm.

Range: [-1, 1]. Value 1 = identical direction, 0 = orthogonal, -1 = opposite.

## Worked Example 1: Axis-Aligned Vectors

### Input

A = [1, 0, 0], B = [1, 0, 0], C = [0, 1, 0]

### Pair A-B (identical)

dot = 1*1 + 0*0 + 0*0 = 1
||A|| = 1, ||B|| = 1
**cos(A,B) = 1/1 = 1.0**

### Pair A-C (orthogonal)

dot = 1*0 + 0*1 + 0*0 = 0
||A|| = 1, ||C|| = 1
**cos(A,C) = 0/1 = 0.0**

### Pair B-C (orthogonal)

Same as A-C since B = A.
**cos(B,C) = 0.0**

## Worked Example 2: 45 Degrees

### Input

A = [1, 1], B = [1, 0]

### Calculation

dot = 1*1 + 1*0 = 1
||A|| = sqrt(1+1) = sqrt(2)
||B|| = sqrt(1+0) = 1
**cos(A,B) = 1 / sqrt(2) = 0.7071...**

This is cos(45 degrees) = 1/sqrt(2), confirming the geometric interpretation: A is at 45 degrees from the x-axis, B is along the x-axis.

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation cosine_similarity
```

Both cases (1.0/0.0 and 0.7071) confirmed.
