// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Analytics and intelligence engine for Converge agents.
//!
//! Provides data processing with Polars and ML training/inference with Burn.
//!
//! - [`batch`]: Temporal feature extraction and z-score analysis
//! - [`engine`]: Analytics engine coordinator
//! - [`model`]: Burn-based ML model definitions
//! - [`training`]: Training pipeline with dataset, feature engineering, and model registry agents
//! - [`storage`]: Async I/O helpers (feature `storage`)

pub mod batch;
pub mod engine;
pub mod ingest;
pub mod model;
#[cfg(feature = "storage")]
pub mod storage;
pub mod training;
