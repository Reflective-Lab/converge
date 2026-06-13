// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Facts and proposed facts — the type boundary.
//!
//! This is the most important design decision in Converge: LLMs suggest,
//! the engine validates. `ProposedFact` is not `Fact`. There is no implicit
//! conversion between them.

use std::{any::Any, collections::HashMap, fmt, sync::Arc};

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use thiserror::Error;

use crate::context::ContextKey;
use crate::types::{
    ActorId, ApprovalId, ArtifactId, ContentHash, FactId, GateId, ObservationId, ProposalId,
    SpanId, SubjectRef, Timestamp, TraceId, TraceReference, TraceSystemId, UnitInterval,
    ValidationCheckId,
};

/// Stable payload-family identifier used by typed facts and wire adapters.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FactFamilyId(String);

impl FactFamilyId {
    /// Creates a payload-family identifier.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the raw identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for FactFamilyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&'static str> for FactFamilyId {
    fn from(value: &'static str) -> Self {
        Self::new(value)
    }
}

impl From<String> for FactFamilyId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Frozen payload schema version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PayloadVersion(u16);

impl PayloadVersion {
    /// Creates a payload version.
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the numeric payload version.
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

impl fmt::Display for PayloadVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u16> for PayloadVersion {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

/// Uniform proposal provenance metadata.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Provenance(String);

impl Provenance {
    /// Creates provenance metadata.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the provenance identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Provenance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Provenance {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Provenance {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

/// Stable, audit-friendly identifier for an extension that emits
/// facts into the convergence loop.
///
/// Implementors are typically zero-sized marker types declared by
/// each fact-emitting crate. The trait gives them a single canonical
/// [`as_str`](ProvenanceSource::as_str) plus a default
/// [`proposed_fact`](ProvenanceSource::proposed_fact) constructor
/// that stamps the resulting [`ProposedFact`] with the right
/// [`Provenance`] string.
///
/// # Migration from the per-crate `ProvenanceSource` enum
///
/// Earlier fact-emitting extensions each duplicated an 8-variant
/// `ProvenanceSource` enum and a `*_PROVENANCE` constant. This trait
/// replaces that pattern. Each crate now declares only its own marker
/// and canonical provenance constant:
///
/// ```ignore
/// use converge_pack::{ProvenanceSource, ContextKey, TextPayload};
///
/// pub struct Arbiter;
/// impl ProvenanceSource for Arbiter {
///     fn as_str(&self) -> &'static str { "arbiter" }
/// }
/// pub const ARBITER_PROVENANCE: Arbiter = Arbiter;
///
/// let provenance = ARBITER_PROVENANCE.provenance();
/// let fact = ARBITER_PROVENANCE.proposed_fact(
///     ContextKey::Diagnostic,
///     "decision-001",
///     TextPayload::new("hello"),
/// );
/// assert_eq!(fact.provenance_ref(), &provenance);
/// ```
///
/// Extensions no longer need to enumerate every sibling extension.
pub trait ProvenanceSource: Copy + Send + Sync + 'static {
    /// Canonical lowercase identifier carried on
    /// [`ProposedFact`]`.provenance`. Stable across the extension's
    /// public API.
    fn as_str(&self) -> &'static str;

    /// Construct typed [`Provenance`] from this marker.
    #[must_use]
    fn provenance(self) -> Provenance {
        Provenance::from(self.as_str())
    }

    /// Construct a [`ProposedFact`] stamped with this provenance and
    /// a typed payload.
    #[must_use]
    fn proposed_fact<T>(
        self,
        key: ContextKey,
        id: impl Into<ProposalId>,
        payload: T,
    ) -> ProposedFact
    where
        T: FactPayload + PartialEq,
    {
        ProposedFact::new(key, id, payload, self.provenance())
    }

    /// Construct a [`ProposedFact`] derived from an existing
    /// [`ContextFact`], preserving the source fact's app-owned subject when
    /// one is present.
    #[must_use]
    fn proposed_fact_for<T>(
        self,
        source: &ContextFact,
        key: ContextKey,
        id: impl Into<ProposalId>,
        payload: T,
    ) -> ProposedFact
    where
        T: FactPayload + PartialEq,
    {
        self.proposed_fact(key, id, payload)
            .with_subject_from(source)
    }
}

/// Typed payload carried by proposed and promoted facts.
///
/// Implementors own a frozen `(FAMILY, VERSION)` tuple. A shape change is a new
/// Rust type and a new `VERSION`, never an implicit registry upgrade.
pub trait FactPayload: fmt::Debug + Clone + Serialize + Send + Sync + 'static {
    /// Stable payload-family identifier.
    const FAMILY: &'static str;
    /// Frozen schema version for this payload type.
    const VERSION: u16;

