// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Shared semantic value types for the public Converge contract.

use serde::de;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;

macro_rules! string_newtype {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(
            Debug,
            Clone,
            PartialEq,
            Eq,
            Hash,
            PartialOrd,
            Ord,
            Serialize,
            Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Create a new typed string value.
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            /// Borrow the underlying string.
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                self.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }

        impl From<&$name> for $name {
            fn from(value: &$name) -> Self {
                value.clone()
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl From<&$name> for String {
            fn from(value: &$name) -> Self {
                value.as_str().to_string()
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.as_str() == other
            }
        }

        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.as_str()
            }
        }

        impl PartialEq<$name> for str {
            fn eq(&self, other: &$name) -> bool {
                self == other.as_str()
            }
        }

        impl PartialEq<String> for $name {
            fn eq(&self, other: &String) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl PartialEq<$name> for String {
            fn eq(&self, other: &$name) -> bool {
                self.as_str() == other.as_str()
            }
        }

        impl PartialEq<&$name> for $name {
            fn eq(&self, other: &&$name) -> bool {
                self == *other
            }
        }
    };
}

/// Error returned when a unit interval value is outside `[0.0, 1.0]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitIntervalError;

impl fmt::Display for UnitIntervalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("value must be finite and in the inclusive range 0.0..=1.0")
    }
}

impl std::error::Error for UnitIntervalError {}

/// A finite value in the inclusive `[0.0, 1.0]` range.
///
/// Use this for confidence, normalized scores, prior weights, and thresholds.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct UnitInterval(f64);

impl UnitInterval {
    /// The minimum unit interval value.
    pub const ZERO: Self = Self(0.0);
    /// The maximum unit interval value.
    pub const ONE: Self = Self(1.0);

    /// Create a validated unit interval.
    ///
    /// Returns an error for NaN, infinity, or values outside `[0.0, 1.0]`.
    pub fn new(value: f64) -> Result<Self, UnitIntervalError> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(UnitIntervalError)
        }
    }

    /// Create a unit interval by clamping finite input.
    ///
    /// Non-finite values become `0.0`.
    #[must_use]
    pub fn clamped(value: f64) -> Self {
        if value.is_finite() {
            Self(value.clamp(0.0, 1.0))
        } else {
            Self::ZERO
        }
    }

    /// Return the underlying `f64`.
    #[must_use]
    pub fn as_f64(self) -> f64 {
        self.0
    }

    /// Add a delta and clamp the result back into the valid range.
    #[must_use]
    pub fn saturating_add(self, delta: f64) -> Self {
        Self::clamped(self.0 + delta)
    }

    /// Multiply two unit interval values.
    #[must_use]
    pub fn scale_by(self, factor: Self) -> Self {
        Self(self.0 * factor.0)
    }

    /// Convert to basis points, rounded to the nearest basis point.
    #[must_use]
    pub fn to_basis_points(self) -> u16 {
        (self.0 * 10_000.0).round() as u16
    }
}

impl Default for UnitInterval {
    fn default() -> Self {
        Self::ZERO
    }
}

impl TryFrom<f64> for UnitInterval {
    type Error = UnitIntervalError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<UnitInterval> for f64 {
    fn from(value: UnitInterval) -> Self {
        value.as_f64()
    }
}

impl<'de> Deserialize<'de> for UnitInterval {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = f64::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

/// Error returned when a basis-point value is outside `0..=10_000`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BasisPointsError;

impl fmt::Display for BasisPointsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("basis points must be in the inclusive range 0..=10000")
    }
}

impl std::error::Error for BasisPointsError {}

/// A unit-range value represented as basis points (`0..=10_000`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct BasisPoints(u16);

impl BasisPoints {
    /// Zero basis points.
    pub const ZERO: Self = Self(0);
    /// Ten thousand basis points, equivalent to `1.0`.
    pub const FULL: Self = Self(10_000);

    /// Create a validated basis-point value.
    pub fn new(value: u16) -> Result<Self, BasisPointsError> {
        if value <= 10_000 {
            Ok(Self(value))
        } else {
            Err(BasisPointsError)
        }
    }

    /// Create a basis-point value by clamping input to `0..=10_000`.
    #[must_use]
    pub fn clamped(value: u16) -> Self {
        Self(value.min(10_000))
    }

    /// Return the raw basis-point value.
    #[must_use]
    pub fn get(self) -> u16 {
        self.0
    }

