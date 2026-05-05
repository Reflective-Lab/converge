// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Facts and proposed facts — the type boundary.
//!
//! This is the most important design decision in Converge: LLMs suggest,
//! the engine validates. `ProposedFact` is not `Fact`. There is no implicit
//! conversion between them.

use serde::{Deserialize, Serialize};

use crate::context::ContextKey;
use crate::types::{
    ActorId, ApprovalId, ArtifactId, ContentHash, FactId, GateId, ObservationId, ProposalId,
    SpanId, Timestamp, TraceId, TraceReference, TraceSystemId, UnitInterval, ValidationCheckId,
};

/// Actor kind recorded on a promoted fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FactActorKind {
    /// Human approver.
    Human,
    /// Suggestor or automated domain actor.
    Suggestor,
    /// Kernel or system component.
    System,
}

/// Read-only actor record attached to authoritative facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactActor {
    id: ActorId,
    kind: FactActorKind,
}

impl FactActor {
    /// Returns the actor identifier.
    #[must_use]
    pub fn id(&self) -> &ActorId {
        &self.id
    }

    /// Returns the actor kind.
    #[must_use]
    pub fn kind(&self) -> FactActorKind {
        self.kind
    }

    #[doc(hidden)]
    pub fn new_projection(id: impl Into<ActorId>, kind: FactActorKind) -> Self {
        Self {
            id: id.into(),
            kind,
        }
    }
}

/// Summary of validation checks attached to an authoritative fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct FactValidationSummary {
    checks_passed: Vec<ValidationCheckId>,
    checks_skipped: Vec<ValidationCheckId>,
    warnings: Vec<String>,
}

impl FactValidationSummary {
    /// Returns validation checks that passed.
    #[must_use]
    pub fn checks_passed(&self) -> &[ValidationCheckId] {
        &self.checks_passed
    }

    /// Returns validation checks that were skipped.
    #[must_use]
    pub fn checks_skipped(&self) -> &[ValidationCheckId] {
        &self.checks_skipped
    }

    /// Returns validation warnings.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    #[doc(hidden)]
    pub fn new_projection(
        checks_passed: Vec<ValidationCheckId>,
        checks_skipped: Vec<ValidationCheckId>,
        warnings: Vec<String>,
    ) -> Self {
        Self {
            checks_passed,
            checks_skipped,
            warnings,
        }
    }
}

/// Typed evidence references attached to an authoritative fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "id")]
pub enum FactEvidenceRef {
    /// Observation used as evidence.
    Observation(ObservationId),
    /// Human approval used as evidence.
    HumanApproval(ApprovalId),
    /// Derived artifact used as evidence.
    Derived(ArtifactId),
}

/// Local replayable trace reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactLocalTrace {
    trace_id: TraceId,
    span_id: SpanId,
    parent_span_id: Option<SpanId>,
    sampled: bool,
}

impl FactLocalTrace {
    /// Returns the trace identifier.
    #[must_use]
    pub fn trace_id(&self) -> &TraceId {
        &self.trace_id
    }

    /// Returns the span identifier.
    #[must_use]
    pub fn span_id(&self) -> &SpanId {
        &self.span_id
    }

    /// Returns the parent span identifier.
    #[must_use]
    pub fn parent_span_id(&self) -> Option<&SpanId> {
        self.parent_span_id.as_ref()
    }

    /// Returns whether the trace was sampled.
    #[must_use]
    pub fn sampled(&self) -> bool {
        self.sampled
    }

    #[doc(hidden)]
    pub fn new_projection(
        trace_id: impl Into<TraceId>,
        span_id: impl Into<SpanId>,
        parent_span_id: Option<SpanId>,
        sampled: bool,
    ) -> Self {
        Self {
            trace_id: trace_id.into(),
            span_id: span_id.into(),
            parent_span_id,
            sampled,
        }
    }
}

/// Remote audit-only trace reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactRemoteTrace {
    system: TraceSystemId,
    reference: TraceReference,
    retrieval_auth: Option<String>,
    retention_hint: Option<String>,
}

