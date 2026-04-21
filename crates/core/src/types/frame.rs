// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Frame types for six-phase flows.
//!
//! Frame defines scope and constraints for a phase of reasoning.
//! Uses typed-builder for ergonomic construction.
//!
//! # Design (per CONTEXT.md)
//!
//! - Frame as separate type: `Frame { scope, constraints, success_criteria }`
//! - Linked to Intent via `linked_intent: Option<IntentId>`
//! - Constraints should be typed (not just String) - uses ConstraintKind enum

use serde::{Deserialize, Serialize};

use super::id::CriterionId;
use typed_builder::TypedBuilder;

// ============================================================================
// FrameId - Unique identifier for a Frame
// ============================================================================

/// Unique identifier for a Frame.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrameId(String);

impl FrameId {
    /// Create a new FrameId.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for FrameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for FrameId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for FrameId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

// ============================================================================
// IntentId - Unique identifier for an Intent
// ============================================================================

/// Unique identifier for an Intent.
///
/// Used for linking Frames to Intents.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct IntentId(String);

impl IntentId {
    /// Create a new IntentId.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Generates a new unique intent ID.
    #[must_use]
    pub fn generate() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        static COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let counter = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let pid = std::process::id();
        Self(format!("intent-{timestamp:x}-{pid:08x}-{counter:04x}"))
    }
}

impl std::fmt::Display for IntentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for IntentId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for IntentId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

// ============================================================================
// ConstraintKind - Typed constraint kinds (not just String)
// ============================================================================

/// Kind of frame constraint.
///
/// Per CONTEXT.md specifics: "Frame constraints should be typed (not just String)"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConstraintKind {
    /// Budget constraint with amount.
    Budget(u64),
    /// Time constraint with description.
    Time(String),
    /// Geographic constraint.
    Geography(String),
    /// Compliance requirement.
    Compliance(String),
    /// Resource limit.
    Resource {
        /// Kind of resource (e.g., "compute", "memory", "api_calls")
        kind: String,
        /// Limit value
        limit: u64,
    },
    /// Custom constraint.
    Custom {
        /// Constraint key
        key: String,
        /// Constraint value
        value: String,
    },
}

// ============================================================================
// ConstraintSeverity - How violations are handled
// ============================================================================

/// Severity of a constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstraintSeverity {
    /// Violation blocks progress.
    Hard,
    /// Violation is logged but doesn't block.
    Soft,
}

impl Default for ConstraintSeverity {
    fn default() -> Self {
        Self::Hard
    }
}

// ============================================================================
// FrameConstraint - A constraint on a frame
// ============================================================================

/// A constraint on a frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameConstraint {
    /// The kind of constraint.
    pub kind: ConstraintKind,
    /// How violations are handled.
    pub severity: ConstraintSeverity,
    /// Optional description.
    pub description: Option<String>,
}

impl FrameConstraint {
    /// Create a new frame constraint.
    pub fn new(kind: ConstraintKind, severity: ConstraintSeverity) -> Self {
        Self {
            kind,
            severity,
            description: None,
        }
    }

    /// Add a description.
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Create a hard budget constraint.
    pub fn budget(amount: u64) -> Self {
        Self::new(ConstraintKind::Budget(amount), ConstraintSeverity::Hard)
    }

    /// Create a hard time constraint.
    pub fn time(description: impl Into<String>) -> Self {
        Self::new(
            ConstraintKind::Time(description.into()),
            ConstraintSeverity::Hard,
        )
    }

    /// Create a compliance constraint.
    pub fn compliance(requirement: impl Into<String>) -> Self {
        Self::new(
            ConstraintKind::Compliance(requirement.into()),
            ConstraintSeverity::Hard,
        )
    }
}

// ============================================================================
// Criterion - Success criterion for a frame
// ============================================================================

/// Success criterion for a frame.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Criterion {
    /// Criterion identifier.
    pub id: CriterionId,
    /// Human-readable description.
    pub description: String,
    /// Whether this criterion is required for success.
    pub required: bool,
}

impl Criterion {
    /// Create a new criterion.
    pub fn new(id: impl Into<CriterionId>, description: impl Into<String>, required: bool) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
            required,
        }
    }

    /// Create a required criterion.
    pub fn required(id: impl Into<CriterionId>, description: impl Into<String>) -> Self {
        Self::new(id, description, true)
    }

    /// Create an optional criterion.
    pub fn optional(id: impl Into<CriterionId>, description: impl Into<String>) -> Self {
        Self::new(id, description, false)
    }
}

// ============================================================================
// Frame - Defines scope and constraints for a phase of reasoning
// ============================================================================

