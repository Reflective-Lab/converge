// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Recall Types — Portable across all backends
//!
//! This module defines the **constitutional types** for semantic recall.
//! These types encode the core axiom: "Recall ≠ Evidence".
//!
//! ## Axiom: Recall ≠ Evidence
//!
//! Recall provides **hints** to guide reasoning, not **citations** to justify claims.
//! Validators MUST reject any output that cites recall content as evidence.
//!
//! ## What lives here (converge-core)
//!
//! - `RecallQuery`, `RecallCandidate`, `RecallPolicy`, `RecallBudgets`
//! - `RecallProvenanceEnvelope`, `RecallTraceLink`
//! - `CandidateSourceType`, `CandidateScore`, `StopReason`
//! - `RecallUse`, `RecallConsumer` (training boundary types)
//!
//! ## What stays in converge-llm
//!
//! - `HashEmbedder`, `SemanticEmbedder` (implementations)
//! - `RecallNormalizer` (tightly coupled to prompt injection)
//! - PII redaction utilities
//! - `MockRecallProvider`

use crate::experience_store::{
    EventQuery, ExperienceEvent, ExperienceRecord, ExperienceStore, ExperienceStoreResult,
    UserExperienceEvent,
};
use crate::kernel_boundary::DecisionStep;
use crate::types::TenantId;
use serde::{Deserialize, Serialize};

// ============================================================================
// Recall Use/Consumer Types (Recall ≠ Training boundary)
// ============================================================================

/// Purpose of a recall operation.
///
/// Distinguishes runtime augmentation (injecting hints into prompts) from
/// training-time candidate selection (building datasets). This separation
/// preserves "Recall ≠ Evidence" and "Recall ≠ Training" boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecallUse {
    /// Runtime prompt augmentation (hints only, not evidence)
    RuntimeAugmentation,
    /// Training data candidate selection (offline, auditable)
    TrainingCandidateSelection,
}

impl Default for RecallUse {
    fn default() -> Self {
        Self::RuntimeAugmentation
    }
}

/// Consumer of recall results.
///
/// Tracks which component is using the recall results for audit trails
/// and to enforce that training consumers cannot masquerade as runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RecallConsumer {
    /// Reasoning kernel (runtime prompts)
    Kernel,
    /// Analytics pipeline (eval, metrics)
    Analytics,
    /// Training pipeline (dataset building)
    Trainer,
}

impl Default for RecallConsumer {
    fn default() -> Self {
        Self::Kernel
    }
}

// ============================================================================
// Recall Policy and Configuration
// ============================================================================

/// Policy controlling recall behavior.
///
/// This is the declarative configuration for recall operations.
/// It controls what is allowed, not how it is implemented.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallPolicy {
    /// Whether recall is enabled
    pub enabled: bool,
    /// Maximum number of candidates to return total
    pub max_k_total: usize,
    /// Maximum tokens to inject from recall context
    pub max_tokens_injection: usize,
    /// Minimum similarity score threshold
    pub min_score_threshold: f64,
    /// Budget constraints
    pub budgets: RecallBudgets,
    /// Allowed recall uses (runtime, training, etc.)
    ///
    /// Defaults to `[RuntimeAugmentation]` only - training use must be
    /// explicitly enabled. This preserves "Recall ≠ Training" boundary.
    #[serde(default = "default_allowed_uses")]
    pub allowed_uses: Vec<RecallUse>,

    /// How strongly recall results weight planning priors when consumed by
    /// `PlanningPriorAgent`. `1.0` means full weight; `0.0` disables prior
    /// adjustment without disabling recall itself. Capped to `[0.0, 1.0]` by
    /// consumers.
    #[serde(default = "default_prior_weight")]
    pub prior_weight: f64,
}

fn default_prior_weight() -> f64 {
    1.0
}

fn default_allowed_uses() -> Vec<RecallUse> {
    vec![RecallUse::RuntimeAugmentation]
}

impl Default for RecallPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            max_k_total: 5,
            max_tokens_injection: 500,
            min_score_threshold: 0.5,
            budgets: RecallBudgets::default(),
            allowed_uses: default_allowed_uses(),
            prior_weight: default_prior_weight(),
        }
    }
}

