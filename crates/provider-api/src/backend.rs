// Copyright 2024-2026 Reflective Labs

// SPDX-License-Identifier: MIT

//! Backend identity and kind.
//!
//! Every external capability in Converge implements [`Backend`]. This trait
//! captures identity and capability declarations — not invocation. Each
//! backend kind has its own invocation traits in its own crate.

use serde::{Deserialize, Serialize};

use crate::capability::Capability;

/// The kind of backend — which agent instantiation strategy it supports.
///
/// This is not a capability (capabilities are declared separately via
/// [`Capability`]). This is the *category* of the backend, used for
/// coarse routing before capability matching.
///
/// # Extensibility
///
/// The `Other(String)` variant allows future backend kinds without
/// breaking the enum. Use it for experimental or domain-specific backends.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BackendKind {
    /// Large language model — cloud or local inference.
    ///
    /// Examples: Anthropic Claude, `OpenAI` GPT, local `llama-burn`.
    Llm,

    /// Policy engine — rule evaluation and access control.
    ///
    /// Examples: Cedar, OPA, Polar (policy mode).
    Policy,

    /// Constraint optimization — resource allocation, scheduling.
    ///
    /// Examples: CP-SAT (OR-Tools), Polar (constraint mode), custom solvers.
    Optimization,

    /// Analytics and ML — embeddings, classification, regression.
    ///
    /// Examples: Burn, `LanceDB`, Polars.
    Analytics,

    /// Search — vector similarity, full-text, semantic.
    ///
    /// Examples: `LanceDB` (vector), Qdrant, Meilisearch.
    Search,

    /// Storage — persistence, event sourcing, document store.
    ///
    /// Examples: `SurrealDB`, `PostgreSQL`, `SQLite`.
    Storage,

    /// Extension point for future or domain-specific backends.
    Other(String),
}

impl std::fmt::Display for BackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Llm => write!(f, "llm"),
            Self::Policy => write!(f, "policy"),
            Self::Optimization => write!(f, "optimization"),
            Self::Analytics => write!(f, "analytics"),
            Self::Search => write!(f, "search"),
            Self::Storage => write!(f, "storage"),
            Self::Other(name) => write!(f, "other:{name}"),
        }
    }
}

/// Identity and capability declaration for any backend in the Converge platform.
///
/// This is the foundational trait that all backend implementations must satisfy.
/// It captures *who* a backend is and *what it can do*, not *how to call it*.
///
/// Specific invocation traits extend this in their own crates:
/// - `ChatBackend` / `EmbedBackend` in converge-core (canonical LLM interfaces)
/// - `PolicyEngine` in converge-policy (Cedar/OPA)
/// - `Solver` in converge-optimization (CP-SAT/Polar)
/// - `Pipeline` in converge-analytics (Burn/LanceDB)
///
/// # Thread Safety
///
/// Backends must be `Send + Sync` for use in concurrent agent execution.
///
/// # Example
///
/// ```
/// use converge_provider_api::{Backend, BackendKind, Capability};
///
/// struct MockLlm;
///
/// impl Backend for MockLlm {
///     fn name(&self) -> &str { "mock-llm" }
///     fn kind(&self) -> BackendKind { BackendKind::Llm }
///     fn capabilities(&self) -> Vec<Capability> {
///         vec![Capability::TextGeneration, Capability::Reasoning]
///     }
/// }
///
/// let backend = MockLlm;
/// assert_eq!(backend.kind(), BackendKind::Llm);
/// assert!(backend.has_capability(Capability::TextGeneration));
/// ```
pub trait Backend: Send + Sync {
    /// Human-readable name for identification and routing.
    ///
    /// Examples: "anthropic-claude", "cedar-policy", "cpsat-optimizer",
    /// "burn-analytics", "lancedb-vector".
    fn name(&self) -> &str;

    /// The kind of backend this is.
    fn kind(&self) -> BackendKind;

    /// Declared capabilities of this backend.
    ///
    /// Used by [`BackendSelector`](crate::selection::BackendSelector) to match
    /// requirements to backends.
    fn capabilities(&self) -> Vec<Capability>;

    /// Provenance string for audit trail.
    ///
    /// Default: `"name:request_id"`. Override for richer provenance.
    fn provenance(&self, request_id: &str) -> String {
        format!("{}:{}", self.name(), request_id)
    }

    /// Check if this backend has a specific capability.
    fn has_capability(&self, capability: Capability) -> bool {
        self.capabilities().contains(&capability)
    }

    /// Whether this backend supports deterministic replay.
    ///
    /// - Local backends with fixed seeds: `true`
    /// - Remote API backends: `false` (model versions can change)
    /// - Policy/optimization engines: typically `true`
    fn supports_replay(&self) -> bool {
        false
    }

    /// Whether this backend requires network access.
    ///
    /// Used for offline-capable deployment decisions.
    fn requires_network(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockLlm;

    impl Backend for MockLlm {
        fn name(&self) -> &str {
            "mock-llm"
        }
        fn kind(&self) -> BackendKind {
            BackendKind::Llm
        }
        fn capabilities(&self) -> Vec<Capability> {
            vec![Capability::TextGeneration, Capability::Reasoning]
        }
    }

    struct OfflinePolicy;

    impl Backend for OfflinePolicy {
        fn name(&self) -> &str {
            "local-policy"
        }
        fn kind(&self) -> BackendKind {
            BackendKind::Policy
        }
        fn capabilities(&self) -> Vec<Capability> {
            vec![Capability::AccessControl]
        }
        fn supports_replay(&self) -> bool {
            true
        }
        fn requires_network(&self) -> bool {
            false
        }
    }

    #[test]
    fn provenance_default() {
        let b = MockLlm;
        assert_eq!(b.provenance("req-123"), "mock-llm:req-123");
    }

    #[test]
    fn has_capability_found() {
        let b = MockLlm;
        assert!(b.has_capability(Capability::TextGeneration));
        assert!(b.has_capability(Capability::Reasoning));
    }

    #[test]
    fn has_capability_not_found() {
        let b = MockLlm;
        assert!(!b.has_capability(Capability::Embedding));
        assert!(!b.has_capability(Capability::AccessControl));
    }

    #[test]
    fn supports_replay_default_false() {
        let b = MockLlm;
        assert!(!b.supports_replay());
    }

    #[test]
    fn supports_replay_override() {
        let b = OfflinePolicy;
        assert!(b.supports_replay());
    }

    #[test]
    fn requires_network_default_true() {
        let b = MockLlm;
        assert!(b.requires_network());
    }

    #[test]
    fn requires_network_override() {
        let b = OfflinePolicy;
        assert!(!b.requires_network());
    }

    #[test]
    fn backend_kind_display() {
        assert_eq!(BackendKind::Llm.to_string(), "llm");
        assert_eq!(BackendKind::Policy.to_string(), "policy");
        assert_eq!(BackendKind::Optimization.to_string(), "optimization");
        assert_eq!(BackendKind::Analytics.to_string(), "analytics");
        assert_eq!(BackendKind::Search.to_string(), "search");
        assert_eq!(BackendKind::Storage.to_string(), "storage");
        assert_eq!(
            BackendKind::Other("custom".into()).to_string(),
            "other:custom"
        );
    }
}
