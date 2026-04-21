// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Shared semantic value types for the public Converge contract.

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

/// ISO-8601 timestamp string.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(String);

impl Timestamp {
    /// Create a new timestamp from an ISO-8601 string.
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

    /// A best-effort timestamp for "now".
    #[must_use]
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let duration = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        Self(format!("{}Z", duration.as_secs()))
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
    fn timestamp_is_transparent() {
        let timestamp = Timestamp::epoch();
        let json = serde_json::to_string(&timestamp).expect("timestamp should serialize");
        assert_eq!(json, r#""1970-01-01T00:00:00Z""#);
    }
}
