//! Meeting Scheduler Pack
//!
//! Selects meeting time slots with hard/soft constraints on attendee availability.
//!
//! ## Problem
//!
//! Given:
//! - Available time slots
//! - Attendees with availability and preferences
//! - Meeting requirements (duration, minimum attendees)
//!
//! Find:
//! - Best slot that maximizes attendance and preferences
//!
//! ## Solver
//!
//! Uses greedy scoring:
//! 1. Score = (required_available * 1000) + (optional_available * 10) + sum(preferences)
//! 2. Filter slots where all required attendees available
//! 3. Apply tie-breaking per DeterminismSpec
//! 4. Return highest-scoring feasible slot

mod invariants;
mod solver;
mod types;

pub use invariants::*;
pub use solver::*;
pub use types::*;

use crate::packs::{InvariantDef, InvariantResult, Pack, PackSolveResult, default_gate_evaluation};
use converge_pack::CONFIDENCE_STEP_MINOR;
use converge_pack::gate::GateResult as Result;
use converge_pack::gate::{KernelTraceLink, ProblemSpec, PromotionGate, ProposedPlan};

/// Meeting Scheduler Pack
pub struct MeetingSchedulerPack;

impl Pack for MeetingSchedulerPack {
    fn name(&self) -> &'static str {
        "meeting-scheduler"
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn validate_inputs(&self, inputs: &serde_json::Value) -> Result<()> {
        let input: MeetingSchedulerInput = serde_json::from_value(inputs.clone()).map_err(|e| {
            converge_pack::GateError::invalid_input(format!("Invalid input: {}", e))
        })?;
        input.validate()
    }

    fn invariants(&self) -> &[InvariantDef] {
        INVARIANTS
    }

    fn solve(&self, spec: &ProblemSpec) -> Result<PackSolveResult> {
        let input: MeetingSchedulerInput = spec.inputs_as()?;
        input.validate()?;

        let solver = GreedySolver;
        let (output, report) = solver.solve_meeting(&input, spec)?;

        let trace = KernelTraceLink::audit_only(format!("trace-{}", spec.problem_id));
        let confidence = calculate_confidence(&output, &input);

        let plan = ProposedPlan::from_payload(
            format!("plan-{}", spec.problem_id),
            self.name(),
            output.summary(),
            &output,
            confidence,
            trace,
        )?;

        Ok(PackSolveResult::new(plan, report))
    }

    fn check_invariants(&self, plan: &ProposedPlan) -> Result<Vec<InvariantResult>> {
        let output: MeetingSchedulerOutput = plan.plan_as()?;
        Ok(check_all_invariants(&output))
    }

    fn evaluate_gate(
        &self,
        plan: &ProposedPlan,
        invariant_results: &[InvariantResult],
    ) -> PromotionGate {
        // Special case: if no slot was selected, reject
        if let Ok(output) = plan.plan_as::<MeetingSchedulerOutput>() {
            if output.selected_slot.is_none() {
                return PromotionGate::reject("No feasible slot found");
            }
        }

        default_gate_evaluation(invariant_results, self.invariants())
    }
}

