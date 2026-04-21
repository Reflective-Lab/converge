// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Formation {
    Static(StaticFormation),
    Scored(ScoredFormation),
    Deliberated(DeliberatedFormation),
    OpenClaw(OpenClawFormation),
}

impl Formation {
    pub fn kind(&self) -> FormationKind {
        match self {
            Self::Static(_) => FormationKind::Static,
            Self::Scored(_) => FormationKind::Scored,
            Self::Deliberated(_) => FormationKind::Deliberated,
            Self::OpenClaw(_) => FormationKind::OpenClaw,
        }
    }

    pub fn candidate_names(&self) -> &[String] {
        match self {
            Self::Static(f) => &f.suggestor_names,
            Self::Scored(f) => &f.candidate_names,
            Self::Deliberated(f) => &f.candidate_names,
            Self::OpenClaw(f) => &f.candidate_names,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormationKind {
    Static,
    Scored,
    Deliberated,
    OpenClaw,
}

impl std::fmt::Display for FormationKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Static => write!(f, "static"),
            Self::Scored => write!(f, "scored"),
            Self::Deliberated => write!(f, "deliberated"),
            Self::OpenClaw => write!(f, "open_claw"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticFormation {
    pub suggestor_names: Vec<String>,
}

impl StaticFormation {
    pub fn new(suggestor_names: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            suggestor_names: suggestor_names.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredFormation {
    pub candidate_names: Vec<String>,
    pub top_n: usize,
    pub scoring_weights: ScoringWeights,
}

impl ScoredFormation {
    pub fn new(candidate_names: impl IntoIterator<Item = impl Into<String>>, top_n: usize) -> Self {
        Self {
            candidate_names: candidate_names.into_iter().map(Into::into).collect(),
            top_n,
            scoring_weights: ScoringWeights::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringWeights {
    pub cost: f32,
    pub latency: f32,
    pub confidence: f32,
    pub role_coverage: f32,
}

impl ScoringWeights {
    pub fn uniform() -> Self {
        Self {
            cost: 0.25,
            latency: 0.25,
            confidence: 0.25,
            role_coverage: 0.25,
        }
    }

    pub fn sum(&self) -> f32 {
        self.cost + self.latency + self.confidence + self.role_coverage
    }
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self::uniform()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliberatedFormation {
    pub candidate_names: Vec<String>,
    pub huddle_max_cycles: u32,
    pub scoring_weights: ScoringWeights,
    pub min_confidence_threshold: f32,
}

impl DeliberatedFormation {
    pub fn new(
        candidate_names: impl IntoIterator<Item = impl Into<String>>,
        huddle_max_cycles: u32,
    ) -> Self {
        Self {
            candidate_names: candidate_names.into_iter().map(Into::into).collect(),
            huddle_max_cycles,
            scoring_weights: ScoringWeights::default(),
            min_confidence_threshold: 0.6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawFormation {
    pub candidate_names: Vec<String>,
    pub max_extra_loops: u32,
    pub formation_variants: Vec<Formation>,
}

impl OpenClawFormation {
    pub fn new(
        candidate_names: impl IntoIterator<Item = impl Into<String>>,
        max_extra_loops: u32,
    ) -> Self {
        Self {
            candidate_names: candidate_names.into_iter().map(Into::into).collect(),
            max_extra_loops,
            formation_variants: Vec::new(),
        }
    }

    pub fn with_variants(mut self, variants: Vec<Formation>) -> Self {
        self.formation_variants = variants;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationDecision {
    pub selected_formation: Formation,
    pub candidates_considered: Vec<String>,
    pub rationale: String,
    pub confidence: f32,
    pub experience_key: Option<String>,
}

impl FormationDecision {
    pub fn new(
        selected_formation: Formation,
        rationale: impl Into<String>,
        confidence: f32,
    ) -> Self {
        let candidates = selected_formation.candidate_names().to_vec();
        Self {
            selected_formation,
            candidates_considered: candidates,
            rationale: rationale.into(),
            confidence,
            experience_key: None,
        }
    }

    pub fn with_experience_key(mut self, key: impl Into<String>) -> Self {
        self.experience_key = Some(key.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormationOutcome {
    pub formation_kind: FormationKind,
    pub suggestors_used: Vec<String>,
    pub fixed_point_reached: bool,
    pub cycles_used: u32,
    pub extra_loops_used: u32,
    pub quality_score: Option<f32>,
    pub forced_fixed_point: bool,
}

impl FormationOutcome {
    pub fn new(
        formation_kind: FormationKind,
        suggestors_used: Vec<String>,
        fixed_point_reached: bool,
        cycles_used: u32,
    ) -> Self {
        Self {
            formation_kind,
            suggestors_used,
            fixed_point_reached,
            cycles_used,
            extra_loops_used: 0,
            quality_score: None,
            forced_fixed_point: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Formation::kind() and candidate_names() ──────────────────────────────

    #[test]
    fn static_formation_kind() {
        let f = Formation::Static(StaticFormation::new(["a", "b"]));
        assert_eq!(f.kind(), FormationKind::Static);
        assert_eq!(f.candidate_names(), &["a", "b"]);
    }

    #[test]
    fn scored_formation_kind() {
        let f = Formation::Scored(ScoredFormation::new(["x", "y", "z"], 2));
        assert_eq!(f.kind(), FormationKind::Scored);
        assert_eq!(f.candidate_names().len(), 3);
    }

    #[test]
    fn deliberated_formation_kind() {
        let f = Formation::Deliberated(DeliberatedFormation::new(["p", "q"], 5));
        assert_eq!(f.kind(), FormationKind::Deliberated);
    }

    #[test]
    fn open_claw_formation_kind() {
        let f = Formation::OpenClaw(OpenClawFormation::new(["a", "b", "c", "d"], 3));
        assert_eq!(f.kind(), FormationKind::OpenClaw);
        assert_eq!(f.candidate_names().len(), 4);
    }

    // ── FormationKind Display ─────────────────────────────────────────────────

    #[test]
    fn formation_kind_display() {
        assert_eq!(FormationKind::Static.to_string(), "static");
        assert_eq!(FormationKind::Scored.to_string(), "scored");
        assert_eq!(FormationKind::Deliberated.to_string(), "deliberated");
        assert_eq!(FormationKind::OpenClaw.to_string(), "open_claw");
    }

    // ── ScoringWeights ────────────────────────────────────────────────────────

    #[test]
    fn scoring_weights_uniform_sum_to_one() {
        let w = ScoringWeights::uniform();
        let total = w.sum();
        assert!(
            (total - 1.0).abs() < f32::EPSILON,
            "expected sum 1.0, got {total}"
        );
    }

    #[test]
    fn scoring_weights_default_equals_uniform() {
        let a = ScoringWeights::default();
        let b = ScoringWeights::uniform();
        assert_eq!(a.cost, b.cost);
        assert_eq!(a.latency, b.latency);
        assert_eq!(a.confidence, b.confidence);
        assert_eq!(a.role_coverage, b.role_coverage);
    }

    // ── OpenClawFormation with_variants ──────────────────────────────────────

    #[test]
    fn open_claw_with_variants() {
        let variant = Formation::Static(StaticFormation::new(["fallback"]));
        let f = OpenClawFormation::new(["a", "b"], 2).with_variants(vec![variant]);
        assert_eq!(f.formation_variants.len(), 1);
        assert_eq!(f.max_extra_loops, 2);
    }

    // ── FormationDecision ─────────────────────────────────────────────────────

    #[test]
    fn formation_decision_captures_candidates() {
        let formation = Formation::Static(StaticFormation::new(["alpha", "beta"]));
        let decision = FormationDecision::new(formation, "best static fit", 0.9);
        assert_eq!(decision.candidates_considered, vec!["alpha", "beta"]);
        assert_eq!(decision.rationale, "best static fit");
        assert!((decision.confidence - 0.9).abs() < f32::EPSILON);
        assert!(decision.experience_key.is_none());
    }

    #[test]
    fn formation_decision_with_experience_key() {
        let formation = Formation::Deliberated(DeliberatedFormation::new(["a"], 3));
        let decision = FormationDecision::new(formation, "deliberated", 0.75)
            .with_experience_key("exp-abc-123");
        assert_eq!(decision.experience_key, Some("exp-abc-123".into()));
    }

    // ── FormationOutcome ──────────────────────────────────────────────────────

    #[test]
    fn formation_outcome_defaults() {
        let outcome =
            FormationOutcome::new(FormationKind::Scored, vec!["a".into(), "b".into()], true, 4);
        assert_eq!(outcome.formation_kind, FormationKind::Scored);
        assert!(outcome.fixed_point_reached);
        assert_eq!(outcome.cycles_used, 4);
        assert_eq!(outcome.extra_loops_used, 0);
        assert!(!outcome.forced_fixed_point);
        assert!(outcome.quality_score.is_none());
    }

    // ── Serde roundtrips ──────────────────────────────────────────────────────

    #[test]
    fn formation_serde_roundtrip_static() {
        let f = Formation::Static(StaticFormation::new(["a", "b"]));
        let json = serde_json::to_string(&f).unwrap();
        let back: Formation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind(), FormationKind::Static);
        assert_eq!(back.candidate_names(), &["a", "b"]);
    }

    #[test]
    fn formation_serde_roundtrip_open_claw() {
        let inner = Formation::Static(StaticFormation::new(["fallback"]));
        let f =
            Formation::OpenClaw(OpenClawFormation::new(["x", "y"], 5).with_variants(vec![inner]));
        let json = serde_json::to_string(&f).unwrap();
        let back: Formation = serde_json::from_str(&json).unwrap();
        assert_eq!(back.kind(), FormationKind::OpenClaw);
        if let Formation::OpenClaw(oc) = back {
            assert_eq!(oc.formation_variants.len(), 1);
        } else {
            panic!("expected OpenClaw");
        }
    }

    #[test]
    fn formation_kind_serde_roundtrip() {
        for kind in [
            FormationKind::Static,
            FormationKind::Scored,
            FormationKind::Deliberated,
            FormationKind::OpenClaw,
        ] {
            let json = serde_json::to_string(&kind).unwrap();
            let back: FormationKind = serde_json::from_str(&json).unwrap();
            assert_eq!(back, kind);
        }
    }

    #[test]
    fn formation_decision_serde_roundtrip() {
        let formation = Formation::Scored(ScoredFormation::new(["a", "b", "c"], 2));
        let decision =
            FormationDecision::new(formation, "top-2 by score", 0.8).with_experience_key("xp-99");
        let json = serde_json::to_string(&decision).unwrap();
        let back: FormationDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(back.rationale, "top-2 by score");
        assert_eq!(back.experience_key, Some("xp-99".into()));
        assert_eq!(back.selected_formation.kind(), FormationKind::Scored);
    }

    #[test]
    fn formation_outcome_serde_roundtrip() {
        let mut outcome =
            FormationOutcome::new(FormationKind::OpenClaw, vec!["a".into()], false, 10);
        outcome.extra_loops_used = 3;
        outcome.forced_fixed_point = true;
        outcome.quality_score = Some(0.72);

        let json = serde_json::to_string(&outcome).unwrap();
        let back: FormationOutcome = serde_json::from_str(&json).unwrap();
        assert_eq!(back.extra_loops_used, 3);
        assert!(back.forced_fixed_point);
        assert!((back.quality_score.unwrap() - 0.72).abs() < f32::EPSILON);
    }
}
