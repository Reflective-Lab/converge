// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Proposal types with type-state pattern.
//!
//! Proposals use marker types (Draft, Validated) to enforce lifecycle at compile-time:
//! - `Proposal<Draft>` -> can be created publicly via `Proposal::new()`
//! - `Proposal<Validated>` -> can only be created by `PromotionGate` via `pub(crate)`
//!
//! # State Transitions
//!
//! ```text
//! Proposal<Draft> --[PromotionGate::validate()]--> Proposal<Validated>
//! Proposal<Validated> --[PromotionGate::promote()]--> Fact
//! ```

use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use super::id::{ContentHash, ObservationId, ProposalId};
use super::observation::CaptureContext;

// ============================================================================
// State Marker Types
// ============================================================================

/// Marker type: proposal in draft state.
///
/// Draft proposals can be created by anyone and have not been validated.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Draft;

/// Marker type: proposal validated and ready for promotion.
///
/// Validated proposals can only be created by PromotionGate and are
/// eligible for promotion to Fact.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Validated;

// ============================================================================
// ProposedContent - The actual content being proposed
// ============================================================================

/// Kind of proposed content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProposedContentKind {
    /// A claim or assertion
    Claim,
    /// An action plan
    Plan,
    /// A classification or categorization
    Classification,
    /// An evaluation with scores
    Evaluation,
    /// A draft document
    Draft,
    /// Raw reasoning output
    Reasoning,
}

impl Default for ProposedContentKind {
    fn default() -> Self {
        Self::Reasoning
    }
}

/// Content of a proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedContent {
    /// What kind of content this is
    pub kind: ProposedContentKind,
    /// The textual content
    pub content: String,
    /// Structured content (if applicable)
    pub structured: Option<serde_json::Value>,
    /// Confidence score. Always in [0.0, 1.0] if present.
    confidence: Option<f32>,
}

impl ProposedContent {
    /// Create new proposed content.
    pub fn new(kind: ProposedContentKind, content: impl Into<String>) -> Self {
        Self {
            kind,
            content: content.into(),
            structured: None,
            confidence: None,
        }
    }

    /// Returns the confidence score if set, always in [0.0, 1.0].
    #[must_use]
    pub fn confidence(&self) -> Option<f32> {
        self.confidence
    }

    /// Add structured content.
    pub fn with_structured(mut self, structured: serde_json::Value) -> Self {
        self.structured = Some(structured);
        self
    }

    /// Set a confidence baseline.
    ///
    /// The value is clamped to [0.0, 1.0]. Non-finite values are treated as 0.0.
    /// Use [`adjust_confidence`][Self::adjust_confidence] to accumulate criteria on top.
    #[must_use]
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = Some(if confidence.is_finite() {
            confidence.clamp(0.0, 1.0)
        } else {
            0.0
        });
        self
    }

    /// Adjust confidence by a delta, clamped to [0.0, 1.0].
    ///
    /// If no confidence has been set, the delta is applied from 0.0.
    #[must_use]
    pub fn adjust_confidence(mut self, delta: f32) -> Self {
        let current = self.confidence.unwrap_or(0.0);
        self.confidence = Some((current + delta).clamp(0.0, 1.0));
        self
    }
}

// ============================================================================
// ObservationProvenance - Link back to source observation
// ============================================================================

/// Provenance from the source observation.
///
/// Links a proposal back to the observation it was derived from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservationProvenance {
    /// ID of the source observation
    pub observation_id: ObservationId,
    /// Hash of the raw payload (for verification)
    pub raw_payload_ref: ContentHash,
    /// Capture context from the observation
    pub capture_context: CaptureContext,
}

impl ObservationProvenance {
    /// Create observation provenance.
    pub fn new(
        observation_id: ObservationId,
        raw_payload_ref: ContentHash,
        capture_context: CaptureContext,
    ) -> Self {
        Self {
            observation_id,
            raw_payload_ref,
            capture_context,
        }
    }
}

// ============================================================================
// Proposal<State> - Type-state pattern implementation
// ============================================================================

/// A proposal in a specific lifecycle state.
///
/// State transitions:
/// - `Proposal<Draft>` -> `Proposal<Validated>` via `PromotionGate::validate()`
/// - `Proposal<Validated>` -> `Fact` via `PromotionGate::promote()`
///
/// # Example
///
/// ```
/// use converge_core::types::{
///     Proposal, Draft, ProposalId, ProposedContent, ProposedContentKind,
///     ObservationProvenance, ObservationId, ContentHash, CaptureContext,
/// };
///
/// // Create a draft proposal (public)
/// let draft = Proposal::<Draft>::new(
///     ProposalId::new("proposal-001"),
///     ProposedContent::new(ProposedContentKind::Claim, "The market is growing"),
///     ObservationProvenance::new(
///         ObservationId::new("obs-001"),
///         ContentHash::zero(),
///         CaptureContext::default(),
///     ),
/// );
///
/// assert_eq!(draft.id().as_str(), "proposal-001");
/// ```
#[derive(Debug, Clone)]
pub struct Proposal<State> {
    id: ProposalId,
    content: ProposedContent,
    provenance: ObservationProvenance,
    _state: PhantomData<State>,
}