    /// Validate domain invariants that the Rust type cannot make
    /// unrepresentable.
    fn validate(&self) -> Result<(), PayloadError> {
        Ok(())
    }
}

trait ErasedFactPayload: fmt::Debug + Send + Sync {
    fn family(&self) -> FactFamilyId;
    fn version(&self) -> PayloadVersion;
    fn validate(&self) -> Result<(), PayloadError>;
    fn as_any(&self) -> &dyn Any;
    fn to_json_value(&self) -> Result<serde_json::Value, PayloadError>;
    fn equivalent(&self, other: &dyn ErasedFactPayload) -> bool;
}

impl<T> ErasedFactPayload for T
where
    T: FactPayload + PartialEq,
{
    fn family(&self) -> FactFamilyId {
        FactFamilyId::from(T::FAMILY)
    }

    fn version(&self) -> PayloadVersion {
        PayloadVersion::new(T::VERSION)
    }

    fn validate(&self) -> Result<(), PayloadError> {
        FactPayload::validate(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn to_json_value(&self) -> Result<serde_json::Value, PayloadError> {
        serde_json::to_value(self).map_err(|err| PayloadError::Serialize {
            family: T::FAMILY.into(),
            version: T::VERSION.into(),
            reason: err.to_string(),
        })
    }

    fn equivalent(&self, other: &dyn ErasedFactPayload) -> bool {
        other.as_any().downcast_ref::<T>() == Some(self)
    }
}

/// Human-readable text payload. This is explicit text, not a generic semantic
/// escape hatch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextPayload {
    text: String,
}

impl TextPayload {
    /// Creates a text payload.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    /// Returns the text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl FactPayload for TextPayload {
    const FAMILY: &'static str = "converge.text";
    const VERSION: u16 = 1;
}

/// Structured diagnostic payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticPayload {
    source: String,
    message: String,
}

impl DiagnosticPayload {
    /// Creates a diagnostic payload.
    #[must_use]
    pub fn new(source: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            message: message.into(),
        }
    }

    /// Returns the diagnostic source.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the diagnostic message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl FactPayload for DiagnosticPayload {
    const FAMILY: &'static str = "converge.diagnostic";
    const VERSION: u16 = 1;
}

/// Crate, extension, or service that produced an execution result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionProducerIdentity {
    /// Producer crate, extension, or service name.
    pub name: String,
    /// Producer version.
    pub version: String,
}

impl ExecutionProducerIdentity {
    /// Creates producer identity metadata.
    #[must_use]
    pub fn new(name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            version: version.into(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        validate_non_empty("producer.name", &self.name)?;
        validate_non_empty("producer.version", &self.version)
    }
}

/// Native backend details for solver, policy, analytics, or model execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeExecutionIdentity {
    /// Native backend name, for example `CVC5`, `OR-Tools:cp_sat`, or `HiGHS`.
    pub backend: String,
    /// Native backend version as reported by the linked library.
    pub version: String,
    /// Source URL for the native dependency when known.
    pub source_url: String,
    /// Expected pinned source commit or release identifier.
    pub expected_commit: String,
    /// Actual source commit or release identifier used at build time.
    pub actual_commit: String,
    /// How the native backend was sourced, for example vendored or external.
    pub source_mode: String,
}

impl NativeExecutionIdentity {
    /// Creates native backend identity metadata.
    #[must_use]
    pub fn new(
        backend: impl Into<String>,
        version: impl Into<String>,
        source_url: impl Into<String>,
        expected_commit: impl Into<String>,
        actual_commit: impl Into<String>,
        source_mode: impl Into<String>,
    ) -> Self {
        Self {
            backend: backend.into(),
            version: version.into(),
            source_url: source_url.into(),
            expected_commit: expected_commit.into(),
            actual_commit: actual_commit.into(),
            source_mode: source_mode.into(),
        }
    }

    fn validate(&self) -> Result<(), String> {
        validate_non_empty("native_identity.backend", &self.backend)?;
        validate_non_empty("native_identity.version", &self.version)?;
        validate_non_empty("native_identity.source_url", &self.source_url)?;
        validate_non_empty("native_identity.expected_commit", &self.expected_commit)?;
        validate_non_empty("native_identity.actual_commit", &self.actual_commit)?;
        validate_non_empty("native_identity.source_mode", &self.source_mode)
    }
}

/// Runtime execution identity shared by evidence-producing extensions.
///
/// This records what executed a result. It is intentionally generic: CVC5,
/// Cedar analysis, OR-Tools, HiGHS, model inference, and deterministic fake
/// backends can all populate the same contract without leaking implementation
/// fields into domain payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionIdentity {
    /// Producer crate, extension, or service.
    pub producer: ExecutionProducerIdentity,
    /// Logical backend or engine name, for example `cvc5` or `cp-sat-v9.15`.
    pub backend: String,
    /// Backend version visible at the safe execution boundary.
    pub backend_version: String,
    /// Build/source/config identity for replay and audit.
    pub build_identity: String,
    /// Runtime options that affect the result.
    pub runtime_config: String,
    /// Native backend details when the execution crossed an FFI/native boundary.
    pub native_identity: Option<NativeExecutionIdentity>,
}

impl ExecutionIdentity {
    /// Creates execution identity metadata.
    #[must_use]
    pub fn new(
        producer: ExecutionProducerIdentity,
        backend: impl Into<String>,
        backend_version: impl Into<String>,
        build_identity: impl Into<String>,
        runtime_config: impl Into<String>,
        native_identity: Option<NativeExecutionIdentity>,
    ) -> Self {
        Self {
            producer,
            backend: backend.into(),
            backend_version: backend_version.into(),
            build_identity: build_identity.into(),
            runtime_config: runtime_config.into(),
            native_identity,
        }
    }

