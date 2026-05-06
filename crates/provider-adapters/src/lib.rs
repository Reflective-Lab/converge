// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

// Pedantic clippy lints — allow stylistic ones that don't improve correctness
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::struct_excessive_bools)]
#![allow(clippy::doc_link_with_quotes)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::unused_self)]
#![allow(clippy::needless_pass_by_value)]

//! Temporary capability adapter staging for the Converge runtime.
//!
//! > **Providers produce observations, never decisions.**
//! > **Converge converges; providers adapt.**
//!
//! This crate only contains adapter families that have not yet moved to their
//! extension repository. The LLM chat adapters and model-selection catalog now
//! live in Manifold. Converge keeps the provider contracts in
//! `converge-provider`; implementation crates import those contracts.
//!
//! # What Is a Provider?
//!
//! A provider is an **adapter** that:
//! - Implements capability traits (`Embedding`, `VectorRecall`, search, feed, etc.)
//! - Returns observations (not facts, not decisions)
//! - Includes provenance metadata for tracing
//! - Is stateless (no hidden lifecycle state)
//!
//! A provider is **NOT**:
//! - An agent (agents live in `converge-core`)
//! - Orchestration (no workflows, no scheduling)
//! - Domain logic (business rules live downstream in packs and products)
//!
// Secret management (SecretProvider trait, EnvSecretProvider default)
pub mod secret;

// Core contract types
pub mod contract;

// Capability registry
mod capability_registry;

// Search providers
#[cfg(feature = "brave")]
pub mod brave;
#[cfg(feature = "feed")]
pub mod feed;
#[cfg(feature = "_http")]
pub mod fetch;
pub mod search;
#[cfg(feature = "tavily")]
pub mod tavily;

// Tool integration (MCP, OpenAPI, GraphQL)
pub mod tools;

// Capability providers
#[cfg(feature = "_http")]
pub mod embedding;
#[cfg(feature = "_http")]
pub mod reranker;
pub mod vector;

// Re-exports: capability registry
pub use capability_registry::{
    CapabilityRegistry, CapabilityRequirements, SearchProviderMeta, WebSearchRequirements,
};

// Re-exports: secret management
pub use secret::{
    EnvSecretProvider, SecretError, SecretProvider, SecretString, StaticSecretProvider,
};

// Re-exports: search providers
#[cfg(feature = "brave")]
pub use brave::{
    BraveCapability, BraveSearchError, BraveSearchProvider, BraveSearchRequest,
    BraveSearchResponse, BraveSearchResult,
};
#[cfg(feature = "feed")]
pub use feed::{
    FeedByteLimit, FeedCandidateLimit, FeedDiscoverySource, FeedEndpointCandidate, FeedError,
    FeedFetchBackend, FeedFetchRequest, FeedFetchResponse, FeedFormat, FeedItem, FeedProbeRequest,
    FeedProbeResponse, FeedTimeoutMs, FeedUrl, HttpFeedProvider, HttpStatusCode,
};
#[cfg(feature = "_http")]
pub use fetch::HttpFetchProvider;
pub use search::{
    SearchDepth, SearchTopic, WebFetchBackend, WebFetchByteLimit, WebFetchError, WebFetchRequest,
    WebFetchResponse, WebFetchTimeoutMs, WebFetchUrl, WebSearchBackend, WebSearchError,
    WebSearchImage, WebSearchRequest, WebSearchResponse, WebSearchResult,
};
#[cfg(feature = "tavily")]
pub use tavily::TavilySearchProvider;

// Re-exports: contract types
pub use contract::{
    CallTimer, Capability, ProviderCallContext, ProviderMeta, ProviderObservation, Region,
    TokenUsage, canonical_hash,
};

// Re-exports: tool integration
pub use tools::{
    GraphQlConfig, GraphQlConverter, GraphQlOperationType, InlineToolConfig, InputSchema,
    OpenApiConfig, OpenApiConverter, SourceFilter, ToolCall, ToolDefinition, ToolError,
    ToolErrorKind, ToolFormat, ToolHandler, ToolRegistry, ToolResult, ToolResultContent,
    ToolSource, ToolsConfig, ToolsConfigError,
};
