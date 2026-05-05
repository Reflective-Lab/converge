// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # converge-storage
//!
//! Object storage contract types for Converge.
//!
//! Concrete builders for local filesystem, S3-compatible stores, and Google
//! Cloud Storage live in adapter extensions such as `manifold`.
//!
//! ## Storage URI Convention
//!
//! - `file:///path/to/dir` or plain paths → local filesystem
//! - `s3://bucket` → S3-compatible (AWS, `MinIO`, `RustFS`)
//! - `gs://bucket` → Google Cloud Storage
//!
//! Object key prefixes are configured separately via [`StorageConfig::prefix`].
//!
mod config;
mod error;
mod uri;

pub use config::StorageConfig;
pub use error::StorageError;
pub use uri::StorageUri;

pub use object_store::path::Path as ObjectPath;
pub use object_store::{self, GetResult, ObjectStore, PutResult};
