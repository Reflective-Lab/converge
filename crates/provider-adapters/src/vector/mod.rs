// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Vector store implementations for Converge.
//!
//! Vector stores are **caches**, not authoritative state. They can be
//! rebuilt from the Context at any time. This follows Converge's principle
//! that vector stores expand what agents can *see*, not what they can *decide*.
//!
//! # Available Stores
//!
//! - [`InMemoryVectorStore`] - In-memory store for testing and small workloads
//! - `LanceStore` - `LanceDB` embedded store in the `manifold` extension
//! - `QdrantStore` - Qdrant distributed store (requires `qdrant` feature)
//!
//! # Example
//!
//! ```
//! use converge_provider_adapters::vector::InMemoryVectorStore;
//! use converge_core::capability::{VectorRecall, VectorRecord, VectorQuery};
//!
//! let store = InMemoryVectorStore::new();
//!
//! // Insert a record
//! store.upsert(&VectorRecord {
//!     id: "doc-1".into(),
//!     vector: vec![0.1, 0.2, 0.3],
//!     payload: serde_json::json!({"title": "Hello World"}),
//! }).unwrap();
//!
//! // Query similar vectors
//! let matches = store.query(&VectorQuery::new(vec![0.1, 0.2, 0.3], 10)).unwrap();
//! ```

mod memory;

pub use memory::InMemoryVectorStore;

#[cfg(feature = "qdrant")]
mod qdrant;

// QdrantStore will be re-exported once implemented in qdrant.rs

// Re-export core types for convenience
pub use converge_core::capability::{
    CapabilityError, VectorMatch, VectorQuery, VectorRecall, VectorRecord,
};

/// Computes cosine similarity between two vectors.
///
/// Returns a value between -1.0 and 1.0, where 1.0 means identical direction.
#[must_use]
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = (norm_a.sqrt() * norm_b.sqrt()).max(1e-8);
    dot / denom
}

/// Normalizes a vector to unit length.
#[must_use]
pub fn normalize(v: &[f32]) -> Vec<f32> {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm < 1e-8 {
        v.to_vec()
    } else {
        v.iter().map(|x| x / norm).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_identical() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.001);
    }

    #[test]
    fn cosine_similarity_opposite() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![-1.0, 0.0, 0.0];
        let sim = cosine_similarity(&a, &b);
        assert!((sim + 1.0).abs() < 0.001);
    }

    #[test]
    fn normalize_unit_vector() {
        let v = normalize(&[3.0, 4.0]);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.001);
    }
}