    /// Creates non-native execution identity metadata.
    #[must_use]
    pub fn non_native(
        producer_name: impl Into<String>,
        producer_version: impl Into<String>,
        backend: impl Into<String>,
        runtime_config: impl Into<String>,
    ) -> Self {
        Self::new(
            ExecutionProducerIdentity::new(producer_name, producer_version),
            backend,
            "not_applicable",
            "not_applicable",
            runtime_config,
            None,
        )
    }

    /// Creates unknown execution identity metadata for placeholders and tests.
    #[must_use]
    pub fn unspecified(
        producer_name: impl Into<String>,
        producer_version: impl Into<String>,
    ) -> Self {
        Self::new(
            ExecutionProducerIdentity::new(producer_name, producer_version),
            "unknown",
            "unknown",
            "unknown",
            "unknown",
            None,
        )
    }

    /// Serializes a typed configuration struct to the canonical
    /// `runtime_config` JSON string.
    ///
    /// This is the workspace-standard encoding for `runtime_config` per
    /// `kb/Standards/Runtime Config Encoding.md`: a JSON object whose keys
    /// are the struct field names and whose values are field values. Empty
    /// configs serialize as `{}`.
    ///
    /// Panics if `T`'s `Serialize` impl is malformed (e.g., non-finite
    /// floats, non-string map keys). For all practical workspace config
    /// structs this is unreachable; a panic here means the caller's config
    /// struct is broken.
    ///
    /// ```ignore
    /// let rc = ExecutionIdentity::runtime_config_from_typed(&my_cfg);
    /// let identity = ExecutionIdentity::non_native("crate", "1.0", "backend", rc);
    /// ```
    #[must_use]
    pub fn runtime_config_from_typed<T: Serialize>(value: &T) -> String {
        serde_json::to_string(value)
            .expect("typed runtime_config must serialize to JSON; check Serialize impl")
    }

    /// Replaces this identity's `runtime_config` with the JSON encoding of a
    /// typed config struct. Builder-style sibling of
    /// [`Self::runtime_config_from_typed`].
    #[must_use]
    pub fn with_runtime_config_typed<T: Serialize>(mut self, value: &T) -> Self {
        self.runtime_config = Self::runtime_config_from_typed(value);
        self
    }

    fn validate(&self) -> Result<(), String> {
        self.producer.validate()?;
        validate_non_empty("backend", &self.backend)?;
        validate_non_empty("backend_version", &self.backend_version)?;
        validate_non_empty("build_identity", &self.build_identity)?;
        validate_non_empty("runtime_config", &self.runtime_config)?;
        if let Some(native_identity) = &self.native_identity {
            native_identity.validate()?;
        }
        Ok(())
    }
}

/// Companion evidence that links a produced fact to its execution identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionIdentityEvidence {
    /// Context key containing the produced subject.
    pub subject_key: ContextKey,
    /// Subject fact/proposal id.
    pub subject_id: String,
    /// Subject payload family.
    pub subject_family: FactFamilyId,
    /// Subject payload version.
    pub subject_version: PayloadVersion,
    /// Execution identity that produced the subject.
    pub identity: ExecutionIdentity,
}

impl ExecutionIdentityEvidence {
    /// Creates execution identity evidence for a known payload family.
    #[must_use]
    pub fn new(
        subject_key: ContextKey,
        subject_id: impl Into<String>,
        subject_family: impl Into<FactFamilyId>,
        subject_version: impl Into<PayloadVersion>,
        identity: ExecutionIdentity,
    ) -> Self {
        Self {
            subject_key,
            subject_id: subject_id.into(),
            subject_family: subject_family.into(),
            subject_version: subject_version.into(),
            identity,
        }
    }

    /// Creates execution identity evidence for a typed fact payload.
    #[must_use]
    pub fn for_payload<T: FactPayload>(
        subject_key: ContextKey,
        subject_id: impl Into<String>,
        identity: ExecutionIdentity,
    ) -> Self {
        Self::new(subject_key, subject_id, T::FAMILY, T::VERSION, identity)
    }
}

impl FactPayload for ExecutionIdentityEvidence {
    const FAMILY: &'static str = "converge.execution_identity.evidence";
    const VERSION: u16 = 1;

    fn validate(&self) -> Result<(), PayloadError> {
        validate_non_empty("subject_id", &self.subject_id).map_err(|reason| {
            PayloadError::Invalid {
                family: Self::FAMILY.into(),
                version: Self::VERSION.into(),
                reason,
            }
        })?;
        self.identity
            .validate()
            .map_err(|reason| PayloadError::Invalid {
                family: Self::FAMILY.into(),
                version: Self::VERSION.into(),
                reason,
            })
    }
}

fn validate_non_empty(field: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} must not be empty"))
    } else {
        Ok(())
    }
}

