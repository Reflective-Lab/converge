//! Solver for Meeting Scheduler pack

use super::types::*;
use converge_pack::PackSolver;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{
    Diagnostic, DiagnosticKind, ProblemSpec, ReplayEnvelope, SolverReport, StopReason,
};

/// Greedy scoring solver for meeting scheduling
///
/// Algorithm:
/// 1. For each slot, calculate score:
///    - required_available * 1000 (heavily weighted)
///    - optional_available * 10
///    - sum of preference scores
/// 2. Filter to slots where all required attendees can attend
/// 3. Sort by score (descending), apply tie-breaking
/// 4. Select highest-scoring feasible slot
pub struct GreedySolver;

impl GreedySolver {
    /// Solve the meeting scheduling problem
    pub fn solve_meeting(
        &self,
        input: &MeetingSchedulerInput,
        spec: &ProblemSpec,
    ) -> Result<(MeetingSchedulerOutput, SolverReport)> {
        let start = std::time::Instant::now();

        // Collect all required attendee IDs
        let required_ids: Vec<&str> = input.required_attendees().map(|a| a.id.as_str()).collect();

        // Score each slot
        let mut scored_slots: Vec<ScoredSlot> = input
            .slots
            .iter()
            .map(|slot| self.score_slot(slot, input, &required_ids))
            .collect();

        // Sort by score descending (higher is better)
        scored_slots.sort_by(|a, b| {
            b.score
                .total_cmp(&a.score)
                .then_with(|| a.slot.id.cmp(&b.slot.id))
        });

        // Apply tie-breaking for equal scores
        let tie_break = &spec.determinism.tie_break;
        let seed = spec.seed();

        // Group by score and apply tie-breaking within groups
        let mut final_slots = Vec::new();
        let mut current_score = f64::NEG_INFINITY;
        let mut score_group = Vec::new();

        for scored in scored_slots {
            if (scored.score - current_score).abs() < f64::EPSILON {
                score_group.push(scored);
            } else {
                if !score_group.is_empty() {
                    // Apply tie-breaking to group
                    score_group.sort_by(|a, b| a.slot.id.cmp(&b.slot.id));
                    if let Some(selected) =
                        tie_break.select_by(&score_group, seed, |a, b| a.slot.id.cmp(&b.slot.id))
                    {
                        final_slots.push(selected.clone());
                    }
                }
                score_group = vec![scored.clone()];
                current_score = scored.score;
            }
        }
        // Don't forget the last group
        if !score_group.is_empty() {
            score_group.sort_by(|a, b| a.slot.id.cmp(&b.slot.id));
            if let Some(selected) =
                tie_break.select_by(&score_group, seed, |a, b| a.slot.id.cmp(&b.slot.id))
            {
                final_slots.push(selected.clone());
            }
        }

        // Find best feasible slot (all required can attend, meets min attendees)
        let best = final_slots
            .into_iter()
            .find(|s| s.is_feasible && s.attending.len() >= input.requirements.min_attendees);

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;

        let (output, stop_reason) = match best {
            Some(scored) => {
                let output = MeetingSchedulerOutput {
                    selected_slot: Some(scored.slot.clone()),
                    attending: scored.attending.clone(),
                    not_attending: scored.not_attending.clone(),
                    conflicts: scored.conflicts.clone(),
                    total_preference_score: scored.preference_score,
                    score_breakdown: scored.breakdown.clone(),
                };
                (output, StopReason::Optimal)
            }
            None => {
                // Build conflict info for why no slot works
                let conflicts = self.build_global_conflicts(input, &required_ids);
                let output = MeetingSchedulerOutput::no_solution(conflicts);
                (output, StopReason::NoFeasible)
            }
        };

        // Build solver report
        let replay = ReplayEnvelope::minimal(seed);
        let report = if output.selected_slot.is_some() {
            SolverReport::optimal("greedy-v1", output.score_breakdown.total_score, replay)
                .with_diagnostic(Diagnostic::performance(
                    "scoring",
                    elapsed_ms,
                    input.slots.len(),
                ))
                .with_diagnostic(Diagnostic::scoring_breakdown(serde_json::json!({
                    "required_score": output.score_breakdown.required_score,
                    "optional_score": output.score_breakdown.optional_score,
                    "preference_score": output.score_breakdown.preference_score,
                })))
        } else {
            SolverReport::infeasible("greedy-v1", vec![], stop_reason, replay).with_diagnostic(
                Diagnostic::new(
                    DiagnosticKind::ConstraintAnalysis,
                    format!(
                        "No slot satisfies all required attendees. {} slots evaluated.",
                        input.slots.len()
                    ),
                ),
            )
        };

        Ok((output, report))
    }