impl FactRemoteTrace {
    /// Returns the remote system identifier.
    #[must_use]
    pub fn system(&self) -> &TraceSystemId {
        &self.system
    }

    /// Returns the remote trace reference.
    #[must_use]
    pub fn reference(&self) -> &TraceReference {
        &self.reference
    }

    /// Returns the retrieval auth hint.
    #[must_use]
    pub fn retrieval_auth(&self) -> Option<&str> {
        self.retrieval_auth.as_deref()
    }

    /// Returns the retention hint.
    #[must_use]
    pub fn retention_hint(&self) -> Option<&str> {
        self.retention_hint.as_deref()
    }

    #[doc(hidden)]
    pub fn new_projection(
        system: impl Into<TraceSystemId>,
        reference: impl Into<TraceReference>,
        retrieval_auth: Option<String>,
        retention_hint: Option<String>,
    ) -> Self {
        Self {
            system: system.into(),
            reference: reference.into(),
            retrieval_auth,
            retention_hint,
        }
    }
}

/// Trace record attached to an authoritative fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FactTraceLink {
    /// Local replayable trace.
    Local(FactLocalTrace),
    /// Remote audit-only trace.
    Remote(FactRemoteTrace),
}

impl FactTraceLink {
    /// Returns whether the trace is replay-eligible.
    #[must_use]
    pub fn is_replay_eligible(&self) -> bool {
        matches!(self, Self::Local(_))
    }
}

/// Read-only promotion record attached to an authoritative fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FactPromotionRecord {
    gate_id: GateId,
    policy_version_hash: ContentHash,
    approver: FactActor,
    validation_summary: FactValidationSummary,
    evidence_refs: Vec<FactEvidenceRef>,
    trace_link: FactTraceLink,
    promoted_at: Timestamp,
}

impl FactPromotionRecord {
    /// Returns the gate identifier that promoted the fact.
    #[must_use]
    pub fn gate_id(&self) -> &GateId {
        &self.gate_id
    }

    /// Returns the policy hash used during promotion.
    #[must_use]
    pub fn policy_version_hash(&self) -> &ContentHash {
        &self.policy_version_hash
    }

    /// Returns the approving actor.
    #[must_use]
    pub fn approver(&self) -> &FactActor {
        &self.approver
    }

    /// Returns the validation summary.
    #[must_use]
    pub fn validation_summary(&self) -> &FactValidationSummary {
        &self.validation_summary
    }

    /// Returns the evidence references used during promotion.
    #[must_use]
    pub fn evidence_refs(&self) -> &[FactEvidenceRef] {
        &self.evidence_refs
    }

    /// Returns the trace link for audit or replay.
    #[must_use]
    pub fn trace_link(&self) -> &FactTraceLink {
        &self.trace_link
    }

    /// Returns the promotion timestamp.
    #[must_use]
    pub fn promoted_at(&self) -> &Timestamp {
        &self.promoted_at
    }

    /// Returns whether the promotion is replay-eligible.
    #[must_use]
    pub fn is_replay_eligible(&self) -> bool {
        self.trace_link.is_replay_eligible()
    }

    #[doc(hidden)]
    pub fn new_projection(
        gate_id: impl Into<GateId>,
        policy_version_hash: ContentHash,
        approver: FactActor,
        validation_summary: FactValidationSummary,
        evidence_refs: Vec<FactEvidenceRef>,
        trace_link: FactTraceLink,
        promoted_at: impl Into<Timestamp>,
    ) -> Self {
        Self {
            gate_id: gate_id.into(),
            policy_version_hash,
            approver,
            validation_summary,
            evidence_refs,
            trace_link,
            promoted_at: promoted_at.into(),
        }
    }
}