/// Errors at the typed payload and wire materialization boundary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PayloadError {
    /// Payload failed validation.
    #[error("invalid payload for {family} v{version}: {reason}")]
    Invalid {
        /// Payload family.
        family: FactFamilyId,
        /// Payload version.
        version: PayloadVersion,
        /// Human-readable reason.
        reason: String,
    },
    /// Payload failed JSON serialization at a border.
    #[error("failed to serialize payload {family} v{version}: {reason}")]
    Serialize {
        /// Payload family.
        family: FactFamilyId,
        /// Payload version.
        version: PayloadVersion,
        /// Human-readable reason.
        reason: String,
    },
    /// Payload failed JSON deserialization at a border.
    #[error("failed to deserialize payload {family} v{version}: {reason}")]
    Deserialize {
        /// Payload family.
        family: FactFamilyId,
        /// Payload version.
        version: PayloadVersion,
        /// Human-readable reason.
        reason: String,
    },
    /// No decoder is registered for the wire family and version.
    #[error("unknown payload family/version: {family} v{version}")]
    UnknownFamilyVersion {
        /// Payload family.
        family: FactFamilyId,
        /// Payload version.
        version: PayloadVersion,
    },
    /// A typed access request used the wrong payload type.
    #[error(
        "payload type mismatch: expected {expected} v{expected_version}, got {actual} v{actual_version}"
    )]
    TypeMismatch {
        /// Expected family.
        expected: FactFamilyId,
        /// Expected version.
        expected_version: PayloadVersion,
        /// Actual family.
        actual: FactFamilyId,
        /// Actual version.
        actual_version: PayloadVersion,
    },
}

/// Stable wire payload shape for HTTP, gRPC, NATS, storage/replay, CLI
/// fixtures, non-Rust clients, and audit export.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireFactPayload {
    /// Payload family.
    pub family: FactFamilyId,
    /// Payload schema version.
    pub version: PayloadVersion,
    /// JSON payload bytes decoded into a value at the border.
    pub payload: serde_json::Value,
}

impl WireFactPayload {
    fn from_erased(payload: &dyn ErasedFactPayload) -> Result<Self, PayloadError> {
        Ok(Self {
            family: payload.family(),
            version: payload.version(),
            payload: payload.to_json_value()?,
        })
    }
}

/// Wire shape for proposed facts. This is the only sanctioned way for borders
/// to materialize proposals without already holding a typed Rust payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireProposedFact {
    /// Destination context key.
    pub key: ContextKey,
    /// Proposal identifier.
    pub id: ProposalId,
    /// Optional app-owned subject this proposal is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<SubjectRef>,
    /// Proposed payload.
    pub payload: WireFactPayload,
    /// Confidence hint.
    pub confidence: UnitInterval,
    /// Uniform provenance metadata.
    pub provenance: Provenance,
}

/// Wire shape for promoted context facts.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireContextFact {
    /// Context key.
    pub key: ContextKey,
    /// Fact identifier.
    pub id: FactId,
    /// Optional app-owned subject this fact is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<SubjectRef>,
    /// Fact payload.
    pub payload: WireFactPayload,
    /// Promotion record.
    pub promotion_record: FactPromotionRecord,
    /// Creation timestamp.
    pub created_at: Timestamp,
}

type PayloadDecoder = Box<
    dyn Fn(serde_json::Value) -> Result<Arc<dyn ErasedFactPayload>, PayloadError> + Send + Sync,
>;

/// Registry used at serialization borders to materialize typed payloads from
/// `(family, version, payload)` tuples.
#[derive(Default)]
pub struct PayloadRegistry {
    decoders: HashMap<(FactFamilyId, PayloadVersion), PayloadDecoder>,
}

impl PayloadRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a registry with payloads owned by `converge-pack`.
    #[must_use]
    pub fn with_pack_payloads() -> Self {
        let mut registry = Self::new();
        registry.register::<TextPayload>();
        registry.register::<DiagnosticPayload>();
        registry.register::<ExecutionIdentityEvidence>();
        registry.register::<crate::governance::Vote>();
        registry.register::<crate::governance::Disagreement>();
        registry.register::<crate::governance::ConsensusOutcome>();
        registry
    }

    /// Registers one frozen payload type.
    pub fn register<T>(&mut self)
    where
        T: FactPayload + PartialEq + DeserializeOwned,
    {
        self.decoders.insert(
            (
                FactFamilyId::from(T::FAMILY),
                PayloadVersion::new(T::VERSION),
            ),
            Box::new(|value| {
                let payload: T =
                    serde_json::from_value(value).map_err(|err| PayloadError::Deserialize {
                        family: T::FAMILY.into(),
                        version: T::VERSION.into(),
                        reason: err.to_string(),
                    })?;
                payload.validate()?;
                Ok(Arc::new(payload))
            }),
        );
    }

    fn decode(
        &self,
        family: &FactFamilyId,
        version: PayloadVersion,
        payload: serde_json::Value,
    ) -> Result<Arc<dyn ErasedFactPayload>, PayloadError> {
        let decoder = self
            .decoders
            .get(&(family.clone(), version))
            .ok_or_else(|| PayloadError::UnknownFamilyVersion {
                family: family.clone(),
                version,
            })?;
        decoder(payload)
    }
}

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
#[derive(Clone)]
pub struct ContextFact {
    /// Which context key this fact belongs to.
    key: ContextKey,
    /// Unique identifier within the context key namespace.
    id: FactId,
    /// App-owned subject this fact is about.
    subject: Option<SubjectRef>,
    /// Typed fact payload.
    payload: Arc<dyn ErasedFactPayload>,
    /// The immutable promotion record that made this fact authoritative.
    promotion_record: FactPromotionRecord,
    /// When the authoritative fact entered context.
    created_at: Timestamp,
}

impl fmt::Debug for ContextFact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ContextFact")
            .field("key", &self.key)
            .field("id", &self.id)
            .field("subject", &self.subject)
            .field("payload_family", &self.payload_family())
            .field("payload_version", &self.payload_version())
            .field("promotion_record", &self.promotion_record)
            .field("created_at", &self.created_at)
            .finish()
    }
}

