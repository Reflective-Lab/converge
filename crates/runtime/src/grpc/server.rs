// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! gRPC server implementation with optional mTLS support.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::sync::mpsc;
use tokio_stream::Stream;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status, Streaming};

use crate::config::GrpcConfig;
use crate::error::RuntimeError;

#[cfg(feature = "security")]
use crate::config::SecurityConfig;
#[cfg(feature = "security")]
use crate::identity::{FileIdentity, FileIdentityConfig, Identity};

use super::generated::{
    Ack, ClientMessage, GetCapabilitiesRequest, GetCapabilitiesResponse, GetEventsRequest,
    GetEventsResponse, GetJobRequest, GetJobResponse, Pong, ServerEvent, StreamingCapabilities,
    SubmitJobRequest, SubmitJobResponse, TransportInfo,
    converge_service_server::{ConvergeService, ConvergeServiceServer},
};

/// gRPC server for the Converge Protocol.
pub struct GrpcServer {
    config: GrpcConfig,
    #[cfg(feature = "security")]
    security_config: Option<SecurityConfig>,
}

impl GrpcServer {
    /// Create a new gRPC server without TLS.
    pub fn new(config: GrpcConfig) -> Self {
        Self {
            config,
            #[cfg(feature = "security")]
            security_config: None,
        }
    }

    /// Create a new gRPC server with security configuration.
    #[cfg(feature = "security")]
    pub fn with_security(config: GrpcConfig, security_config: SecurityConfig) -> Self {
        Self {
            config,
            security_config: Some(security_config),
        }
    }

    /// Start the gRPC server.
    pub async fn start(self) -> Result<(), RuntimeError> {
        let addr = self.config.bind;
        let service = ConvergeServiceImpl::new();

        #[cfg(feature = "security")]
        if let Some(security_config) = self.security_config.take() {
            return Self::start_with_tls(addr, service, &security_config).await;
        }

        tracing::info!(%addr, tls = false, "Starting gRPC server");

        tonic::transport::Server::builder()
            .add_service(ConvergeServiceServer::new(service))
            .serve(addr)
            .await
            .map_err(|e| RuntimeError::Io(std::io::Error::other(e)))?;

        Ok(())
    }

    /// Start the gRPC server with mTLS.
    #[cfg(feature = "security")]
    async fn start_with_tls(
        addr: std::net::SocketAddr,
        service: ConvergeServiceImpl,
        security_config: &SecurityConfig,
    ) -> Result<(), RuntimeError> {
        use tonic::transport::{Certificate, Identity as TonicIdentity, Server, ServerTlsConfig};

        // Load identity from file
        let cert_path = security_config
            .cert_path
            .as_ref()
            .ok_or_else(|| RuntimeError::Config("security.cert_path required for TLS".into()))?;
        let key_path = security_config
            .key_path
            .as_ref()
            .ok_or_else(|| RuntimeError::Config("security.key_path required for TLS".into()))?;
        let ca_path = security_config
            .ca_path
            .as_ref()
            .ok_or_else(|| RuntimeError::Config("security.ca_path required for TLS".into()))?;

        let identity_config = FileIdentityConfig::new(cert_path, key_path, ca_path);
        let identity = FileIdentity::new(identity_config)
            .map_err(|e| RuntimeError::Config(format!("Failed to load identity: {e}")))?;

        tracing::info!(
            %addr,
            service_id = %identity.service_id(),
            tls = true,
            "Starting gRPC server with mTLS"
        );

        // Read cert and key files for tonic
        let cert_pem = std::fs::read_to_string(cert_path)
            .map_err(|e| RuntimeError::Config(format!("Failed to read cert: {e}")))?;
        let key_pem = std::fs::read_to_string(key_path)
            .map_err(|e| RuntimeError::Config(format!("Failed to read key: {e}")))?;
        let ca_pem = std::fs::read_to_string(ca_path)
            .map_err(|e| RuntimeError::Config(format!("Failed to read CA: {e}")))?;

        let server_identity = TonicIdentity::from_pem(&cert_pem, &key_pem);
        let client_ca = Certificate::from_pem(&ca_pem);

        let tls_config = ServerTlsConfig::new()
            .identity(server_identity)
            .client_ca_root(client_ca);

        Server::builder()
            .tls_config(tls_config)
            .map_err(|e| RuntimeError::Config(format!("TLS config error: {e}")))?
            .add_service(ConvergeServiceServer::new(service))
            .serve(addr)
            .await
            .map_err(|e| RuntimeError::Io(std::io::Error::other(e)))?;

        Ok(())
    }
}

/// Implementation of the ConvergeService gRPC service.
struct ConvergeServiceImpl {
    /// Monotonic sequence counter for events.
    sequence: AtomicU64,
}

