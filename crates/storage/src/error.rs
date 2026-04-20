// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("unsupported storage scheme: {0}")]
    UnsupportedScheme(String),

    #[error("invalid storage URI: {0}")]
    InvalidUri(String),

    #[error("local path does not exist: {}", .0.display())]
    LocalPathNotFound(PathBuf),

    #[error("object store error: {0}")]
    ObjectStore(#[from] object_store::Error),

    #[error("configuration error: {0}")]
    Config(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_unsupported_scheme() {
        let err = StorageError::UnsupportedScheme("ftp".into());
        assert_eq!(err.to_string(), "unsupported storage scheme: ftp");
    }

    #[test]
    fn display_invalid_uri() {
        let err = StorageError::InvalidUri("not://valid".into());
        assert_eq!(err.to_string(), "invalid storage URI: not://valid");
    }

    #[test]
    fn display_local_path_not_found() {
        let err = StorageError::LocalPathNotFound(PathBuf::from("/missing/dir"));
        assert_eq!(err.to_string(), "local path does not exist: /missing/dir");
    }

    #[test]
    fn display_config() {
        let err = StorageError::Config("bad value".into());
        assert_eq!(err.to_string(), "configuration error: bad value");
    }

    #[test]
    fn from_object_store_error() {
        let obj_err = object_store::Error::Generic {
            store: "test",
            source: "boom".into(),
        };
        let err: StorageError = obj_err.into();
        assert!(matches!(err, StorageError::ObjectStore(_)));
        assert!(err.to_string().contains("object store error"));
    }

    #[test]
    fn error_trait_source_for_object_store_variant() {
        use std::error::Error;

        let obj_err = object_store::Error::Generic {
            store: "test",
            source: "inner".into(),
        };
        let err = StorageError::ObjectStore(obj_err);
        assert!(err.source().is_some());
    }

    #[test]
    fn error_trait_source_is_none_for_plain_variants() {
        use std::error::Error;

        assert!(
            StorageError::UnsupportedScheme("x".into())
                .source()
                .is_none()
        );
        assert!(StorageError::InvalidUri("x".into()).source().is_none());
        assert!(
            StorageError::LocalPathNotFound(PathBuf::from("/x"))
                .source()
                .is_none()
        );
        assert!(StorageError::Config("x".into()).source().is_none());
    }
}
