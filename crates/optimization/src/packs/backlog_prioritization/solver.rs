//! Solver for Backlog Prioritization pack

use super::types::*;
use converge_pack::PackSolver;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport, StopReason};

/// WSJF-based solver for backlog prioritization
///
/// Algorithm:
/// 1. Calculate WSJF score for each item
/// 2. Topological sort to respect dependencies
/// 3. Greedy selection: pick highest WSJF items that fit in capacity
pub struct WsjfSolver;

impl WsjfSolver {
    /// Solve the backlog prioritization problem
    pub fn solve_backlog(
        &self,
        input: &BacklogPrioritizationInput,
        spec: &ProblemSpec,
    ) -> Result<(BacklogPrioritizationOutput, SolverReport)> {
        let seed = spec.seed();

        // Calculate WSJF scores and sort
        let mut scored_items: Vec<_> = input
            .items
            .iter()
            .map(|item| (item, item.wsjf_score()))
            .collect();

        // Sort by WSJF descending
        scored_items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Apply tie-breaking for equal scores
        let _tie_break = &spec.determinism.tie_break;

        // Group by WSJF score and apply tie-breaking
        let mut final_order: Vec<(&BacklogItem, f64)> = Vec::new();
        let mut current_score = f64::NEG_INFINITY;
        let mut score_group: Vec<(&BacklogItem, f64)> = vec![];

        for (item, score) in scored_items {
            if (score - current_score).abs() < 0.01 {
                score_group.push((item, score));
            } else {
                if !score_group.is_empty() {
                    // Sort by ID for deterministic tie-breaking
                    score_group.sort_by(|a, b| a.0.id.cmp(&b.0.id));
                    final_order.extend(score_group.drain(..));
                }
                score_group = vec![(item, score)];
                current_score = score;
            }
        }
        // Don't forget the last group
        if !score_group.is_empty() {
            score_group.sort_by(|a, b| a.0.id.cmp(&b.0.id));
            final_order.extend(score_group.drain(..));
        }

        // Now process with dependency awareness
        let mut ranked_items = Vec::new();
        let mut completed: Vec<&str> = Vec::new();
        let mut cumulative_effort: i64 = 0;
        let mut total_value = 0.0;

        // Track which items have been ranked
        let mut pending: Vec<_> = final_order;
        let mut rank = 1;

        while !pending.is_empty() {
            let before_len = pending.len();

            // Find items whose dependencies are satisfied
            let mut to_remove = Vec::new();
            for (i, (item, wsjf)) in pending.iter().enumerate() {
                if item.dependencies_satisfied(&completed) {
                    let included = cumulative_effort + item.effort_points <= input.capacity_points;

                    if included {
                        cumulative_effort += item.effort_points;
                        total_value += item.business_value;
                    }

                    ranked_items.push(RankedItem {
                        item_id: item.id.clone(),
                        item_title: item.title.clone(),
                        rank,
                        wsjf_score: *wsjf,
                        included_in_capacity: included,
                        cumulative_effort: if included { cumulative_effort } else { 0 },
                        ranking_reason: if item.dependencies.is_empty() {
                            format!("WSJF score: {:.2}", wsjf)
                        } else {
                            format!(
                                "WSJF: {:.2}, after dependencies: {:?}",
                                wsjf, item.dependencies
                            )
                        },
                    });

                    completed.push(&item.id);
                    to_remove.push(i);
                    rank += 1;
                    break; // Process one at a time to maintain WSJF order
                }
            }

            // Remove processed items
            for i in to_remove.into_iter().rev() {
                pending.remove(i);
            }

            // If no progress, there's a cycle - break out
            if pending.len() == before_len {
                // Add remaining items as unresolvable
                for (item, wsjf) in pending.drain(..) {
                    ranked_items.push(RankedItem {
                        item_id: item.id.clone(),
                        item_title: item.title.clone(),
                        rank,
                        wsjf_score: wsjf,
                        included_in_capacity: false,
                        cumulative_effort: 0,
                        ranking_reason: format!(
                            "Dependency cycle detected: {:?}",
                            item.dependencies
                        ),
                    });
                    rank += 1;
                }
            }
        }

        let included_count = ranked_items
            .iter()
            .filter(|r| r.included_in_capacity)
            .count();
        let excluded_count = ranked_items.len() - included_count;

        let output = BacklogPrioritizationOutput {
            ranked_items,
            total_value,
            total_effort: cumulative_effort,
            included_count,
            excluded_count,
        };

        let replay = ReplayEnvelope::minimal(seed);
        let report = if included_count > 0 {
            SolverReport::optimal("wsjf-v1", total_value, replay)
        } else {
            SolverReport::feasible("wsjf-v1", 0.0, StopReason::Feasible, replay)
        };

        Ok((output, report))
    }
}

