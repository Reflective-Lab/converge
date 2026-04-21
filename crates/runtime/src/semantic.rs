// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Runtime-local semantic types for identities, methods, and wiring identifiers.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::borrow::Borrow;
use std::fmt;
use std::ops::Deref;
use utoipa::ToSchema;

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
            ToSchema,
        )]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

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

        impl From<&String> for $name {
            fn from(value: &String) -> Self {
                Self::new(value.clone())
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

        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.as_str()
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
    };
}

macro_rules! validated_string_newtype {
    ($(#[$meta:meta])* $name:ident, $validator:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, ToSchema)]
        #[schema(value_type = String)]
        pub struct $name(String);

        impl $name {
            pub fn try_new(value: impl Into<String>) -> Result<Self, String> {
                let value = value.into();
                $validator(&value)?;
                Ok(Self(value))
            }

            #[must_use]
            pub fn new(value: impl Into<String>) -> Self {
                let value = value.into();
                match Self::try_new(value.clone()) {
                    Ok(parsed) => parsed,
                    Err(error) => panic!("{error}"),
                }
            }

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

        impl From<&String> for $name {
            fn from(value: &String) -> Self {
                Self::new(value.clone())
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

        impl Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let raw = String::deserialize(deserializer)?;
                Self::try_new(raw).map_err(serde::de::Error::custom)
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.as_str()
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
    };
}

string_newtype!(
    /// Open identifier for a service principal.
    ServiceId
);
string_newtype!(
    /// Open identifier for an end user principal.
    UserId
);
string_newtype!(
    /// Open identifier for a role.
    RoleId
);
string_newtype!(
    /// Open identifier for an organization or tenant.
    OrgId
);
string_newtype!(
    /// SPIFFE identifier captured from mTLS.
    SpiffeId
);
string_newtype!(
    /// JWT identifier for audit and replay protection.
    JwtId
);
string_newtype!(
    /// Certificate fingerprint captured during authentication.
    CertificateFingerprint
);
string_newtype!(
    /// Fully-qualified gRPC method path.
    GrpcMethod
);

validated_string_newtype!(
    /// Runtime provider identifier.
    ProviderId,
    validate_provider_id
);
validated_string_newtype!(
    /// Runtime pack name.
    PackName,
    validate_pack_name
);
validated_string_newtype!(
    /// Runtime suggestor wiring identifier.
    AgentName,
    validate_agent_name
);
validated_string_newtype!(
    /// Runtime pack semantic version.
    PackVersion,
    validate_pack_version
);
validated_string_newtype!(
    /// Runtime compatibility version requirement.
    VersionRequirement,
    validate_version_requirement
);

/// Closed vocabulary for runtime selection presets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RequirementPreset {
    FastExtraction,
    Analysis,
    Synthesis,
    DeepResearch,
    Deterministic,
}

impl RequirementPreset {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FastExtraction => "fast_extraction",
            Self::Analysis => "analysis",
            Self::Synthesis => "synthesis",
            Self::DeepResearch => "deep_research",
            Self::Deterministic => "deterministic",
        }
    }
}

impl fmt::Display for RequirementPreset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A quality threshold constrained to the closed unit interval.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, ToSchema)]
#[schema(value_type = f64)]
pub struct QualityThreshold(f64);

