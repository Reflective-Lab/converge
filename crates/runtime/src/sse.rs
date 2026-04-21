// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Server-Sent Events (SSE) streaming endpoints.
//!
//! This module provides SSE fallback for clients that cannot use gRPC streaming.
//! It implements the same Converge Protocol semantics as the gRPC Stream RPC.
//!
//! # Transport Priority
//!
//! 1. gRPC bidirectional streaming (primary)
//! 2. Server-Sent Events (this module) - fallback for restricted networks
//! 3. REST + polling (degraded mode)
//!
//! # Usage
//!
//! ```text
//! GET /api/v1/stream/events?job_id=xxx&since_seq=0
//! Accept: text/event-stream
//!
//! Event stream:
//! event: entry
//! data: {"entry_id":"...","entry_type":"fact",...}
//!
//! event: run_status
//! data: {"run_id":"...","status":"running",...}
//!
//! event: heartbeat
//! data: {"server_time_ns":...}
//! ```

use std::convert::Infallible;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::{
    Router,
    extract::{Query, State},
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
};
use futures::StreamExt as FuturesStreamExt;
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use tokio::time::interval;
use tracing::info;

use crate::state::AppState;

// =============================================================================
// SSE Event Types (mirror gRPC ServerEvent)
// =============================================================================

/// SSE event for context entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseContextEntry {
    pub entry_id: String,
    pub entry_type: String, // fact, proposal, trace, decision
    pub correlation_id: String,
    pub run_id: String,
    pub job_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub truth_id: Option<String>,
    pub actor: SseActor,
    pub sequence: u64,
    pub timestamp_ns: u64,
    pub context_version: u64,
    pub payload: serde_json::Value,
}

/// SSE event for actor information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseActor {
    #[serde(rename = "type")]
    pub actor_type: String, // user, agent, system, external
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

/// SSE event for run status changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseRunStatus {
    pub run_id: String,
    pub job_id: String,
    pub status: String, // pending, running, converged, halted, waiting, cancelled
    pub previous_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub halt_info: Option<SseHaltInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_info: Option<SseWaitingInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub converged_info: Option<SseConvergedInfo>,
    pub facts_count: u32,
    pub pending_proposals: u32,
    pub cycles_completed: u32,
}

/// SSE event for halt information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseHaltInfo {
    pub truth_id: String,
    pub reason: String,
    pub can_restart: bool,
    pub break_glass_available: bool,
}

/// SSE event for waiting information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseWaitingInfo {
    pub waiting_for: Vec<String>,
    pub actor_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ns: Option<u64>,
}

/// SSE event for convergence information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseConvergedInfo {
    pub duration_ms: u64,
    pub total_facts: u32,
    pub total_cycles: u32,
}

/// SSE event for job lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseJobEvent {
    pub job_id: String,
    pub run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blueprint_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub final_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_facts: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_cycles: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

/// SSE heartbeat event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseHeartbeat {
    pub server_time_ns: u64,
    pub sequence: u64,
}

/// SSE error event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

// =============================================================================
// Query Parameters
// =============================================================================

/// Query parameters for event stream subscription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamQuery {
    /// Job ID to subscribe to.
    #[serde(default)]
    pub job_id: Option<String>,

    /// Run ID to subscribe to.
    #[serde(default)]
    pub run_id: Option<String>,

    /// Correlation ID to subscribe to.
    #[serde(default)]
    pub correlation_id: Option<String>,

    /// Resume from this sequence (0 = from now).
    #[serde(default)]
    pub since_seq: u64,

    /// Filter by entry types (comma-separated: fact,proposal,trace,decision).
    #[serde(default)]
    pub entry_types: Option<String>,
}

/// Query parameters for capabilities endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesQuery {
    /// Device ID.
    #[serde(default)]
    pub device_id: Option<String>,

    /// App version.
    #[serde(default)]
    pub app_version: Option<String>,

    /// Platform (ios, android, cli, web).
    #[serde(default)]
    pub platform: Option<String>,
}

// =============================================================================
// SSE State
// =============================================================================

/// Shared state for SSE endpoints.
#[derive(Debug, Clone)]
pub struct SseState {
    /// Monotonic sequence counter.
    sequence: Arc<AtomicU64>,
}

impl Default for SseState {
    fn default() -> Self {
        Self::new()
    }
}

impl SseState {
    pub fn new() -> Self {
        Self {
            sequence: Arc::new(AtomicU64::new(0)),
        }
    }

    fn next_sequence(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::SeqCst)
    }

    fn now_ns() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }
}

