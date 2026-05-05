---
source: llm
---

# Cosine Similarity

**Complexity:** O(n^2 * d) for all pairs, where n = items, d = dimensions
**Converge module:** `prism::packs::similarity::PairwiseSimilaritySolver`
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

## Why it matters for agents

**Business decision:** How similar are these two things — independent of their magnitude. Cosine similarity measures directional alignment, which makes it the right choice for comparing embeddings, capability profiles, and preference vectors where "how much" matters less than "in what direction."

Typical decisions: which candidate profile best matches a job requirement, which document in the knowledge base is most relevant to this query, which existing customer most resembles this new lead, which agent in the formation has the closest capability profile to the required role.

**Formation arc — capability matching in FormationAssemblySuggestor**

When assembling a formation, `FormationAssemblySuggestor` needs to score how well each available agent's capability vector matches each required role's capability vector. Cosine similarity produces that score — ranging from 0 (no match) to 1.0 (perfect match) — which feeds into the cost matrix for the Hungarian assignment step.

```
Capabilities stored as embeddings (illustrative 4D):
  role "analyst":      [0.8, 0.6, 0.2, 0.1]   ← heavy on analysis, light on execution
  agent "alice":       [0.9, 0.5, 0.3, 0.1]   ← similar profile
  agent "bob":         [0.2, 0.3, 0.9, 0.7]   ← execution-heavy, wrong profile

cos(role, alice) = 0.97   ← near-perfect match
cos(role, bob)   = 0.41   ← poor match

cost matrix entry: cost[alice][analyst] = 1 - 0.97 = 0.03
                   cost[bob][analyst]   = 1 - 0.41 = 0.59
```

The cost matrix is then handed to `AssignmentSuggestor` (Hungarian) which finds the globally optimal assignment across all (agent, role) pairs. Cosine similarity is the bridge from semantic capability description to numerical optimization.

**Why the math matters:** Euclidean distance penalizes magnitude — an agent who says "yes" to 10 skills looks far from an agent who says "yes" to 5, even if the 5 are exactly the right ones. Cosine similarity ignores magnitude and measures only the angle between the two profiles, which is what "fit" actually means.

## Converge Validation

```
cargo test -p prism --test reference_validation cosine_similarity
```

Both cases (1.0/0.0 and 0.7071) confirmed.