impl PartialEq for ContextFact {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.id == other.id
            && self.subject == other.subject
            && self.payload_family() == other.payload_family()
            && self.payload_version() == other.payload_version()
            && self.payload.equivalent(other.payload.as_ref())
            && self.promotion_record == other.promotion_record
            && self.created_at == other.created_at
    }
}

impl Eq for ContextFact {}

impl Serialize for ContextFact {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_wire()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl ContextFact {
    /// Creates a read-only context projection.
    ///
    /// This constructor does not promote anything and is intentionally named as
    /// a projection constructor. The engine is still the only component that can
    /// add context facts to a live `ContextState`.
    #[must_use]
    pub fn new_projection<T>(
        key: ContextKey,
        id: impl Into<FactId>,
        payload: T,
        promotion_record: FactPromotionRecord,
        created_at: impl Into<Timestamp>,
    ) -> Self
    where
        T: FactPayload + PartialEq,
    {
        Self {
            key,
            id: id.into(),
            subject: None,
            payload: Arc::new(payload),
            promotion_record,
            created_at: created_at.into(),
        }
    }

    /// Creates a context fact from a wire representation at a serialization
    /// border.
    pub fn from_wire(
        wire: WireContextFact,
        registry: &PayloadRegistry,
    ) -> Result<Self, PayloadError> {
        let payload = registry.decode(
            &wire.payload.family,
            wire.payload.version,
            wire.payload.payload,
        )?;
        Ok(Self {
            key: wire.key,
            id: wire.id,
            subject: wire.subject,
            payload,
            promotion_record: wire.promotion_record,
            created_at: wire.created_at,
        })
    }

    /// Converts this fact to the stable wire shape.
    pub fn to_wire(&self) -> Result<WireContextFact, PayloadError> {
        Ok(WireContextFact {
            key: self.key,
            id: self.id.clone(),
            subject: self.subject.clone(),
            payload: WireFactPayload::from_erased(self.payload.as_ref())?,
            promotion_record: self.promotion_record.clone(),
            created_at: self.created_at.clone(),
        })
    }

    /// Attaches the app-owned subject this fact is about.
    #[must_use]
    pub fn with_subject(mut self, subject: SubjectRef) -> Self {
        self.subject = Some(subject);
        self
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

    /// Returns the app-owned subject this fact is about, when tagged.
    #[must_use]
    pub fn subject(&self) -> Option<&SubjectRef> {
        self.subject.as_ref()
    }

    /// Returns the typed payload when the requested type matches the stored
    /// payload family/version.
    #[must_use]
    pub fn payload<T: FactPayload>(&self) -> Option<&T> {
        self.payload.as_any().downcast_ref::<T>()
    }

    /// Returns the typed payload or a mismatch error.
    pub fn require_payload<T: FactPayload>(&self) -> Result<&T, PayloadError> {
        self.payload::<T>()
            .ok_or_else(|| PayloadError::TypeMismatch {
                expected: T::FAMILY.into(),
                expected_version: T::VERSION.into(),
                actual: self.payload_family(),
                actual_version: self.payload_version(),
            })
    }

    /// Returns the payload family.
    #[must_use]
    pub fn payload_family(&self) -> FactFamilyId {
        self.payload.family()
    }

    /// Returns the payload version.
    #[must_use]
    pub fn payload_version(&self) -> PayloadVersion {
        self.payload.version()
    }

    /// Returns the payload as text when this is a [`TextPayload`].
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.payload::<TextPayload>().map(TextPayload::as_str)
    }

    /// Validates the stored payload.
    pub fn validate_payload(&self) -> Result<(), PayloadError> {
        self.payload.validate()
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
}

/// An unvalidated suggestion from a non-authoritative source.
///
/// Proposed facts live in `ContextKey::Proposals` until a `ValidationAgent`
/// promotes them to `Fact`. The proposal tracks its origin for audit trail.
#[derive(Clone)]
pub struct ProposedFact {
    /// The context key this proposal targets.
    pub key: ContextKey,
    /// Unique identifier encoding origin and target.
    pub id: ProposalId,
    /// App-owned subject this proposal is about.
    subject: Option<SubjectRef>,
    /// Typed proposed payload.
    payload: Arc<dyn ErasedFactPayload>,
    /// Confidence hint from the source. Always in [0.0, 1.0].
    confidence: UnitInterval,
    /// Provenance information (e.g., model ID, prompt hash).
    pub provenance: Provenance,
}

impl fmt::Debug for ProposedFact {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProposedFact")
            .field("key", &self.key)
            .field("id", &self.id)
            .field("subject", &self.subject)
            .field("payload_family", &self.payload_family())
            .field("payload_version", &self.payload_version())
            .field("confidence", &self.confidence)
            .field("provenance", &self.provenance)
            .finish()
    }
}

impl PartialEq for ProposedFact {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.id == other.id
            && self.subject == other.subject
            && self.payload_family() == other.payload_family()
            && self.payload_version() == other.payload_version()
            && self.payload.equivalent(other.payload.as_ref())
            && self.confidence == other.confidence
            && self.provenance == other.provenance
    }
}