// =============================================================================
// SSE Endpoints
// =============================================================================

/// Stream events for a job/run.
///
/// This is the primary SSE endpoint. Clients subscribe to events and receive
/// them as they occur. Supports resume from a specific sequence number.
///
/// # Query Parameters
///
/// - `job_id`: Job ID to subscribe to
/// - `run_id`: Run ID to subscribe to
/// - `correlation_id`: Correlation ID to subscribe to
/// - `since_seq`: Resume from this sequence (0 = from now)
/// - `entry_types`: Filter by entry types (comma-separated)
///
/// # Events
///
/// - `connected`: Initial connection confirmation
/// - `entry`: Context entry (fact, proposal, trace, decision)
/// - `run_status`: Run status change
/// - `job_created`: Job created
/// - `job_completed`: Job completed
/// - `heartbeat`: Keep-alive with server time
/// - `error`: Error notification
pub async fn stream_events(
    Query(query): Query<StreamQuery>,
    State(sse_state): State<SseState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    info!(
        job_id = ?query.job_id,
        run_id = ?query.run_id,
        since_seq = query.since_seq,
        "SSE stream started"
    );

    // Create the event stream
    let stream = create_event_stream(sse_state, query);

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    )
}

/// Create the SSE event stream.
fn create_event_stream(
    sse_state: SseState,
    query: StreamQuery,
) -> impl Stream<Item = Result<Event, Infallible>> {
    // Initial connection event
    let initial_seq = sse_state.next_sequence();
    let connected_event = Event::default()
        .event("connected")
        .json_data(serde_json::json!({
            "subscription_id": format!("sse_{}", initial_seq),
            "current_sequence": initial_seq,
            "job_id": query.job_id,
            "run_id": query.run_id,
            "correlation_id": query.correlation_id,
            "server_time_ns": SseState::now_ns(),
        }))
        .unwrap();

    // Create a heartbeat stream that emits every 30 seconds
    let heartbeat_state = sse_state.clone();
    let heartbeat_stream =
        stream::unfold(interval(Duration::from_secs(30)), move |mut interval| {
            let state = heartbeat_state.clone();
            async move {
                interval.tick().await;
                let seq = state.next_sequence();
                let event = Event::default()
                    .event("heartbeat")
                    .json_data(SseHeartbeat {
                        server_time_ns: SseState::now_ns(),
                        sequence: seq,
                    })
                    .unwrap();
                Some((Ok(event), interval))
            }
        });

    // TODO: In a real implementation, this would:
    // 1. Look up the job/run in storage
    // 2. Subscribe to the event bus for real-time updates
    // 3. Replay events since `since_seq` if resuming
    // 4. Stream actual context entries as they arrive

    // For now, emit a demo run status event after connection
    let demo_state = sse_state.clone();
    let job_id = query
        .job_id
        .clone()
        .unwrap_or_else(|| "demo_job".to_string());
    let run_id = query
        .run_id
        .clone()
        .unwrap_or_else(|| "demo_run".to_string());

    let demo_events = stream::once(async move {
        // Small delay to simulate initial event
        tokio::time::sleep(Duration::from_millis(100)).await;

        let seq = demo_state.next_sequence();
        let event = Event::default()
            .event("run_status")
            .id(seq.to_string())
            .json_data(SseRunStatus {
                run_id,
                job_id,
                status: "running".to_string(),
                previous_status: "pending".to_string(),
                halt_info: None,
                waiting_info: None,
                converged_info: None,
                facts_count: 0,
                pending_proposals: 0,
                cycles_completed: 0,
            })
            .unwrap();
        Ok(event)
    });

    // Combine streams: initial + demo + heartbeat
    FuturesStreamExt::chain(
        FuturesStreamExt::chain(stream::once(async { Ok(connected_event) }), demo_events),
        heartbeat_stream,
    )
}

/// Get streaming capabilities.
///
/// Returns information about available streaming transports and features.
/// This mirrors the gRPC GetCapabilities RPC.
pub async fn get_capabilities(
    Query(query): Query<CapabilitiesQuery>,
) -> axum::Json<serde_json::Value> {
    info!(
        device_id = ?query.device_id,
        platform = ?query.platform,
        "SSE capabilities request"
    );

    axum::Json(serde_json::json!({
        "packs": [],
        "active_truths": [],
        "streaming": {
            "transports": [
                { "type": "h2-grpc", "status": "supported" },
                { "type": "sse", "status": "supported" },
                { "type": "websocket", "status": "planned" }
            ],
            "default_transport": "h2-grpc",
            "resume_supported": true,
            "max_resume_gap": 10000
        },
        "determinism_mode_available": true,
        "server_version": env!("CARGO_PKG_VERSION"),
        "server_time_ns": SseState::now_ns()
    }))
}