impl RecallPolicy {
    /// Create an enabled recall policy with default settings.
    #[must_use]
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Create a disabled recall policy.
    #[must_use]
    pub fn disabled() -> Self {
        Self::default()
    }

    /// Check if a specific recall use is allowed by this policy.
    ///
    /// Returns `true` if the policy allows the given use, `false` otherwise.
    /// This is the primary enforcement point for "Recall ≠ Training" boundary.
    #[must_use]
    pub fn is_use_allowed(&self, purpose: RecallUse) -> bool {
        self.allowed_uses.contains(&purpose)
    }

    /// Compute a deterministic hash of this policy for provenance tracking.
    ///
    /// This enables replay verification: same policy hash → same behavior.
    /// Note: Includes `allowed_uses` in the hash for full provenance.
    #[must_use]
    pub fn snapshot_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.enabled.hash(&mut hasher);
        self.max_k_total.hash(&mut hasher);
        self.max_tokens_injection.hash(&mut hasher);
        (self.min_score_threshold as u64).hash(&mut hasher);
        self.budgets.max_latency_ms.hash(&mut hasher);
        self.budgets.max_embedding_calls.hash(&mut hasher);
        self.budgets.max_tokens_per_candidate.hash(&mut hasher);
        for use_type in &self.allowed_uses {
            (*use_type as u8).hash(&mut hasher);
        }
        (self.prior_weight as u64).hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }
}

/// Check if a recall use is allowed by the given policy.
///
/// Standalone function for use at kernel boundary enforcement.
/// Returns `true` if the policy allows the given purpose.
#[must_use]
pub fn recall_use_allowed(policy: &RecallPolicy, purpose: RecallUse) -> bool {
    policy.is_use_allowed(purpose)
}

/// Budget constraints for recall operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallBudgets {
    /// Maximum latency in milliseconds for recall operations
    pub max_latency_ms: u64,
    /// Maximum number of embedding calls per chain
    pub max_embedding_calls: usize,
    /// Maximum tokens per candidate summary
    pub max_tokens_per_candidate: usize,
}

impl Default for RecallBudgets {
    fn default() -> Self {
        Self {
            max_latency_ms: 100,
            max_embedding_calls: 3,
            max_tokens_per_candidate: 100,
        }
    }
}

// ============================================================================
// Recall Query and Candidate Types
// ============================================================================

/// A query for semantic recall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallQuery {
    /// The text to find similar items for
    pub query_text: String,
    /// Number of candidates to return
    pub top_k: usize,
    /// Optional step context for filtering
    pub step_context: Option<DecisionStep>,
    /// Optional tenant scope
    pub tenant_scope: Option<String>,
}

impl RecallQuery {
    /// Create a new recall query.
    #[must_use]
    pub fn new(query_text: impl Into<String>, top_k: usize) -> Self {
        Self {
            query_text: query_text.into(),
            top_k,
            step_context: None,
            tenant_scope: None,
        }
    }

    /// Add step context filter.
    #[must_use]
    pub fn with_step_context(mut self, step: DecisionStep) -> Self {
        self.step_context = Some(step);
        self
    }

    /// Add tenant scope filter.
    #[must_use]
    pub fn with_tenant_scope(mut self, tenant: impl Into<String>) -> Self {
        self.tenant_scope = Some(tenant.into());
        self
    }

    /// Compute a deterministic hash of this query for provenance tracking.
    #[must_use]
    pub fn query_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.query_text.hash(&mut hasher);
        self.top_k.hash(&mut hasher);
        if let Some(ref step) = self.step_context {
            step.as_str().hash(&mut hasher);
        }
        if let Some(ref tenant) = self.tenant_scope {
            tenant.hash(&mut hasher);
        }
        format!("{:016x}", hasher.finish())
    }
}

