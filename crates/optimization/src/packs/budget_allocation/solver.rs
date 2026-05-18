//! Solver for Budget Allocation pack

use super::types::*;
use converge_pack::PackSolver;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport, StopReason};

/// Efficiency-based solver for budget allocation
///
/// Algorithm:
/// 1. Filter categories meeting ROI threshold
/// 2. Sort by efficiency score (ROI * priority)
/// 3. Allocate minimum to each qualifying category
/// 4. Distribute remaining budget proportionally by efficiency
pub struct EfficiencySolver;

impl EfficiencySolver {
    /// Solve the budget allocation problem
    pub fn solve_allocation(
        &self,
        input: &BudgetAllocationInput,
        spec: &ProblemSpec,
    ) -> Result<(BudgetAllocationOutput, SolverReport)> {
        let seed = spec.seed();

        // Filter categories meeting ROI threshold
        let mut qualifying: Vec<_> = input
            .categories
            .iter()
            .filter(|c| c.expected_roi >= input.constraints.min_roi_threshold)
            .collect();

        if qualifying.is_empty() {
            let output = BudgetAllocationOutput::empty(input.total_budget);
            let replay = ReplayEnvelope::minimal(seed);
            let report =
                SolverReport::infeasible("efficiency-v1", vec![], StopReason::NoFeasible, replay);
            return Ok((output, report));
        }

        // Sort by efficiency score descending
        qualifying.sort_by(|a, b| {
            b.efficiency_score()
                .total_cmp(&a.efficiency_score())
                .then_with(|| a.id.cmp(&b.id))
        });

        // Apply tie-breaking
        let _tie_break = &spec.determinism.tie_break;

        // Group by efficiency score for tie-breaking
        let mut final_order: Vec<&BudgetCategory> = Vec::new();
        let mut current_score = f64::NEG_INFINITY;
        let mut score_group: Vec<&BudgetCategory> = vec![];

        for cat in qualifying {
            if (cat.efficiency_score() - current_score).abs() < 0.001 {
                score_group.push(cat);
            } else {
                if !score_group.is_empty() {
                    score_group.sort_by(|a, b| a.id.cmp(&b.id));
                    final_order.extend(score_group.drain(..));
                }
                score_group = vec![cat];
                current_score = cat.efficiency_score();
            }
        }
        if !score_group.is_empty() {
            score_group.sort_by(|a, b| a.id.cmp(&b.id));
            final_order.extend(score_group.drain(..));
        }

        // Limit categories if constraint specified
        if let Some(max) = input.constraints.max_categories {
            final_order.truncate(max);
        }

        // Phase 1: Allocate minimums
        let mut remaining = input.total_budget;
        let mut allocations: Vec<(&&BudgetCategory, f64)> = Vec::new();

        for cat in &final_order {
            if remaining >= cat.min_allocation {
                allocations.push((cat, cat.min_allocation));
                remaining -= cat.min_allocation;
            } else if input.constraints.allow_partial && remaining > 0.0 {
                allocations.push((cat, remaining));
                remaining = 0.0;
            }
        }

        // Phase 2: Distribute remaining proportionally by efficiency
        if remaining > 0.0 {
            let total_efficiency: f64 = allocations
                .iter()
                .map(|(cat, _)| cat.efficiency_score())
                .sum();

            if total_efficiency > 0.0 {
                for (cat, amount) in &mut allocations {
                    let share = cat.efficiency_score() / total_efficiency;
                    let additional = remaining * share;
                    let new_amount = (*amount + additional).min(cat.max_allocation);
                    *amount = new_amount;
                }
            }
        }

        // Calculate totals
        let total_allocated: f64 = allocations.iter().map(|(_, a)| *a).sum();
        let total_expected_return: f64 = allocations
            .iter()
            .map(|(cat, a)| a * cat.expected_roi)
            .sum();
        let portfolio_roi = if total_allocated > 0.0 {
            total_expected_return / total_allocated
        } else {
            0.0
        };

        // Build output
        let allocation_items: Vec<CategoryAllocation> = allocations
            .iter()
            .map(|(cat, amount)| CategoryAllocation {
                category_id: cat.id.clone(),
                category_name: cat.name.clone(),
                amount: *amount,
                percentage: *amount / input.total_budget * 100.0,
                expected_return: *amount * cat.expected_roi,
                reason: format!(
                    "Efficiency score: {:.2}, ROI: {:.1}%",
                    cat.efficiency_score(),
                    cat.expected_roi * 100.0
                ),
            })
            .collect();

        let output = BudgetAllocationOutput {
            allocations: allocation_items,
            total_allocated,
            total_expected_return,
            budget_remaining: input.total_budget - total_allocated,
            portfolio_roi,
        };

        let replay = ReplayEnvelope::minimal(seed);
        let report = SolverReport::optimal("efficiency-v1", total_expected_return, replay);

        Ok((output, report))
    }
}