/// Calculate confidence score based on output quality
fn calculate_confidence(output: &MeetingSchedulerOutput, input: &MeetingSchedulerInput) -> f64 {
    if output.selected_slot.is_none() {
        return 0.0;
    }

    let total_attendees = input.attendees.len();
    if total_attendees == 0 {
        return 0.5;
    }

    let attending = output.attending.len();
    let attendance_ratio = attending as f64 / total_attendees as f64;

    // Base confidence from attendance
    let mut confidence = 0.5 + (attendance_ratio * 0.3);

    // Bonus for no conflicts
    if output.conflicts.is_empty() {
        confidence += CONFIDENCE_STEP_MINOR;
    }

    // Bonus for high preference score
    if output.total_preference_score > 0.0 {
        confidence += 0.1_f64.min(output.total_preference_score / 100.0);
    }

    confidence.min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_pack::gate::{ObjectiveSpec, SolveBudgets};

    fn create_test_input() -> MeetingSchedulerInput {
        MeetingSchedulerInput {
            slots: vec![
                TimeSlot {
                    id: "slot-1".to_string(),
                    start: 1000,
                    end: 1060,
                    room: Some("Room A".to_string()),
                    capacity: 10,
                },
                TimeSlot {
                    id: "slot-2".to_string(),
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
                    available_slots: vec!["slot-1".to_string(), "slot-2".to_string()],
                    preferences: vec![SlotPreference {
                        slot_id: "slot-1".to_string(),
                        score: 10.0,
                    }],
                },
                Attendee {
                    id: "bob".to_string(),
                    name: "Bob".to_string(),
                    required: false,
                    available_slots: vec!["slot-1".to_string()],
                    preferences: vec![],
                },
            ],
            requirements: MeetingRequirements {
                duration_minutes: 60,
                min_attendees: 1,
                require_room: false,
            },
        }
    }

    #[test]
    fn test_pack_name() {
        let pack = MeetingSchedulerPack;
        assert_eq!(pack.name(), "meeting-scheduler");
    }

    #[test]
    fn test_validate_inputs() {
        let pack = MeetingSchedulerPack;
        let input = create_test_input();
        let json = serde_json::to_value(&input).unwrap();
        assert!(pack.validate_inputs(&json).is_ok());
    }

    #[test]
    fn test_solve_basic() {
        let pack = MeetingSchedulerPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-001", "test-tenant")
            .objective(ObjectiveSpec::maximize("attendance"))
            .inputs(&input)
            .unwrap()
            .budgets(SolveBudgets::with_time_limit(10))
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        assert!(result.is_feasible());

        let output: MeetingSchedulerOutput = result.plan.plan_as().unwrap();
        assert!(output.selected_slot.is_some());
        // Should select slot-1 because both Alice and Bob can attend
        assert_eq!(output.selected_slot.as_ref().unwrap().id, "slot-1");
    }

    #[test]
    fn test_check_invariants() {
        let pack = MeetingSchedulerPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-002", "test-tenant")
            .objective(ObjectiveSpec::maximize("attendance"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();

        // All invariants should pass for valid solution
        let critical_passes = invariants
            .iter()
            .filter(|r| r.invariant == "all_required_attend")
            .all(|r| r.passed);
        assert!(critical_passes);
    }

    #[test]
    fn test_evaluate_gate() {
        let pack = MeetingSchedulerPack;
        let input = create_test_input();

        let spec = ProblemSpec::builder("test-003", "test-tenant")
            .objective(ObjectiveSpec::maximize("attendance"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let invariants = pack.check_invariants(&result.plan).unwrap();
        let gate = pack.evaluate_gate(&result.plan, &invariants);

        assert!(gate.is_promoted());
    }

    #[test]
    fn test_no_feasible_slot() {
        let pack = MeetingSchedulerPack;

        // Create input where required attendee can't attend any slot
        let input = MeetingSchedulerInput {
            slots: vec![TimeSlot {
                id: "slot-1".to_string(),
                start: 1000,
                end: 1060,
                room: None,
                capacity: 10,
            }],
            attendees: vec![Attendee {
                id: "alice".to_string(),
                name: "Alice".to_string(),
                required: true,
                available_slots: vec![], // Can't attend any slot
                preferences: vec![],
            }],
            requirements: MeetingRequirements {
                duration_minutes: 60,
                min_attendees: 1,
                require_room: false,
            },
        };

        let spec = ProblemSpec::builder("test-004", "test-tenant")
            .objective(ObjectiveSpec::maximize("attendance"))
            .inputs(&input)
            .unwrap()
            .seed(42)
            .build()
            .unwrap();

        let result = pack.solve(&spec).unwrap();
        let output: MeetingSchedulerOutput = result.plan.plan_as().unwrap();

        // No slot should be selected
        assert!(output.selected_slot.is_none());

        // Gate should reject
        let invariants = pack.check_invariants(&result.plan).unwrap();
        let gate = pack.evaluate_gate(&result.plan, &invariants);
        assert!(gate.is_rejected());
    }

    #[test]
    fn test_determinism() {
        let pack = MeetingSchedulerPack;
        let input = create_test_input();

        // Run twice with same seed
        let spec1 = ProblemSpec::builder("test-005a", "test-tenant")
            .objective(ObjectiveSpec::maximize("attendance"))
            .inputs(&input)
            .unwrap()
            .seed(12345)
            .build()
            .unwrap();

        let spec2 = ProblemSpec::builder("test-005b", "test-tenant")
            .objective(ObjectiveSpec::maximize("attendance"))
            .inputs(&input)
            .unwrap()
            .seed(12345)
            .build()
            .unwrap();

        let result1 = pack.solve(&spec1).unwrap();
        let result2 = pack.solve(&spec2).unwrap();

        let output1: MeetingSchedulerOutput = result1.plan.plan_as().unwrap();
        let output2: MeetingSchedulerOutput = result2.plan.plan_as().unwrap();

        assert_eq!(
            output1.selected_slot.as_ref().map(|s| &s.id),
            output2.selected_slot.as_ref().map(|s| &s.id)
        );
    }
}
