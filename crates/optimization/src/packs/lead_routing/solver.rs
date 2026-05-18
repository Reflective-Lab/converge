//! Solver for Lead Routing pack

use super::types::*;
use converge_pack::PackSolver;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{ProblemSpec, ReplayEnvelope, SolverReport, StopReason};
use std::collections::HashMap;

/// Scoring-based assignment solver for lead routing
///
/// Algorithm:
/// 1. Sort leads by priority (higher priority first) and score (higher first)
/// 2. For each lead, calculate fit scores with all available reps
/// 3. Filter reps by territory requirement if configured
/// 4. Assign lead to best-scoring rep with available capacity
/// 5. Update rep loads and continue
pub struct ScoreBasedRoutingSolver;

impl ScoreBasedRoutingSolver {
    /// Solve the lead routing problem
    pub fn solve_routing(
        &self,
        input: &LeadRoutingInput,
        spec: &ProblemSpec,
    ) -> Result<(LeadRoutingOutput, SolverReport)> {
        let seed = spec.seed();
        let config = &input.config;

        // Track rep loads (mutable copy)
        let mut rep_loads: HashMap<String, i64> = input
            .reps
            .iter()
            .map(|r| (r.id.clone(), r.current_load))
            .collect();

        let mut rep_new_assignments: HashMap<String, i64> =
            input.reps.iter().map(|r| (r.id.clone(), 0)).collect();

        // Sort leads by priority (ascending = higher priority) then by score (descending)
        let mut sorted_leads: Vec<&Lead> = input.leads.iter().collect();
        sorted_leads.sort_by(|a, b| {
            a.priority
                .cmp(&b.priority)
                .then_with(|| b.score.total_cmp(&a.score))
                .then_with(|| a.id.cmp(&b.id))
        });

        let mut assignments = Vec::new();
        let mut unassigned = Vec::new();
        let mut total_fit_score = 0.0;
        let mut total_value = 0.0;

        // Process each lead
        for lead in sorted_leads {
            let assignment_result = self.find_best_rep(lead, input, config, &rep_loads, spec);

            match assignment_result {
                Some((rep, fit_score, rationale)) => {
                    // Update rep load
                    *rep_loads.get_mut(&rep.id).unwrap() += 1;
                    *rep_new_assignments.get_mut(&rep.id).unwrap() += 1;

                    assignments.push(LeadAssignment {
                        lead_id: lead.id.clone(),
                        rep_id: rep.id.clone(),
                        rep_name: rep.name.clone(),
                        fit_score,
                        scoring_rationale: rationale,
                    });

                    total_fit_score += fit_score;
                    total_value += lead.estimated_value;
                }
                None => {
                    let reason = self.determine_unassigned_reason(lead, input, config, &rep_loads);
                    unassigned.push(UnassignedLead {
                        lead_id: lead.id.clone(),
                        reason,
                    });
                }
            }
        }

        // Build rep utilization
        let rep_utilization: Vec<RepUtilization> = input
            .reps
            .iter()
            .filter(|r| rep_new_assignments.get(&r.id).copied().unwrap_or(0) > 0)
            .map(|r| {
                let new_assignments = rep_new_assignments.get(&r.id).copied().unwrap_or(0);
                let total_load = rep_loads.get(&r.id).copied().unwrap_or(r.current_load);
                RepUtilization {
                    rep_id: r.id.clone(),
                    rep_name: r.name.clone(),
                    new_assignments,
                    total_load,
                    capacity: r.capacity,
                    utilization_pct: (total_load as f64 / r.capacity as f64) * 100.0,
                }
            })
            .collect();

        let avg_fit = if !assignments.is_empty() {
            total_fit_score / assignments.len() as f64
        } else {
            0.0
        };

        let stats = RoutingStats {
            total_leads: input.leads.len(),
            assigned_leads: assignments.len(),
            unassigned_leads: unassigned.len(),
            average_fit_score: avg_fit,
            total_estimated_value: total_value,
            summary: if unassigned.is_empty() {
                format!("All {} leads assigned successfully", assignments.len())
            } else {
                format!(
                    "Assigned {} leads, {} could not be assigned",
                    assignments.len(),
                    unassigned.len()
                )
            },
        };

        let output = LeadRoutingOutput {
            assignments,
            unassigned,
            rep_utilization,
            stats,
        };

        let replay = ReplayEnvelope::minimal(seed);
        let report = if output.stats.assigned_leads > 0 {
            SolverReport::optimal("score-routing-v1", avg_fit, replay)
        } else {
            SolverReport::infeasible("score-routing-v1", vec![], StopReason::NoFeasible, replay)
        };

        Ok((output, report))
    }