impl QualityThreshold {
    pub fn try_new(value: f64) -> Result<Self, String> {
        if !value.is_finite() {
            return Err("quality threshold must be finite".to_string());
        }
        if !(0.0..=1.0).contains(&value) {
            return Err(format!(
                "quality threshold must be in [0.0, 1.0], got {value}"
            ));
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn new(value: f64) -> Self {
        match Self::try_new(value) {
            Ok(parsed) => parsed,
            Err(error) => panic!("{error}"),
        }
    }

    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

impl Serialize for QualityThreshold {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for QualityThreshold {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = f64::deserialize(deserializer)?;
        Self::try_new(raw).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for QualityThreshold {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Explicit matcher semantics for open identifier sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector<T> {
    Any,
    Exact(Vec<T>),
}

impl<T> Default for Selector<T> {
    fn default() -> Self {
        Self::Exact(Vec::new())
    }
}

impl<T> Selector<T> {
    #[must_use]
    pub fn any() -> Self {
        Self::Any
    }

    #[must_use]
    pub fn exact(values: Vec<T>) -> Self {
        Self::Exact(values)
    }

    #[must_use]
    pub fn is_any(&self) -> bool {
        matches!(self, Self::Any)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        matches!(self, Self::Exact(values) if values.is_empty())
    }

    #[must_use]
    pub fn values(&self) -> &[T] {
        match self {
            Self::Any => &[],
            Self::Exact(values) => values,
        }
    }
}

impl<T> Selector<T>
where
    T: PartialEq,
{
    #[must_use]
    pub fn matches(&self, value: &T) -> bool {
        match self {
            Self::Any => true,
            Self::Exact(values) => values.iter().any(|candidate| candidate == value),
        }
    }

    #[must_use]
    pub fn matches_any<'a>(&self, values: impl IntoIterator<Item = &'a T>) -> bool
    where
        T: 'a,
    {
        match self {
            Self::Any => true,
            Self::Exact(expected) => values
                .into_iter()
                .any(|value| expected.iter().any(|candidate| candidate == value)),
        }
    }
}

impl<T> Serialize for Selector<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Any => serializer.serialize_str("*"),
            Self::Exact(values) => values.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for Selector<T>
where
    T: Deserialize<'de> + AsRef<str>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr<T> {
            Wildcard(String),
            Values(Vec<T>),
        }

        match Repr::<T>::deserialize(deserializer)? {
            Repr::Wildcard(value) => {
                if value == "*" {
                    Ok(Self::Any)
                } else {
                    Err(serde::de::Error::custom(
                        "selector string must be '*' or a list of values",
                    ))
                }
            }
            Repr::Values(values) => {
                if values.len() == 1 && values[0].as_ref() == "*" {
                    Ok(Self::Any)
                } else {
                    Ok(Self::Exact(values))
                }
            }
        }
    }
}

fn validate_pack_name(value: &str) -> Result<(), String> {
    validate_lower_identifier("pack name", value)
}

fn validate_provider_id(value: &str) -> Result<(), String> {
    validate_lower_identifier("provider id", value)
}

fn validate_lower_identifier(kind: &str, value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{kind} cannot be empty"));
    }
    if value.len() > 64 {
        return Err(format!("{kind} must be at most 64 characters"));
    }
    if !value
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase())
    {
        return Err(format!("{kind} must start with a lowercase ASCII letter"));
    }
    if value.starts_with('-') || value.ends_with('-') || value.contains("--") {
        return Err(format!("{kind} cannot start, end, or repeat '-'"));
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(format!(
            "{kind} may contain only lowercase ASCII letters, digits, and '-'"
        ));
    }
    Ok(())
}

fn validate_agent_name(value: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err("agent name cannot be empty".to_string());
    }
    if value.len() > 64 {
        return Err("agent name must be at most 64 characters".to_string());
    }
    if !value
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphabetic())
    {
        return Err("agent name must start with an ASCII letter".to_string());
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return Err("agent name may contain only ASCII letters, digits, '_' and '-'".to_string());
    }
    Ok(())
}

fn validate_pack_version(value: &str) -> Result<(), String> {
    semver::Version::parse(value)
        .map(|_| ())
        .map_err(|error| format!("invalid pack version '{value}': {error}"))
}

fn validate_version_requirement(value: &str) -> Result<(), String> {
    semver::VersionReq::parse(value)
        .map(|_| ())
        .map_err(|error| format!("invalid version requirement '{value}': {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_parses_wildcard_string() {
        let parsed: Selector<RoleId> = serde_yaml::from_str("\"*\"").expect("selector parses");
        assert!(parsed.is_any());
    }

    #[test]
    fn selector_parses_value_list() {
        let parsed: Selector<ServiceId> =
            serde_yaml::from_str("[api-gateway, scheduler]").expect("selector parses");
        assert_eq!(
            parsed,
            Selector::exact(vec![
                ServiceId::new("api-gateway"),
                ServiceId::new("scheduler")
            ])
        );
    }

    #[test]
    fn pack_name_rejects_invalid_values() {
        assert!(PackName::try_new("Growth Strategy").is_err());
        assert!(PackName::try_new("growth_strategy").is_err());
        assert!(PackName::try_new("growth--strategy").is_err());
    }

    #[test]
    fn agent_name_accepts_pascal_case_and_snake_case() {
        assert_eq!(AgentName::new("ReleaseReadyAgent"), "ReleaseReadyAgent");
        assert_eq!(AgentName::new("signal_ingest"), "signal_ingest");
    }

    #[test]
    fn pack_version_requires_semver() {
        assert!(PackVersion::try_new("1.2.3").is_ok());
        assert!(PackVersion::try_new("v1").is_err());
    }

    #[test]
    fn quality_threshold_rejects_out_of_range_values() {
        assert!(QualityThreshold::try_new(-0.1).is_err());
        assert!(QualityThreshold::try_new(1.1).is_err());
        assert_eq!(QualityThreshold::new(0.75).get(), 0.75);
    }
}