/// A candidate returned by recall.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallCandidate {
    /// Unique identifier for this candidate
    pub id: String,
    /// Summary text of the candidate
    pub summary: String,
    /// Raw similarity score from vector search
    pub raw_score: f64,
    /// Final normalized score
    pub final_score: f64,
    /// Relevance level
    pub relevance: RelevanceLevel,
    /// Source type (failure, success, runbook, etc.)
    pub source_type: CandidateSourceType,
    /// Provenance information
    pub provenance: CandidateProvenance,
    /// Per-candidate confidence in `[0.0, 1.0]`. Reflects how much weight a
    /// downstream consumer (e.g. `PlanningPriorAgent`) should give this entry
    /// when adjusting priors. Defaults to `0.5` for backends that do not yet
    /// emit calibrated confidence.
    #[serde(default = "default_candidate_confidence")]
    pub confidence: f64,
}

fn default_candidate_confidence() -> f64 {
    0.5
}

/// Relevance level for a recall candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelevanceLevel {
    High,
    Medium,
    Low,
}

impl RelevanceLevel {
    /// Create from a score (0.0-1.0).
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        if score >= 0.8 {
            Self::High
        } else if score >= 0.5 {
            Self::Medium
        } else {
            Self::Low
        }
    }

    /// Get the string representation.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// Source type for a recall candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CandidateSourceType {
    SimilarFailure,
    SimilarSuccess,
    Runbook,
    AdapterConfig,
    AntiPattern,
}

/// Provenance information for a recall candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateProvenance {
    /// When this record was created
    pub created_at: String,
    /// Chain ID that produced this record
    pub source_chain_id: Option<String>,
    /// Step that produced this record
    pub source_step: Option<DecisionStep>,
    /// Corpus version when this was indexed
    pub corpus_version: String,
}

// ============================================================================
// Recall Provenance Types
// ============================================================================

/// Trace link for recall operations (enables reproducibility).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallTraceLink {
    /// Hash of the query embedding vector
    pub embedding_hash: String,
    /// Corpus version used for search
    pub corpus_version: String,
    /// Embedder ID used
    pub embedder_id: String,
    /// Number of candidates searched
    pub candidates_searched: usize,
    /// Number of candidates returned
    pub candidates_returned: usize,
    /// Latency in milliseconds
    pub latency_ms: u64,
}

/// A candidate ID with its score, for ordered provenance tracking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CandidateScore {
    /// Candidate ID
    pub id: String,
    /// Final normalized score
    pub score: f64,
}

/// Complete provenance envelope for recall operations.
///
/// This captures ALL information needed to:
/// - Replay the exact same recall query
/// - Audit why specific candidates were returned
/// - Verify determinism across runs
///
/// All fields are required (non-optional) to make it impossible to be vague.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecallProvenanceEnvelope {
    // --- Query Provenance ---
    /// Hash of the original query (before embedding)
    pub query_hash: String,

    /// Hash of the canonicalized embedding input text
    /// (after PII redaction, whitespace normalization, Unicode NFKC)
    pub embedding_input_hash: String,

    /// Hash of the resulting embedding vector
    pub embedding_hash: String,

    // --- Embedder Provenance ---
    /// Embedder identifier
    pub embedder_id: String,

    /// Hash of embedder settings (model, normalization, etc.)
    pub embedder_settings_hash: String,

    // --- Corpus Provenance ---
    /// Full corpus fingerprint string
    pub corpus_fingerprint: String,

    // --- Policy Provenance ---
    /// Hash of the RecallPolicy that was applied
    pub policy_snapshot_hash: String,

    // --- Use/Consumer Provenance (Recall ≠ Training boundary) ---
    /// Purpose of this recall operation
    ///
    /// Defaults to `RuntimeAugmentation`. Training use must be explicit.
    #[serde(default)]
    pub purpose: RecallUse,

    /// Consumers that will receive these results
    ///
    /// Empty by default; runtime typically sets `[Kernel]`.
    /// Training pipelines would set `[Trainer]` or `[Analytics, Trainer]`.
    #[serde(default)]
    pub consumers: Vec<RecallConsumer>,

    // --- Results Provenance ---
    /// Ordered list of (candidate_id, final_score) pairs
    /// Order matters for determinism verification
    pub candidate_scores: Vec<CandidateScore>,

    /// Number of candidates in corpus that were searched
    pub candidates_searched: usize,

    /// Number of candidates returned (after filtering)
    pub candidates_returned: usize,

    /// Why recall stopped (if applicable)
    pub stop_reason: Option<StopReason>,

    // --- Timing ---
    /// Latency in milliseconds
    pub latency_ms: u64,

    /// Timestamp when recall was performed (ISO 8601)
    pub timestamp: String,

    // --- Future-proofing for signing ---
    /// Optional signature for multi-tenant verification
    /// Format: "unsigned" | "sha256:`<hash>`" | "sig://`<key-id>`/`<signature>`"
    #[serde(default = "default_signature")]
    pub signature: String,
}