    /// Find the best rep for a lead
    fn find_best_rep<'a>(
        &self,
        lead: &Lead,
        input: &'a LeadRoutingInput,
        config: &RoutingConfig,
        rep_loads: &HashMap<String, i64>,
        spec: &ProblemSpec,
    ) -> Option<(&'a SalesRep, f64, ScoringRationale)> {
        let tie_break = &spec.determinism.tie_break;
        let seed = spec.seed();

        // Find candidates with available capacity
        let mut candidates: Vec<(&SalesRep, f64, ScoringRationale)> = input
            .reps
            .iter()
            .filter_map(|rep| {
                // Check capacity
                let current_load = rep_loads.get(&rep.id).copied().unwrap_or(rep.current_load);
                if current_load >= rep.capacity {
                    return None;
                }

                // Check territory requirement
                if config.require_territory_match && !rep.covers_territory(&lead.territory) {
                    return None;
                }

                // Calculate detailed scoring
                let (fit_score, rationale) =
                    self.calculate_detailed_score(lead, rep, current_load, config);

                // Require minimum fit score
                if fit_score < 10.0 {
                    return None;
                }

                Some((rep, fit_score, rationale))
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Sort by fit score descending, then by rep ID for determinism
        candidates.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.id.cmp(&b.0.id)));

        // Handle ties deterministically using tie_break
        let best_score = candidates[0].1;
        let tied: Vec<_> = candidates
            .iter()
            .filter(|(_, score, _)| (score - best_score).abs() < 0.01)
            .collect();

        if tied.len() == 1 {
            let (rep, score, rationale) = candidates.remove(0);
            Some((rep, score, rationale))
        } else {
            // Select from tied candidates using deterministic tie-breaking
            let selected = tie_break.select_by(&tied, seed, |a, b| a.0.id.cmp(&b.0.id));

            if let Some(&(rep, score, rationale)) = selected {
                Some((rep, *score, rationale.clone()))
            } else {
                // Fallback: take first (already sorted by ID)
                let (rep, score, rationale) = candidates.remove(0);
                Some((rep, score, rationale))
            }
        }
    }

    /// Calculate detailed fit score with rationale
    fn calculate_detailed_score(
        &self,
        lead: &Lead,
        rep: &SalesRep,
        current_load: i64,
        config: &RoutingConfig,
    ) -> (f64, ScoringRationale) {
        // Base scores (each out of 100 for normalization)
        let territory_score = if rep.covers_territory(&lead.territory) {
            100.0
        } else {
            0.0
        };

        let segment_score = if rep.segments.contains(&lead.segment) {
            100.0
        } else if rep.segments.is_empty() {
            50.0 // Generalist gets partial credit
        } else {
            0.0
        };

        let skills_score = if lead.required_skills.is_empty() {
            100.0
        } else {
            let matched = lead
                .required_skills
                .iter()
                .filter(|s| rep.skills.contains(*s))
                .count();
            (matched as f64 / lead.required_skills.len() as f64) * 100.0
        };

        let performance_score = rep.performance_score;

        // Capacity factor (penalize high utilization if load balancing enabled)
        let capacity_factor = if config.balance_load {
            let utilization = current_load as f64 / rep.capacity as f64;
            1.0 - (utilization * 0.4) // Max 40% penalty at full load
        } else {
            1.0
        };

        // Weighted combination
        let raw_score = (territory_score * config.territory_weight)
            + (segment_score * config.expertise_weight * 0.5)
            + (skills_score * config.expertise_weight * 0.5)
            + (performance_score * 0.1);

        let final_score = raw_score * capacity_factor;

        // Build explanation
        let mut explanation_parts = Vec::new();
        if territory_score > 0.0 {
            explanation_parts.push("territory match".to_string());
        }
        if segment_score >= 100.0 {
            explanation_parts.push("segment match".to_string());
        }
        if skills_score >= 100.0 && !lead.required_skills.is_empty() {
            explanation_parts.push("full skills match".to_string());
        } else if skills_score > 0.0 && skills_score < 100.0 {
            explanation_parts.push("partial skills match".to_string());
        }
        if capacity_factor < 1.0 {
            explanation_parts.push(format!("load factor {:.0}%", capacity_factor * 100.0));
        }

        let explanation = if explanation_parts.is_empty() {
            "baseline score".to_string()
        } else {
            explanation_parts.join(", ")
        };

        let rationale = ScoringRationale {
            territory_score,
            segment_score,
            skills_score,
            performance_score,
            capacity_factor,
            explanation,
        };

        (final_score, rationale)
    }

    /// Determine why a lead could not be assigned
    fn determine_unassigned_reason(
        &self,
        lead: &Lead,
        input: &LeadRoutingInput,
        config: &RoutingConfig,
        rep_loads: &HashMap<String, i64>,
    ) -> String {
        // Check if any rep has capacity
        let any_capacity = input.reps.iter().any(|r| {
            let load = rep_loads.get(&r.id).copied().unwrap_or(r.current_load);
            load < r.capacity
        });

        if !any_capacity {
            return "All reps are at full capacity".to_string();
        }

        // Check territory constraint
        if config.require_territory_match {
            let territory_reps: Vec<_> = input
                .reps
                .iter()
                .filter(|r| r.covers_territory(&lead.territory))
                .collect();

            if territory_reps.is_empty() {
                return format!("No reps cover territory '{}'", lead.territory);
            }

            let available_territory_reps: Vec<_> = territory_reps
                .iter()
                .filter(|r| {
                    let load = rep_loads.get(&r.id).copied().unwrap_or(r.current_load);
                    load < r.capacity
                })
                .collect();

            if available_territory_reps.is_empty() {
                return format!(
                    "All reps covering territory '{}' are at capacity",
                    lead.territory
                );
            }
        }

        // Generic fallback
        "No suitable rep found with available capacity".to_string()
    }
}