impl Serialize for ProposedFact {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_wire()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

impl ProposedFact {
    /// Create a new draft proposal with explicit provenance.
    ///
    /// Confidence defaults to 1.0. Override with [`with_confidence`][Self::with_confidence].
    #[must_use]
    pub fn new<T>(
        key: ContextKey,
        id: impl Into<ProposalId>,
        payload: T,
        provenance: impl Into<Provenance>,
    ) -> Self
    where
        T: FactPayload + PartialEq,
    {
        Self {
            key,
            id: id.into(),
            subject: None,
            payload: Arc::new(payload),
            confidence: UnitInterval::ONE,
            provenance: provenance.into(),
        }
    }

    /// Materializes a proposal from the stable wire shape at an external
    /// border.
    pub fn from_wire(
        wire: WireProposedFact,
        registry: &PayloadRegistry,
    ) -> Result<Self, PayloadError> {
        let payload = registry.decode(
            &wire.payload.family,
            wire.payload.version,
            wire.payload.payload,
        )?;
        Ok(Self {
            key: wire.key,
            id: wire.id,
            subject: wire.subject,
            payload,
            confidence: wire.confidence,
            provenance: wire.provenance,
        })
    }

    /// Converts this proposal to the stable wire shape.
    pub fn to_wire(&self) -> Result<WireProposedFact, PayloadError> {
        Ok(WireProposedFact {
            key: self.key,
            id: self.id.clone(),
            subject: self.subject.clone(),
            payload: WireFactPayload::from_erased(self.payload.as_ref())?,
            confidence: self.confidence,
            provenance: self.provenance.clone(),
        })
    }

    /// Creates a promoted context projection that preserves this proposal's
    /// exact typed payload.
    ///
    /// Promotion authority still belongs to the engine/gate. This method only
    /// avoids re-serializing or textifying a payload after that authority has
    /// already accepted the proposal.
    #[must_use]
    pub fn to_context_fact(
        &self,
        id: impl Into<FactId>,
        promotion_record: FactPromotionRecord,
        created_at: impl Into<Timestamp>,
    ) -> ContextFact {
        ContextFact {
            key: self.key,
            id: id.into(),
            subject: self.subject.clone(),
            payload: Arc::clone(&self.payload),
            promotion_record,
            created_at: created_at.into(),
        }
    }

    /// Attaches the app-owned subject this proposal is about.
    #[must_use]
    pub fn with_subject(mut self, subject: SubjectRef) -> Self {
        self.subject = Some(subject);
        self
    }

    /// Copies the app-owned subject from a promoted source fact when present.
    ///
    /// This is the common Suggestor pattern: a specialist derives a proposal
    /// from an input fact without interpreting the subject vocabulary itself.
    #[must_use]
    pub fn with_subject_from(mut self, source: &ContextFact) -> Self {
        self.subject = source.subject().cloned();
        self
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

    /// Returns the app-owned subject this proposal is about, when tagged.
    #[must_use]
    pub fn subject(&self) -> Option<&SubjectRef> {
        self.subject.as_ref()
    }

    /// Returns the typed payload when the requested type matches the stored
    /// payload family/version.
    #[must_use]
    pub fn payload<T: FactPayload>(&self) -> Option<&T> {
        self.payload.as_any().downcast_ref::<T>()
    }

    /// Returns the typed payload or a mismatch error.
    pub fn require_payload<T: FactPayload>(&self) -> Result<&T, PayloadError> {
        self.payload::<T>()
            .ok_or_else(|| PayloadError::TypeMismatch {
                expected: T::FAMILY.into(),
                expected_version: T::VERSION.into(),
                actual: self.payload_family(),
                actual_version: self.payload_version(),
            })
    }

    /// Returns the payload family.
    #[must_use]
    pub fn payload_family(&self) -> FactFamilyId {
        self.payload.family()
    }

    /// Returns the payload version.
    #[must_use]
    pub fn payload_version(&self) -> PayloadVersion {
        self.payload.version()
    }

    /// Returns the payload as text when this is a [`TextPayload`].
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.payload::<TextPayload>().map(TextPayload::as_str)
    }

    /// Validates the stored payload.
    pub fn validate_payload(&self) -> Result<(), PayloadError> {
        self.payload.validate()
    }

    /// Returns the proposal provenance value.
    #[must_use]
    pub fn provenance_ref(&self) -> &Provenance {
        &self.provenance
    }

    /// Returns the proposal provenance string for wire/logging boundaries.
    #[must_use]
    pub fn provenance(&self) -> &str {
        self.provenance.as_str()
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

    /// Adjust confidence by a named step, clamped to [0.0, 1.0].
    ///
    /// This is the recommended way to express confidence in suggestors and pack
    /// solvers. Use the `CONFIDENCE_STEP_*` constants as the vocabulary:
    ///
    /// ```rust,ignore
    /// use converge_pack::{CONFIDENCE_STEP_MAJOR, CONFIDENCE_STEP_MINOR, CONFIDENCE_STEP_TINY, TextPayload};
    ///
    /// let proposal = EXAMPLE_PROVENANCE.proposed_fact(key, id, TextPayload::new(content))
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

    #[derive(Clone, Copy, Debug)]
    struct TestProvenance;

    impl ProvenanceSource for TestProvenance {
        fn as_str(&self) -> &'static str {
            "test-provenance"
        }
    }

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
        ContextFact::new_projection(
            key,
            id,
            TextPayload::new(content),
            projection_record(),
            Timestamp::epoch(),
        )
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct TestPayload {
        kind: String,
        score: f64,
    }

