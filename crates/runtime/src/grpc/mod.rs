// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! gRPC server implementation for the Converge Protocol.
//!
//! This module provides the gRPC server that mobile and CLI clients connect to.
//! It implements the Converge Protocol defined in `schema/proto/converge.proto`.
//!
//! # Architecture
//!
//! The Converge Protocol is stream-first:
//! - Clients open a bidirectional stream
//! - Clients send control messages (submit job, submit observation, approve, etc.)
//! - Server sends events (facts, proposals, traces, decisions, status)
//!
//! Unary RPCs are available as fallback for degraded network conditions.
//!
//! # Transport Priority
//!
//! 1. gRPC bidirectional streaming (primary)
//! 2. Server-Sent Events (fallback)
//! 3. REST + polling (degraded mode)
//!
//! # Usage
//!
//! ```ignore
//! use converge_runtime::grpc::GrpcServer;
//! use converge_runtime::config::GrpcConfig;
//!
//! let config = GrpcConfig::default();
//! let server = GrpcServer::new(config);
//! server.start().await?;
//! ```

#[allow(clippy::result_large_err)] // tonic::Status is the idiomatic gRPC error type
mod server;

pub use server::GrpcServer;

// Generated protobuf types from the canonical shared protocol crate
#[allow(clippy::all)]
#[allow(clippy::pedantic)]
#[allow(clippy::nursery)]
pub mod generated {
    pub use converge_protocol::v1::*;
}

// Re-export commonly used types
pub use generated::{
    // Ack/Error
    Ack,
    // Core types
    Actor,
    ActorType,
    // Run control
    ApproveProposalRequest,
    // Job management
    Budget,
    CancelJobRequest,
    // Stream messages
    ClientMessage,
    ContextEntry,
    // Run lifecycle
    ConvergedInfo,
    EntryType,

    Error,

    // Capabilities
    GetCapabilitiesRequest,
    GetCapabilitiesResponse,
    // Polling fallback
    GetEventsRequest,
    GetEventsResponse,

    GetJobRequest,
    GetJobResponse,
    HaltInfo,
    JobCompleted,
    JobCreated,
    PackInfo,
    PauseRunRequest,
    // Heartbeat
    Ping,
    Pong,

    RejectProposalRequest,
    // Stream control
    ResumeFromSequenceRequest,
    ResumeRunRequest,
    ResumedFrom,
    RunStatus,
    RunStatusChanged,
    SeedFact,
    ServerEvent,

    StreamingCapabilities,
    SubmitJobRequest,
    SubmitJobResponse,

    SubmitObservationRequest,
    SubscribeRequest,
    Subscribed,
    TransportInfo,
    TruthInfo,

    UnsubscribeRequest,

    UpdateBudgetRequest,

    WaitingInfo,

    // Service traits
    converge_service_client::ConvergeServiceClient,
    converge_service_server::{ConvergeService, ConvergeServiceServer},
};
