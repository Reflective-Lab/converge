---
source: llm
---

# 0-1 Knapsack (Dynamic Programming)

**Complexity:** O(nW) where n = items, W = capacity (pseudo-polynomial)
**Converge module:** `converge_optimization::knapsack`
**Reference:** Martello, S.; Toth, P. (1990). *Knapsack Problems: Algorithms and Computer Implementations.* Wiley.

## Problem

Given n items, each with weight w_i and value v_i, and a knapsack of capacity W, select a subset of items to maximize total value without exceeding capacity. Each item is either taken (1) or not (0).

## Algorithm Steps

1. **Create DP table** -- dp[i][w] = max value using items 1..i with capacity w.
2. **Base case** -- dp[0][w] = 0 for all w (no items = no value).
3. **Recurrence** -- For each item i and capacity w:
   - If w_i > w: dp[i][w] = dp[i-1][w] (item doesn't fit)
   - Else: dp[i][w] = max(dp[i-1][w], dp[i-1][w - w_i] + v_i)
4. **Answer** -- dp[n][W].
5. **Backtrack** -- Trace which items were selected.

## Worked Example 1: 5 Items (CLRS variant)

### Input

| Item | Weight | Value |
|------|--------|-------|
|  1   |   2    |   3   |
|  2   |   3    |   4   |
|  3   |   4    |   5   |
|  4   |   5    |   8   |
|  5   |   9    |  10   |

Capacity = 20.

### Solution by enumeration

Try promising subsets:
- Items {1,2,3,4}: weight=14, value=20. Room for item 5? 14+9=23 > 20. No.
- Items {1,2,4,5}: weight=2+3+5+9=19, value=3+4+8+10=25. Fits.
- Items {1,3,4,5}: weight=2+4+5+9=20, value=3+5+8+10=**26**. Fits exactly!
- Items {2,3,4,5}: weight=3+4+5+9=21 > 20. No.

**Optimal: items {1,3,4,5}, value = 26, weight = 20.**

## Worked Example 2: Rosetta Code (12 items)

### Input

| Item     | Weight | Value |
|----------|--------|-------|
| map      |   9    |  150  |
| compass  |  13    |   35  |
| water    | 153    |  200  |
| sandwich |  50    |  160  |
| glucose  |  15    |   60  |
| tin      |  68    |   45  |
| banana   |  27    |   60  |
| apple    |  39    |   40  |
| cheese   |  23    |   30  |
| beer     |  52    |   10  |
| cream    |  11    |   70  |
| camera   |  32    |   30  |

Capacity = 400.

### Solution

Take all items except tin (68, 45) and beer (52, 10):
- Weight: 9+13+153+50+15+27+39+23+11+32 = 372
- Value: 150+35+200+160+60+60+40+30+70+30 = **835**

Remaining capacity: 400-372 = 28. Neither tin (68) nor beer (52) fits.

## Why it matters for agents

**Business decision:** What do we fund. Any time a budget cap forces a binary choice — take this initiative or not, ship this feature or not, hire this person or not — and each option has a cost and an expected return, this is the algorithm.

Typical decisions: Q3 project portfolio selection, sprint backlog prioritization under velocity cap, marketing campaign mix under budget constraint.

**Formation arc — quarterly initiative selection**

A strategy formation receives 12 candidate initiatives from an LLM ideation suggestor, each with an estimated effort (story points) and expected business value (0–100 score). The engineering capacity cap is 80 story points for the quarter.

```
Seeds ← "portfolio-request:q3-initiatives"
  budget: 80
  items:
    ("API redesign",       weight: 20, value: 40)
    ("Mobile app",         weight: 35, value: 55)
    ("Data pipeline",      weight: 15, value: 30)
    ("Customer portal",    weight: 25, value: 50)
    ("Analytics dashboard",weight: 10, value: 25)
    ... (7 more)
```

`PortfolioSuggestor` runs and writes:

```
Strategies ← "portfolio-selection:q3-initiatives"
  selected: ["Mobile app", "Customer portal", "Data pipeline",
             "Analytics dashboard"]
  total_value: 160
  total_weight: 85 → wait, 35+25+15+10 = 85 > 80
```

Actually the DP finds the tighter-fitting subset: it will try all 2¹² = 4096 combinations implicitly, finding the exact subset that fits within 80 while maximizing value. A downstream `WorkScheduleSuggestor` then schedules the selected items across the quarter. The formation converges when both selection and schedule are stable.

**Why the math matters:** A product manager eyeballing the list typically picks high-value items first (greedy by value), often leaving expensive-but-valuable items out while cheaper-but-mediocre items slip in. DP guarantees the true optimum. For 12 items the difference in value is typically 10–20%. At company scale, that compounds.

## Converge Validation

```
cargo test -p converge-optimization --test reference_validation knapsack
```

Both cases (5-item=26, 12-item=835) pass.