/// Connection state endpoint.
///
/// Returns the current connection state for a client. Useful for
/// clients to check if they should reconnect.
pub async fn connection_state(State(sse_state): State<SseState>) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "connected",
        "transport": "sse",
        "current_sequence": sse_state.sequence.load(Ordering::SeqCst),
        "server_time_ns": SseState::now_ns()
    }))
}

// =============================================================================
// Control Endpoints (SSE is unidirectional, so control uses REST)
// =============================================================================

/// Request body for submitting an observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmitObservationRequest {
    pub run_id: String,
    pub key: String,
    pub payload: serde_json::Value,
    #[serde(default)]
    pub target_truth_id: Option<String>,
    pub idempotency_key: String,
}

/// Request body for approving a proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApproveProposalRequest {
    pub run_id: String,
    pub proposal_id: String,
    #[serde(default)]
    pub comment: Option<String>,
}

/// Request body for rejecting a proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectProposalRequest {
    pub run_id: String,
    pub proposal_id: String,
    pub reason: String,
}

/// Submit an observation to a running context.
///
/// SSE equivalent of the gRPC SubmitObservation control message.
pub async fn submit_observation(
    State(sse_state): State<SseState>,
    axum::Json(request): axum::Json<SubmitObservationRequest>,
) -> axum::Json<serde_json::Value> {
    let seq = sse_state.next_sequence();

    info!(
        run_id = %request.run_id,
        key = %request.key,
        idempotency_key = %request.idempotency_key,
        "Submitting observation via SSE control"
    );

    // TODO: Actually stage the observation into the running context

    axum::Json(serde_json::json!({
        "success": true,
        "sequence": seq,
        "entry_id": format!("observation_{}", uuid::Uuid::new_v4())
    }))
}

/// Approve a pending proposal.
///
/// SSE equivalent of the gRPC ApproveProposal control message.
pub async fn approve_proposal(
    State(sse_state): State<SseState>,
    axum::Json(request): axum::Json<ApproveProposalRequest>,
) -> axum::Json<serde_json::Value> {
    let seq = sse_state.next_sequence();

    info!(
        run_id = %request.run_id,
        proposal_id = %request.proposal_id,
        "Approving proposal via SSE control"
    );

    // TODO: Actually approve the proposal

    axum::Json(serde_json::json!({
        "success": true,
        "sequence": seq
    }))
}

/// Reject a pending proposal.
///
/// SSE equivalent of the gRPC RejectProposal control message.
pub async fn reject_proposal(
    State(sse_state): State<SseState>,
    axum::Json(request): axum::Json<RejectProposalRequest>,
) -> axum::Json<serde_json::Value> {
    let seq = sse_state.next_sequence();

    info!(
        run_id = %request.run_id,
        proposal_id = %request.proposal_id,
        reason = %request.reason,
        "Rejecting proposal via SSE control"
    );

    // TODO: Actually reject the proposal

    axum::Json(serde_json::json!({
        "success": true,
        "sequence": seq
    }))
}

// =============================================================================
// Streaming Job Execution
// =============================================================================

/// Request to execute a job with SSE streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingJobRequest {
    /// Pack ID to use.
    pub pack: String,
    /// Seed facts.
    #[serde(default)]
    pub seeds: Vec<SeedFactRequest>,
    /// Budget overrides.
    pub budget: Option<BudgetRequest>,
    /// Whether to use real LLM providers (requires API keys).
    #[serde(default)]
    pub use_llm: bool,
}

/// Seed fact in request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeedFactRequest {
    pub id: String,
    pub content: String,
}

/// Budget overrides in request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetRequest {
    #[serde(default = "default_max_cycles")]
    pub max_cycles: u32,
    #[serde(default = "default_max_facts")]
    pub max_facts: u32,
}

fn default_max_cycles() -> u32 {
    100
}
fn default_max_facts() -> u32 {
    500
}

/// Source reference for ask streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskSourceRequest {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

/// Execute a grounded ask request with SSE streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskStreamingRequest {
    pub question: String,
    #[serde(default)]
    pub sources: Vec<AskSourceRequest>,
    #[serde(default)]
    pub use_llm: bool,
    pub budget: Option<BudgetRequest>,
}

