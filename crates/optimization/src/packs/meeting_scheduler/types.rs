//! Types for Meeting Scheduler pack

use converge_pack::PackSchema;
use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

/// Input for meeting scheduler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingSchedulerInput {
    /// Available time slots
    pub slots: Vec<TimeSlot>,
    /// Attendees with their constraints
    pub attendees: Vec<Attendee>,
    /// Meeting requirements
    pub requirements: MeetingRequirements,
}

impl MeetingSchedulerInput {
    /// Validate the input
    pub fn validate(&self) -> Result<()> {
        if self.slots.is_empty() {
            return Err(converge_pack::GateError::invalid_input("no slots provided"));
        }
        if self.attendees.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "no attendees provided",
            ));
        }

        // Validate each slot
        for slot in &self.slots {
            slot.validate()?;
        }

        // Validate each attendee
        for attendee in &self.attendees {
            attendee.validate()?;
        }

        // Validate requirements
        self.requirements.validate()?;

        Ok(())
    }

    /// Get required attendees
    pub fn required_attendees(&self) -> impl Iterator<Item = &Attendee> {
        self.attendees.iter().filter(|a| a.required)
    }

    /// Get optional attendees
    pub fn optional_attendees(&self) -> impl Iterator<Item = &Attendee> {
        self.attendees.iter().filter(|a| !a.required)
    }

    /// Get a slot by ID
    pub fn get_slot(&self, id: &str) -> Option<&TimeSlot> {
        self.slots.iter().find(|s| s.id == id)
    }

    /// Get an attendee by ID
    pub fn get_attendee(&self, id: &str) -> Option<&Attendee> {
        self.attendees.iter().find(|a| a.id == id)
    }
}

impl PackSchema for MeetingSchedulerInput {
    fn validate(&self) -> Result<()> {
        MeetingSchedulerInput::validate(self)
    }
}

/// A potential meeting time slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlot {
    /// Unique slot identifier
    pub id: String,
    /// Start time (unix timestamp in seconds)
    pub start: i64,
    /// End time (unix timestamp in seconds)
    pub end: i64,
    /// Room name (optional)
    pub room: Option<String>,
    /// Room capacity
    pub capacity: usize,
}

impl TimeSlot {
    /// Validate the slot
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "slot id is required",
            ));
        }
        if self.start >= self.end {
            return Err(converge_pack::GateError::invalid_input(
                "slot start must be before end",
            ));
        }
        if self.capacity == 0 {
            return Err(converge_pack::GateError::invalid_input(
                "slot capacity must be positive",
            ));
        }
        Ok(())
    }

    /// Get duration in minutes
    pub fn duration_minutes(&self) -> i64 {
        (self.end - self.start) / 60
    }
}

/// An attendee with availability constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attendee {
    /// Unique attendee identifier
    pub id: String,
    /// Display name
    pub name: String,
    /// Whether this attendee must attend
    pub required: bool,
    /// Slot IDs this attendee can attend
    pub available_slots: Vec<String>,
    /// Preferences for specific slots
    pub preferences: Vec<SlotPreference>,
}

impl Attendee {
    /// Validate the attendee
    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "attendee id is required",
            ));
        }
        Ok(())
    }

    /// Check if attendee can attend a slot
    pub fn can_attend(&self, slot_id: &str) -> bool {
        self.available_slots.iter().any(|s| s == slot_id)
    }

    /// Get preference score for a slot (0 if no preference)
    pub fn preference_for(&self, slot_id: &str) -> f64 {
        self.preferences
            .iter()
            .find(|p| p.slot_id == slot_id)
            .map(|p| p.score)
            .unwrap_or(0.0)
    }
}

/// Preference for a specific slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlotPreference {
    /// Slot ID
    pub slot_id: String,
    /// Preference score (higher is better)
    pub score: f64,
}

/// Meeting requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingRequirements {
    /// Required meeting duration in minutes
    pub duration_minutes: i64,
    /// Minimum number of attendees
    pub min_attendees: usize,
    /// Whether a room is required
    pub require_room: bool,
}

impl MeetingRequirements {
    /// Validate requirements
    pub fn validate(&self) -> Result<()> {
        if self.duration_minutes <= 0 {
            return Err(converge_pack::GateError::invalid_input(
                "duration_minutes must be positive",
            ));
        }
        Ok(())
    }
}

/// Output plan from meeting scheduler
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeetingSchedulerOutput {
    /// Selected time slot (None if no feasible slot)
    pub selected_slot: Option<TimeSlot>,
    /// Attendee IDs who will attend
    pub attending: Vec<String>,
    /// Attendee IDs who cannot attend
    pub not_attending: Vec<String>,
    /// Scheduling conflicts
    pub conflicts: Vec<ConflictInfo>,
    /// Total preference score
    pub total_preference_score: f64,
    /// Score breakdown for explanation
    pub score_breakdown: ScoreBreakdown,
}