impl Proposal<Draft> {
    /// Public constructor for draft proposals.
    ///
    /// Anyone can create a draft proposal. It must be validated
    /// by a PromotionGate before it can become a Fact.
    pub fn new(
        id: ProposalId,
        content: ProposedContent,
        provenance: ObservationProvenance,
    ) -> Self {
        Self {
            id,
            content,
            provenance,
            _state: PhantomData,
        }
    }
}

impl Proposal<Validated> {
    /// Private constructor - only callable by PromotionGate (in gates module).
    ///
    /// External code cannot create validated proposals directly.
    pub(crate) fn from_validated(
        id: ProposalId,
        content: ProposedContent,
        provenance: ObservationProvenance,
    ) -> Self {
        Self {
            id,
            content,
            provenance,
            _state: PhantomData,
        }
    }
}

// Shared methods for all proposal states
impl<State> Proposal<State> {
    /// Get the proposal ID.
    pub fn id(&self) -> &ProposalId {
        &self.id
    }

    /// Get the proposed content.
    pub fn content(&self) -> &ProposedContent {
        &self.content
    }

    /// Get the observation provenance.
    pub fn provenance(&self) -> &ObservationProvenance {
        &self.provenance
    }
}

// ============================================================================
// Serialize/Deserialize implementations
// ============================================================================

// Manual Serialize implementation since PhantomData doesn't need serialization
impl<S> Serialize for Proposal<S> {
    fn serialize<Ser>(&self, serializer: Ser) -> Result<Ser::Ok, Ser::Error>
    where
        Ser: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Proposal", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("content", &self.content)?;
        state.serialize_field("provenance", &self.provenance)?;
        state.end()
    }
}

// Deserialize implementation for Draft proposals only
// (Validated proposals should only come from PromotionGate)
impl<'de> Deserialize<'de> for Proposal<Draft> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ProposalData {
            id: ProposalId,
            content: ProposedContent,
            provenance: ObservationProvenance,
        }

        let data = ProposalData::deserialize(deserializer)?;
        Ok(Proposal::new(data.id, data.content, data.provenance))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_provenance() -> ObservationProvenance {
        ObservationProvenance::new(
            ObservationId::new("obs-001"),
            ContentHash::zero(),
            CaptureContext::default(),
        )
    }

    #[test]
    fn draft_proposal_creation() {
        let draft = Proposal::<Draft>::new(
            ProposalId::new("proposal-001"),
            ProposedContent::new(ProposedContentKind::Claim, "Test claim"),
            make_provenance(),
        );

        assert_eq!(draft.id().as_str(), "proposal-001");
        assert_eq!(draft.content().kind, ProposedContentKind::Claim);
    }

    #[test]
    fn proposed_content_builder() {
        let content = ProposedContent::new(ProposedContentKind::Evaluation, "Good")
            .with_structured(serde_json::json!({"score": 0.8}))
            .with_confidence(0.85);

        assert_eq!(content.kind, ProposedContentKind::Evaluation);
        assert!(content.structured.is_some());
        assert_eq!(content.confidence(), Some(0.85));
    }

    #[test]
    fn proposal_serialization() {
        let draft = Proposal::<Draft>::new(
            ProposalId::new("proposal-001"),
            ProposedContent::new(ProposedContentKind::Claim, "Test"),
            make_provenance(),
        );

        let json = serde_json::to_string(&draft).unwrap();
        assert!(json.contains("\"id\":\"proposal-001\""));
        assert!(json.contains("\"content\""));
        assert!(json.contains("\"provenance\""));
    }

    #[test]
    fn draft_proposal_deserialization() {
        let json = r#"{
            "id": "proposal-002",
            "content": {
                "kind": "Claim",
                "content": "Deserialized claim",
                "structured": null,
                "confidence": null
            },
            "provenance": {
                "observation_id": "obs-002",
                "raw_payload_ref": "0000000000000000000000000000000000000000000000000000000000000000",
                "capture_context": {
                    "request_params": null,
                    "environment": {},
                    "session_id": null,
                    "correlation_id": null
                }
            }
        }"#;

        let draft: Proposal<Draft> = serde_json::from_str(json).unwrap();
        assert_eq!(draft.id().as_str(), "proposal-002");
        assert_eq!(draft.content().content, "Deserialized claim");
    }

    // Note: Proposal<Validated> cannot be constructed outside the crate.
    // This is enforced at compile-time by pub(crate) visibility.
    // The following would NOT compile in external code:
    //
    // let validated = Proposal::<Validated>::from_validated(...);
    //
    // error: associated function `from_validated` is private
}
