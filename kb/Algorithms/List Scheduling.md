---
source: llm
---

# List Scheduling

**Complexity:** O(n log n) for sorting + O(n) for scheduling
**Converge module:** `converge_optimization::scheduling::list_schedule`
**Reference:** Graham, R. L. (1969). "Bounds on multiprocessing timing anomalies." *SIAM Journal on Applied Mathematics*, 17(2), 416-429.

## Problem

Given n tasks with earliest start times, latest end times, and durations, schedule them on machines (disjunctive = one machine, no overlap) to minimize makespan (completion time of the last task).

## Algorithm Steps (Earliest Due Date)

1. **Sort** tasks by latest end time (EDD rule).
2. **Assign** each task in order to the earliest available time slot, respecting the earliest start constraint.
3. **Track** the current time on each machine.

For a single machine (disjunctive scheduling), this reduces to sequencing all tasks one after another.

## Worked Example

### Input

| Task | Earliest Start | Latest End | Duration |
|------|---------------|------------|----------|
|  0   |       0       |     10     |    3     |
|  1   |       0       |     10     |    2     |
|  2   |       0       |     10     |    4     |

1 machine, no overlap allowed.

### Step-by-step

All tasks have the same deadline (10), so order is stable.

| Time | Action          |
|------|-----------------|
|  0   | Start task 0    |
|  3   | Finish task 0, start task 1 |
|  5   | Finish task 1, start task 2 |
|  9   | Finish task 2   |

### Result

**Makespan = 3 + 2 + 4 = 9.**

All tasks complete by time 9, which is within the latest end time of 10.

## Converge Validation

```
cargo test -p converge-optimization --test reference_validation scheduling
```

Makespan = 9 confirmed.