fn build_streaming_job_stream(
    pack_id: String,
    seeds: Vec<crate::templates::SeedFact>,
    budget: converge_core::Budget,
    use_llm: bool,
) -> impl Stream<Item = Result<Event, Infallible>> {
    use crate::execution::{JobExecutor, StreamingEvent};

    // Create channel for events
    let (tx, mut rx) = tokio::sync::mpsc::channel::<StreamingEvent>(256);

    // Spawn execution in background task
    tokio::spawn({
        let pack_id = pack_id.clone();
        let seeds = seeds.clone();
        async move {
            let mut builder = JobExecutor::builder()
                .with_pack(&pack_id)
                .with_seeds(seeds)
                .with_budget(budget)
                .with_streaming(256);

            // Use real LLM if requested, otherwise use mock
            if use_llm {
                builder = builder.with_real_llm();
            } else {
                builder = builder.with_mock_llm();
            }

            let (handle, mut event_rx) = builder
                .execute_with_streaming()
                .expect("Failed to start streaming job");

            // Forward events from the execution to our channel.
            while let Some(event) = event_rx.recv().await {
                if tx.send(event).await.is_err() {
                    break; // Client disconnected
                }
            }

            // Wait for execution to complete
            let _ = handle.await;
        }
    });

    async_stream::stream! {
        // Initial job started event
        let job_id = format!("job_{}", uuid::Uuid::new_v4());
        let started_event = Event::default()
            .event("job_started")
            .json_data(serde_json::json!({
                "job_id": job_id,
                "pack": pack_id,
                "timestamp_ns": SseState::now_ns(),
            }))
            .unwrap();
        yield Ok(started_event);

        // Stream events from the execution
        while let Some(event) = rx.recv().await {
            let sse_event = match &event {
                StreamingEvent::Fact { sequence, cycle, key, id, content, timestamp_ns } => {
                    Event::default()
                        .event("fact")
                        .id(sequence.to_string())
                        .json_data(serde_json::json!({
                            "sequence": sequence,
                            "cycle": cycle,
                            "key": key,
                            "id": id,
                            "content": content,
                            "timestamp_ns": timestamp_ns,
                        }))
                        .unwrap()
                }
                StreamingEvent::CycleStart { sequence, cycle, timestamp_ns } => {
                    Event::default()
                        .event("cycle_start")
                        .id(sequence.to_string())
                        .json_data(serde_json::json!({
                            "sequence": sequence,
                            "cycle": cycle,
                            "timestamp_ns": timestamp_ns,
                        }))
                        .unwrap()
                }
                StreamingEvent::CycleEnd { sequence, cycle, facts_added, timestamp_ns } => {
                    Event::default()
                        .event("cycle_end")
                        .id(sequence.to_string())
                        .json_data(serde_json::json!({
                            "sequence": sequence,
                            "cycle": cycle,
                            "facts_added": facts_added,
                            "timestamp_ns": timestamp_ns,
                        }))
                        .unwrap()
                }
                StreamingEvent::Converged { sequence, cycles, total_facts, duration_ms, timestamp_ns } => {
                    Event::default()
                        .event("converged")
                        .id(sequence.to_string())
                        .json_data(serde_json::json!({
                            "sequence": sequence,
                            "cycles": cycles,
                            "total_facts": total_facts,
                            "duration_ms": duration_ms,
                            "timestamp_ns": timestamp_ns,
                        }))
                        .unwrap()
                }
                StreamingEvent::Halted { sequence, cycles, reason, timestamp_ns } => {
                    Event::default()
                        .event("halted")
                        .id(sequence.to_string())
                        .json_data(serde_json::json!({
                            "sequence": sequence,
                            "cycles": cycles,
                            "reason": reason,
                            "timestamp_ns": timestamp_ns,
                        }))
                        .unwrap()
                }
            };
            yield Ok(sse_event);
        }
    }
}

