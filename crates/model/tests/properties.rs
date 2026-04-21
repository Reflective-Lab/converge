// Property tests for converge-model types.
//
// Verify invariants hold across randomized inputs:
// - Serde round-trip for all ID types
// - Display/FromStr consistency
// - Builder invariants
// - Hypothesis confidence clamping

use converge_core::ContentHash;
use converge_model::{
    CaptureContext, ConflictType, FactId, FrameId, Hypothesis, ObservationId, ObservationKind,
    ProposalId, Tension, TensionId, TensionResolution, TensionSide, Timestamp,
    TypesValidationError,
};
use proptest::prelude::*;

// ── ID types: serde round-trip ──

proptest! {
    #[test]
    fn fact_id_serde_roundtrip(s in "[a-zA-Z0-9_-]{1,64}") {
        let id = FactId::new(&s);
        let json = serde_json::to_string(&id).unwrap();
        let round: FactId = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(id.as_str(), round.as_str());
    }

    #[test]
    fn proposal_id_serde_roundtrip(s in "[a-zA-Z0-9_-]{1,64}") {
        let id = ProposalId::new(&s);
        let json = serde_json::to_string(&id).unwrap();
        let round: ProposalId = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(id.as_str(), round.as_str());
    }

    #[test]
    fn tension_id_serde_roundtrip(s in "[a-zA-Z0-9_-]{1,64}") {
        let id = TensionId::new(&s);
        let json = serde_json::to_string(&id).unwrap();
        let round: TensionId = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(id.as_str(), round.as_str());
    }

    #[test]
    fn frame_id_serde_roundtrip(s in "[a-zA-Z0-9_-]{1,64}") {
        let id = FrameId::new(&s);
        let json = serde_json::to_string(&id).unwrap();
        let round: FrameId = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(id.as_str(), round.as_str());
    }

    #[test]
    fn observation_id_serde_roundtrip(s in "[a-zA-Z0-9_-]{1,64}") {
        let id = ObservationId::new(&s);
        let json = serde_json::to_string(&id).unwrap();
        let round: ObservationId = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(id.as_str(), round.as_str());
    }

    #[test]
    fn timestamp_serde_roundtrip(s in "[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z") {
        let ts = Timestamp::new(&s);
        let json = serde_json::to_string(&ts).unwrap();
        let round: Timestamp = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(ts.as_str(), round.as_str());
    }
}

// ── ID Display is identity ──

proptest! {
    #[test]
    fn fact_id_display_is_inner(s in "[a-z]{1,32}") {
        let id = FactId::new(&s);
        prop_assert_eq!(id.to_string(), s);
    }

    #[test]
    fn proposal_id_display_is_inner(s in "[a-z]{1,32}") {
        let id = ProposalId::new(&s);
        prop_assert_eq!(id.to_string(), s);
    }
}

// ── Hypothesis confidence always clamped to [0.0, 1.0] ──

proptest! {
    #[test]
    fn hypothesis_confidence_always_clamped(conf in -100.0f32..100.0) {
        let hyp = Hypothesis::new("h", "claim").with_confidence(conf);
        prop_assert!(hyp.confidence >= 0.0);
        prop_assert!(hyp.confidence <= 1.0);
    }

    #[test]
    fn hypothesis_confidence_preserves_valid(conf in 0.0f32..=1.0) {
        let hyp = Hypothesis::new("h", "claim").with_confidence(conf);
        prop_assert!((hyp.confidence - conf).abs() < f32::EPSILON);
    }
}

// ── Tension invariants ──

