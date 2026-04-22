---
source: llm
---

# Logistic Classification (Sigmoid)

**Complexity:** O(n * d)
**Converge module:** `converge_analytics::packs::classification::LogisticClassifier`
**Reference:** Sigmoid function: standard ML classification. Verifiable against any scikit-learn implementation.

## Formula

```
z = w . x + b
p = sigmoid(z) = 1 / (1 + exp(-z))
class = positive if p >= threshold, else negative
```

## Properties of Sigmoid

- sigmoid(0) = 0.5
- sigmoid(x) + sigmoid(-x) = 1.0
- Monotonically increasing
- Maps any real number to (0, 1)

## Worked Example

### Input

Model: sigmoid(3*x1 + 0*x2 - 1.5), threshold = 0.5

Weights: [3, 0], Bias: -1.5

### Record 1: [1, 0]

z = 3(1) + 0(0) - 1.5 = 1.5
p = 1 / (1 + exp(-1.5)) = 1 / (1 + 0.22313) = 1 / 1.22313 = **0.8176**
Class: positive (0.8176 >= 0.5)

### Record 2: [0, 0]

z = 3(0) + 0(0) - 1.5 = -1.5
p = 1 / (1 + exp(1.5)) = 1 / (1 + 4.48169) = 1 / 5.48169 = **0.1824**
Class: negative (0.1824 < 0.5)

### Verification

p([1,0]) + p([0,0]) = 0.8176 + 0.1824 = 1.0 (because z values are symmetric: +1.5 and -1.5).

### Result

| Record | z    | p      | Class    |
|--------|------|--------|----------|
| [1,0]  | 1.5  | 0.8176 | positive |
| [0,0]  | -1.5 | 0.1824 | negative |

1 positive, 1 negative.

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation logistic
```

Both probabilities and classifications confirmed.
