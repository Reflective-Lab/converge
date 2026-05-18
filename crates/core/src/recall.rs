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
use converge_pack::UnitInterval;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    pub min_score_threshold: UnitInterval,
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
    pub prior_weight: UnitInterval,
}

fn default_prior_weight() -> UnitInterval {
    UnitInterval::ONE
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
            min_score_threshold: UnitInterval::clamped(0.5),
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
        let mut hasher = StableHash::new("recall-policy-v1");
        hasher.bool(self.enabled);
        hasher.usize(self.max_k_total);
        hasher.usize(self.max_tokens_injection);
        hasher.u16(self.min_score_threshold.to_basis_points());
        hasher.u64(self.budgets.max_latency_ms);
        hasher.usize(self.budgets.max_embedding_calls);
        hasher.usize(self.budgets.max_tokens_per_candidate);
        for use_type in &self.allowed_uses {
            hasher.u8(*use_type as u8);
        }
        hasher.u16(self.prior_weight.to_basis_points());
        hasher.finish_hex()
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
        let mut hasher = StableHash::new("recall-query-v1");
        hasher.str(&self.query_text);
        hasher.usize(self.top_k);
        if let Some(ref step) = self.step_context {
            hasher.str(step.as_str());
        } else {
            hasher.none();
        }
        if let Some(ref tenant) = self.tenant_scope {
            hasher.str(tenant);
        } else {
            hasher.none();
        }
        hasher.finish_hex()
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
    pub raw_score: UnitInterval,
    /// Final normalized score
    pub final_score: UnitInterval,
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
    pub confidence: UnitInterval,
}

fn default_candidate_confidence() -> UnitInterval {
    UnitInterval::clamped(0.5)
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
    pub fn from_score(score: UnitInterval) -> Self {
        let score = score.as_f64();
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
    pub score: UnitInterval,
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
        let mut hasher = StableHash::new("recall-envelope-v1");
        hasher.str(&self.query_hash);
        hasher.str(&self.embedding_input_hash);
        hasher.str(&self.embedding_hash);
        hasher.str(&self.embedder_id);
        hasher.str(&self.embedder_settings_hash);
        hasher.str(&self.corpus_fingerprint);
        hasher.str(&self.policy_snapshot_hash);
        hasher.u8(self.purpose as u8);
        for consumer in &self.consumers {
            hasher.u8(*consumer as u8);
        }
        for cs in &self.candidate_scores {
            hasher.str(&cs.id);
            hasher.u16(cs.score.to_basis_points());
        }
        hasher.usize(self.candidates_searched);
        hasher.usize(self.candidates_returned);
        hasher.u64(self.latency_ms);
        hasher.str(&self.timestamp);
        hasher.finish_hex()
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

struct StableHash {
    hasher: Sha256,
}

impl StableHash {
    fn new(domain: &'static str) -> Self {
        let mut stable = Self {
            hasher: Sha256::new(),
        };
        stable.str(domain);
        stable
    }

    fn bytes(&mut self, bytes: &[u8]) {
        self.hasher.update((bytes.len() as u64).to_be_bytes());
        self.hasher.update(bytes);
    }

    fn str(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    fn none(&mut self) {
        self.bytes(&[]);
    }

    fn u8(&mut self, value: u8) {
        self.bytes(&[value]);
    }

    fn u16(&mut self, value: u16) {
        self.bytes(&value.to_be_bytes());
    }

    fn u64(&mut self, value: u64) {
        self.bytes(&value.to_be_bytes());
    }

    fn usize(&mut self, value: usize) {
        self.u64(value as u64);
    }

    fn finish_hex(self) -> String {
        hex::encode(self.hasher.finalize())
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
            c.confidence = c.confidence.scale_by(policy.prior_weight);
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
                UnitInterval::clamped(0.9),
                CandidateSourceType::AntiPattern,
            )),
            UserExperienceEvent::UserApprovalGranted { reason, .. } => Some(make_candidate(
                env.event_id.as_str(),
                env.occurred_at.as_str(),
                format!("user approval: {}", reason.as_deref().unwrap_or("granted")),
                UnitInterval::clamped(0.7),
                CandidateSourceType::SimilarSuccess,
            )),
            UserExperienceEvent::UserApprovalRejected { reason, .. } => Some(make_candidate(
                env.event_id.as_str(),
                env.occurred_at.as_str(),
                format!(
                    "user rejection: {}",
                    reason.as_deref().unwrap_or("declined")
                ),
                UnitInterval::clamped(0.7),
                CandidateSourceType::AntiPattern,
            )),
            UserExperienceEvent::UserCorrection { target, reason, .. } => Some(make_candidate(
                env.event_id.as_str(),
                env.occurred_at.as_str(),
                format!("correction ({}): {reason}", target.kind_label()),
                UnitInterval::clamped(0.85),
                CandidateSourceType::Runbook,
            )),
            UserExperienceEvent::UserBoundaryAdjusted {
                boundary,
                target,
                reason,
                ..
            } => Some(make_candidate(
                env.event_id.as_str(),
                env.occurred_at.as_str(),
                format!(
                    "{} boundary adjusted on {}: {reason}",
                    boundary_kind_label(*boundary),
                    boundary_target_label(target)
                ),
                UnitInterval::clamped(0.8),
                CandidateSourceType::Runbook,
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
                UnitInterval::clamped(0.6),
                CandidateSourceType::SimilarFailure,
            )),
            _ => None,
        },
    }
}

