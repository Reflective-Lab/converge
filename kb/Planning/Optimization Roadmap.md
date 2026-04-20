---
tags: [planning, optimization]
source: mixed
---
# Optimization Roadmap

Missing Suggestor Packs that need implementation. These are CORE —
the optimization module must cover all major problem families.

## Current (16 Packs)

| Pack | Family | Status |
|------|--------|--------|
| BudgetAllocation | Choose items | Done |
| BacklogPrioritization | Choose items | Done |
| VendorShortlist | Choose items | Done |
| PricingGuardrails | Choose items | Done |
| AssignmentPack | Assign resources | Done |
| LeadRouting | Assign resources | Done |
| MeetingScheduler | Assign resources | Done |
| CapacityPlanning | Assign resources | Done |
| VehicleRouting | Move things | Done |
| ShippingChoice | Move things | Done |
| NetworkFlow | Move things | Done |
| InventoryReplenishment | Move things | Done |
| InventoryRebalancing | Move things | Done |
| FacilityLocation | Decide locations | Done |
| BinPacking | Split groups | Done |
| AnomalyTriage | Choose items | Done |

## Missing (Priority order)

### 1. JobShopSchedulingPack — Assign resources (time)

```
Problem: Schedule N jobs on M machines, each job has ordered operations
Input: jobs: Vec<Job { operations: Vec<Operation { machine, duration }> }>
         machines: usize
Output: schedule: Vec<(job, op, machine, start_time)>, makespan: usize
Solver: Priority dispatch rules (shortest processing time first)
```

Use case: Manufacturing scheduling, build pipelines, CI/CD ordering.

### 2. ConstraintProgrammingPack — General (any constraint problem)

```
Problem: Satisfy constraints and optimize objective over discrete variables
Input: variables: Vec<Variable { name, domain: Range }>,
       constraints: Vec<Constraint>,
       objective: Objective
Output: assignments: HashMap<String, i64>, objective_value: f64
Solver: Wrap existing `cp` module (Varisat/SAT when available, backtracking otherwise)
```

Use case: Universal — any problem expressible as constraints. The most
powerful Pack because it can solve scheduling, assignment, routing, etc.
when domain-specific heuristics aren't enough.

### 3. StaffRosteringPack — Assign resources (people × time)

```
Problem: Assign staff to shifts respecting preferences, skills, regulations
Input: staff: Vec<Staff { skills, preferences, max_hours }>,
       shifts: Vec<Shift { required_skills, duration, period }>,
       regulations: Vec<Regulation>
Output: roster: Vec<(staff_id, shift_id)>, coverage: f64, violations: Vec<String>
Solver: Greedy with constraint satisfaction (skill matching + load balancing)
```

Use case: Healthcare, retail, support teams, any shift-based operation.

### 4. GraphPartitioningPack — Split groups

```
Problem: Partition graph into balanced subsets minimizing edge cuts
Input: nodes: usize, edges: Vec<(usize, usize, f64)>, partitions: usize
Output: assignment: Vec<usize>, cut_weight: f64, balance: f64
Solver: Kernighan-Lin heuristic or spectral (simplified)
```

Use case: Load balancing, microservice decomposition, team splitting.

### 5. TravelingSalesmanPack — Move things (tour)

```
Problem: Find shortest tour visiting all cities exactly once
Input: cities: Vec<(f64, f64)>
Output: tour: Vec<usize>, total_distance: f64
Solver: 2-opt local search from nearest-neighbor start
```

Use case: Route optimization, circuit board drilling, delivery sequencing.

## Implementation Priority

1. **ConstraintProgrammingPack** — highest leverage (solves many problems)
2. **JobShopSchedulingPack** — most common real-world problem after assignment
3. **StaffRosteringPack** — high business value
4. **TravelingSalesmanPack** — classic, many applications
5. **GraphPartitioningPack** — infrastructure use cases

## Design Note

Each Pack wraps a heuristic solver. Exact algorithms (MIP, CP-SAT) are
available behind the `sat` and `ffi` features for users who need optimality
guarantees. The default heuristic Packs give good-enough solutions in
milliseconds — suitable for convergence loop participation where speed
matters more than proving optimality.