impl PackSolver for WsjfSolver {
    fn id(&self) -> &'static str {
        "wsjf-v1"
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<(serde_json::Value, SolverReport)> {
        let input: BacklogPrioritizationInput = spec.inputs_as()?;
        let (output, report) = self.solve_backlog(&input, spec)?;
        let json = serde_json::to_value(&output)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;
        Ok((json, report))
    }

    fn is_exact(&self) -> bool {
        false // Greedy, not optimal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> BacklogPrioritizationInput {
        BacklogPrioritizationInput {
            items: vec![
                BacklogItem {
                    id: "feat-1".to_string(),
                    title: "High Value Feature".to_string(),
                    business_value: 80.0,
                    time_criticality: 60.0,
                    risk_reduction: 40.0,
                    effort_points: 5,
                    dependencies: vec![],
                },
                BacklogItem {
                    id: "feat-2".to_string(),
                    title: "Quick Win".to_string(),
                    business_value: 40.0,
                    time_criticality: 80.0,
                    risk_reduction: 30.0,
                    effort_points: 2,
                    dependencies: vec![],
                },
                BacklogItem {
                    id: "feat-3".to_string(),
                    title: "Dependent Feature".to_string(),
                    business_value: 90.0,
                    time_criticality: 50.0,
                    risk_reduction: 60.0,
                    effort_points: 8,
                    dependencies: vec!["feat-1".to_string()],
                },
            ],
            capacity_points: 15,
        }
    }

    fn create_spec(input: &BacklogPrioritizationInput, seed: u64) -> ProblemSpec {
        ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::maximize("value"))
            .inputs(input)
            .unwrap()
            .seed(seed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_wsjf_ranking() {
        let solver = WsjfSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, report) = solver.solve_backlog(&input, &spec).unwrap();

        assert!(report.feasible);
        assert_eq!(output.ranked_items.len(), 3);

        // feat-2 has highest WSJF: (40+80+30)/2 = 75
        // feat-1 has WSJF: (80+60+40)/5 = 36
        // feat-3 depends on feat-1, WSJF: (90+50+60)/8 = 25
        assert_eq!(output.ranked_items[0].item_id, "feat-2");
    }

    #[test]
    fn test_dependency_ordering() {
        let solver = WsjfSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_backlog(&input, &spec).unwrap();

        // Find positions
        let feat1_rank = output
            .ranked_items
            .iter()
            .find(|r| r.item_id == "feat-1")
            .unwrap()
            .rank;
        let feat3_rank = output
            .ranked_items
            .iter()
            .find(|r| r.item_id == "feat-3")
            .unwrap()
            .rank;

        // feat-1 must come before feat-3
        assert!(feat1_rank < feat3_rank);
    }

    #[test]
    fn test_capacity_constraint() {
        let solver = WsjfSolver;
        let mut input = create_test_input();
        input.capacity_points = 7; // Only enough for feat-2 (2) and feat-1 (5)

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_backlog(&input, &spec).unwrap();

        assert_eq!(output.included_count, 2);
        assert_eq!(output.total_effort, 7);
    }

    #[test]
    fn test_determinism() {
        let solver = WsjfSolver;
        let input = create_test_input();

        let spec1 = create_spec(&input, 12345);
        let spec2 = create_spec(&input, 12345);

        let (output1, _) = solver.solve_backlog(&input, &spec1).unwrap();
        let (output2, _) = solver.solve_backlog(&input, &spec2).unwrap();

        for (a, b) in output1.ranked_items.iter().zip(output2.ranked_items.iter()) {
            assert_eq!(a.item_id, b.item_id);
            assert_eq!(a.rank, b.rank);
        }
    }
}
