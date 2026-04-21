// Integration test: verify converge-model re-exports are usable as a consumer.

use converge_core::ContentHash;
use converge_model::{
    CaptureContext, ConflictType, Criterion, FactId, Frame, FrameConstraint, FrameId, Hypothesis,
    Observation, ObservationError, ObservationId, ObservationKind, PromotionError, ProposalId,
    Tension, TensionId, TensionResolution, TensionSide, TypeError, TypesValidationError,
};

#[test]
fn can_create_proposed_fact_from_model_crate() {
    use converge_model::{ContextKey, ProposedFact, ValidationError};

    let fact = ProposedFact::new(
        ContextKey::Hypotheses,
        "fact-1",
        "The market is growing",
        "observation-1",
    );
    assert_eq!(fact.id, "fact-1");
    assert_eq!(fact.confidence, 1.0);

    let with_conf =
        ProposedFact::new(ContextKey::Strategies, "f-2", "content", "obs-2").with_confidence(0.8);
    assert!((with_conf.confidence - 0.8).abs() < f64::EPSILON);

    let err = ValidationError {
        reason: "test".to_string(),
    };
    assert!(err.to_string().contains("test"));
}

#[test]
fn fact_id_serde_transparent_via_model() {
    let id = FactId::new("fact-abc");
    let json = serde_json::to_string(&id).unwrap();
    assert_eq!(json, r#""fact-abc""#);

    let round: FactId = serde_json::from_str(&json).unwrap();
    assert_eq!(round.as_str(), "fact-abc");
}

#[test]
fn tension_lifecycle_via_model() {
    let mut tension = Tension::new(
        TensionId::new("t-1"),
        TensionSide::new(ProposalId::new("p1"), "Enterprise focus"),
        TensionSide::new(ProposalId::new("p2"), "SMB focus"),
        ConflictType::PriorityMismatch,
    );

    assert!(!tension.is_resolved());
    assert!(tension.winner().is_none());

    tension.resolve(TensionResolution::choose_right("SMB has faster ROI", "cto"));
    assert!(tension.is_resolved());
    assert_eq!(tension.winner().unwrap().as_str(), "p2");
}

#[test]
fn observation_factory_methods_via_model() {
    let obs = Observation::from_user_input(
        ObservationId::new("obs-1"),
        ContentHash::zero(),
        CaptureContext::new(),
    );
    assert_eq!(obs.kind, ObservationKind::UserInput);
    assert_eq!(obs.provider.name, "user");
}

#[test]
fn hypothesis_confidence_boundaries_via_model() {
    let low = Hypothesis::new("h1", "unlikely").with_confidence(0.1);
    assert!(low.is_low_confidence());
    assert!(!low.is_high_confidence());

    let high = Hypothesis::new("h2", "very likely").with_confidence(0.9);
    assert!(high.is_high_confidence());
    assert!(!high.is_low_confidence());

    let mid = Hypothesis::new("h3", "uncertain").with_confidence(0.5);
    assert!(!mid.is_high_confidence());
    assert!(!mid.is_low_confidence());
}

#[test]
fn error_types_accessible_via_model() {
    let te = TypeError::invalid_transition("Draft", "Fact");
    assert!(te.to_string().contains("Draft"));

    let pe = PromotionError::gate_rejected("gate-1", "low confidence");
    assert!(pe.to_string().contains("gate-1"));

    let oe = ObservationError::timeout(3000);
    assert!(oe.to_string().contains("3000"));

    let ve = TypesValidationError::EmptyContent;
    assert!(ve.to_string().contains("empty"));
}

#[test]
fn frame_builder_via_model() {
    let frame = Frame::builder()
        .id(FrameId::new("frame-1"))
        .scope("quarterly-planning".to_string())
        .constraints(vec![FrameConstraint::budget(100_000)])
        .success_criteria(vec![Criterion::required("crit-1", "revenue target met")])
        .build();

    assert_eq!(frame.id.as_str(), "frame-1");
    assert!(frame.has_required_criteria());
    assert_eq!(frame.hard_constraints().count(), 1);
}

#[test]
fn validation_error_serde_via_model() {
    let err = TypesValidationError::schema_mismatch("v1", "v2");
    let json = serde_json::to_string(&err).unwrap();
    let round: TypesValidationError = serde_json::from_str(&json).unwrap();
    assert_eq!(err, round);
}