/// Read-only projection of a validated assertion in the context.
///
/// This type is not promotion authority. It is the value suggestors and
/// pack authors can read from context after the engine has promoted a
/// proposal. Constructing one locally does not admit it into Converge; there is
/// no public API that accepts a `ContextFact` as promoted truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextFact {
    /// Which context key this fact belongs to.
    key: ContextKey,
    /// Unique identifier within the context key namespace.
    id: FactId,
    /// The fact's content as a string. Interpretation is key-dependent.
    content: String,
    /// The immutable promotion record that made this fact authoritative.
    promotion_record: FactPromotionRecord,
    /// When the authoritative fact entered context.
    created_at: Timestamp,
}

impl ContextFact {
    /// Creates a read-only context projection.
    ///
    /// This constructor does not promote anything and is intentionally named as
    /// a projection constructor. The engine is still the only component that can
    /// add context facts to a live `ContextState`.
    #[must_use]
    pub fn new_projection(
        key: ContextKey,
        id: impl Into<FactId>,
        content: impl Into<String>,
        promotion_record: FactPromotionRecord,
        created_at: impl Into<Timestamp>,
    ) -> Self {
        Self {
            key,
            id: id.into(),
            content: content.into(),
            promotion_record,
            created_at: created_at.into(),
        }
    }

    /// Returns the context key this fact belongs to.
    #[must_use]
    pub fn key(&self) -> ContextKey {
        self.key
    }

    /// Returns the fact identifier.
    #[must_use]
    pub fn id(&self) -> &FactId {
        &self.id
    }

    /// Returns the fact content.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the immutable promotion record for this fact.
    #[must_use]
    pub fn promotion_record(&self) -> &FactPromotionRecord {
        &self.promotion_record
    }

    /// Returns the fact creation timestamp.
    #[must_use]
    pub fn created_at(&self) -> &Timestamp {
        &self.created_at
    }

    /// Returns whether the fact is replay-eligible.
    #[must_use]
    pub fn is_replay_eligible(&self) -> bool {
        self.promotion_record.is_replay_eligible()
    }

    /// Parse the fact's content as JSON into a typed value.
    ///
    /// This helper is deliberately named for JSON and preserves parse errors.
    /// Callers that use another representation should parse `content` with
    /// that representation's decoder.
    pub fn parse_json_content<T: serde::de::DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::from_str(&self.content)
    }
}

/// An unvalidated suggestion from a non-authoritative source.
///
/// Proposed facts live in `ContextKey::Proposals` until a `ValidationAgent`
/// promotes them to `Fact`. The proposal tracks its origin for audit trail.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProposedFact {
    /// The context key this proposal targets.
    pub key: ContextKey,
    /// Unique identifier encoding origin and target.
    pub id: ProposalId,
    /// The proposed content.
    pub content: String,
    /// Confidence hint from the source. Always in [0.0, 1.0].
    confidence: UnitInterval,
    /// Provenance information (e.g., model ID, prompt hash).
    pub provenance: String,
}

impl ProposedFact {
    /// Create a new draft proposal with explicit provenance.
    ///
    /// Confidence defaults to 1.0. Override with [`with_confidence`][Self::with_confidence].
    #[must_use]
    pub fn new(
        key: ContextKey,
        id: impl Into<ProposalId>,
        content: impl Into<String>,
        provenance: impl Into<String>,
    ) -> Self {
        Self {
            key,
            id: id.into(),
            content: content.into(),
            confidence: UnitInterval::ONE,
            provenance: provenance.into(),
        }
    }

    /// Returns the context key this proposal targets.
    #[must_use]
    pub fn key(&self) -> ContextKey {
        self.key
    }

    /// Returns the proposal identifier.
    #[must_use]
    pub fn id(&self) -> &ProposalId {
        &self.id
    }

    /// Returns the proposed content.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the proposal provenance string.
    #[must_use]
    pub fn provenance(&self) -> &str {
        &self.provenance
    }

    /// Returns the confidence value, always in [0.0, 1.0].
    #[must_use]
    pub fn confidence(&self) -> f64 {
        self.confidence.as_f64()
    }