fn default_signature() -> String {
    "unsigned".to_string()
}

impl RecallProvenanceEnvelope {
    /// Compute a hash of the entire provenance envelope.
    ///
    /// This can be used for quick equality checks and audit trails.
    #[must_use]
    pub fn envelope_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.query_hash.hash(&mut hasher);
        self.embedding_input_hash.hash(&mut hasher);
        self.embedding_hash.hash(&mut hasher);
        self.embedder_id.hash(&mut hasher);
        self.embedder_settings_hash.hash(&mut hasher);
        self.corpus_fingerprint.hash(&mut hasher);
        self.policy_snapshot_hash.hash(&mut hasher);
        (self.purpose as u8).hash(&mut hasher);
        for consumer in &self.consumers {
            (*consumer as u8).hash(&mut hasher);
        }
        for cs in &self.candidate_scores {
            cs.id.hash(&mut hasher);
            (cs.score as u64).hash(&mut hasher);
        }
        self.candidates_searched.hash(&mut hasher);
        self.candidates_returned.hash(&mut hasher);
        self.latency_ms.hash(&mut hasher);
        self.timestamp.hash(&mut hasher);
        format!("{:016x}", hasher.finish())
    }

    /// Check if this envelope matches another for replay verification.
    ///
    /// Two envelopes match if they have identical:
    /// - query_hash
    /// - embedding_input_hash
    /// - embedder_id + embedder_settings_hash
    /// - corpus_fingerprint
    /// - policy_snapshot_hash
    /// - purpose + consumers (Recall ≠ Training boundary)
    /// - candidate_scores (order-sensitive)
    #[must_use]
    pub fn matches_for_replay(&self, other: &Self) -> bool {
        self.query_hash == other.query_hash
            && self.embedding_input_hash == other.embedding_input_hash
            && self.embedder_id == other.embedder_id
            && self.embedder_settings_hash == other.embedder_settings_hash
            && self.corpus_fingerprint == other.corpus_fingerprint
            && self.policy_snapshot_hash == other.policy_snapshot_hash
            && self.purpose == other.purpose
            && self.consumers == other.consumers
            && self.candidate_scores == other.candidate_scores
    }

    /// Get a short summary for logging.
    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "Recall[query:{:.8}...][corpus:{:.8}...][{}/{} candidates][{}ms]",
            self.query_hash,
            self.corpus_fingerprint,
            self.candidates_returned,
            self.candidates_searched,
            self.latency_ms
        )
    }
}

/// Reason why recall stopped returning results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StopReason {
    /// Reached the requested top_k
    ReachedTopK,
    /// Reached max_k_total budget
    BudgetExhausted,
    /// All remaining candidates below threshold
    BelowThreshold,
    /// Reached max tokens for injection
    TokenLimitReached,
    /// Latency budget exceeded
    LatencyExceeded,
    /// Embedder is not deterministic and policy requires replayability
    ///
    /// When `RecallUse::TrainingCandidateSelection` or kernel requires
    /// deterministic replay, but embedder is `Stochastic` or `Unknown`.
    /// Results may still be returned but marked as audit-only.
    EmbedderNotDeterministic,
    /// Tenant scope required but not provided
    ///
    /// The corpus has `TenantPolicy::Required` but the query did not
    /// include a tenant scope. No results returned.
    TenantScopeMissing,
}

// ============================================================================
// Recall Executor — turns ExperienceRecords into RecallCandidates
// ============================================================================