    /// Score a single slot
    fn score_slot(
        &self,
        slot: &TimeSlot,
        input: &MeetingSchedulerInput,
        required_ids: &[&str],
    ) -> ScoredSlot {
        let mut attending = Vec::new();
        let mut not_attending = Vec::new();
        let mut conflicts = Vec::new();
        let mut required_count = 0;
        let mut optional_count = 0;
        let mut preference_score = 0.0;

        for attendee in &input.attendees {
            if attendee.can_attend(&slot.id) {
                attending.push(attendee.id.clone());
                preference_score += attendee.preference_for(&slot.id);

                if attendee.required {
                    required_count += 1;
                } else {
                    optional_count += 1;
                }
            } else {
                not_attending.push(attendee.id.clone());
                if attendee.required {
                    conflicts.push(ConflictInfo::not_available(&attendee.id, &slot.id));
                }
            }
        }

        // Check if all required attendees can attend
        let is_feasible = required_count == required_ids.len();

        // Calculate score (heavily weight required attendance)
        let required_score = required_count as f64 * 1000.0;
        let optional_score = optional_count as f64 * 10.0;
        let total_score = required_score + optional_score + preference_score;

        ScoredSlot {
            slot: slot.clone(),
            score: total_score,
            is_feasible,
            attending,
            not_attending,
            conflicts,
            preference_score,
            breakdown: ScoreBreakdown::new(required_score, optional_score, preference_score),
        }
    }

    /// Build conflict info explaining why no slot is feasible globally
    fn build_global_conflicts(
        &self,
        input: &MeetingSchedulerInput,
        required_ids: &[&str],
    ) -> Vec<ConflictInfo> {
        let mut conflicts = Vec::new();

        for &required_id in required_ids {
            if let Some(attendee) = input.get_attendee(required_id) {
                if attendee.available_slots.is_empty() {
                    conflicts.push(ConflictInfo::new(
                        required_id,
                        "Required attendee has no available slots",
                    ));
                }
            }
        }

        // Check if any slot has all required attendees
        let any_feasible = input.slots.iter().any(|slot| {
            required_ids.iter().all(|id| {
                input
                    .get_attendee(id)
                    .map(|a| a.can_attend(&slot.id))
                    .unwrap_or(false)
            })
        });

        if !any_feasible && conflicts.is_empty() {
            conflicts.push(ConflictInfo::new(
                "system",
                "No slot has availability overlap for all required attendees",
            ));
        }

        conflicts
    }
}

impl PackSolver for GreedySolver {
    fn id(&self) -> &'static str {
        "greedy-v1"
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<(serde_json::Value, SolverReport)> {
        let input: MeetingSchedulerInput = spec.inputs_as()?;
        let (output, report) = self.solve_meeting(&input, spec)?;
        let json = serde_json::to_value(&output)
            .map_err(|e| converge_pack::GateError::invalid_input(e.to_string()))?;
        Ok((json, report))
    }

    fn is_exact(&self) -> bool {
        // Greedy is exact for meeting scheduling (we enumerate all slots)
        true
    }
}

/// Internal scored slot representation
#[derive(Debug, Clone)]
struct ScoredSlot {
    slot: TimeSlot,
    score: f64,
    is_feasible: bool,
    attending: Vec<String>,
    not_attending: Vec<String>,
    conflicts: Vec<ConflictInfo>,
    preference_score: f64,
    breakdown: ScoreBreakdown,
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::{ObjectiveSpec, SolveBudgets};

    fn create_test_input() -> MeetingSchedulerInput {
        MeetingSchedulerInput {
            slots: vec![
                TimeSlot {
                    id: "slot-a".to_string(),
                    start: 1000,
                    end: 1060,
                    room: Some("Room A".to_string()),
                    capacity: 10,
                },
                TimeSlot {
                    id: "slot-b".to_string(),
                    start: 1100,
                    end: 1160,
                    room: Some("Room B".to_string()),
                    capacity: 5,
                },
            ],
            attendees: vec![
                Attendee {
                    id: "alice".to_string(),
                    name: "Alice".to_string(),
                    required: true,
                    available_slots: vec!["slot-a".to_string(), "slot-b".to_string()],
                    preferences: vec![SlotPreference {
                        slot_id: "slot-b".to_string(),
                        score: 20.0,
                    }],
                },
                Attendee {
                    id: "bob".to_string(),
                    name: "Bob".to_string(),
                    required: false,
                    available_slots: vec!["slot-a".to_string()],
                    preferences: vec![SlotPreference {
                        slot_id: "slot-a".to_string(),
                        score: 5.0,
                    }],
                },
            ],
            requirements: MeetingRequirements {
                duration_minutes: 60,
                min_attendees: 1,
                require_room: false,
            },
        }
    }

    fn create_spec(input: &MeetingSchedulerInput, seed: u64) -> ProblemSpec {
        ProblemSpec::builder("test", "tenant")
            .objective(ObjectiveSpec::maximize("attendance"))
            .inputs(input)
            .unwrap()
            .budgets(SolveBudgets::with_time_limit(10))
            .seed(seed)
            .build()
            .unwrap()
    }

    #[test]
    fn test_greedy_solver_basic() {
        let solver = GreedySolver;
        let input = create_test_input();
        let spec = create_spec(&input, 42);

        let (output, report) = solver.solve_meeting(&input, &spec).unwrap();

        assert!(output.selected_slot.is_some());
        assert!(report.feasible);
        assert_eq!(report.stop_reason, StopReason::Optimal);
    }

