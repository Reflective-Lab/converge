// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # converge-analytics
//!
//! ML and analytics capabilities as Suggestors for the Converge Engine.
//!
//! Access through [`FeatureAgent`] and the training pipeline agents.
//! Register them in a formation for data-driven convergence.
//!
//! ## Usage
//!
//! ```rust,ignore
//! use converge_analytics::FeatureAgent;
//!
//! engine.register_suggestor(FeatureAgent::new(config));
//! ```
//!
//! ## Available Suggestors
//!
//! - [`FeatureAgent`] — Polars-based feature extraction
//! - Training pipeline: DataValidation, FeatureEngineering, ModelTraining,
//!   ModelEvaluation, ModelRegistry, Monitoring, Deployment, etc.

// ── Public API: Suggestor interfaces ──────────────────────────────────

pub mod engine;
pub mod training;

pub use engine::FeatureAgent;

// ── Internal: data processing utilities ───────────────────────────────
// These are utility modules for feature agents to use internally.
// They will be wired into Suggestors as the analytics pipeline matures.

#[allow(dead_code)]
mod batch;
#[allow(dead_code)]
mod ingest;
#[allow(dead_code)]
mod model;
#[cfg(feature = "storage")]
#[allow(dead_code)]
mod storage;
