// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Formation-kind tag exposed to suggestors.
//!
//! The full `Formation` enum (with its variant payloads — `StaticFormation`,
//! `ScoredFormation`, `DeliberatedFormation`, `OpenClawFormation`) lives in
//! `converge-core::formation`. That crate owns formation *configuration* and
//! orchestration. Pack only needs the **tag** so a `Suggestor` running inside
//! a formation can ask "what kind of formation am I in?" without depending
//! on core.
//!
//! This split keeps the layer rule intact (pack does not depend on core)
//! while letting suggestors adapt behavior to formation context. A formation
//! harness that orchestrates inner suggestors is responsible for setting
//! this tag on the `Context` it passes down via
//! [`Context::formation_kind`](crate::Context::formation_kind). Default is
//! `None`, meaning the suggestor is running outside any formation harness
//! and should fall back to its standalone behavior.

use serde::{Deserialize, Serialize};

/// The kind of formation orchestrating a suggestor's current execution.
///
/// Exposed to `Suggestor` implementations via
/// [`Context::formation_kind`](crate::Context::formation_kind). Suggestors
/// should treat this as advisory; pure-context determinism is still the
/// primary contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormationKind {
    /// Fixed ordered set of suggestors; all run every cycle.
    Static,
    /// Ranked candidates; top-N by score participate.
    Scored,
    /// Multi-cycle huddle with confidence threshold.
    Deliberated,
    /// Adaptive variant selection with extra-loop budget.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formation_kind_display() {
        assert_eq!(FormationKind::Static.to_string(), "static");
        assert_eq!(FormationKind::Scored.to_string(), "scored");
        assert_eq!(FormationKind::Deliberated.to_string(), "deliberated");
        assert_eq!(FormationKind::OpenClaw.to_string(), "open_claw");
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
}
