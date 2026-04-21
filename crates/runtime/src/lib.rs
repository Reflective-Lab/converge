// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Converge Runtime - Execution Service
//!
//! This crate provides the execution layer for Converge:
//! - HTTP/gRPC interfaces
//! - Job lifecycle management
//! - Template registry
//! - Observability
//!
//! # Architecture Role
//!
//! > `converge-runtime` owns **execution**, not **semantics**.
//!
//! Runtime is a thin execution boundary around domain + core.
//! It wires jobs, it doesn't define meaning.
//!
//! # Usage
//!
//! ```ignore
//! use converge_runtime::http::HttpServer;
//! use converge_runtime::state::AppState;
//! use converge_runtime::templates::TemplateRegistry;
//!
//! let templates = TemplateRegistry::with_defaults();
//! let state = AppState::with_templates(templates);
//! let server = HttpServer::new(config, state);
//! server.start().await?;
//! ```

// Scaffolding code - allow pedantic lints during development
#![allow(dead_code)]
#![allow(clippy::needless_for_each)]
#![allow(clippy::unused_self)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::unused_async)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::no_effect_underscore_binding)]

mod api;
pub mod error;
pub mod execution;
pub mod handlers;
pub mod pilot;
mod semantic;
pub mod sse;

// Public modules for use by converge-app
pub mod config;
pub mod http;
#[cfg(feature = "auth")]
pub mod http_auth;
pub mod state;
pub mod templates;

// Feature-gated modules
#[cfg(feature = "grpc")]
pub mod grpc;
#[cfg(feature = "grpc")]
pub mod ledger;

#[cfg(feature = "gcp")]
pub mod db;
#[cfg(feature = "gcp")]
pub mod gcp;

#[cfg(feature = "metrics")]
pub mod metrics;

#[cfg(feature = "telemetry")]
pub mod telemetry;

// Security modules
#[cfg(feature = "security")]
pub mod identity;

#[cfg(feature = "nats")]
pub mod nats;

#[cfg(feature = "auth")]
pub mod auth;

#[cfg(all(feature = "auth", feature = "grpc"))]
pub mod interceptor;

#[cfg(feature = "auth")]
pub mod policy;

#[cfg(feature = "nats")]
pub mod audit;

#[cfg(feature = "crypto")]
pub mod crypto;

#[cfg(feature = "auth")]
pub mod secrets;

#[cfg(feature = "wasm")]
pub mod wasm;