    impl FactPayload for TestPayload {
        const FAMILY: &'static str = "test.payload";
        const VERSION: u16 = 1;
    }

    fn native_identity() -> NativeExecutionIdentity {
        NativeExecutionIdentity::new(
            "CVC5",
            "1.3.3",
            "https://github.com/cvc5/cvc5",
            "expected",
            "actual",
            "vendored",
        )
    }

    #[test]
    fn execution_identity_evidence_targets_typed_payload() {
        let identity = ExecutionIdentity::new(
            ExecutionProducerIdentity::new("soter", "0.1.0"),
            "cvc5",
            "1.3.3",
            "configure_flags=--no-poly",
            "timeout_ms=5000",
            Some(native_identity()),
        );
        let evidence = ExecutionIdentityEvidence::for_payload::<TestPayload>(
            ContextKey::Evaluations,
            "smt-report-q1",
            identity,
        );

        assert_eq!(evidence.subject_key, ContextKey::Evaluations);
        assert_eq!(evidence.subject_id, "smt-report-q1");
        assert_eq!(evidence.subject_family, FactFamilyId::from("test.payload"));
        assert_eq!(evidence.subject_version, PayloadVersion::new(1));
        assert_eq!(evidence.identity.backend, "cvc5");
        assert!(FactPayload::validate(&evidence).is_ok());
    }

    #[test]
    fn execution_identity_evidence_rejects_empty_subject_id() {
        let evidence = ExecutionIdentityEvidence::for_payload::<TestPayload>(
            ContextKey::Strategies,
            "",
            ExecutionIdentity::non_native("ferrox", "0.5.1", "greedy", "tasks=3"),
        );

        assert!(matches!(
            FactPayload::validate(&evidence),
            Err(PayloadError::Invalid { .. })
        ));
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
        let pf = ProposedFact::new(
            ContextKey::Hypotheses,
            "p1",
            TextPayload::new("my content"),
            TestProvenance.provenance(),
        );
        assert_eq!(pf.key, ContextKey::Hypotheses);
        assert_eq!(pf.id, "p1");
        assert_eq!(pf.text(), Some("my content"));
        assert_eq!(pf.confidence(), 1.0);
        assert_eq!(pf.provenance(), "test-provenance");
    }

    #[test]
    fn proposed_fact_with_confidence() {
        let pf = ProposedFact::new(
            ContextKey::Signals,
            "p2",
            TextPayload::new("c"),
            TestProvenance.provenance(),
        )
        .with_confidence(0.42);
        assert!((pf.confidence() - 0.42).abs() < f64::EPSILON);
    }

    #[test]
    fn adjust_confidence_accumulates() {
        let pf = ProposedFact::new(
            ContextKey::Seeds,
            "p",
            TextPayload::new("c"),
            TestProvenance.provenance(),
        )
        .with_confidence(0.5)
        .adjust_confidence(CONFIDENCE_STEP_MINOR)
        .adjust_confidence(CONFIDENCE_STEP_MAJOR);
        assert!((pf.confidence() - 0.8).abs() < f64::EPSILON);
    }

    #[test]
    fn adjust_confidence_clamps_at_one() {
        let pf = ProposedFact::new(
            ContextKey::Seeds,
            "p",
            TextPayload::new("c"),
            TestProvenance.provenance(),
        )
        .with_confidence(0.9)
        .adjust_confidence(CONFIDENCE_STEP_MAJOR);
        assert_eq!(pf.confidence(), 1.0);
    }

    #[test]
    fn adjust_confidence_clamps_at_zero() {
        let pf = ProposedFact::new(
            ContextKey::Seeds,
            "p",
            TextPayload::new("c"),
            TestProvenance.provenance(),
        )
        .with_confidence(0.1)
        .adjust_confidence(-0.5);
        assert_eq!(pf.confidence(), 0.0);
    }

    #[test]
    fn with_confidence_clamps_high() {
        let pf = ProposedFact::new(
            ContextKey::Seeds,
            "p",
            TextPayload::new("c"),
            TestProvenance.provenance(),
        )
        .with_confidence(1.5);
        assert_eq!(pf.confidence(), 1.0);
    }

    #[test]
    fn with_confidence_clamps_negative() {
        let pf = ProposedFact::new(
            ContextKey::Seeds,
            "p",
            TextPayload::new("c"),
            TestProvenance.provenance(),
        )
        .with_confidence(-0.1);
        assert_eq!(pf.confidence(), 0.0);
    }

    #[test]
    fn with_confidence_normalizes_nan() {
        let pf = ProposedFact::new(
            ContextKey::Seeds,
            "p",
            TextPayload::new("c"),
            TestProvenance.provenance(),
        )
        .with_confidence(f64::NAN);
        assert_eq!(pf.confidence(), 0.0);
    }

    #[test]
    fn with_confidence_normalizes_infinity() {
        let pf = ProposedFact::new(
            ContextKey::Seeds,
            "p",
            TextPayload::new("c"),
            TestProvenance.provenance(),
        )
        .with_confidence(f64::INFINITY);
        assert_eq!(pf.confidence(), 0.0);
    }

    #[test]
    fn wire_proposed_fact_deserialization_rejects_out_of_range_confidence() {
        let json = r#"{
            "key":"Seeds",
            "id":"p",
            "payload":{
                "family":"converge.text",
                "version":1,
                "payload":{"text":"c"}
            },
            "confidence":1.5,
            "provenance":"test"
        }"#;
        let result = serde_json::from_str::<WireProposedFact>(json);
        assert!(result.is_err());
    }