/// Pull recall candidates from an [`ExperienceStore`].
///
/// First implementation: scans the ledger for recall-relevant records (user
/// overrides, user approvals, failed engine outcomes), maps each to a
/// `RecallCandidate`, applies `policy.min_score_threshold` and `prior_weight`,
/// then trims to the smaller of `query.top_k` and `policy.max_k_total`.
///
/// Semantic ranking by embedding similarity is intentionally deferred — the
/// goal here is to wire planning to history end-to-end. Ranking by recency is
/// a placeholder that will be replaced once a recall provider is in place.
pub fn recall_from_store(
    store: &dyn ExperienceStore,
    query: &RecallQuery,
    policy: &RecallPolicy,
) -> ExperienceStoreResult<Vec<RecallCandidate>> {
    if !policy.enabled {
        return Ok(Vec::new());
    }

    let event_query = EventQuery {
        tenant_id: query.tenant_scope.as_deref().map(TenantId::new),
        ..Default::default()
    };

    let records = store.query_records(&event_query)?;
    let limit = query.top_k.min(policy.max_k_total);

    let candidates = records
        .iter()
        .rev()
        .filter_map(record_to_candidate)
        .filter(|c| c.confidence >= policy.min_score_threshold)
        .take(limit)
        .map(|mut c| {
            c.confidence = (c.confidence * policy.prior_weight).clamp(0.0, 1.0);
            c
        })
        .collect();

    Ok(candidates)
}

fn record_to_candidate(record: &ExperienceRecord) -> Option<RecallCandidate> {
    match record {
        ExperienceRecord::User(env) => match &env.event {
            UserExperienceEvent::UserOverrideIssued { reason, .. } => Some(make_candidate(
                env.event_id.as_str(),
                env.occurred_at.as_str(),
                format!("user override: {reason}"),
                0.9,
                CandidateSourceType::AntiPattern,
            )),
            UserExperienceEvent::UserApprovalGranted { reason, .. } => Some(make_candidate(
                env.event_id.as_str(),
                env.occurred_at.as_str(),
                format!("user approval: {}", reason.as_deref().unwrap_or("granted")),
                0.7,
                CandidateSourceType::SimilarSuccess,
            )),
        },
        ExperienceRecord::Engine(env) => match &env.event {
            ExperienceEvent::OutcomeRecorded {
                passed: false,
                stop_reason,
                ..
            } => Some(make_candidate(
                env.event_id.as_str(),
                env.occurred_at.as_str(),
                format!(
                    "outcome failed: {}",
                    stop_reason
                        .as_ref()
                        .map_or_else(|| "unspecified".to_string(), ToString::to_string)
                ),
                0.6,
                CandidateSourceType::SimilarFailure,
            )),
            _ => None,
        },
    }
}