impl PackSolver for ScoreBasedRoutingSolver {
    fn id(&self) -> &'static str {
        "score-routing-v1"
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<(serde_json::Value, SolverReport)> {
        let input: LeadRoutingInput = spec.inputs_as()?;
        let (output, report) = self.solve_routing(&input, spec)?;
        let json = serde_json::to_value(&output)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;
        Ok((json, report))
    }

    fn is_exact(&self) -> bool {
        false // Greedy heuristic, not globally optimal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::ObjectiveSpec;

    fn create_test_input() -> LeadRoutingInput {
        LeadRoutingInput {
            leads: vec![
                Lead {
                    id: "lead-1".to_string(),
                    score: 85.0,
                    territory: "west".to_string(),
                    segment: "enterprise".to_string(),
                    required_skills: vec!["cloud".to_string()],
                    estimated_value: 100000.0,
                    priority: 1,
                },
                Lead {
                    id: "lead-2".to_string(),
                    score: 70.0,
                    territory: "east".to_string(),
                    segment: "smb".to_string(),
                    required_skills: vec![],
                    estimated_value: 25000.0,
                    priority: 3,
                },
                Lead {
                    id: "lead-3".to_string(),
                    score: 90.0,
                    territory: "west".to_string(),
                    segment: "enterprise".to_string(),
                    required_skills: vec!["cloud".to_string(), "ai".to_string()],
                    estimated_value: 200000.0,
                    priority: 1,
                },
            ],
            reps: vec![
                SalesRep {
                    id: "rep-1".to_string(),
                    name: "Alice Johnson".to_string(),
                    capacity: 10,
                    current_load: 7,
                    territories: vec!["west".to_string()],
                    segments: vec!["enterprise".to_string()],
                    skills: vec!["cloud".to_string(), "ai".to_string()],
                    performance_score: 92.0,
                },
                SalesRep {
                    id: "rep-2".to_string(),
                    name: "Bob Smith".to_string(),
                    capacity: 8,
                    current_load: 3,
                    territories: vec!["east".to_string(), "midwest".to_string()],
                    segments: vec!["smb".to_string(), "mid-market".to_string()],
                    skills: vec!["demos".to_string()],
                    performance_score: 78.0,
                },
                SalesRep {
                    id: "rep-3".to_string(),
                    name: "Carol Davis".to_string(),
                    capacity: 12,
                    current_load: 5,
                    territories: vec!["west".to_string(), "east".to_string()],
                    segments: vec!["enterprise".to_string(), "smb".to_string()],
                    skills: vec!["cloud".to_string()],
                    performance_score: 85.0,
                },
            ],
            config: RoutingConfig::default(),
        }
    }

    fn create_spec(input: &LeadRoutingInput, seed: u64) -> ProblemSpec {
        ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::maximize("conversion"))
            .inputs(input)
            .unwrap()
            .seed(seed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_basic_routing() {
        let solver = ScoreBasedRoutingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, report) = solver.solve_routing(&input, &spec).unwrap();

        assert!(report.feasible);
        assert_eq!(output.stats.total_leads, 3);
        assert_eq!(output.stats.assigned_leads, 3);
        assert!(output.unassigned.is_empty());
    }

    #[test]
    fn test_territory_matching() {
        let solver = ScoreBasedRoutingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_routing(&input, &spec).unwrap();

        // Check that west leads go to reps covering west
        for assignment in &output.assignments {
            let lead = input
                .leads
                .iter()
                .find(|l| l.id == assignment.lead_id)
                .unwrap();
            let rep = input
                .reps
                .iter()
                .find(|r| r.id == assignment.rep_id)
                .unwrap();

            // Either territory matches or it's not required
            assert!(
                rep.covers_territory(&lead.territory) || !input.config.require_territory_match,
                "Lead {} in {} assigned to rep {} covering {:?}",
                lead.id,
                lead.territory,
                rep.id,
                rep.territories
            );
        }
    }

    #[test]
    fn test_capacity_constraint() {
        let solver = ScoreBasedRoutingSolver;
        let mut input = create_test_input();

        // Set all reps to near capacity
        for rep in &mut input.reps {
            rep.current_load = rep.capacity - 1;
        }

        // Add more leads than available capacity
        for i in 0..10 {
            input.leads.push(Lead {
                id: format!("overflow-{}", i),
                score: 50.0,
                territory: "west".to_string(),
                segment: "smb".to_string(),
                required_skills: vec![],
                estimated_value: 10000.0,
                priority: 5,
            });
        }

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_routing(&input, &spec).unwrap();

        // Should have some unassigned due to capacity
        assert!(!output.unassigned.is_empty());

        // Total load should not exceed capacity for any rep
        for util in &output.rep_utilization {
            assert!(util.total_load <= util.capacity);
        }
    }

    #[test]
    fn test_priority_ordering() {
        let solver = ScoreBasedRoutingSolver;
        let input = LeadRoutingInput {
            leads: vec![
                Lead {
                    id: "low-priority".to_string(),
                    score: 95.0,
                    territory: "west".to_string(),
                    segment: "enterprise".to_string(),
                    required_skills: vec![],
                    estimated_value: 500000.0,
                    priority: 5, // Low priority
                },
                Lead {
                    id: "high-priority".to_string(),
                    score: 60.0,
                    territory: "west".to_string(),
                    segment: "enterprise".to_string(),
                    required_skills: vec![],
                    estimated_value: 50000.0,
                    priority: 1, // High priority
                },
            ],
            reps: vec![SalesRep {
                id: "rep-1".to_string(),
                name: "Only Rep".to_string(),
                capacity: 1, // Can only take one lead
                current_load: 0,
                territories: vec!["west".to_string()],
                segments: vec!["enterprise".to_string()],
                skills: vec![],
                performance_score: 80.0,
            }],
            config: RoutingConfig::default(),
        };

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_routing(&input, &spec).unwrap();

        // High priority lead should be assigned (despite lower score)
        assert_eq!(output.assignments.len(), 1);
        assert_eq!(output.assignments[0].lead_id, "high-priority");
        assert_eq!(output.unassigned.len(), 1);
        assert_eq!(output.unassigned[0].lead_id, "low-priority");
    }

    #[test]
    fn test_required_territory_match() {
        let solver = ScoreBasedRoutingSolver;
        let input = LeadRoutingInput {
            leads: vec![Lead {
                id: "lead-1".to_string(),
                score: 80.0,
                territory: "south".to_string(), // No rep covers this
                segment: "enterprise".to_string(),
                required_skills: vec![],
                estimated_value: 50000.0,
                priority: 1,
            }],
            reps: vec![SalesRep {
                id: "rep-1".to_string(),
                name: "Rep 1".to_string(),
                capacity: 10,
                current_load: 0,
                territories: vec!["west".to_string(), "east".to_string()],
                segments: vec!["enterprise".to_string()],
                skills: vec![],
                performance_score: 80.0,
            }],
            config: RoutingConfig {
                require_territory_match: true,
                ..Default::default()
            },
        };

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_routing(&input, &spec).unwrap();

        // Lead should be unassigned due to territory requirement
        assert!(output.assignments.is_empty());
        assert_eq!(output.unassigned.len(), 1);
        assert!(output.unassigned[0].reason.contains("territory"));
    }

    #[test]
    fn test_scoring_rationale() {
        let solver = ScoreBasedRoutingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_routing(&input, &spec).unwrap();

        for assignment in &output.assignments {
            // Each assignment should have scoring rationale
            let rationale = &assignment.scoring_rationale;
            assert!(!rationale.explanation.is_empty());
            assert!(rationale.capacity_factor > 0.0);
            assert!(rationale.capacity_factor <= 1.0);
        }
    }

    #[test]
    fn test_determinism() {
        let solver = ScoreBasedRoutingSolver;
        let input = create_test_input();

        let spec1 = create_spec(&input, 12345);
        let spec2 = create_spec(&input, 12345);

        let (output1, _) = solver.solve_routing(&input, &spec1).unwrap();
        let (output2, _) = solver.solve_routing(&input, &spec2).unwrap();

        assert_eq!(output1.assignments.len(), output2.assignments.len());
        for (a1, a2) in output1.assignments.iter().zip(output2.assignments.iter()) {
            assert_eq!(a1.lead_id, a2.lead_id);
            assert_eq!(a1.rep_id, a2.rep_id);
        }
    }

    #[test]
    fn test_rep_utilization_output() {
        let solver = ScoreBasedRoutingSolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, _) = solver.solve_routing(&input, &spec).unwrap();

        // Check utilization stats
        for util in &output.rep_utilization {
            assert!(util.utilization_pct >= 0.0);
            assert!(util.utilization_pct <= 100.0);
            assert!(util.new_assignments > 0);
        }
    }
}