    #[test]
    fn proposed_fact_wire_round_trips_through_registry() {
        let payload = TestPayload {
            kind: "vote".into(),
            score: 0.7,
        };
        let subject = SubjectRef::parse("atlas://acquisition-assets/shared-identity-core")
            .expect("valid subject");
        let pf = ProposedFact::new(
            ContextKey::Hypotheses,
            "p",
            payload.clone(),
            TestProvenance.provenance(),
        )
        .with_subject(subject.clone());
        let wire = pf.to_wire().unwrap();
        let mut registry = PayloadRegistry::new();
        registry.register::<TestPayload>();

        let decoded = ProposedFact::from_wire(wire, &registry).unwrap();

        assert_eq!(decoded.key, ContextKey::Hypotheses);
        assert_eq!(decoded.id, "p");
        assert_eq!(decoded.subject(), Some(&subject));
        assert_eq!(decoded.provenance(), "test-provenance");
        assert_eq!(decoded.require_payload::<TestPayload>().unwrap(), &payload);
    }

    #[test]
    fn proposed_fact_from_wire_fails_closed_for_unknown_family_version() {
        let wire = WireProposedFact {
            key: ContextKey::Hypotheses,
            id: "p".into(),
            subject: None,
            payload: WireFactPayload {
                family: FactFamilyId::new("unknown.payload"),
                version: PayloadVersion::new(1),
                payload: serde_json::json!({"kind":"vote"}),
            },
            confidence: UnitInterval::ONE,
            provenance: TestProvenance.provenance(),
        };

        let registry = PayloadRegistry::new();
        let result = ProposedFact::from_wire(wire, &registry);

        assert!(matches!(
            result,
            Err(PayloadError::UnknownFamilyVersion { .. })
        ));
    }

    #[test]
    fn context_fact_wire_round_trips_through_registry() {
        let payload = TestPayload {
            kind: "fact".into(),
            score: 0.9,
        };
        let subject = SubjectRef::parse("quorum://unresolved-questions/identity-owner-coverage")
            .expect("valid subject");
        let fact = ContextFact::new_projection(
            ContextKey::Seeds,
            "f",
            payload.clone(),
            projection_record(),
            Timestamp::epoch(),
        )
        .with_subject(subject.clone());
        let wire = fact.to_wire().unwrap();
        let mut registry = PayloadRegistry::new();
        registry.register::<TestPayload>();

        let decoded = ContextFact::from_wire(wire, &registry).unwrap();

        assert_eq!(decoded.key(), ContextKey::Seeds);
        assert_eq!(decoded.id(), "f");
        assert_eq!(decoded.subject(), Some(&subject));
        assert_eq!(decoded.require_payload::<TestPayload>().unwrap(), &payload);
    }

    #[test]
    fn proposed_fact_to_context_fact_preserves_typed_payload() {
        let payload = TestPayload {
            kind: "proposal".into(),
            score: 0.8,
        };
        let proposal = ProposedFact::new(
            ContextKey::Strategies,
            "p",
            payload.clone(),
            TestProvenance.provenance(),
        )
        .with_subject(
            SubjectRef::parse("warden://dd-gates/dd-evidence.identity-data-residency")
                .expect("valid subject"),
        );

        let fact = proposal.to_context_fact("f", projection_record(), Timestamp::epoch());

        assert_eq!(fact.key(), ContextKey::Strategies);
        assert_eq!(fact.subject(), proposal.subject());
        assert_eq!(fact.require_payload::<TestPayload>().unwrap(), &payload);
    }

    #[test]
    fn proposed_fact_with_subject_from_copies_source_subject() {
        let subject = SubjectRef::parse("atlas://acquisition-assets/shared-identity-core")
            .expect("valid subject");
        let source = projection_fact(ContextKey::Seeds, "seed-1", "source").with_subject(subject);

        let pf = ProposedFact::new(
            ContextKey::Hypotheses,
            "p1",
            TextPayload::new("derived"),
            TestProvenance.provenance(),
        )
        .with_subject_from(&source);

        assert_eq!(pf.subject(), source.subject());
    }

    #[test]
    fn provenance_source_proposed_fact_for_copies_source_subject() {
        let subject = SubjectRef::parse("atlas://acquisition-assets/shared-identity-core")
            .expect("valid subject");
        let source = projection_fact(ContextKey::Seeds, "seed-1", "source").with_subject(subject);

        let pf = TestProvenance.proposed_fact_for(
            &source,
            ContextKey::Hypotheses,
            "p1",
            TextPayload::new("derived"),
        );

        assert_eq!(pf.subject(), source.subject());
        assert_eq!(pf.provenance(), "test-provenance");
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
        assert_eq!(fact.text(), Some("body"));
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
                let pf = ProposedFact::new(
                    key,
                    id.clone(),
                    TextPayload::new(content.clone()),
                    Provenance::new(prov.clone()),
                );
                prop_assert_eq!(pf.key, key);
                prop_assert_eq!(&pf.id, &id);
                prop_assert_eq!(pf.text(), Some(content.as_str()));
                prop_assert_eq!(pf.provenance(), prov.as_str());
                prop_assert!((pf.confidence() - 1.0).abs() < f64::EPSILON);
            }
        }
    }
}
