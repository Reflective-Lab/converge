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

## Why it matters for agents

**Business decision:** When does each task happen. After a formation has settled who does what (Hungarian) and what to do at all (Knapsack), it still needs to answer: in what order, on which resource, starting when. That is this algorithm.

Typical decisions: sprint sequencing, manufacturing job scheduling, onboarding task sequencing for new hires, service appointment booking.

**Formation arc — post-assignment sprint planning**

`AssignmentSuggestor` has already converged and written `assignment-plan:sprint-14` to Strategies. A translation suggestor reads the plan and constructs a schedule request, mapping each (agent, task) pair to a task with duration and deadline constraints.

```
Seeds ← "schedule-request:sprint-14"
  capacity: None   ← one engineer per task (disjunctive)
  tasks:
    ("auth-redesign",   earliest_start: 0,  latest_end: 10, duration: 3)
    ("api-gateway",     earliest_start: 0,  latest_end: 14, duration: 5)
    ("data-migration",  earliest_start: 3,  latest_end: 14, duration: 4)
    ("mobile-login",    earliest_start: 0,  latest_end: 10, duration: 2)
```

`WorkScheduleSuggestor` runs EDD (earliest deadline first) and writes:

```
Strategies ← "schedule-plan:sprint-14"
  makespan: 14
  efficiency: 0.0     ← makespan / max_window = 14/14
  scheduled:
    ("mobile-login",     start: 0,  end: 2)
    ("auth-redesign",    start: 2,  end: 5)
    ("api-gateway",      start: 5,  end: 10)
    ("data-migration",   start: 10, end: 14)
```

If `makespan = -1` (infeasible — a task cannot be scheduled within its window), the low confidence triggers a re-plan: the assignment suggestor can propose different agent-task pairings, and the formation re-converges to a feasible schedule.

**Why the math matters:** Random sequencing can violate deadline constraints and produce a longer makespan. EDD is provably optimal for minimizing maximum lateness on a single machine. A formation that misses deadlines is useless; this algorithm ensures it doesn't.

## Converge Validation

```
cargo test -p converge-optimization --test reference_validation scheduling
```

Makespan = 9 confirmed.