fn boundary_kind_label(kind: crate::BoundaryKind) -> &'static str {
    match kind {
        crate::BoundaryKind::Authority => "authority",
        crate::BoundaryKind::Forbidden => "forbidden",
        crate::BoundaryKind::Expiry => "expiry",
        crate::BoundaryKind::Reversibility => "reversibility",
    }
}

fn boundary_target_label(target: &crate::BoundaryTarget) -> String {
    match target {
        crate::BoundaryTarget::Pack { pack_id } => format!("pack:{}", pack_id.as_str()),
        crate::BoundaryTarget::Intent { intent_id } => format!("intent:{}", intent_id.as_str()),
        crate::BoundaryTarget::Global => "global".to_string(),
    }
}

fn make_candidate(
    id: &str,
    occurred_at: &str,
    summary: String,
    confidence: UnitInterval,
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
    use crate::{
        BoundaryKind, BoundaryTarget, ContentHash, CorrectionTarget, ExperienceRecord, FactContent,
        FactContentKind, UserExperienceEventEnvelope,
    };

    fn candidate_for_user_event(event: UserExperienceEvent) -> RecallCandidate {
        let envelope = UserExperienceEventEnvelope::new("evt-user", event);
        record_to_candidate(&ExperienceRecord::User(envelope)).expect("candidate")
    }

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
        assert_eq!(
            RelevanceLevel::from_score(UnitInterval::clamped(0.9)),
            RelevanceLevel::High
        );
        assert_eq!(
            RelevanceLevel::from_score(UnitInterval::clamped(0.6)),
            RelevanceLevel::Medium
        );
        assert_eq!(
            RelevanceLevel::from_score(UnitInterval::clamped(0.3)),
            RelevanceLevel::Low
        );
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
    fn recall_maps_rejected_user_approval_to_antipattern() {
        let candidate = candidate_for_user_event(UserExperienceEvent::UserApprovalRejected {
            gate_request_id: "gate-1".into(),
            actor: "operator-1".into(),
            policy_snapshot_hash: None,
            reason: Some("risk too high".into()),
        });

        assert_eq!(candidate.summary, "user rejection: risk too high");
        assert_eq!(candidate.confidence, UnitInterval::clamped(0.7));
        assert_eq!(candidate.source_type, CandidateSourceType::AntiPattern);
    }

    #[test]
    fn recall_maps_user_correction_to_runbook() {
        let candidate = candidate_for_user_event(UserExperienceEvent::UserCorrection {
            target: CorrectionTarget::Fact {
                fact_id: "fact-1".into(),
            },
            actor: "operator-1".into(),
            policy_snapshot_hash: None,
            original_content: ContentHash::zero(),
            corrected_content: FactContent::new(FactContentKind::Claim, "corrected"),
            reason: "source was stale".into(),
        });

        assert_eq!(candidate.summary, "correction (fact): source was stale");
        assert_eq!(candidate.confidence, UnitInterval::clamped(0.85));
        assert_eq!(candidate.source_type, CandidateSourceType::Runbook);
    }

    #[test]
    fn recall_maps_boundary_adjustment_to_scoped_runbook() {
        let candidate = candidate_for_user_event(UserExperienceEvent::UserBoundaryAdjusted {
            boundary: BoundaryKind::Authority,
            target: BoundaryTarget::Pack {
                pack_id: "loan-pack".into(),
            },
            actor: "operator-1".into(),
            policy_snapshot_hash: None,
            previous_value: serde_json::json!({"limit": 100}),
            new_value: serde_json::json!({"limit": 50}),
            reason: "manual review needed".into(),
        });

        assert_eq!(
            candidate.summary,
            "authority boundary adjusted on pack:loan-pack: manual review needed"
        );
        assert_eq!(candidate.confidence, UnitInterval::clamped(0.8));
        assert_eq!(candidate.source_type, CandidateSourceType::Runbook);
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
    fn recall_policy_deserialization_rejects_out_of_range_threshold() {
        let json = r#"{
            "enabled": true,
            "max_k_total": 5,
            "max_tokens_injection": 500,
            "min_score_threshold": 1.2,
            "budgets": {
                "max_latency_ms": 100,
                "max_embedding_calls": 3,
                "max_tokens_per_candidate": 100
            },
            "allowed_uses": ["RuntimeAugmentation"],
            "prior_weight": 1.0
        }"#;
        let result = serde_json::from_str::<RecallPolicy>(json);
        assert!(result.is_err());
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
