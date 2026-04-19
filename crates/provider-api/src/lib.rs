// Copyright 2024-2026 Reflective Labs

// SPDX-License-Identifier: MIT

//! # Converge Provider API
//!
//! This crate defines the provider-facing capability contract for Converge.
//! It is deliberately separate from pack authoring:
//!
//! - chat traits and request/response types live here
//! - [`Backend`] and [`BackendKind`] describe provider identity
//! - [`Capability`] describes what a provider can do
//! - [`BackendRequirements`] and [`BackendSelector`] drive routing
//! - [`BackendError`] is the generic provider error surface
//!
//! Suggestor and invariant authoring do not live here. Those contracts belong to
//! `converge-pack`.

pub mod backend;
pub mod capability;
pub mod chat;
pub mod error;
pub mod selection;

pub use backend::{Backend, BackendKind};
pub use capability::Capability;
pub use chat::{
    BoxFuture, ChatBackend, ChatMessage, ChatRequest, ChatResponse, ChatRole, DynChatBackend,
    FinishReason, LlmError, ResponseFormat, TokenUsage, ToolCall, ToolDefinition,
};
pub use error::{BackendError, BackendErrorKind};
pub use selection::{
    AgentRequirements, BackendRequirements, BackendSelector, ComplianceLevel, CostClass, CostTier,
    DataSovereignty, Jurisdiction, LatencyClass, ModelSelectorTrait, RequiredCapabilities,
    SelectionCriteria, TaskComplexity,
};