/// Execute a job with SSE streaming.
///
/// This endpoint runs a convergence job and streams events in real-time
/// as facts are produced. Unlike the regular job endpoint, this provides
/// live visibility into the convergence process.
///
/// # Request Body
///
/// ```json
/// {
///   "pack": "growth-strategy",
///   "seeds": [
///     { "id": "market:nordic", "content": "Nordic B2B market" }
///   ],
///   "budget": { "max_cycles": 50, "max_facts": 200 }
/// }
/// ```
///
/// # Events
///
/// - `job_started`: Job execution started
/// - `cycle_start`: Cycle started
/// - `fact`: New fact produced
/// - `cycle_end`: Cycle ended
/// - `converged`: Job converged successfully
/// - `halted`: Job halted due to error
pub async fn execute_streaming_job(
    axum::Json(request): axum::Json<StreamingJobRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    use converge_core::Budget;

    info!(
        pack = %request.pack,
        seeds = request.seeds.len(),
        "Starting streaming job execution"
    );

    let pack_id = request.pack.clone();
    let seeds: Vec<crate::templates::SeedFact> = request
        .seeds
        .iter()
        .map(|s| crate::templates::SeedFact {
            id: s.id.clone().into(),
            content: s.content.clone(),
        })
        .collect();
    let budget = request
        .budget
        .map(|b| Budget {
            max_cycles: b.max_cycles,
            max_facts: b.max_facts,
        })
        .unwrap_or_default();
    let use_llm = request.use_llm;

    let stream = build_streaming_job_stream(pack_id, seeds, budget, use_llm);

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    )
}

/// Execute a grounded ask request with SSE streaming.
pub async fn execute_ask_stream(
    axum::Json(request): axum::Json<AskStreamingRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    if request.question.trim().is_empty() {
        let stream = stream::once(async {
            Ok(Event::default()
                .event("error")
                .json_data(SseError {
                    code: "invalid_request".to_string(),
                    message: "question is required".to_string(),
                    recoverable: false,
                })
                .unwrap())
        })
        .boxed();
        return Sse::new(stream).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("heartbeat"),
        );
    }

    if request.sources.is_empty() {
        let stream = stream::once(async {
            Ok(Event::default()
                .event("error")
                .json_data(SseError {
                    code: "missing_sources".to_string(),
                    message: "at least one source is required".to_string(),
                    recoverable: false,
                })
                .unwrap())
        })
        .boxed();
        return Sse::new(stream).keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("heartbeat"),
        );
    }

    let mut seeds = Vec::with_capacity(1 + request.sources.len());
    seeds.push(crate::templates::SeedFact {
        id: "ask:question".into(),
        content: request.question.clone(),
    });

    for (index, source) in request.sources.iter().enumerate() {
        let source_id = if source.id.trim().is_empty() {
            format!("source-{}", index + 1)
        } else {
            source.id.clone()
        };
        let payload = serde_json::json!({
            "id": source_id,
            "title": source.title,
            "url": source.url,
            "content": source.content,
        });
        seeds.push(crate::templates::SeedFact {
            id: format!("ask:source:{}", index + 1).into(),
            content: payload.to_string(),
        });
    }

    let budget = request
        .budget
        .map(|b| converge_core::Budget {
            max_cycles: b.max_cycles,
            max_facts: b.max_facts,
        })
        .unwrap_or_default();

    let stream =
        build_streaming_job_stream("ask-converge".to_string(), seeds, budget, request.use_llm)
            .boxed();

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("heartbeat"),
    )
}

// =============================================================================
// Router
// =============================================================================

/// Build the SSE router.
///
/// # Endpoints
///
/// Streaming (GET with Accept: text/event-stream):
/// - `GET /api/v1/stream/events` - Subscribe to events
/// - `GET /api/v1/stream/capabilities` - Get streaming capabilities
/// - `GET /api/v1/stream/state` - Get connection state
///
/// Streaming Job Execution:
/// - `POST /api/v1/stream/jobs` - Execute a job with real-time SSE streaming
///
/// Ask (grounded Q&A):
/// - `POST /api/v1/ask` - Execute ask-converge with SSE streaming
///
/// Control (POST):
/// - `POST /api/v1/stream/observations` - Submit an observation
/// - `POST /api/v1/stream/approve` - Approve a proposal
/// - `POST /api/v1/stream/reject` - Reject a proposal
pub fn router() -> Router<AppState> {
    let sse_state = SseState::new();

    Router::new()
        // Streaming endpoints
        .route("/api/v1/stream/events", get(stream_events))
        .route("/api/v1/stream/capabilities", get(get_capabilities))
        .route("/api/v1/stream/state", get(connection_state))
        // Streaming job execution (real-time fact streaming)
        .route(
            "/api/v1/stream/jobs",
            axum::routing::post(execute_streaming_job),
        )
        .route("/api/v1/ask", axum::routing::post(execute_ask_stream))
        // Control endpoints (REST, since SSE is unidirectional)
        .route(
            "/api/v1/stream/observations",
            axum::routing::post(submit_observation),
        )
        .route(
            "/api/v1/stream/approve",
            axum::routing::post(approve_proposal),
        )
        .route(
            "/api/v1/stream/reject",
            axum::routing::post(reject_proposal),
        )
        .with_state(sse_state)
}
