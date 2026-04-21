//! Types for Staff Rostering pack

use converge_pack::gate::GateResult as Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffMember {
    pub id: String,
    pub skills: Vec<String>,
    pub max_hours: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shift {
    pub id: String,
    pub required_skill: String,
    pub hours: u64,
    pub period: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffRosteringInput {
    pub staff: Vec<StaffMember>,
    pub shifts: Vec<Shift>,
    pub max_consecutive_shifts: Option<usize>,
}

impl StaffRosteringInput {
    pub fn validate(&self) -> Result<()> {
        if self.staff.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one staff member required",
            ));
        }
        if self.shifts.is_empty() {
            return Err(converge_pack::GateError::invalid_input(
                "At least one shift required",
            ));
        }
        for staff in &self.staff {
            if staff.max_hours == 0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Staff '{}' has zero max hours",
                    staff.id
                )));
            }
        }
        for shift in &self.shifts {
            if shift.hours == 0 {
                return Err(converge_pack::GateError::invalid_input(format!(
                    "Shift '{}' has zero hours",
                    shift.id
                )));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RosterAssignment {
    pub staff_id: String,
    pub shift_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffRosteringOutput {
    pub assignments: Vec<RosterAssignment>,
    pub coverage: f64,
    pub unassigned_shifts: Vec<String>,
}

impl StaffRosteringOutput {
    pub fn summary(&self) -> String {
        format!(
            "Assigned {} shifts with {:.0}% coverage ({} unassigned)",
            self.assignments.len(),
            self.coverage * 100.0,
            self.unassigned_shifts.len()
        )
    }
}