impl ConvergeServiceImpl {
    fn new() -> Self {
        Self {
            sequence: AtomicU64::new(0),
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

    #[cfg(feature = "auth")]
    fn extract_bearer_token<T>(request: &Request<T>) -> Result<Option<String>, Status> {
        let auth_header = match request.metadata().get("authorization") {
            Some(value) => value,
            None => return Ok(None),
        };

        let auth_value = auth_header
            .to_str()
            .map_err(|_| Status::unauthenticated("Invalid authorization header encoding"))?;

        let token = auth_value
            .strip_prefix("Bearer ")
            .or_else(|| auth_value.strip_prefix("bearer "))
            .ok_or_else(|| Status::unauthenticated("Authorization header must be Bearer token"))?;

        Ok(Some(token.to_string()))
    }

    #[cfg(feature = "auth")]
    async fn authenticate(method: &str, bearer_token: Option<String>) -> Result<(), Status> {
        if method == "/converge.ConvergeService/GetCapabilities" {
            return Ok(());
        }

        let token = bearer_token.ok_or_else(|| Status::unauthenticated("Missing bearer token"))?;

        crate::http_auth::validate_token(&token)
            .await
            .map(|_| ())
            .map_err(|err| match err {
                RuntimeError::Authentication(message) => Status::unauthenticated(message),
                RuntimeError::Config(message) => Status::failed_precondition(message),
                _ => Status::internal("Authentication failed"),
            })
    }
}

#[tonic::async_trait]
impl ConvergeService for ConvergeServiceImpl {
    type StreamStream = Pin<Box<dyn Stream<Item = Result<ServerEvent, Status>> + Send + 'static>>;

    /// Bidirectional stream for live convergence.
    async fn stream(
        &self,
        request: Request<Streaming<ClientMessage>>,
    ) -> Result<Response<Self::StreamStream>, Status> {
        #[cfg(feature = "auth")]
        Self::authenticate(
            "/converge.ConvergeService/Stream",
            Self::extract_bearer_token(&request)?,
        )
        .await?;

        let mut in_stream = request.into_inner();
        let (tx, rx) = mpsc::channel(128);

        // Clone sequence counter for the spawned task
        let sequence = Arc::new(AtomicU64::new(self.sequence.load(Ordering::SeqCst)));

        tokio::spawn(async move {
            while let Ok(Some(msg)) = in_stream.message().await {
                let seq = sequence.fetch_add(1, Ordering::SeqCst);

                // Handle client messages
                let event = match msg.message {
                    Some(super::generated::client_message::Message::Ping(ping)) => ServerEvent {
                        sequence: seq,
                        timestamp_ns: Self::now_ns(),
                        event: Some(super::generated::server_event::Event::Pong(Pong {
                            client_time_ns: ping.client_time_ns,
                            server_time_ns: Self::now_ns(),
                        })),
                    },
                    Some(super::generated::client_message::Message::Subscribe(_)) => ServerEvent {
                        sequence: seq,
                        timestamp_ns: Self::now_ns(),
                        event: Some(super::generated::server_event::Event::Subscribed(
                            super::generated::Subscribed {
                                subscription_id: format!("sub_{}", seq),
                                current_sequence: seq,
                            },
                        )),
                    },
                    _ => {
                        // Acknowledge other messages
                        ServerEvent {
                            sequence: seq,
                            timestamp_ns: Self::now_ns(),
                            event: Some(super::generated::server_event::Event::Ack(Ack {
                                request_id: msg.request_id,
                                success: true,
                                error_code: None,
                                error_message: None,
                            })),
                        }
                    }
                };

                if tx.send(Ok(event)).await.is_err() {
                    break;
                }
            }
        });

        let out_stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(out_stream)))
    }

    /// Submit a new job.
    async fn submit_job(
        &self,
        request: Request<SubmitJobRequest>,
    ) -> Result<Response<SubmitJobResponse>, Status> {
        #[cfg(feature = "auth")]
        Self::authenticate(
            "/converge.ConvergeService/SubmitJob",
            Self::extract_bearer_token(&request)?,
        )
        .await?;

        use crate::execution::JobExecutor;
        use converge_core::Budget;

        let req = request.into_inner();
        let seq = self.next_sequence();

        let job_id = format!("job_{}", uuid::Uuid::new_v4());
        let run_id = format!("run_{}", uuid::Uuid::new_v4());

        tracing::info!(
            job_id = %job_id,
            blueprint_id = %req.blueprint_id,
            idempotency_key = %req.idempotency_key,
            seeds = req.seeds.len(),
            "Job submitted, starting execution"
        );

        // Build the job executor
        let pack_id = if req.blueprint_id.is_empty() {
            "growth-strategy".to_string() // Default pack
        } else {
            req.blueprint_id.clone()
        };

        // Build budget from request
        let budget = req
            .budget
            .map(|b| Budget {
                max_cycles: b.max_cycles.unwrap_or(100),
                max_facts: b.max_facts.unwrap_or(500),
            })
            .unwrap_or_default();

        // Convert seeds to our format
        // Note: prost_types::Struct doesn't implement Serialize directly,
        // so we convert via debug format for now
        let seeds: Vec<crate::templates::SeedFact> = req
            .seeds
            .iter()
            .map(|s| crate::templates::SeedFact {
                id: s.key.clone(),
                content: s
                    .value
                    .as_ref()
                    .map(|v| format!("{v:?}"))
                    .unwrap_or_default(),
            })
            .collect();

        let result = JobExecutor::builder()
            .with_pack(&pack_id)
            .with_seeds(seeds)
            .with_budget(budget)
            .with_mock_llm()
            .execute()
            .await
            .map_err(|e| Status::internal(format!("Execution error: {e}")))?;

        tracing::info!(
            job_id = %job_id,
            converged = result.converged,
            cycles = result.cycles,
            duration_ms = result.duration_ms,
            "Job completed"
        );

        Ok(Response::new(SubmitJobResponse {
            job_id,
            run_id,
            sequence: seq,
        }))
    }

