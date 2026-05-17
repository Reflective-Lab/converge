// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Polars + Parquet bridge over the workspace `ObjectStore` contract.
//!
//! Lifted out of `crucible-models::storage` so any extension that wants
//! to fetch / scan / write Parquet from any backend can share a single
//! implementation. Crucible's training pipeline was the first consumer;
//! the same surface now serves Mnemos persistence, Prism dataset
//! loading, and any future training / analytics workload that needs
//! columnar data from `file://`, `s3://`, `gs://`, or
//! `hf://` (via manifold's `HuggingFaceObjectStore`).
//!
//! The bridge is opinionated about one thing: it materializes blobs
//! into a local cache directory before handing them to Polars. Polars'
//! Parquet scan uses memory-mapped I/O and that does not interact well
//! with `object_store`'s streaming `GetResult` body — so we go through
//! disk. The cache is content-keyed by storage key (not content hash)
//! so callers can pre-clear stale entries themselves; the bridge does
//! not validate freshness.

use std::path::{Path, PathBuf};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use anyhow::{Result, anyhow};
use polars::prelude::*;

use crate::object_store::ObjectStoreExt;
use crate::{ObjectPath, ObjectStore};

/// Fetch a parquet file from storage into a local cache and return the local path.
///
/// Consumers can then pass this path to Polars `scan_parquet` on their own
/// terms (sync context avoids nested tokio runtime issues).
///
/// If the file already exists in `cache_dir`, it is reused without re-downloading.
pub async fn fetch_parquet(
    store: &dyn ObjectStore,
    key: &str,
    cache_dir: &Path,
) -> Result<PathBuf> {
    fetch_to_cache(store, key, cache_dir).await
}

/// Fetch a blob from storage into a local cache directory.
///
/// Returns the local path to the cached file. Skips download if the file
/// already exists (by name — no checksum validation). Callers that need
/// freshness must clear the cache themselves.
pub async fn fetch_to_cache(
    store: &dyn ObjectStore,
    key: &str,
    cache_dir: &Path,
) -> Result<PathBuf> {
    let local_path = cache_path_for_key(key, cache_dir);

    if local_path.exists() {
        tracing::debug!(path = %local_path.display(), "using cached file");
        return Ok(local_path);
    }

    std::fs::create_dir_all(cache_dir)?;

    let obj_path = ObjectPath::from(key);
    let result = store.get(&obj_path).await?;
    let bytes = result.bytes().await?;

    std::fs::write(&local_path, &bytes)?;
    tracing::info!(
        key,
        path = %local_path.display(),
        bytes = bytes.len(),
        "fetched from storage to cache"
    );

    Ok(local_path)
}

/// Compute the local cache path for a storage key. Two keys that share a
/// basename (`tenant-a/data.parquet` and `tenant-b/data.parquet`) get
/// distinct cache files because the key itself feeds the hash prefix.
fn cache_path_for_key(key: &str, cache_dir: &Path) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    let hash = hasher.finish();

    let filename = key
        .rsplit('/')
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("blob");
    let filename = filename
        .chars()
        .map(|ch| match ch {
            ':' | '<' | '>' | '"' | '\\' | '|' | '?' | '*' => '_',
            _ if ch.is_ascii_control() => '_',
            _ => ch,
        })
        .collect::<String>();

    cache_dir.join(format!("{hash:016x}-{filename}"))
}

/// Write a Polars `DataFrame` as parquet to an object store.
pub async fn write_parquet_to_store(
    store: &dyn ObjectStore,
    key: &str,
    df: &mut DataFrame,
) -> Result<()> {
    let mut buf = Vec::new();
    ParquetWriter::new(&mut buf).finish(df)?;

    let obj_path = ObjectPath::from(key);
    let payload = crate::object_store::PutPayload::from(buf);
    store
        .put(&obj_path, payload)
        .await
        .map_err(|e| anyhow!("failed to write parquet to storage: {e}"))?;

    tracing::info!(key, "wrote parquet to storage");
    Ok(())
}

