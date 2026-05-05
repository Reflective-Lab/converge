---
tags: [architecture]
source: mixed
---
# Transport Protocols

Converge supports three transport tiers for client-server communication. All are driving adapters on the left side of the [[Architecture/Hexagonal Architecture|hexagon]].

## Tier 1: gRPC Bidirectional Streaming (Primary)

**Protocol definition**: `schema/proto/converge.proto`
**Framework**: Tonic
**Protocols**: HTTP/2 (h2-grpc), HTTP/3 (h3-grpc)

The primary transport. Full duplex — clients send commands, servers push events, simultaneously.

### Client Messages

| Message | Purpose |
|---|---|
| `SubmitJobRequest` | Start a convergence run |
| `CancelJobRequest` | Cancel a running job |
| `SubmitObservationRequest` | Submit a human or system observation into the truth pipeline |
| `ApproveProposalRequest` | HITL approval |
| `RejectProposalRequest` | HITL rejection |
| `PauseRunRequest` / `ResumeRunRequest` | Flow control |
| `UpdateBudgetRequest` | Adjust budget mid-run |
| `SubscribeRequest` / `UnsubscribeRequest` | Event filtering |
| `ResumeFromSequenceRequest` | Reconnection |
| `Ping` | Heartbeat |

### Server Events

| Event | Purpose |
|---|---|
| `ContextEntry` | Fact, proposal, trace, or decision |
| `RunStatusChanged` | pending, running, converged, halted, waiting, cancelled |
| `JobCreated` / `JobCompleted` | Lifecycle |
| `Subscribed` / `ResumedFrom` | Stream control |
| `Ack` | Request acknowledgment |
| `Pong` | Heartbeat response |
| `Error` | Error reporting |

### Resume Support

Every event carries a monotonic sequence number. On reconnect, the client sends `ResumeFromSequenceRequest(last_seen)`. The server replays missed events or sends a snapshot if the gap is large. No events are lost.

## Tier 2: Server-Sent Events (Fallback)

**Endpoint**: `GET /api/v1/stream/events?job_id=xxx&since_seq=0`
**Content-Type**: `text/event-stream`
**Framework**: Axum

When gRPC is blocked (corporate firewalls, restricted networks), SSE provides one-way server-to-client streaming over plain HTTP.

| Event Type | Payload |
|---|---|
| `entry` | Context entry (mirrors gRPC `ContextEntry`) |
| `run_status` | Run state changes |
| `heartbeat` | Server timestamp |
| `halt` | Halt reason |
| `waiting` | Waiting for HITL |

Client commands go through REST endpoints alongside the SSE stream.

## Tier 3: REST + Polling (Degraded)

**Framework**: Axum
**OpenAPI**: `GET /api-docs/openapi.json`

When streaming is unavailable, clients poll:

```
GET  /api/v1/jobs/{job_id}
GET  /api/v1/events?since_sequence=N
POST /api/v1/jobs
```

Highest latency. Use only when both gRPC and SSE are blocked.

## WebSocket

WebSocket is used by the SurrealDB adapter (`surrealdb::engine::remote::ws::Ws`), not as a client-facing Converge transport. The gRPC bidirectional stream provides the same full-duplex semantics with better tooling.

## Capability Negotiation

On connect, clients call `GetCapabilities` to discover:
- Supported transports (h2-grpc, h3-grpc, websocket, sse)
- Resume support
- Available features

The client selects the best available transport automatically.

See also: [[Architecture/Hexagonal Architecture]], [[Architecture/Ports]]