    /// Set an explicit confidence baseline for this proposal.
    ///
    /// Use this to establish a starting point, then accumulate criteria with
    /// [`adjust_confidence`][Self::adjust_confidence]. The value is clamped to
    /// [0.0, 1.0]; non-finite values (NaN, infinity) are treated as 0.0.
    ///
    /// For computed confidence (e.g. from a solver), pass the result directly.
    #[must_use]
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = UnitInterval::clamped(confidence);
        self
    }

    /// Parse the proposal's content as JSON into a typed value.
    ///
    /// This helper is deliberately named for JSON and preserves parse errors.
    /// Callers that use another representation should parse `content` with
    /// that representation's decoder.
    pub fn parse_json_content<T: serde::de::DeserializeOwned>(&self) -> serde_json::Result<T> {
        serde_json::from_str(&self.content)
    }

    /// Construct a proposal whose content is `payload` serialized to JSON.
    ///
    /// Symmetric with [`parse_json_content`][Self::parse_json_content] and named
    /// for the same reason: callers using another representation should serialize
    /// `content` themselves and pass it to [`new`][Self::new].
    ///
    /// Returns a `serde_json::Error` only if `T` is non-representable as JSON
    /// (e.g. floats with NaN, maps with non-string keys). For payload types that
    /// are always representable, callers can `.expect("payload always serializable")`.
    pub fn from_json_payload<T: serde::Serialize>(
        key: ContextKey,
        id: impl Into<ProposalId>,
        payload: &T,
        provenance: impl Into<String>,
    ) -> serde_json::Result<Self> {
        Ok(Self::new(
            key,
            id,
            serde_json::to_string(payload)?,
            provenance,
        ))
    }

    /// Adjust confidence by a named step, clamped to [0.0, 1.0].
    ///
    /// This is the recommended way to express confidence in suggestors and pack
    /// solvers. Use the `CONFIDENCE_STEP_*` constants as the vocabulary:
    ///
    /// ```rust,ignore
    /// use converge_pack::{CONFIDENCE_STEP_MAJOR, CONFIDENCE_STEP_MINOR, CONFIDENCE_STEP_TINY};
    ///
    /// let proposal = ProposedFact::new(key, id, content, prov)
    ///     .with_confidence(0.5)                        // baseline
    ///     .adjust_confidence(CONFIDENCE_STEP_MAJOR)    // primary criterion met
    ///     .adjust_confidence(CONFIDENCE_STEP_MINOR)    // supporting criterion met
    ///     .adjust_confidence(CONFIDENCE_STEP_TINY);    // tiebreaker bonus
    /// ```
    ///
    /// Prefer this over accumulating a local `f64` and calling `with_confidence`
    /// at the end — the clamping is automatic and the intent is explicit at each step.
    #[must_use]
    pub fn adjust_confidence(mut self, delta: f64) -> Self {
        self.confidence = self.confidence.saturating_add(delta);
        self
    }
}

/// Tiny confidence step — use for marginal or tiebreaker criteria (0.05).
pub const CONFIDENCE_STEP_TINY: f64 = 0.05;

/// Minor confidence step — use for supporting criteria (0.1).
pub const CONFIDENCE_STEP_MINOR: f64 = 0.1;

/// Medium confidence step — use for moderately significant criteria (0.15).
pub const CONFIDENCE_STEP_MEDIUM: f64 = 0.15;

/// Major confidence step — use for significant criteria (0.2).
pub const CONFIDENCE_STEP_MAJOR: f64 = 0.2;

/// Primary confidence step — use for decisive or high-weight criteria (0.25).
pub const CONFIDENCE_STEP_PRIMARY: f64 = 0.25;

