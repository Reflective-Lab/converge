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
string_newtype!(
    /// Runtime provider identifier.
    ProviderId
);
string_newtype!(
    /// Runtime pack name.
    PackName
);
string_newtype!(
    /// Runtime suggestor wiring identifier.
    AgentName
);

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
}