    /// Convert to a unit interval.
    #[must_use]
    pub fn as_unit_interval(self) -> UnitInterval {
        UnitInterval::clamped(f64::from(self.0) / 10_000.0)
    }
}

impl Default for BasisPoints {
    fn default() -> Self {
        Self::ZERO
    }
}

impl TryFrom<u16> for BasisPoints {
    type Error = BasisPointsError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<BasisPoints> for u16 {
    fn from(value: BasisPoints) -> Self {
        value.get()
    }
}

impl<'de> Deserialize<'de> for BasisPoints {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u16::deserialize(deserializer)?;
        Self::new(value).map_err(de::Error::custom)
    }
}

string_newtype!(
    /// Unique identifier for a promoted fact.
    FactId
);
string_newtype!(
    /// Unique identifier for a proposal.
    ProposalId
);
string_newtype!(
    /// Unique identifier for a raw observation.
    ObservationId
);
string_newtype!(
    /// Unique identifier for a human approval.
    ApprovalId
);
string_newtype!(
    /// Unique identifier for a derived artifact.
    ArtifactId
);
string_newtype!(
    /// Unique identifier for a promotion gate.
    GateId
);
string_newtype!(
    /// Identifier for a recorded actor.
    ActorId
);
string_newtype!(
    /// Identifier for a named validation check.
    ValidationCheckId
);
string_newtype!(
    /// Identifier for a trace.
    TraceId
);
string_newtype!(
    /// Identifier for a trace span.
    SpanId
);
string_newtype!(
    /// Identifier for an external trace system.
    TraceSystemId
);
string_newtype!(
    /// Reference into an external trace system.
    TraceReference
);
string_newtype!(
    /// Identifier for a flow-gate principal.
    PrincipalId
);
string_newtype!(
    /// Identifier for an experience event envelope.
    EventId
);
string_newtype!(
    /// Identifier for a tenant scope.
    TenantId
);
string_newtype!(
    /// Identifier for correlating related events or runs.
    CorrelationId
);
string_newtype!(
    /// Identifier for a convergence chain or run.
    ChainId
);
string_newtype!(
    /// Identifier for a stored replay trace link.
    TraceLinkId
);
string_newtype!(
    /// Identifier for a backend, provider, or adapter.
    BackendId
);
string_newtype!(
    /// Identifier for a named pack.
    PackId
);
string_newtype!(
    /// Identifier for a truth definition.
    TruthId
);
string_newtype!(
    /// Identifier for a policy definition.
    PolicyId
);
string_newtype!(
    /// Identifier for an approval point or workflow reference.
    ApprovalPointId
);
string_newtype!(
    /// Identifier for an individual vote cast on a topic.
    VoteId
);
string_newtype!(
    /// Identifier for the topic a vote or disagreement is about.
    VoteTopicId
);
string_newtype!(
    /// Identifier for a recorded disagreement.
    DisagreementId
);
string_newtype!(
    /// Identifier for a success criterion.
    CriterionId
);
string_newtype!(
    /// Consumer-owned name for a constraint.
    ConstraintName
);
string_newtype!(
    /// Consumer-owned value for a constraint.
    ConstraintValue
);
string_newtype!(
    /// Identifier for a business or governance domain.
    DomainId
);
string_newtype!(
    /// Identifier for a bound policy version label.
    PolicyVersionId
);
string_newtype!(
    /// Identifier for a converging resource or flow.
    ResourceId
);
string_newtype!(
    /// Open identifier for a resource kind owned by the consumer domain.
    ResourceKind
);

/// Content-addressable hash encoded as 32 raw bytes and serialized as hex.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ContentHash(#[serde(with = "hex_bytes")] [u8; 32]);

impl ContentHash {
    /// Create a new content hash from raw bytes.
    #[must_use]
    pub fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Create a content hash from a hex string.
    ///
    /// # Panics
    ///
    /// Panics if the hex string is not exactly 64 hexadecimal characters.
    #[must_use]
    pub fn from_hex(hex: &str) -> Self {
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(hex, &mut bytes).expect("invalid hex string");
        Self(bytes)
    }

    /// Borrow the raw bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Convert to lowercase hex.
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// The zero hash, useful for deterministic stubs.
    #[must_use]
    pub fn zero() -> Self {
        Self([0u8; 32])
    }
}

impl Default for ContentHash {
    fn default() -> Self {
        Self::zero()
    }
}

impl fmt::Display for ContentHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