    #[test]
    fn test_prefers_more_attendees() {
        let solver = GreedySolver;

        // Custom input where optional attendee outweighs preference
        let input = MeetingSchedulerInput {
            slots: vec![
                TimeSlot {
                    id: "slot-a".to_string(),
                    start: 1000,
                    end: 1060,
                    room: Some("Room A".to_string()),
                    capacity: 10,
                },
                TimeSlot {
                    id: "slot-b".to_string(),
                    start: 1100,
                    end: 1160,
                    room: Some("Room B".to_string()),
                    capacity: 5,
                },
            ],
            attendees: vec![
                Attendee {
                    id: "alice".to_string(),
                    name: "Alice".to_string(),
                    required: true,
                    available_slots: vec!["slot-a".to_string(), "slot-b".to_string()],
                    preferences: vec![SlotPreference {
                        slot_id: "slot-b".to_string(),
                        score: 5.0, // Small preference for slot-b
                    }],
                },
                Attendee {
                    id: "bob".to_string(),
                    name: "Bob".to_string(),
                    required: false,
                    available_slots: vec!["slot-a".to_string()], // Only available for slot-a
                    preferences: vec![],
                },
            ],
            requirements: MeetingRequirements {
                duration_minutes: 60,
                min_attendees: 1,
                require_room: false,
            },
        };

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_meeting(&input, &spec).unwrap();

        // slot-a: 1000 (required) + 10 (optional Bob) + 0 (no pref) = 1010
        // slot-b: 1000 (required) + 0 + 5 (Alice pref) = 1005
        // slot-a wins due to more attendees
        assert_eq!(output.selected_slot.as_ref().unwrap().id, "slot-a");
        assert!(output.attending.contains(&"alice".to_string()));
        assert!(output.attending.contains(&"bob".to_string()));
    }

    #[test]
    fn test_preference_breaks_ties() {
        let solver = GreedySolver;

        // Both slots have same attendance
        let input = MeetingSchedulerInput {
            slots: vec![
                TimeSlot {
                    id: "slot-a".to_string(),
                    start: 1000,
                    end: 1060,
                    room: None,
                    capacity: 10,
                },
                TimeSlot {
                    id: "slot-b".to_string(),
                    start: 1100,
                    end: 1160,
                    room: None,
                    capacity: 10,
                },
            ],
            attendees: vec![Attendee {
                id: "alice".to_string(),
                name: "Alice".to_string(),
                required: true,
                available_slots: vec!["slot-a".to_string(), "slot-b".to_string()],
                preferences: vec![SlotPreference {
                    slot_id: "slot-b".to_string(),
                    score: 50.0,
                }],
            }],
            requirements: MeetingRequirements {
                duration_minutes: 60,
                min_attendees: 1,
                require_room: false,
            },
        };

        let spec = create_spec(&input, 42);
        let (output, _) = solver.solve_meeting(&input, &spec).unwrap();

        // slot-b should win due to preference
        assert_eq!(output.selected_slot.as_ref().unwrap().id, "slot-b");
    }

    #[test]
    fn test_infeasible_no_overlap() {
        let solver = GreedySolver;

        let input = MeetingSchedulerInput {
            slots: vec![
                TimeSlot {
                    id: "slot-a".to_string(),
                    start: 1000,
                    end: 1060,
                    room: None,
                    capacity: 10,
                },
                TimeSlot {
                    id: "slot-b".to_string(),
                    start: 1100,
                    end: 1160,
                    room: None,
                    capacity: 10,
                },
            ],
            attendees: vec![
                Attendee {
                    id: "alice".to_string(),
                    name: "Alice".to_string(),
                    required: true,
                    available_slots: vec!["slot-a".to_string()], // only slot-a
                    preferences: vec![],
                },
                Attendee {
                    id: "bob".to_string(),
                    name: "Bob".to_string(),
                    required: true,
                    available_slots: vec!["slot-b".to_string()], // only slot-b
                    preferences: vec![],
                },
            ],
            requirements: MeetingRequirements {
                duration_minutes: 60,
                min_attendees: 1,
                require_room: false,
            },
        };

        let spec = create_spec(&input, 42);
        let (output, report) = solver.solve_meeting(&input, &spec).unwrap();

        assert!(output.selected_slot.is_none());
        assert!(!report.feasible);
        assert_eq!(report.stop_reason, StopReason::NoFeasible);
    }

    #[test]
    fn test_solver_determinism() {
        let solver = GreedySolver;
        let input = create_test_input();

        // Run twice with same seed
        let spec1 = create_spec(&input, 99999);
        let spec2 = create_spec(&input, 99999);

        let (output1, _) = solver.solve_meeting(&input, &spec1).unwrap();
        let (output2, _) = solver.solve_meeting(&input, &spec2).unwrap();

        assert_eq!(
            output1.selected_slot.as_ref().map(|s| &s.id),
            output2.selected_slot.as_ref().map(|s| &s.id)
        );
        assert_eq!(output1.attending, output2.attending);
    }
}
