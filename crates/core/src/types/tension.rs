// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Tension types for six-phase flows.
//!
//! Tension represents explicit conflict between two proposals.
//! Per CONTEXT.md: `Tension { left: ProposedFact, right: ProposedFact, conflict_type }`
//!
//! We use references (IDs) rather than owned proposals for flexibility.

use serde::{Deserialize, Serialize};

use super::id::{FactId, ProposalId, Timestamp};

// ============================================================================
// TensionId - Unique identifier for a Tension
// ============================================================================

/// Unique identifier for a Tension.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TensionId(String);

impl TensionId {
    /// Create a new TensionId.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Get the ID as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TensionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for TensionId {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for TensionId {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

// ============================================================================
// ConflictType - Type of conflict between proposals
// ============================================================================

/// Type of conflict between proposals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictType {
    /// Direct contradiction (A says X, B says not-X).
    Contradiction,
    /// Resource competition (both need same limited resource).
    ResourceContention,
    /// Temporal conflict (mutually exclusive time windows).
    TemporalOverlap,
    /// Priority conflict (different prioritization).
    PriorityMismatch,
    /// Scope conflict (overlapping but different scopes).
    ScopeOverlap,
    /// Custom conflict type.
    Custom(String),
}

impl Default for ConflictType {
    fn default() -> Self {
        Self::Contradiction
    }
}

// ============================================================================
// TensionSide - Reference to a proposal in a tension
// ============================================================================

/// Reference to a proposal in a tension (by ID, not owned).
///
/// Contains summary and supporting evidence for audit purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensionSide {
    /// ID of the proposal.
    pub proposal_id: ProposalId,
    /// Summary of the proposal's position.
    pub summary: String,
    /// IDs of facts that support this position.
    pub supporting_evidence: Vec<FactId>,
}

impl TensionSide {
    /// Create a new tension side.
    pub fn new(proposal_id: ProposalId, summary: impl Into<String>) -> Self {
        Self {
            proposal_id,
            summary: summary.into(),
            supporting_evidence: Vec::new(),
        }
    }

    /// Add supporting evidence.
    pub fn with_evidence(mut self, evidence: Vec<FactId>) -> Self {
        self.supporting_evidence = evidence;
        self
    }

    /// Add a single piece of evidence.
    pub fn add_evidence(&mut self, fact_id: FactId) {
        self.supporting_evidence.push(fact_id);
    }
}

// ============================================================================
// ChosenSide - Which side was chosen in resolution
// ============================================================================

/// Which side was chosen in tension resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChosenSide {
    /// Left side was chosen.
    Left,
    /// Right side was chosen.
    Right,
    /// Both were rejected.
    Neither,
    /// Combined into a new proposal.
    Merged,
}

// ============================================================================
// TensionResolution - How a tension was resolved
// ============================================================================

/// How a tension was resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensionResolution {
    /// Which side was chosen (or neither/merged).
    pub chosen_side: ChosenSide,
    /// Rationale for the resolution.
    pub rationale: String,
    /// When the tension was resolved.
    pub resolved_at: Timestamp,
    /// Actor who resolved the tension.
    pub resolver: String,
}

impl TensionResolution {
    /// Create a new resolution.
    pub fn new(
        chosen_side: ChosenSide,
        rationale: impl Into<String>,
        resolver: impl Into<String>,
    ) -> Self {
        Self {
            chosen_side,
            rationale: rationale.into(),
            resolved_at: Timestamp::now(),
            resolver: resolver.into(),
        }
    }

    /// Create a resolution choosing the left side.
    pub fn choose_left(rationale: impl Into<String>, resolver: impl Into<String>) -> Self {
        Self::new(ChosenSide::Left, rationale, resolver)
    }

    /// Create a resolution choosing the right side.
    pub fn choose_right(rationale: impl Into<String>, resolver: impl Into<String>) -> Self {
        Self::new(ChosenSide::Right, rationale, resolver)
    }

    /// Create a resolution rejecting both sides.
    pub fn reject_both(rationale: impl Into<String>, resolver: impl Into<String>) -> Self {
        Self::new(ChosenSide::Neither, rationale, resolver)
    }