/// Error when a `ProposedFact` fails validation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationError {
    /// Reason the proposal was rejected.
    pub reason: String,
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "validation failed: {}", self.reason)
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn projection_record() -> FactPromotionRecord {
        FactPromotionRecord::new_projection(
            "projection-test",
            ContentHash::from_hex(
                "1111111111111111111111111111111111111111111111111111111111111111",
            ),
            FactActor::new_projection("actor-1", FactActorKind::System),
            FactValidationSummary::default(),
            Vec::new(),
            FactTraceLink::Local(FactLocalTrace::new_projection(
                "trace-1", "span-1", None, true,
            )),
            Timestamp::epoch(),
        )
    }

    fn projection_fact(
        key: ContextKey,
        id: impl Into<FactId>,
        content: impl Into<String>,
    ) -> ContextFact {
        ContextFact::new_projection(key, id, content, projection_record(), Timestamp::epoch())
    }

    #[test]
    fn trace_link_local_is_replay_eligible() {
        let local = FactTraceLink::Local(FactLocalTrace {
            trace_id: "t1".into(),
            span_id: "s1".into(),
            parent_span_id: None,
            sampled: true,
        });
        assert!(local.is_replay_eligible());
    }

    #[test]
    fn trace_link_remote_is_not_replay_eligible() {
        let remote = FactTraceLink::Remote(FactRemoteTrace {
            system: "datadog".into(),
            reference: "ref-1".into(),
            retrieval_auth: None,
            retention_hint: None,
        });
        assert!(!remote.is_replay_eligible());
    }

    #[test]
    fn promotion_record_delegates_replay_eligibility() {
        let local_record = FactPromotionRecord::new_projection(
            "gate-1",
            ContentHash::from_hex(
                "1111111111111111111111111111111111111111111111111111111111111111",
            ),
            FactActor::new_projection("actor-1", FactActorKind::Human),
            FactValidationSummary::default(),
            Vec::new(),
            FactTraceLink::Local(FactLocalTrace::new_projection("t1", "s1", None, true)),
            "2026-01-01T00:00:00Z",
        );
        assert!(local_record.is_replay_eligible());

        let remote_record = FactPromotionRecord::new_projection(
            "gate-2",
            ContentHash::from_hex(
                "2222222222222222222222222222222222222222222222222222222222222222",
            ),
            FactActor::new_projection("actor-2", FactActorKind::System),
            FactValidationSummary::default(),
            Vec::new(),
            FactTraceLink::Remote(FactRemoteTrace::new_projection("dd", "ref-1", None, None)),
            "2026-01-01T00:00:00Z",
        );
        assert!(!remote_record.is_replay_eligible());
    }

    #[test]
    fn fact_delegates_replay_eligibility() {
        let fact = projection_fact(ContextKey::Seeds, "f1", "content");
        assert!(fact.is_replay_eligible());
    }

    #[test]
    fn proposed_fact_new_sets_fields() {
        let pf = ProposedFact::new(ContextKey::Hypotheses, "p1", "my content", "gpt-4");
        assert_eq!(pf.key, ContextKey::Hypotheses);
        assert_eq!(pf.id, "p1");
        assert_eq!(pf.content, "my content");
        assert_eq!(pf.confidence(), 1.0);
        assert_eq!(pf.provenance, "gpt-4");
    }

    #[test]
    fn proposed_fact_with_confidence() {
        let pf = ProposedFact::new(ContextKey::Signals, "p2", "c", "prov").with_confidence(0.42);
        assert!((pf.confidence() - 0.42).abs() < f64::EPSILON);
    }

    #[test]
    fn adjust_confidence_accumulates() {
        let pf = ProposedFact::new(ContextKey::Seeds, "p", "c", "x")
            .with_confidence(0.5)
            .adjust_confidence(CONFIDENCE_STEP_MINOR)
            .adjust_confidence(CONFIDENCE_STEP_MAJOR);
        assert!((pf.confidence() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn adjust_confidence_clamps_at_one() {
        let pf = ProposedFact::new(ContextKey::Seeds, "p", "c", "x")
            .with_confidence(0.9)
            .adjust_confidence(CONFIDENCE_STEP_MAJOR);
        assert_eq!(pf.confidence(), 1.0);
    }

    #[test]
    fn adjust_confidence_clamps_at_zero() {
        let pf = ProposedFact::new(ContextKey::Seeds, "p", "c", "x")
            .with_confidence(0.1)
            .adjust_confidence(-0.5);
        assert_eq!(pf.confidence(), 0.0);
    }

    #[test]
    fn with_confidence_clamps_high() {
        let pf = ProposedFact::new(ContextKey::Seeds, "p", "c", "x").with_confidence(1.5);
        assert_eq!(pf.confidence(), 1.0);
    }

    #[test]
    fn with_confidence_clamps_negative() {
        let pf = ProposedFact::new(ContextKey::Seeds, "p", "c", "x").with_confidence(-0.1);
        assert_eq!(pf.confidence(), 0.0);
    }

    #[test]
    fn with_confidence_normalizes_nan() {
        let pf = ProposedFact::new(ContextKey::Seeds, "p", "c", "x").with_confidence(f64::NAN);
        assert_eq!(pf.confidence(), 0.0);
    }

    #[test]
    fn with_confidence_normalizes_infinity() {
        let pf = ProposedFact::new(ContextKey::Seeds, "p", "c", "x").with_confidence(f64::INFINITY);
        assert_eq!(pf.confidence(), 0.0);
    }

    #[test]
    fn proposed_fact_deserialization_rejects_out_of_range_confidence() {
        let json = r#"{
            "key":"Seeds",
            "id":"p",
            "content":"c",
            "confidence":1.5,
            "provenance":"test"
        }"#;
        let result = serde_json::from_str::<ProposedFact>(json);
        assert!(result.is_err());
    }

    #[test]
    fn proposed_fact_parse_json_content_succeeds_for_valid_json() {
        #[derive(serde::Deserialize, PartialEq, Debug)]
        struct Payload {
            kind: String,
            score: f64,
        }
        let pf = ProposedFact::new(
            ContextKey::Hypotheses,
            "p",
            r#"{"kind":"vote","score":0.7}"#,
            "test",
        );
        let parsed: Payload = pf.parse_json_content().unwrap();
        assert_eq!(
            parsed,
            Payload {
                kind: "vote".into(),
                score: 0.7,
            }
        );
    }

    #[test]
    fn proposed_fact_parse_json_content_returns_error_for_invalid_json() {
        let pf = ProposedFact::new(ContextKey::Hypotheses, "p", "not json", "test");
        let parsed = pf.parse_json_content::<serde_json::Value>();
        assert!(parsed.is_err());
    }

    #[test]
    fn fact_parse_json_content_succeeds_for_valid_json() {
        #[derive(serde::Deserialize, PartialEq, Debug)]
        struct Payload {
            label: String,
        }
        let fact = projection_fact(ContextKey::Seeds, "f", r#"{"label":"x"}"#);
        let parsed: Payload = fact.parse_json_content().unwrap();
        assert_eq!(parsed, Payload { label: "x".into() });
    }

    #[test]
    fn proposed_fact_from_json_payload_round_trips() {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Payload {
            kind: String,
            score: f64,
        }
        let payload = Payload {
            kind: "vote".into(),
            score: 0.7,
        };
        let pf =
            ProposedFact::from_json_payload(ContextKey::Hypotheses, "p", &payload, "test").unwrap();
        assert_eq!(pf.key, ContextKey::Hypotheses);
        assert_eq!(pf.id, "p");
        assert_eq!(pf.provenance, "test");
        let parsed: Payload = pf.parse_json_content().unwrap();
        assert_eq!(parsed, payload);
    }

    #[test]
    fn proposed_fact_from_json_payload_propagates_serialization_error() {
        use std::collections::HashMap;
        let mut map: HashMap<Vec<u8>, &str> = HashMap::new();
        map.insert(vec![1, 2, 3], "value");
        let result = ProposedFact::from_json_payload(ContextKey::Hypotheses, "p", &map, "test");
        assert!(result.is_err());
    }

    #[test]
    fn fact_projection_json_payload_round_trips() {
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct Payload {
            label: String,
        }
        let payload = Payload { label: "x".into() };
        let fact = projection_fact(
            ContextKey::Seeds,
            "f",
            serde_json::to_string(&payload).unwrap(),
        );
        let parsed: Payload = fact.parse_json_content().unwrap();
        assert_eq!(parsed, payload);
    }

    #[test]
    fn validation_error_display() {
        let err = ValidationError {
            reason: "bad input".into(),
        };
        assert_eq!(err.to_string(), "validation failed: bad input");
    }

    #[test]
    fn validation_error_is_std_error() {
        let err = ValidationError {
            reason: "test".into(),
        };
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn fact_accessors() {
        let fact = projection_fact(ContextKey::Constraints, "f2", "body");
        assert_eq!(fact.key(), ContextKey::Constraints);
        assert_eq!(fact.id(), "f2");
        assert_eq!(fact.content(), "body");
        assert_eq!(fact.created_at(), "1970-01-01T00:00:00Z");
        assert_eq!(fact.promotion_record().gate_id(), "projection-test");
    }

    #[test]
    fn fact_actor_accessors() {
        let actor = FactActor::new_projection("agent-x", FactActorKind::Suggestor);
        assert_eq!(actor.id(), "agent-x");
        assert_eq!(actor.kind(), FactActorKind::Suggestor);
    }

    #[test]
    fn validation_summary_accessors() {
        let vs = FactValidationSummary::new_projection(
            vec!["check-a".into()],
            vec!["check-b".into()],
            vec!["warn-c".into()],
        );
        assert_eq!(vs.checks_passed(), &["check-a"]);
        assert_eq!(vs.checks_skipped(), &["check-b"]);
        assert_eq!(vs.warnings(), &["warn-c"]);
    }

    #[test]
    fn local_trace_accessors() {
        let lt =
            FactLocalTrace::new_projection("trace-1", "span-1", Some("parent-1".into()), false);
        assert_eq!(lt.trace_id(), "trace-1");
        assert_eq!(lt.span_id(), "span-1");
        assert_eq!(lt.parent_span_id().map(SpanId::as_str), Some("parent-1"));
        assert!(!lt.sampled());
    }

    #[test]
    fn remote_trace_accessors() {
        let rt =
            FactRemoteTrace::new_projection("sys", "ref", Some("auth".into()), Some("30d".into()));
        assert_eq!(rt.system(), "sys");
        assert_eq!(rt.reference(), "ref");
        assert_eq!(rt.retrieval_auth(), Some("auth"));
        assert_eq!(rt.retention_hint(), Some("30d"));
    }

    mod prop {
        use super::*;
        use proptest::prelude::*;

        fn arb_context_key() -> impl Strategy<Value = ContextKey> {
            prop_oneof![
                Just(ContextKey::Seeds),
                Just(ContextKey::Hypotheses),
                Just(ContextKey::Strategies),
                Just(ContextKey::Constraints),
                Just(ContextKey::Signals),
                Just(ContextKey::Competitors),
                Just(ContextKey::Evaluations),
                Just(ContextKey::Proposals),
                Just(ContextKey::Diagnostic),
                Just(ContextKey::Votes),
                Just(ContextKey::Disagreements),
                Just(ContextKey::ConsensusOutcomes),
            ]
        }

        proptest! {
            #[test]
            fn proposed_fact_always_constructible(
                key in arb_context_key(),
                id in "[a-z]{1,20}",
                content in ".*",
                prov in "[a-z0-9-]{1,30}",
            ) {
                let pf = ProposedFact::new(key, id.clone(), content.clone(), prov.clone());
                prop_assert_eq!(pf.key, key);
                prop_assert_eq!(&pf.id, &id);
                prop_assert_eq!(&pf.content, &content);
                prop_assert_eq!(&pf.provenance, &prov);
                prop_assert!((pf.confidence() - 1.0).abs() < f64::EPSILON);
            }
        }
    }
}