/// Frame - defines scope and constraints for a phase of reasoning.
///
/// Linked to Intent, contains constraints and success criteria.
///
/// # Example
///
/// ```
/// use converge_core::types::{Frame, FrameId, FrameConstraint, ConstraintKind, ConstraintSeverity, Criterion};
///
/// let frame = Frame::builder()
///     .id(FrameId::new("frame-1"))
///     .scope("Nordic B2B market expansion")
///     .constraints(vec![
///         FrameConstraint::budget(1_000_000),
///         FrameConstraint::time("Q4 2025"),
///     ])
///     .success_criteria(vec![
///         Criterion::required("roi", "Achieve 20% ROI"),
///     ])
///     .build();
///
/// assert_eq!(frame.id.as_str(), "frame-1");
/// ```
#[derive(Debug, Clone, TypedBuilder, Serialize, Deserialize)]
pub struct Frame {
    /// Unique identifier for this frame.
    pub id: FrameId,
    /// Scope description for this frame.
    #[builder(setter(into))]
    pub scope: String,
    /// Constraints that apply to this frame.
    #[builder(default)]
    pub constraints: Vec<FrameConstraint>,
    /// Success criteria for this frame.
    #[builder(default)]
    pub success_criteria: Vec<Criterion>,
    /// Linked intent (if this frame is for a specific intent).
    #[builder(setter(into), default)]
    pub linked_intent: Option<IntentId>,
    /// Parent frame (for nested frames).
    #[builder(default)]
    pub parent_frame: Option<FrameId>,
}

impl Frame {
    /// Check if all required criteria are defined.
    pub fn has_required_criteria(&self) -> bool {
        self.success_criteria.iter().any(|c| c.required)
    }

    /// Get hard constraints only.
    pub fn hard_constraints(&self) -> impl Iterator<Item = &FrameConstraint> {
        self.constraints
            .iter()
            .filter(|c| c.severity == ConstraintSeverity::Hard)
    }

    /// Get soft constraints only.
    pub fn soft_constraints(&self) -> impl Iterator<Item = &FrameConstraint> {
        self.constraints
            .iter()
            .filter(|c| c.severity == ConstraintSeverity::Soft)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_id_display() {
        let id = FrameId::new("frame-123");
        assert_eq!(id.to_string(), "frame-123");
        assert_eq!(id.as_str(), "frame-123");
    }

    #[test]
    fn intent_id_from_string() {
        let id: IntentId = "intent-456".into();
        assert_eq!(id.as_str(), "intent-456");
    }

    #[test]
    fn frame_builder() {
        let frame = Frame::builder()
            .id(FrameId::new("frame-1"))
            .scope("Nordic B2B market")
            .constraints(vec![
                FrameConstraint::budget(1_000_000),
                FrameConstraint::time("Q4 2025"),
            ])
            .success_criteria(vec![Criterion::required("roi", "Achieve 20% ROI")])
            .build();

        assert_eq!(frame.id.as_str(), "frame-1");
        assert_eq!(frame.scope, "Nordic B2B market");
        assert_eq!(frame.constraints.len(), 2);
        assert!(frame.has_required_criteria());
    }

    #[test]
    fn frame_with_linked_intent() {
        let frame = Frame::builder()
            .id(FrameId::new("frame-1"))
            .scope("Test scope")
            .linked_intent(IntentId::new("intent-1"))
            .build();

        assert_eq!(
            frame.linked_intent.as_ref().map(IntentId::as_str),
            Some("intent-1")
        );
    }

    #[test]
    fn constraint_helpers() {
        let budget = FrameConstraint::budget(500_000);
        assert!(matches!(budget.kind, ConstraintKind::Budget(500_000)));
        assert_eq!(budget.severity, ConstraintSeverity::Hard);

        let time = FrameConstraint::time("End of year").with_description("Fiscal year deadline");
        assert!(matches!(time.kind, ConstraintKind::Time(_)));
        assert!(time.description.is_some());
    }

    #[test]
    fn criterion_helpers() {
        let required = Criterion::required("growth", "10% growth rate");
        assert!(required.required);

        let optional = Criterion::optional("stretch", "15% stretch goal");
        assert!(!optional.required);
    }

    #[test]
    fn frame_constraint_filtering() {
        let frame = Frame::builder()
            .id(FrameId::new("frame-1"))
            .scope("Test")
            .constraints(vec![
                FrameConstraint::budget(100),
                FrameConstraint::new(
                    ConstraintKind::Geography("EU".into()),
                    ConstraintSeverity::Soft,
                ),
            ])
            .build();

        assert_eq!(frame.hard_constraints().count(), 1);
        assert_eq!(frame.soft_constraints().count(), 1);
    }

    #[test]
    fn frame_serialization() {
        let frame = Frame::builder()
            .id(FrameId::new("frame-1"))
            .scope("Test scope")
            .build();

        let json = serde_json::to_string(&frame).unwrap();
        assert!(json.contains("\"id\":\"frame-1\""));
        assert!(json.contains("\"scope\":\"Test scope\""));

        let deserialized: Frame = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id.as_str(), "frame-1");
    }
}
