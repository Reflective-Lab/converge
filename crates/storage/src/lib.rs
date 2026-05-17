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
//! - `hf://...` → HuggingFace datasets (via manifold's `HuggingFaceObjectStore`)
//!
//! Object key prefixes are configured separately via [`StorageConfig::prefix`].
//!
//! ## Optional Polars / Parquet bridge
//!
//! Enable the `polars` feature to bring in the
//! [`polars_bridge`](crate::polars_bridge) module — a shared
//! Parquet read/write surface that fetches blobs from any
//! `ObjectStore` into a local cache, then hands them to Polars for
//! zero-copy memory-mapped scans. Lifted out of `crucible-models` so
//! every extension that needs columnar data from remote storage (the
//! training pipeline, KB persistence, app-level dataset ingest) shares
//! a single implementation rather than re-inventing the cache logic.
//!
mod config;
mod error;
#[cfg(feature = "polars")]
pub mod polars_bridge;
mod uri;

pub use config::StorageConfig;
pub use error::StorageError;
pub use uri::StorageUri;

pub use object_store::path::Path as ObjectPath;
pub use object_store::{self, GetResult, ObjectStore, PutResult};