impl MeetingSchedulerOutput {
    /// Create an empty (no solution) output
    pub fn no_solution(conflicts: Vec<ConflictInfo>) -> Self {
        Self {
            selected_slot: None,
            attending: Vec::new(),
            not_attending: Vec::new(),
            conflicts,
            total_preference_score: 0.0,
            score_breakdown: ScoreBreakdown::default(),
        }
    }

    /// Create a human-readable summary
    pub fn summary(&self) -> String {
        match &self.selected_slot {
            Some(slot) => {
                let room = slot.room.as_deref().unwrap_or("TBD");
                format!(
                    "Selected slot {} in {} with {} attendees",
                    slot.id,
                    room,
                    self.attending.len()
                )
            }
            None => "No feasible meeting slot found".to_string(),
        }
    }
}

impl PackSchema for MeetingSchedulerOutput {
    fn validate(&self) -> Result<()> {
        // Output is always valid by construction
        Ok(())
    }
}

/// Information about a scheduling conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// Affected attendee ID
    pub attendee_id: String,
    /// Reason for conflict
    pub reason: String,
}

impl ConflictInfo {
    /// Create a new conflict
    pub fn new(attendee_id: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            attendee_id: attendee_id.into(),
            reason: reason.into(),
        }
    }

    /// Create a "not available" conflict
    pub fn not_available(attendee_id: impl Into<String>, slot_id: &str) -> Self {
        Self::new(attendee_id, format!("Not available for slot {}", slot_id))
    }
}

/// Score breakdown for explanation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScoreBreakdown {
    /// Score from required attendees
    pub required_score: f64,
    /// Score from optional attendees
    pub optional_score: f64,
    /// Score from preferences
    pub preference_score: f64,
    /// Total score
    pub total_score: f64,
}

impl ScoreBreakdown {
    /// Create a new breakdown
    pub fn new(required_score: f64, optional_score: f64, preference_score: f64) -> Self {
        Self {
            required_score,
            optional_score,
            preference_score,
            total_score: required_score + optional_score + preference_score,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_slot_validation() {
        let valid = TimeSlot {
            id: "slot-1".to_string(),
            start: 1000,
            end: 2000,
            room: Some("A".to_string()),
            capacity: 10,
        };
        assert!(valid.validate().is_ok());

        let invalid_time = TimeSlot {
            id: "slot-1".to_string(),
            start: 2000,
            end: 1000, // end before start
            room: None,
            capacity: 10,
        };
        assert!(invalid_time.validate().is_err());

        let no_id = TimeSlot {
            id: "".to_string(),
            start: 1000,
            end: 2000,
            room: None,
            capacity: 10,
        };
        assert!(no_id.validate().is_err());
    }

    #[test]
    fn test_attendee_availability() {
        let attendee = Attendee {
            id: "alice".to_string(),
            name: "Alice".to_string(),
            required: true,
            available_slots: vec!["slot-1".to_string(), "slot-2".to_string()],
            preferences: vec![SlotPreference {
                slot_id: "slot-1".to_string(),
                score: 10.0,
            }],
        };

        assert!(attendee.can_attend("slot-1"));
        assert!(attendee.can_attend("slot-2"));
        assert!(!attendee.can_attend("slot-3"));

        assert_eq!(attendee.preference_for("slot-1"), 10.0);
        assert_eq!(attendee.preference_for("slot-2"), 0.0);
    }

    #[test]
    fn test_input_validation() {
        let input = MeetingSchedulerInput {
            slots: vec![TimeSlot {
                id: "slot-1".to_string(),
                start: 1000,
                end: 2000,
                room: None,
                capacity: 10,
            }],
            attendees: vec![Attendee {
                id: "alice".to_string(),
                name: "Alice".to_string(),
                required: true,
                available_slots: vec!["slot-1".to_string()],
                preferences: vec![],
            }],
            requirements: MeetingRequirements {
                duration_minutes: 60,
                min_attendees: 1,
                require_room: false,
            },
        };

        assert!(input.validate().is_ok());
    }

    #[test]
    fn test_empty_slots_invalid() {
        let input = MeetingSchedulerInput {
            slots: vec![],
            attendees: vec![Attendee {
                id: "alice".to_string(),
                name: "Alice".to_string(),
                required: true,
                available_slots: vec![],
                preferences: vec![],
            }],
            requirements: MeetingRequirements {
                duration_minutes: 60,
                min_attendees: 1,
                require_room: false,
            },
        };

        assert!(input.validate().is_err());
    }

    #[test]
    fn test_output_summary() {
        let output = MeetingSchedulerOutput {
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
        };

        let summary = output.summary();
        assert!(summary.contains("slot-1"));
        assert!(summary.contains("Room A"));
        assert!(summary.contains("2 attendees"));
    }
}
