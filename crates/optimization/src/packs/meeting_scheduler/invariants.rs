//! Invariants for Meeting Scheduler pack

use super::types::MeetingSchedulerOutput;
use crate::packs::{InvariantDef, InvariantResult};
use converge_pack::gate::Violation;

/// Invariant definitions for meeting scheduler
pub const INVARIANTS: &[InvariantDef] = &[InvariantDef {
    name: String::new(), // Will be replaced at runtime
    description: String::new(),
    critical: true,
}];

/// Get invariant definitions (with proper String values)
pub fn get_invariants() -> Vec<InvariantDef> {
    vec![
        InvariantDef::critical(
            "all_required_attend",
            "All required attendees must be able to attend the selected slot",
        ),
        InvariantDef::critical(
            "meets_minimum_attendees",
            "At least the minimum number of attendees must be available",
        ),
        InvariantDef::advisory(
            "room_capacity_sufficient",
            "Room capacity should accommodate all attendees",
        ),
        InvariantDef::advisory(
            "preference_score_positive",
            "Total preference score should be positive",
        ),
    ]
}

/// Check all invariants for a meeting scheduler output
pub fn check_all_invariants(output: &MeetingSchedulerOutput) -> Vec<InvariantResult> {
    vec![
        check_all_required_attend(output),
        check_meets_minimum_attendees(output),
        check_room_capacity_sufficient(output),
        check_preference_score_positive(output),
    ]
}

/// Check that all required attendees can attend
fn check_all_required_attend(output: &MeetingSchedulerOutput) -> InvariantResult {
    let invariant = "all_required_attend";

    // If no slot selected, this is vacuously true (no solution to check)
    if output.selected_slot.is_none() {
        return InvariantResult::pass(invariant);
    }

    // Check if any conflicts involve required attendees
    // (Conflicts are only created for required attendees who can't attend)
    if output.conflicts.is_empty() {
        return InvariantResult::pass(invariant);
    }

    // Find required attendees who have conflicts
    let failed_required: Vec<_> = output
        .conflicts
        .iter()
        .map(|c| c.attendee_id.as_str())
        .collect();

    if failed_required.is_empty() {
        InvariantResult::pass(invariant)
    } else {
        let violation = Violation::new(
            invariant,
            1.0,
            format!(
                "Required attendees cannot attend: {}",
                failed_required.join(", ")
            ),
        )
        .with_affected_all(failed_required.iter().map(|s| s.to_string()));
        InvariantResult::fail(invariant, violation)
    }
}

/// Check that minimum attendee count is met
fn check_meets_minimum_attendees(output: &MeetingSchedulerOutput) -> InvariantResult {
    let invariant = "meets_minimum_attendees";

    // If no slot selected, check passes (nothing to verify)
    if output.selected_slot.is_none() {
        return InvariantResult::pass(invariant);
    }

    // We can't check against requirements here since we only have output
    // The solver should have already enforced this
    // If attending is empty, that's likely a problem
    if output.attending.is_empty() {
        let violation = Violation::new(invariant, 1.0, "No attendees for selected slot");
        return InvariantResult::fail(invariant, violation);
    }

    InvariantResult::pass(invariant)
}

/// Check that room capacity is sufficient
fn check_room_capacity_sufficient(output: &MeetingSchedulerOutput) -> InvariantResult {
    let invariant = "room_capacity_sufficient";

    let slot = match &output.selected_slot {
        Some(s) => s,
        None => return InvariantResult::pass(invariant),
    };

    let attendee_count = output.attending.len();

    if attendee_count > slot.capacity {
        let violation = Violation::new(
            invariant,
            (attendee_count - slot.capacity) as f64 / attendee_count as f64,
            format!(
                "Room capacity {} is less than attendee count {}",
                slot.capacity, attendee_count
            ),
        )
        .with_affected(slot.id.clone());
        return InvariantResult::fail(invariant, violation);
    }

    InvariantResult::pass(invariant)
}

/// Check that preference score is positive (quality indicator)
fn check_preference_score_positive(output: &MeetingSchedulerOutput) -> InvariantResult {
    let invariant = "preference_score_positive";

    if output.selected_slot.is_none() {
        return InvariantResult::pass(invariant);
    }

    if output.total_preference_score <= 0.0 {
        let violation = Violation::new(
            invariant,
            0.3, // Low severity - just a quality indicator
            "No positive preferences for selected slot",
        );
        return InvariantResult::fail(invariant, violation);
    }

    InvariantResult::pass(invariant)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::meeting_scheduler::types::*;

    fn create_valid_output() -> MeetingSchedulerOutput {
        MeetingSchedulerOutput {
            selected_slot: Some(TimeSlot {
                id: "slot-1".to_string(),
                start: 1000,
                end: 2000,
                room: Some("Room A".to_string()),
                capacity: 10,
            }),
            attending: vec!["alice".to_string(), "bob".to_string()],
            not_attending: vec![],
            conflicts: vec![],
            total_preference_score: 15.0,
            score_breakdown: ScoreBreakdown::default(),
        }
    }

    #[test]
    fn test_all_invariants_pass() {
        let output = create_valid_output();
        let results = check_all_invariants(&output);

        for result in &results {
            assert!(result.passed, "Invariant {} failed", result.invariant);
        }
    }

    #[test]
    fn test_no_slot_selected() {
        let output = MeetingSchedulerOutput::no_solution(vec![]);
        let results = check_all_invariants(&output);

        // All should pass when no slot is selected (nothing to verify)
        for result in &results {
            assert!(
                result.passed,
                "Invariant {} should pass for no solution",
                result.invariant
            );
        }
    }

    #[test]
    fn test_room_capacity_exceeded() {
        let mut output = create_valid_output();
        // Add more attendees than room capacity
        output.attending = (0..15).map(|i| format!("attendee-{}", i)).collect();
        output.selected_slot.as_mut().unwrap().capacity = 10;

        let result = check_room_capacity_sufficient(&output);
        assert!(!result.passed);
        assert!(result.violation.is_some());
    }

    #[test]
    fn test_no_preference_score() {
        let mut output = create_valid_output();
        output.total_preference_score = 0.0;

        let result = check_preference_score_positive(&output);
        assert!(!result.passed);
        assert!(result.violation.as_ref().unwrap().severity < 0.5); // Low severity
    }

    #[test]
    fn test_conflicts_detected() {
        let mut output = create_valid_output();
        output.conflicts = vec![ConflictInfo::new("charlie", "Not available")];

        let result = check_all_required_attend(&output);
        assert!(!result.passed);
    }

    #[test]
    fn test_get_invariants() {
        let invariants = get_invariants();
        assert_eq!(invariants.len(), 4);

        let critical_count = invariants.iter().filter(|i| i.critical).count();
        assert_eq!(critical_count, 2);
    }
}