mod hex_bytes {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        let mut bytes = [0u8; 32];
        hex::decode_to_slice(raw, &mut bytes).map_err(serde::de::Error::custom)?;
        Ok(bytes)
    }
}

/// Timestamp string used at Converge boundaries.
///
/// Runtime-visible timestamps may be wall-clock strings supplied by a host or
/// logical Lamport clock stamps produced by the kernel. Core deterministic
/// promotion paths use Lamport stamps instead of reading wall-clock time.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(String);

impl Timestamp {
    /// Create a new timestamp from an already formatted string.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Borrow the timestamp string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The Unix epoch in ISO-8601 form.
    #[must_use]
    pub fn epoch() -> Self {
        Self::new("1970-01-01T00:00:00Z")
    }

    /// Create a deterministic timestamp from Lamport logical time.
    #[must_use]
    pub fn lamport(time: u64) -> Self {
        Self(format!("lamport:{time}"))
    }

    /// A deterministic zero logical timestamp.
    ///
    /// Hosts that need wall-clock timestamps should construct them explicitly at
    /// the runtime boundary rather than letting core code read a hidden clock.
    #[must_use]
    pub fn now() -> Self {
        Self::lamport(0)
    }
}

impl Deref for Timestamp {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for Timestamp {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for Timestamp {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for Timestamp {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<Timestamp> for String {
    fn from(value: Timestamp) -> Self {
        value.0
    }
}

impl From<&Timestamp> for String {
    fn from(value: &Timestamp) -> Self {
        value.as_str().to_string()
    }
}

impl PartialEq<&str> for Timestamp {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<str> for Timestamp {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<Timestamp> for &str {
    fn eq(&self, other: &Timestamp) -> bool {
        *self == other.as_str()
    }
}

impl PartialEq<Timestamp> for str {
    fn eq(&self, other: &Timestamp) -> bool {
        self == other.as_str()
    }
}

impl PartialEq<String> for Timestamp {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other.as_str()
    }
}

impl PartialEq<Timestamp> for String {
    fn eq(&self, other: &Timestamp) -> bool {
        self.as_str() == other.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_newtypes_compare_like_strings_without_erasing_type_identity() {
        let fact_id = FactId::new("fact-1");
        let proposal_id = ProposalId::new("fact-1");

        assert_eq!(fact_id, "fact-1");
        assert_eq!("fact-1", fact_id);
        assert_ne!(fact_id.to_string(), "");
        assert_ne!(fact_id.as_str(), "");
        assert_eq!(proposal_id.as_str(), "fact-1");
    }

    #[test]
    fn content_hash_hex_roundtrip() {
        let hash = ContentHash::from_hex(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        );
        assert_eq!(
            hash.to_hex(),
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
        );
    }

    #[test]
    fn timestamp_now_is_deterministic_logical_zero() {
        assert_eq!(Timestamp::now().as_str(), "lamport:0");
    }

    #[test]
    fn timestamp_can_represent_lamport_time() {
        assert_eq!(Timestamp::lamport(42).as_str(), "lamport:42");
    }

    #[test]
    fn unit_interval_accepts_only_finite_closed_range_values() {
        assert_eq!(UnitInterval::new(0.0).unwrap().as_f64(), 0.0);
        assert_eq!(UnitInterval::new(1.0).unwrap().as_f64(), 1.0);
        assert!(UnitInterval::new(-0.1).is_err());
        assert!(UnitInterval::new(1.1).is_err());
        assert!(UnitInterval::new(f64::NAN).is_err());
    }

    #[test]
    fn unit_interval_deserialization_rejects_out_of_range_values() {
        assert!(serde_json::from_str::<UnitInterval>("0.75").is_ok());
        assert!(serde_json::from_str::<UnitInterval>("1.75").is_err());
    }

    #[test]
    fn basis_points_accepts_only_unit_range_basis_points() {
        assert_eq!(BasisPoints::new(0).unwrap().get(), 0);
        assert_eq!(BasisPoints::new(10_000).unwrap().get(), 10_000);
        assert!(BasisPoints::new(10_001).is_err());
        assert_eq!(BasisPoints::clamped(20_000).get(), 10_000);
    }

    #[test]
    fn timestamp_is_transparent() {
        let timestamp = Timestamp::epoch();
        let json = serde_json::to_string(&timestamp).expect("timestamp should serialize");
        assert_eq!(json, r#""1970-01-01T00:00:00Z""#);
    }
}