    /// Create a resolution merging both sides.
    pub fn merge(rationale: impl Into<String>, resolver: impl Into<String>) -> Self {
        Self::new(ChosenSide::Merged, rationale, resolver)
    }
}

// ============================================================================
// Tension - Explicit conflict between two proposals
// ============================================================================

/// Tension - explicit conflict between two proposals.
///
/// Per CONTEXT.md: `Tension { left: ProposedFact, right: ProposedFact, conflict_type }`
/// We use references (IDs) rather than owned proposals for flexibility.
///
/// # Example
///
/// ```
/// use converge_core::types::{Tension, TensionId, TensionSide, ConflictType, ProposalId, Timestamp};
///
/// let tension = Tension::new(
///     TensionId::new("tension-1"),
///     TensionSide::new(ProposalId::new("p1"), "Focus on enterprise sales"),
///     TensionSide::new(ProposalId::new("p2"), "Focus on SMB market"),
///     ConflictType::PriorityMismatch,
/// );
///
/// assert_eq!(tension.id.as_str(), "tension-1");
/// assert!(!tension.is_resolved());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tension {
    /// Unique identifier for this tension.
    pub id: TensionId,
    /// Left side of the conflict.
    pub left: TensionSide,
    /// Right side of the conflict.
    pub right: TensionSide,
    /// Type of conflict.
    pub conflict_type: ConflictType,
    /// When the tension was detected.
    pub detected_at: Timestamp,
    /// Resolution (if resolved).
    pub resolution: Option<TensionResolution>,
}

impl Tension {
    /// Create a new unresolved tension.
    pub fn new(
        id: TensionId,
        left: TensionSide,
        right: TensionSide,
        conflict_type: ConflictType,
    ) -> Self {
        Self {
            id,
            left,
            right,
            conflict_type,
            detected_at: Timestamp::now(),
            resolution: None,
        }
    }

    /// Check if the tension has been resolved.
    pub fn is_resolved(&self) -> bool {
        self.resolution.is_some()
    }

    /// Resolve the tension.
    pub fn resolve(&mut self, resolution: TensionResolution) {
        self.resolution = Some(resolution);
    }

    /// Get the winning proposal ID (if resolved with a winner).
    pub fn winner(&self) -> Option<&ProposalId> {
        self.resolution.as_ref().and_then(|r| match r.chosen_side {
            ChosenSide::Left => Some(&self.left.proposal_id),
            ChosenSide::Right => Some(&self.right.proposal_id),
            _ => None,
        })
    }
}

// ============================================================================
// Hypothesis - Exploration phase artifact
// ============================================================================

/// Hypothesis - exploration phase artifact.
///
/// Represents a testable claim during the exploration phase of the six-phase flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hypothesis {
    /// Unique identifier.
    pub id: String,
    /// The claim being hypothesized.
    pub claim: String,
    /// Proposals that support this hypothesis.
    pub supporting_proposals: Vec<ProposalId>,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f32,
    /// Whether this hypothesis is testable.
    pub testable: bool,
}

