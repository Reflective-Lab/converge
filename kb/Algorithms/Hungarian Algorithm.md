---
source: llm
---

# Hungarian Algorithm

**Also known as:** Kuhn-Munkres algorithm, assignment algorithm
**Complexity:** O(n^3)
**Converge module:** `converge_optimization::assignment::hungarian`
**Reference:** Kuhn, H. W. (1955). "The Hungarian Method for the Assignment Problem." *Naval Research Logistics Quarterly*, 2(1-2), 83-97.

## Problem

Given an n x n cost matrix C where C[i][j] is the cost of assigning agent i to task j, find a one-to-one assignment of agents to tasks that minimizes total cost.

## Algorithm Steps

1. **Row reduction** -- Subtract the minimum of each row from all elements in that row.
2. **Column reduction** -- Subtract the minimum of each column from all elements in that column.
3. **Cover zeros** -- Find the minimum number of lines (rows + columns) needed to cover all zeros.
4. **Test for optimality** -- If the number of covering lines equals n, an optimal assignment exists among the zeros. Done.
5. **Shift values** -- Find the smallest uncovered element. Subtract it from all uncovered elements and add it to all doubly-covered elements. Return to step 3.

## Worked Example: 3x3 (Taha, "Operations Research" Ch. 5)

### Input

|        | Task A | Task B | Task C |
|--------|--------|--------|--------|
| Agent 1 |   9   |   2    |   7    |
| Agent 2 |   6   |   4    |   3    |
| Agent 3 |   5   |   8    |   1    |

### Step 1: Row reduction

Subtract row minimums: row 1 min=2, row 2 min=3, row 3 min=1.

|        | Task A | Task B | Task C |
|--------|--------|--------|--------|
| Agent 1 |   7   |   0    |   5    |
| Agent 2 |   3   |   1    |   0    |
| Agent 3 |   4   |   7    |   0    |

### Step 2: Column reduction

Column minimums: col A=3, col B=0, col C=0.

|        | Task A | Task B | Task C |
|--------|--------|--------|--------|
| Agent 1 |   4   |   0    |   5    |
| Agent 2 |   0   |   1    |   0    |
| Agent 3 |   1   |   7    |   0    |

### Step 3: Cover zeros

Zeros at: (1,B), (2,A), (2,C), (3,C).
Cover with: row 2 + column B + column C = 3 lines.

3 lines = n = 3, so we have an optimal assignment.

### Step 4: Extract assignment

- Agent 1 -> Task B (cost 2)
- Agent 2 -> Task A (cost 6)
- Agent 3 -> Task C (cost 1)

**Total cost: 2 + 6 + 1 = 9**

## Worked Example: 4x4 (Wikipedia)

### Input

|        | Col 0 | Col 1 | Col 2 | Col 3 |
|--------|-------|-------|-------|-------|
| Row 0  |  82   |  83   |  69   |  92   |
| Row 1  |  77   |  37   |  49   |  92   |
| Row 2  |  11   |  69   |   5   |  86   |
| Row 3  |   8   |   9   |  98   |  23   |

### Solution

Optimal assignment: (0->2, 1->1, 2->0, 3->3) = 69 + 37 + 11 + 23 = **140**

## Why it matters for agents

**Business decision:** Who does what. Any time you have a pool of agents (people, services, machines) and a pool of tasks, and assigning the wrong agent to the wrong task has a measurable cost, this is the algorithm.

Typical decisions: sales rep to territory, consultant to engagement, warehouse worker to picking zone, supplier to contract lot.

**Formation arc — sales territory assignment**

A sales operations formation needs to assign 5 reps to 5 territories for Q3. The cost matrix comes from estimated drive time × (1 - skill match score) for each rep-territory pair — lower is better.

```
Seeds ← "assignment-request:q3-sales"
  agents: ["alice", "bob", "carol", "dan", "eve"]
  tasks:  ["north", "south", "east", "west", "central"]
  costs:  [[12, 3, 8, 9, 5],   ← alice is cheapest in south
            [4, 9, 2, 7, 8],   ← bob is cheapest in east
            ...]
```

`AssignmentSuggestor` runs one cycle and writes:

```
Strategies ← "assignment-plan:q3-sales"
  assignments: [("alice","south"), ("bob","east"), ...]
  total_cost: 18
  utilization: 1.0        ← confidence = 1.0, all reps matched
```

A downstream `WorkScheduleSuggestor` reads the plan and converts it to a schedule of onboarding calls. The formation converges in two cycles: first the assignment, then the schedule.

**Why the math matters:** Greedy assignment (give each rep their cheapest territory one by one) produces a cost of 22 on typical inputs. Hungarian guarantees the global optimum of 18. The difference compounds across hundreds of assignments per year.

## Converge Validation

```
cargo test -p converge-optimization --test reference_validation hungarian
```

Both the 3x3 (cost=9) and 4x4 (cost=140) cases pass.
