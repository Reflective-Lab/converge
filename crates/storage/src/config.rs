// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

use serde::{Deserialize, Serialize};

use crate::StorageUri;

/// Storage configuration, typically loaded from TOML:
///
/// ```toml
/// [storage]
/// uri = "./data/parquet"        # local
/// # uri = "s3://my-bucket"      # S3/MinIO/RustFS bucket
/// # uri = "gs://my-bucket"      # GCS bucket
/// prefix = "datasets/"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StorageConfig {
    pub uri: StorageUri,

    /// Object key prefix applied to all operations.
    /// This is stored separately from `uri`.
    ///
    /// e.g., `"datasets/"` means `get("file.parquet")` resolves to `datasets/file.parquet`.
    #[serde(default)]
    pub prefix: Option<String>,

    /// Skip request signing for public buckets.
    /// Only used when `uri` is `s3://` or `gs://`.
    #[serde(default)]
    pub public: bool,

    /// S3-compatible endpoint override (`MinIO`, `RustFS`, etc.).
    /// Only used when `uri` is `s3://`.
    #[serde(default)]
    pub endpoint: Option<String>,

    /// AWS region. Only used when `uri` is `s3://`.
    #[serde(default)]
    pub region: Option<String>,
}

impl StorageConfig {
    /// Resolve a key relative to the configured prefix.
    #[must_use]
    pub fn resolve_key(&self, key: &str) -> String {
        let key = key.trim_start_matches('/');

        match self
            .prefix
            .as_deref()
            .map(|prefix| prefix.trim_matches('/'))
            .filter(|prefix| !prefix.is_empty())
        {
            Some(prefix) if key.is_empty() => prefix.to_string(),
            Some(prefix) => format!("{prefix}/{key}"),
            None => key.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_local_config() {
        let toml_str = r#"
            uri = "./data/parquet"
            prefix = "datasets/"
        "#;
        let config: StorageConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.resolve_key("file.parquet"), "datasets/file.parquet");
    }

    #[test]
    fn parse_s3_config() {
        let toml_str = r#"
            uri = "s3://my-bucket"
            prefix = "v1/"
            endpoint = "http://localhost:9000"
            region = "us-east-1"
        "#;
        let config: StorageConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.uri.scheme(), "s3");
        assert_eq!(config.endpoint.as_deref(), Some("http://localhost:9000"));
    }

    #[test]
    fn no_prefix() {
        let toml_str = r#"uri = "gs://my-bucket""#;
        let config: StorageConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.resolve_key("file.parquet"), "file.parquet");
    }

    #[test]
    fn unknown_field_fails_loudly() {
        let toml_str = r#"
            uri = "./data/parquet"
            cache = true
        "#;
        let err = toml::from_str::<StorageConfig>(toml_str).unwrap_err();
        let message = err.to_string();
        assert!(
            message.contains("unknown field"),
            "expected unknown field error, got: {message}"
        );
        assert!(
            message.contains("cache"),
            "expected error to mention unknown field, got: {message}"
        );
    }

    #[test]
    fn resolve_key_normalizes_slashes() {
        let config = StorageConfig {
            uri: StorageUri::Local("./data".into()),
            prefix: Some("/datasets/".to_string()),
            public: false,
            endpoint: None,
            region: None,
        };

        assert_eq!(
            config.resolve_key("/nested/file.parquet"),
            "datasets/nested/file.parquet"
        );
    }
}