fn make_candidate(
    id: &str,
    occurred_at: &str,
    summary: String,
    confidence: f64,
    source_type: CandidateSourceType,
) -> RecallCandidate {
    RecallCandidate {
        id: id.to_string(),
        summary,
        raw_score: confidence,
        final_score: confidence,
        relevance: RelevanceLevel::from_score(confidence),
        source_type,
        provenance: CandidateProvenance {
            created_at: occurred_at.to_string(),
            source_chain_id: None,
            source_step: None,
            corpus_version: "experience-store-v0".to_string(),
        },
        confidence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recall_policy_enabled() {
        let policy = RecallPolicy::enabled();
        assert!(policy.enabled);
    }

    #[test]
    fn test_recall_policy_disabled() {
        let policy = RecallPolicy::disabled();
        assert!(!policy.enabled);
    }

    #[test]
    fn test_relevance_from_score() {
        assert_eq!(RelevanceLevel::from_score(0.9), RelevanceLevel::High);
        assert_eq!(RelevanceLevel::from_score(0.6), RelevanceLevel::Medium);
        assert_eq!(RelevanceLevel::from_score(0.3), RelevanceLevel::Low);
    }

    #[test]
    fn test_recall_query_builder() {
        let query = RecallQuery::new("test", 5)
            .with_step_context(DecisionStep::Reasoning)
            .with_tenant_scope("tenant-1");

        assert_eq!(query.query_text, "test");
        assert_eq!(query.top_k, 5);
        assert_eq!(query.step_context, Some(DecisionStep::Reasoning));
        assert_eq!(query.tenant_scope, Some("tenant-1".to_string()));
    }

    #[test]
    fn test_recall_policy_defaults_to_runtime_only() {
        let policy = RecallPolicy::default();
        assert!(
            policy
                .allowed_uses
                .contains(&RecallUse::RuntimeAugmentation),
            "Default policy must allow RuntimeAugmentation"
        );
        assert!(
            !policy
                .allowed_uses
                .contains(&RecallUse::TrainingCandidateSelection),
            "Default policy must NOT allow TrainingCandidateSelection"
        );
    }

    #[test]
    fn test_recall_training_purpose_is_blocked_in_kernel() {
        let policy = RecallPolicy {
            allowed_uses: vec![RecallUse::RuntimeAugmentation],
            ..Default::default()
        };

        assert!(
            recall_use_allowed(&policy, RecallUse::RuntimeAugmentation),
            "RuntimeAugmentation must be allowed"
        );
        assert!(
            !recall_use_allowed(&policy, RecallUse::TrainingCandidateSelection),
            "TrainingCandidateSelection must be blocked when not in allowed_uses"
        );
    }

    #[test]
    fn test_recall_training_can_be_explicitly_enabled() {
        let policy = RecallPolicy {
            allowed_uses: vec![
                RecallUse::RuntimeAugmentation,
                RecallUse::TrainingCandidateSelection,
            ],
            ..Default::default()
        };

        assert!(recall_use_allowed(&policy, RecallUse::RuntimeAugmentation));
        assert!(recall_use_allowed(
            &policy,
            RecallUse::TrainingCandidateSelection
        ));
    }

    #[test]
    fn test_policy_hash_deterministic() {
        let policy = RecallPolicy::default();
        let hash1 = policy.snapshot_hash();
        let hash2 = policy.snapshot_hash();
        assert_eq!(hash1, hash2, "Same policy must produce same hash");
    }

    #[test]
    fn test_policy_hash_changes_with_allowed_uses() {
        let policy1 = RecallPolicy::default();
        let policy2 = RecallPolicy {
            allowed_uses: vec![
                RecallUse::RuntimeAugmentation,
                RecallUse::TrainingCandidateSelection,
            ],
            ..Default::default()
        };

        assert_ne!(
            policy1.snapshot_hash(),
            policy2.snapshot_hash(),
            "Different allowed_uses must produce different hash"
        );
    }

    #[test]
    fn test_recall_query_hash_deterministic() {
        let query = RecallQuery::new("test query", 5);
        let hash1 = query.query_hash();
        let hash2 = query.query_hash();
        assert_eq!(hash1, hash2, "Same query must produce same hash");
    }

    #[test]
    fn test_recall_provenance_matches_for_replay() {
        let env = RecallProvenanceEnvelope {
            query_hash: "q".to_string(),
            embedding_input_hash: "e".to_string(),
            embedding_hash: "h".to_string(),
            embedder_id: "id".to_string(),
            embedder_settings_hash: "s".to_string(),
            corpus_fingerprint: "c".to_string(),
            policy_snapshot_hash: "p".to_string(),
            purpose: RecallUse::RuntimeAugmentation,
            consumers: vec![RecallConsumer::Kernel],
            candidate_scores: vec![],
            candidates_searched: 10,
            candidates_returned: 2,
            stop_reason: None,
            latency_ms: 10,
            timestamp: "t".to_string(),
            signature: "unsigned".to_string(),
        };

        // Same envelope matches
        assert!(env.matches_for_replay(&env.clone()));

        // Different purpose does not match
        let mut env2 = env.clone();
        env2.purpose = RecallUse::TrainingCandidateSelection;
        assert!(
            !env.matches_for_replay(&env2),
            "Different purpose must not match"
        );

        // Different consumers does not match
        let mut env3 = env.clone();
        env3.consumers = vec![RecallConsumer::Trainer];
        assert!(
            !env.matches_for_replay(&env3),
            "Different consumers must not match"
        );
    }
}