    /// Get current job status.
    async fn get_job(
        &self,
        request: Request<GetJobRequest>,
    ) -> Result<Response<GetJobResponse>, Status> {
        #[cfg(feature = "auth")]
        Self::authenticate(
            "/converge.ConvergeService/GetJob",
            Self::extract_bearer_token(&request)?,
        )
        .await?;

        let req = request.into_inner();

        // TODO: Lookup actual job status
        Ok(Response::new(GetJobResponse {
            job_id: req.job_id.clone(),
            run_id: format!("run_{}", req.job_id),
            status: super::generated::RunStatus::Pending.into(),
            latest_sequence: self.sequence.load(Ordering::SeqCst),
            facts_count: 0,
            pending_proposals: 0,
            cycles_completed: 0,
            halt_info: None,
            converged_info: None,
        }))
    }

    /// Get events since sequence (polling fallback).
    async fn get_events(
        &self,
        request: Request<GetEventsRequest>,
    ) -> Result<Response<GetEventsResponse>, Status> {
        #[cfg(feature = "auth")]
        Self::authenticate(
            "/converge.ConvergeService/GetEvents",
            Self::extract_bearer_token(&request)?,
        )
        .await?;

        let _req = request.into_inner();

        // TODO: Return actual events from storage
        Ok(Response::new(GetEventsResponse {
            entries: vec![],
            latest_sequence: self.sequence.load(Ordering::SeqCst),
            has_more: false,
        }))
    }

    /// Session handshake - capability negotiation.
    async fn get_capabilities(
        &self,
        request: Request<GetCapabilitiesRequest>,
    ) -> Result<Response<GetCapabilitiesResponse>, Status> {
        #[cfg(feature = "auth")]
        Self::authenticate(
            "/converge.ConvergeService/GetCapabilities",
            Self::extract_bearer_token(&request)?,
        )
        .await?;

        use crate::execution::PackRegistry;

        let req = request.into_inner();

        tracing::debug!(
            device_id = %req.device_id,
            app_version = %req.app_version,
            platform = %req.platform,
            "Capability negotiation"
        );

        // Load pack info from registry
        let registry = PackRegistry::new();
        let packs: Vec<super::generated::PackInfo> = registry
            .list()
            .iter()
            .map(|p| super::generated::PackInfo {
                pack_id: p.id.clone(),
                name: p.name.clone(),
                version: p.version.clone(),
                truth_ids: p.invariants.clone(),
            })
            .collect();

        // Build truth info from invariants
        let active_truths: Vec<super::generated::TruthInfo> = registry
            .list()
            .iter()
            .flat_map(|p| {
                p.invariants
                    .iter()
                    .map(move |inv| super::generated::TruthInfo {
                        truth_id: inv.clone(),
                        pack_id: p.id.clone(),
                        description: format!("Invariant from {} pack", p.name),
                    })
            })
            .collect();

        Ok(Response::new(GetCapabilitiesResponse {
            packs,
            active_truths,
            streaming: Some(StreamingCapabilities {
                transports: vec![
                    TransportInfo {
                        r#type: "h2-grpc".to_string(),
                        status: "supported".to_string(),
                    },
                    TransportInfo {
                        r#type: "sse".to_string(),
                        status: "supported".to_string(),
                    },
                    TransportInfo {
                        r#type: "websocket".to_string(),
                        status: "planned".to_string(),
                    },
                ],
                default_transport: "h2-grpc".to_string(),
                resume_supported: true,
                max_resume_gap: 10000,
            }),
            determinism_mode_available: true,
            server_version: env!("CARGO_PKG_VERSION").to_string(),
            server_time_ns: Self::now_ns(),
        }))
    }
}