/// Load a parquet file from a local path into a Polars `LazyFrame`.
///
/// This is a sync helper — call it outside of an async context if Polars
/// uses tokio internally (avoids nested runtime panics).
pub fn scan_local_parquet(path: &Path) -> Result<LazyFrame> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow!("path is not valid utf-8: {}", path.display()))?;
    let pl_path = PlPath::from_str(path_str);
    Ok(LazyFrame::scan_parquet(pl_path, Default::default())?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::object_store::memory::InMemory;

    #[tokio::test]
    async fn roundtrip_parquet_through_store() {
        // Intent: a DataFrame written via `write_parquet_to_store` and read
        // back via `fetch_parquet` + `scan_local_parquet` must reproduce the
        // exact same rows and columns. This is the load-bearing replay
        // guarantee — if it ever fails, training reproducibility breaks for
        // every downstream consumer (crucible, mnemos persistence, fathom).
        let store = InMemory::new();
        let cache_dir = tempfile::tempdir().unwrap();

        let mut df = df! {
            "id" => &[1i64, 2, 3],
            "value" => &[10.0f64, 20.0, 30.0],
        }
        .unwrap();
        let original_height = df.height();
        let original_width = df.width();

        write_parquet_to_store(&store, "test/data.parquet", &mut df)
            .await
            .unwrap();

        let local_path = fetch_parquet(&store, "test/data.parquet", cache_dir.path())
            .await
            .unwrap();

        let result = tokio::task::spawn_blocking(move || {
            scan_local_parquet(&local_path).unwrap().collect().unwrap()
        })
        .await
        .unwrap();

        assert_eq!(
            result.height(),
            original_height,
            "row count must round-trip exactly — Polars↔Parquet drift would silently lose rows"
        );
        assert_eq!(
            result.width(),
            original_width,
            "column count must round-trip exactly — Polars↔Parquet drift would silently lose columns"
        );

        let id_col = result.column("id").unwrap();
        let value_col = result.column("value").unwrap();
        assert_eq!(
            id_col.i64().unwrap().to_vec(),
            vec![Some(1), Some(2), Some(3)],
            "`id` values must round-trip — type-loss here breaks audit replay"
        );
        let values: Vec<Option<f64>> = value_col.f64().unwrap().into_iter().collect();
        assert_eq!(
            values,
            vec![Some(10.0), Some(20.0), Some(30.0)],
            "`value` values must round-trip — type-loss here breaks audit replay"
        );
    }

    #[tokio::test]
    async fn fetch_uses_cache_on_second_call() {
        // Intent: a hot cache must short-circuit the second fetch — repeat
        // training runs and replay loops would melt bandwidth budgets if
        // every call hit the network. Returning the SAME path proves the
        // cache filename derivation is deterministic; an additional check
        // that the file exists guards against accidental cleanup paths.
        let store = InMemory::new();
        let cache_dir = tempfile::tempdir().unwrap();

        let obj_path = ObjectPath::from("cached.parquet");
        let payload = crate::object_store::PutPayload::from(vec![1u8, 2, 3]);
        store.put(&obj_path, payload).await.unwrap();

        let path1 = fetch_to_cache(&store, "cached.parquet", cache_dir.path())
            .await
            .unwrap();
        assert!(path1.exists(), "first fetch must materialize the file");

        let path2 = fetch_to_cache(&store, "cached.parquet", cache_dir.path())
            .await
            .unwrap();
        assert_eq!(
            path1, path2,
            "cache must be content-stable: same key returns same path on the hot path"
        );
    }

    #[tokio::test]
    async fn fetch_distinguishes_keys_with_same_basename() {
        // Intent: multi-tenant fan-in regularly produces sibling keys with
        // identical basenames (`tenant-a/data.parquet`,
        // `tenant-b/data.parquet`). If the cache keyed only on basename,
        // tenant A's bytes would be served to tenant B — a confidentiality
        // and correctness break. This test pins the key-prefixed hash
        // contract so a future "simplification" cannot regress it silently.
        let store = InMemory::new();
        let cache_dir = tempfile::tempdir().unwrap();

        let path_a = ObjectPath::from("tenant-a/data.parquet");
        let path_b = ObjectPath::from("tenant-b/data.parquet");

        store
            .put(
                &path_a,
                crate::object_store::PutPayload::from(vec![1u8, 2, 3]),
            )
            .await
            .unwrap();
        store
            .put(
                &path_b,
                crate::object_store::PutPayload::from(vec![4u8, 5, 6]),
            )
            .await
            .unwrap();

        let local_a = fetch_to_cache(&store, "tenant-a/data.parquet", cache_dir.path())
            .await
            .unwrap();
        let local_b = fetch_to_cache(&store, "tenant-b/data.parquet", cache_dir.path())
            .await
            .unwrap();

        assert_ne!(
            local_a, local_b,
            "tenants with same basename must NOT collide on the cache path"
        );
        assert_eq!(
            std::fs::read(local_a).unwrap(),
            vec![1u8, 2, 3],
            "tenant-a cache must hold tenant-a bytes — cross-tenant bleed is a P0"
        );
        assert_eq!(
            std::fs::read(local_b).unwrap(),
            vec![4u8, 5, 6],
            "tenant-b cache must hold tenant-b bytes — cross-tenant bleed is a P0"
        );
    }
}