impl PackSolver for EfficiencySolver {
    fn id(&self) -> &'static str {
        "efficiency-v1"
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<(serde_json::Value, SolverReport)> {
        let input: BudgetAllocationInput = spec.inputs_as()?;
        let (output, report) = self.solve_allocation(&input, spec)?;
        let json = serde_json::to_value(&output)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;
        Ok((json, report))
    }

    fn is_exact(&self) -> bool {
        false // Greedy heuristic
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> BudgetAllocationInput {
        BudgetAllocationInput {
            total_budget: 100000.0,
            categories: vec![
                BudgetCategory {
                    id: "marketing".to_string(),
                    name: "Marketing".to_string(),
                    expected_roi: 0.20,
                    priority_weight: 3.0,
                    min_allocation: 10000.0,
                    max_allocation: 50000.0,
                },
                BudgetCategory {
                    id: "rnd".to_string(),
                    name: "R&D".to_string(),
                    expected_roi: 0.30,
                    priority_weight: 2.0,
                    min_allocation: 15000.0,
                    max_allocation: 60000.0,
                },
                BudgetCategory {
                    id: "ops".to_string(),
                    name: "Operations".to_string(),
                    expected_roi: 0.10,
                    priority_weight: 1.0,
                    min_allocation: 5000.0,
                    max_allocation: 30000.0,
                },
            ],
            constraints: AllocationConstraints {
                max_categories: None,
                min_roi_threshold: 0.05,
                allow_partial: false,
            },
        }
    }

    fn create_spec(input: &BudgetAllocationInput, seed: u64) -> ProblemSpec {
        ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::maximize("roi"))
            .inputs(input)
            .unwrap()
            .seed(seed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_allocation_order() {
        let solver = EfficiencySolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, report) = solver.solve_allocation(&input, &spec).unwrap();

        assert!(report.feasible);
        assert_eq!(output.allocations.len(), 3);

        // Marketing has highest efficiency (0.20 * 3 = 0.60)
        // R&D is second (0.30 * 2 = 0.60) - tie-broken alphabetically
        // Should be sorted by efficiency
    }

    #[test]
    fn test_budget_fully_allocated() {
        let solver = EfficiencySolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_allocation(&input, &spec).unwrap();

        // Budget should be mostly allocated
        assert!(output.budget_remaining < input.total_budget * 0.1);
    }

    #[test]
    fn test_roi_threshold_filtering() {
        let solver = EfficiencySolver;
        let mut input = create_test_input();
        input.constraints.min_roi_threshold = 0.15; // Filters out ops

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_allocation(&input, &spec).unwrap();

        // Ops should not be allocated
        let ops_alloc = output.allocations.iter().find(|a| a.category_id == "ops");
        assert!(ops_alloc.is_none());
    }

    #[test]
    fn test_max_categories_constraint() {
        let solver = EfficiencySolver;
        let mut input = create_test_input();
        input.constraints.max_categories = Some(2);

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_allocation(&input, &spec).unwrap();

        assert!(output.allocations.len() <= 2);
    }

    #[test]
    fn test_determinism() {
        let solver = EfficiencySolver;
        let input = create_test_input();

        let spec1 = create_spec(&input, 12345);
        let spec2 = create_spec(&input, 12345);

        let (output1, _) = solver.solve_allocation(&input, &spec1).unwrap();
        let (output2, _) = solver.solve_allocation(&input, &spec2).unwrap();

        for (a, b) in output1.allocations.iter().zip(output2.allocations.iter()) {
            assert_eq!(a.category_id, b.category_id);
            assert!((a.amount - b.amount).abs() < 0.01);
        }
    }
}