proptest! {
    #[test]
    fn tension_unresolved_has_no_winner(
        id in "[a-z]{1,8}",
        p1 in "[a-z]{1,8}",
        p2 in "[a-z]{1,8}",
    ) {
        let tension = Tension::new(
            TensionId::new(&id),
            TensionSide::new(ProposalId::new(&p1), "left"),
            TensionSide::new(ProposalId::new(&p2), "right"),
            ConflictType::Contradiction,
        );
        prop_assert!(!tension.is_resolved());
        prop_assert!(tension.winner().is_none());
    }

    #[test]
    fn tension_resolved_left_returns_left_proposal(
        p1 in "[a-z]{1,8}",
        p2 in "[a-z]{1,8}",
    ) {
        let mut tension = Tension::new(
            TensionId::new("t"),
            TensionSide::new(ProposalId::new(&p1), "left"),
            TensionSide::new(ProposalId::new(&p2), "right"),
            ConflictType::Contradiction,
        );
        tension.resolve(TensionResolution::choose_left("reason", "actor"));
        prop_assert_eq!(tension.winner().unwrap().as_str(), p1.as_str());
    }

    #[test]
    fn tension_resolved_right_returns_right_proposal(
        p1 in "[a-z]{1,8}",
        p2 in "[a-z]{1,8}",
    ) {
        let mut tension = Tension::new(
            TensionId::new("t"),
            TensionSide::new(ProposalId::new(&p1), "left"),
            TensionSide::new(ProposalId::new(&p2), "right"),
            ConflictType::Contradiction,
        );
        tension.resolve(TensionResolution::choose_right("reason", "actor"));
        prop_assert_eq!(tension.winner().unwrap().as_str(), p2.as_str());
    }
}

// ── Tension full serde roundtrip ──

proptest! {
    #[test]
    fn tension_serde_roundtrip(
        id in "[a-z]{1,8}",
        p1 in "[a-z]{1,8}",
        p2 in "[a-z]{1,8}",
    ) {
        let tension = Tension::new(
            TensionId::new(&id),
            TensionSide::new(ProposalId::new(&p1), "left"),
            TensionSide::new(ProposalId::new(&p2), "right"),
            ConflictType::ResourceContention,
        );
        let json = serde_json::to_string(&tension).unwrap();
        let round: Tension = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(round.id.as_str(), id.as_str());
        prop_assert_eq!(round.left.proposal_id.as_str(), p1.as_str());
        prop_assert_eq!(round.right.proposal_id.as_str(), p2.as_str());
    }
}

// ── TypesValidationError serde roundtrip ──

proptest! {
    #[test]
    fn validation_error_serde_roundtrip(
        field in "[a-z_]{1,16}",
        constraint in "[a-z ]{1,32}",
    ) {
        let variants = vec![
            TypesValidationError::EmptyContent,
            TypesValidationError::missing_field(&field),
            TypesValidationError::constraint_violated(&constraint),
        ];
        for v in &variants {
            let json = serde_json::to_string(v).unwrap();
            let round: TypesValidationError = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(v, &round);
        }
    }
}

// ── ContentHash roundtrip ──

proptest! {
    #[test]
    fn content_hash_hex_roundtrip(bytes in prop::array::uniform32(0u8..)) {
        let hash = ContentHash::new(bytes);
        let hex = hash.to_hex();
        let round = ContentHash::from_hex(&hex);
        prop_assert_eq!(hash.as_bytes(), round.as_bytes());
    }
}

// ── Observation serde roundtrip ──

#[test]
fn observation_all_kinds_serialize() {
    use converge_core::types::ProviderIdentity;

    let kinds = [
        ObservationKind::ApiResponse,
        ObservationKind::UserInput,
        ObservationKind::SystemEvent,
        ObservationKind::ExternalFeed,
    ];

    for kind in &kinds {
        let obs = converge_model::Observation::new(
            ObservationId::new("obs-1"),
            *kind,
            ContentHash::zero(),
            ProviderIdentity::new("test", "v1"),
            CaptureContext::new(),
            Timestamp::now(),
        );
        let json = serde_json::to_string(&obs).unwrap();
        let round: converge_model::Observation = serde_json::from_str(&json).unwrap();
        assert_eq!(round.kind, *kind);
    }
}
