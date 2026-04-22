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

## Why it matters for agents

**Business decision:** Will this happen or not — and how confident are we. Logistic classification produces a probability, not just a label, which makes it more useful than a hard rule: the probability becomes the `confidence` of a ProposedFact, letting the engine and downstream suggestors reason about uncertainty rather than treating every prediction as equally reliable.

Typical decisions: will this customer churn this month, will this loan default, will this applicant pass the screening, is this transaction fraudulent.

**Formation arc — churn prediction → intervention routing**

A customer success formation runs every night. For each at-risk account, it classifies churn probability using a pre-fitted logistic model, then routes high-probability accounts to an intervention suggestor.

```
Constraints ← "churn-model:enterprise-tier"
  weights: [3.0, -2.0, 1.5]   ← login_freq, support_tickets, nps_score
  bias: -1.5
  threshold: 0.5

Seeds ← "accounts:renewal-check-2026-04"
  [("acme-corp",   x=[0, 3, 2]),   ← low logins, 3 tickets, low NPS
   ("globex-inc",  x=[1, 0, 4]),   ← active, no tickets, high NPS
   ("initech-llc", x=[0, 1, 2])]
```

A `ChurnClassifierSuggestor` runs and writes:

```
Signals ← "churn-risk:2026-04"
  [("acme-corp",   p=0.83, class=positive),   ← high churn risk
   ("globex-inc",  p=0.12, class=negative),
   ("initech-llc", p=0.54, class=positive)]   ← borderline

Proposals ← "intervention:acme-corp"
  content: "schedule executive business review"
  confidence: 0.83   ← probability becomes confidence
```

The borderline case (0.54) gets a lower-confidence proposal — a human-in-the-loop gate fires before committing the intervention, since the model isn't sure. High-confidence cases (0.83) auto-route. The formation converges with differentiated responses calibrated to actual risk level.

**Why the math matters:** A hard threshold rule ("flag any account with > 2 support tickets") has no sense of degree. An account with 3 tickets and high NPS is very different from one with 3 tickets and low NPS. Logistic regression captures the interaction and produces a probability the whole formation can act on proportionally.

## Converge Validation

```
cargo test -p converge-analytics --test reference_validation logistic
```

Both probabilities and classifications confirmed.