impl Hypothesis {
    /// Create a new hypothesis.
    pub fn new(id: impl Into<String>, claim: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            claim: claim.into(),
            supporting_proposals: Vec::new(),
            confidence: 0.5,
            testable: true,
        }
    }

    /// Set confidence score.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Add supporting proposals.
    pub fn with_support(mut self, proposals: Vec<ProposalId>) -> Self {
        self.supporting_proposals = proposals;
        self
    }

    /// Mark as untestable.
    pub fn untestable(mut self) -> Self {
        self.testable = false;
        self
    }

    /// Check if confidence is high (>= 0.7).
    pub fn is_high_confidence(&self) -> bool {
        self.confidence >= 0.7
    }

    /// Check if confidence is low (< 0.3).
    pub fn is_low_confidence(&self) -> bool {
        self.confidence < 0.3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tension_id_display() {
        let id = TensionId::new("tension-123");
        assert_eq!(id.to_string(), "tension-123");
        assert_eq!(id.as_str(), "tension-123");
    }

    #[test]
    fn tension_creation() {
        let tension = Tension::new(
            TensionId::new("t-1"),
            TensionSide::new(ProposalId::new("p1"), "Position A"),
            TensionSide::new(ProposalId::new("p2"), "Position B"),
            ConflictType::Contradiction,
        );

        assert_eq!(tension.id.as_str(), "t-1");
        assert_eq!(tension.left.proposal_id.as_str(), "p1");
        assert_eq!(tension.right.proposal_id.as_str(), "p2");
        assert!(!tension.is_resolved());
    }

    #[test]
    fn tension_resolution() {
        let mut tension = Tension::new(
            TensionId::new("t-1"),
            TensionSide::new(ProposalId::new("p1"), "Position A"),
            TensionSide::new(ProposalId::new("p2"), "Position B"),
            ConflictType::PriorityMismatch,
        );

        assert!(!tension.is_resolved());
        assert!(tension.winner().is_none());

        tension.resolve(TensionResolution::choose_left(
            "Position A has more evidence",
            "human-reviewer",
        ));

        assert!(tension.is_resolved());
        assert_eq!(tension.winner().as_ref().map(|p| p.as_str()), Some("p1"));
    }

    #[test]
    fn tension_side_with_evidence() {
        let side = TensionSide::new(ProposalId::new("p1"), "Test position")
            .with_evidence(vec![FactId::new("f1"), FactId::new("f2")]);

        assert_eq!(side.supporting_evidence.len(), 2);
    }

    #[test]
    fn conflict_types() {
        let contradiction = ConflictType::Contradiction;
        let custom = ConflictType::Custom("domain-specific".into());

        assert!(matches!(contradiction, ConflictType::Contradiction));
        assert!(matches!(custom, ConflictType::Custom(_)));
    }

    #[test]
    fn resolution_helpers() {
        let left = TensionResolution::choose_left("reason", "actor");
        assert_eq!(left.chosen_side, ChosenSide::Left);

        let merged = TensionResolution::merge("combined", "actor");
        assert_eq!(merged.chosen_side, ChosenSide::Merged);
    }

    #[test]
    fn hypothesis_creation() {
        let hyp = Hypothesis::new("h-1", "The market will grow 20%")
            .with_confidence(0.8)
            .with_support(vec![ProposalId::new("p1")]);

        assert_eq!(hyp.id, "h-1");
        assert!((hyp.confidence - 0.8_f32).abs() < f32::EPSILON);
        assert!(hyp.is_high_confidence());
        assert!(!hyp.is_low_confidence());
        assert!(hyp.testable);
    }

    #[test]
    fn hypothesis_confidence_clamping() {
        let hyp = Hypothesis::new("h-1", "Test").with_confidence(1.5);
        assert!((hyp.confidence - 1.0_f32).abs() < f32::EPSILON);

        let hyp2 = Hypothesis::new("h-2", "Test").with_confidence(-0.5);
        assert!((hyp2.confidence - 0.0_f32).abs() < f32::EPSILON);
    }

    #[test]
    fn tension_serialization() {
        let tension = Tension::new(
            TensionId::new("t-1"),
            TensionSide::new(ProposalId::new("p1"), "A"),
            TensionSide::new(ProposalId::new("p2"), "B"),
            ConflictType::ResourceContention,
        );

        let json = serde_json::to_string(&tension).unwrap();
        assert!(json.contains("\"id\":\"t-1\""));
        assert!(json.contains("\"conflict_type\":\"ResourceContention\""));

        let deserialized: Tension = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id.as_str(), "t-1");
    }

    // ========================================================================
    // Negative tests — Tension state machine edge cases
    // ========================================================================

    #[test]
    fn winner_returns_none_when_neither_chosen() {
        let mut tension = Tension::new(
            TensionId::new("t-1"),
            TensionSide::new(ProposalId::new("p1"), "A"),
            TensionSide::new(ProposalId::new("p2"), "B"),
            ConflictType::Contradiction,
        );
        tension.resolve(TensionResolution::reject_both("both flawed", "reviewer"));
        assert!(tension.is_resolved());
        assert!(tension.winner().is_none());
    }

    #[test]
    fn winner_returns_none_when_merged() {
        let mut tension = Tension::new(
            TensionId::new("t-1"),
            TensionSide::new(ProposalId::new("p1"), "A"),
            TensionSide::new(ProposalId::new("p2"), "B"),
            ConflictType::ScopeOverlap,
        );
        tension.resolve(TensionResolution::merge("combined approach", "mediator"));
        assert!(tension.is_resolved());
        assert!(tension.winner().is_none());
    }

    #[test]
    fn double_resolve_overwrites_resolution() {
        let mut tension = Tension::new(
            TensionId::new("t-1"),
            TensionSide::new(ProposalId::new("p1"), "A"),
            TensionSide::new(ProposalId::new("p2"), "B"),
            ConflictType::PriorityMismatch,
        );

        tension.resolve(TensionResolution::choose_left("first pass", "actor"));
        assert_eq!(tension.winner().unwrap().as_str(), "p1");

        tension.resolve(TensionResolution::choose_right("reversed", "actor"));
        assert_eq!(tension.winner().unwrap().as_str(), "p2");
    }

    #[test]
    fn tension_side_add_evidence_accumulates() {
        let mut side = TensionSide::new(ProposalId::new("p1"), "position");
        assert!(side.supporting_evidence.is_empty());

        side.add_evidence(FactId::new("f1"));
        side.add_evidence(FactId::new("f2"));
        side.add_evidence(FactId::new("f3"));
        assert_eq!(side.supporting_evidence.len(), 3);
    }

    #[test]
    fn tension_with_custom_conflict_type() {
        let tension = Tension::new(
            TensionId::new("t-custom"),
            TensionSide::new(ProposalId::new("p1"), "A"),
            TensionSide::new(ProposalId::new("p2"), "B"),
            ConflictType::Custom("regulatory-clash".into()),
        );
        assert!(
            matches!(tension.conflict_type, ConflictType::Custom(ref s) if s == "regulatory-clash")
        );
    }

    #[test]
    fn conflict_type_default_is_contradiction() {
        assert_eq!(ConflictType::default(), ConflictType::Contradiction);
    }

    #[test]
    fn hypothesis_untestable_flag() {
        let hyp = Hypothesis::new("h-1", "untestable claim").untestable();
        assert!(!hyp.testable);
    }

    #[test]
    fn hypothesis_boundary_confidence_values() {
        // Exactly at boundaries
        let at_low = Hypothesis::new("h", "c").with_confidence(0.3);
        assert!(!at_low.is_low_confidence()); // 0.3 is NOT < 0.3

        let at_high = Hypothesis::new("h", "c").with_confidence(0.7);
        assert!(at_high.is_high_confidence()); // 0.7 IS >= 0.7

        // Zero and one
        let zero = Hypothesis::new("h", "c").with_confidence(0.0);
        assert!(zero.is_low_confidence());

        let one = Hypothesis::new("h", "c").with_confidence(1.0);
        assert!(one.is_high_confidence());
    }

    #[test]
    fn tension_id_from_conversions() {
        let from_str: TensionId = "t-1".into();
        let from_string: TensionId = String::from("t-1").into();
        assert_eq!(from_str, from_string);
    }

    #[test]
    fn tension_serialization_with_resolution() {
        let mut tension = Tension::new(
            TensionId::new("t-ser"),
            TensionSide::new(ProposalId::new("p1"), "A"),
            TensionSide::new(ProposalId::new("p2"), "B"),
            ConflictType::TemporalOverlap,
        );
        tension.resolve(TensionResolution::choose_left("reason", "actor"));

        let json = serde_json::to_string(&tension).unwrap();
        let round: Tension = serde_json::from_str(&json).unwrap();

        assert!(round.is_resolved());
        assert_eq!(round.winner().unwrap().as_str(), "p1");
        assert_eq!(
            round.resolution.as_ref().unwrap().chosen_side,
            ChosenSide::Left
        );
    }
}
