// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Idiomatic Rust client for the Converge remote protocol.
//!
//! `converge-client` is intentionally thin. The wire contract lives in
//! `converge-protocol`; this crate adds a stable connection surface and typed
//! request envelopes for Rust consumers.

use tonic::transport::{Channel, Endpoint};

pub use converge_protocol as protocol;
pub use converge_protocol::{prost_types, v1};
pub use tonic::Streaming;
pub use tonic::transport::{Channel as TransportChannel, Endpoint as TransportEndpoint};

/// The streamed server event type returned by the Converge stream RPC.
pub type EventStream = Streaming<v1::ServerEvent>;

/// The generated tonic client used underneath the stable SDK wrapper.
pub type RawConvergeClient = v1::converge_service_client::ConvergeServiceClient<Channel>;

/// Errors returned by [`ConvergeClient`] operations.
#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// The endpoint URI could not be parsed.
    #[error("invalid endpoint: {0}")]
    InvalidEndpoint(String),
    /// Transport-level failure (connection refused, TLS error, etc.).
    #[error(transparent)]
    Transport(#[from] tonic::transport::Error),
    /// The server returned a gRPC status error.
    #[error(transparent)]
    Status(#[from] tonic::Status),
}

/// Stable Rust SDK for talking to a remote Converge runtime.
#[derive(Debug, Clone)]
pub struct ConvergeClient {
    inner: RawConvergeClient,
}

impl ConvergeClient {
    /// Connect to a remote Converge endpoint.
    pub async fn connect(uri: impl Into<String>) -> Result<Self, ClientError> {
        let endpoint = Endpoint::from_shared(uri.into())
            .map_err(|err| ClientError::InvalidEndpoint(err.to_string()))?;
        Self::connect_endpoint(endpoint).await
    }

    /// Connect using a pre-configured tonic endpoint.
    pub async fn connect_endpoint(endpoint: Endpoint) -> Result<Self, ClientError> {
        let inner = RawConvergeClient::connect(endpoint).await?;
        Ok(Self { inner })
    }

    /// Build a client from an existing tonic channel.
    pub fn from_channel(channel: Channel) -> Self {
        Self {
            inner: RawConvergeClient::new(channel),
        }
    }

    /// Access the underlying generated tonic client.
    pub fn inner(&self) -> &RawConvergeClient {
        &self.inner
    }

    /// Mutably access the underlying generated tonic client.
    pub fn inner_mut(&mut self) -> &mut RawConvergeClient {
        &mut self.inner
    }

    /// Consume the wrapper and return the underlying generated tonic client.
    pub fn into_inner(self) -> RawConvergeClient {
        self.inner
    }

    /// Submit a new job to the runtime.
    pub async fn submit_job(
        &mut self,
        request: v1::SubmitJobRequest,
    ) -> Result<v1::SubmitJobResponse, ClientError> {
        Ok(self.inner.submit_job(request).await?.into_inner())
    }

    /// Retrieve the current state of a job.
    pub async fn get_job(
        &mut self,
        request: v1::GetJobRequest,
    ) -> Result<v1::GetJobResponse, ClientError> {
        Ok(self.inner.get_job(request).await?.into_inner())
    }

    /// Retrieve events for a job or run.
    pub async fn get_events(
        &mut self,
        request: v1::GetEventsRequest,
    ) -> Result<v1::GetEventsResponse, ClientError> {
        Ok(self.inner.get_events(request).await?.into_inner())
    }

    /// Query the runtime's advertised capabilities.
    pub async fn get_capabilities(
        &mut self,
        request: v1::GetCapabilitiesRequest,
    ) -> Result<v1::GetCapabilitiesResponse, ClientError> {
        Ok(self.inner.get_capabilities(request).await?.into_inner())
    }

    /// Open a bidirectional stream for real-time interaction with the runtime.
    pub async fn stream<S>(&mut self, request: S) -> Result<EventStream, ClientError>
    where
        S: tonic::IntoStreamingRequest<Message = v1::ClientMessage>,
    {
        Ok(self.inner.stream(request).await?.into_inner())
    }
}

/// Helpers for wrapping typed requests into stream [`ClientMessage`](v1::ClientMessage) envelopes.
///
/// Each function constructs a `ClientMessage` with the appropriate variant
/// and a caller-provided `request_id` for correlation.
pub mod messages {
    use super::v1::{
        ApproveProposalRequest, CancelJobRequest, ClientMessage, PauseRunRequest, Ping,
        RejectProposalRequest, ResumeFromSequenceRequest, ResumeRunRequest, SubmitJobRequest,
        SubmitObservationRequest, SubscribeRequest, UnsubscribeRequest, UpdateBudgetRequest,
        client_message,
    };

    /// Wrap a [`SubmitJobRequest`] for the stream.
    pub fn submit_job(request_id: impl Into<String>, request: SubmitJobRequest) -> ClientMessage {
        envelope(request_id, client_message::Message::SubmitJob(request))
    }

    /// Wrap a [`CancelJobRequest`] for the stream.
    pub fn cancel_job(request_id: impl Into<String>, request: CancelJobRequest) -> ClientMessage {
        envelope(request_id, client_message::Message::CancelJob(request))
    }

    /// Wrap a [`SubmitObservationRequest`] for the stream.
    pub fn submit_observation(
        request_id: impl Into<String>,
        request: SubmitObservationRequest,
    ) -> ClientMessage {
        envelope(
            request_id,
            client_message::Message::SubmitObservation(request),
        )
    }

    /// Wrap an [`ApproveProposalRequest`] for the stream.
    pub fn approve(
        request_id: impl Into<String>,
        request: ApproveProposalRequest,
    ) -> ClientMessage {
        envelope(request_id, client_message::Message::Approve(request))
    }

    /// Wrap a [`RejectProposalRequest`] for the stream.
    pub fn reject(request_id: impl Into<String>, request: RejectProposalRequest) -> ClientMessage {
        envelope(request_id, client_message::Message::Reject(request))
    }

    /// Wrap a [`PauseRunRequest`] for the stream.
    pub fn pause(request_id: impl Into<String>, request: PauseRunRequest) -> ClientMessage {
        envelope(request_id, client_message::Message::Pause(request))
    }

    /// Wrap a [`ResumeRunRequest`] for the stream.
    pub fn resume(request_id: impl Into<String>, request: ResumeRunRequest) -> ClientMessage {
        envelope(request_id, client_message::Message::Resume(request))
    }

    /// Wrap an [`UpdateBudgetRequest`] for the stream.
    pub fn update_budget(
        request_id: impl Into<String>,
        request: UpdateBudgetRequest,
    ) -> ClientMessage {
        envelope(request_id, client_message::Message::UpdateBudget(request))
    }

    /// Wrap a [`SubscribeRequest`] for the stream.
    pub fn subscribe(request_id: impl Into<String>, request: SubscribeRequest) -> ClientMessage {
        envelope(request_id, client_message::Message::Subscribe(request))
    }

    /// Wrap an [`UnsubscribeRequest`] for the stream.
    pub fn unsubscribe(
        request_id: impl Into<String>,
        request: UnsubscribeRequest,
    ) -> ClientMessage {
        envelope(request_id, client_message::Message::Unsubscribe(request))
    }

    /// Wrap a [`ResumeFromSequenceRequest`] for the stream.
    pub fn resume_from(
        request_id: impl Into<String>,
        request: ResumeFromSequenceRequest,
    ) -> ClientMessage {
        envelope(request_id, client_message::Message::ResumeFrom(request))
    }

    /// Wrap a [`Ping`] for the stream.
    pub fn ping(request_id: impl Into<String>, request: Ping) -> ClientMessage {
        envelope(request_id, client_message::Message::Ping(request))
    }

    fn envelope(request_id: impl Into<String>, message: client_message::Message) -> ClientMessage {
        ClientMessage {
            request_id: request_id.into(),
            message: Some(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_message_wraps_the_expected_variant() {
        let message = messages::pause(
            "pause-1",
            v1::PauseRunRequest {
                run_id: "run-123".to_string(),
                reason: Some("operator requested pause".to_string()),
            },
        );

        assert_eq!(message.request_id, "pause-1");
        match message.message {
            Some(v1::client_message::Message::Pause(request)) => {
                assert_eq!(request.run_id, "run-123");
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn subscribe_message_wraps_the_expected_variant() {
        let message = messages::subscribe(
            "sub-1",
            v1::SubscribeRequest {
                job_id: None,
                run_id: Some("run-123".to_string()),
                correlation_id: None,
                since_sequence: 42,
                entry_types: vec![],
            },
        );

        assert_eq!(message.request_id, "sub-1");
        match message.message {
            Some(v1::client_message::Message::Subscribe(request)) => {
                assert_eq!(request.run_id.as_deref(), Some("run-123"));
                assert_eq!(request.since_sequence, 42);
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn submit_job_message_wraps_the_expected_variant() {
        let message = messages::submit_job(
            "submit-1",
            v1::SubmitJobRequest {
                idempotency_key: "key-1".to_string(),
                blueprint_id: "bp-42".to_string(),
                pack_ids: vec!["pack-a".to_string()],
                seeds: vec![],
                budget: None,
                correlation_id: Some("corr-1".to_string()),
                parent_trace_id: None,
            },
        );

        assert_eq!(message.request_id, "submit-1");
        match message.message {
            Some(v1::client_message::Message::SubmitJob(request)) => {
                assert_eq!(request.blueprint_id, "bp-42");
                assert_eq!(request.idempotency_key, "key-1");
                assert_eq!(request.pack_ids, vec!["pack-a"]);
                assert_eq!(request.correlation_id.as_deref(), Some("corr-1"));
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn cancel_job_message_wraps_the_expected_variant() {
        let message = messages::cancel_job(
            "cancel-1",
            v1::CancelJobRequest {
                job_id: "job-99".to_string(),
                reason: Some("no longer needed".to_string()),
            },
        );

        assert_eq!(message.request_id, "cancel-1");
        match message.message {
            Some(v1::client_message::Message::CancelJob(request)) => {
                assert_eq!(request.job_id, "job-99");
                assert_eq!(request.reason.as_deref(), Some("no longer needed"));
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn submit_observation_message_wraps_the_expected_variant() {
        let message = messages::submit_observation(
            "obs-1",
            v1::SubmitObservationRequest {
                run_id: "run-7".to_string(),
                key: "temperature".to_string(),
                payload: None,
                target_truth_id: Some("truth-1".to_string()),
                idempotency_key: "idem-1".to_string(),
            },
        );

        assert_eq!(message.request_id, "obs-1");
        match message.message {
            Some(v1::client_message::Message::SubmitObservation(request)) => {
                assert_eq!(request.run_id, "run-7");
                assert_eq!(request.key, "temperature");
                assert_eq!(request.target_truth_id.as_deref(), Some("truth-1"));
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn approve_message_wraps_the_expected_variant() {
        let message = messages::approve(
            "approve-1",
            v1::ApproveProposalRequest {
                run_id: "run-5".to_string(),
                proposal_id: "prop-3".to_string(),
                comment: Some("looks good".to_string()),
            },
        );

        assert_eq!(message.request_id, "approve-1");
        match message.message {
            Some(v1::client_message::Message::Approve(request)) => {
                assert_eq!(request.run_id, "run-5");
                assert_eq!(request.proposal_id, "prop-3");
                assert_eq!(request.comment.as_deref(), Some("looks good"));
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn reject_message_wraps_the_expected_variant() {
        let message = messages::reject(
            "reject-1",
            v1::RejectProposalRequest {
                run_id: "run-5".to_string(),
                proposal_id: "prop-3".to_string(),
                reason: "insufficient evidence".to_string(),
            },
        );

        assert_eq!(message.request_id, "reject-1");
        match message.message {
            Some(v1::client_message::Message::Reject(request)) => {
                assert_eq!(request.run_id, "run-5");
                assert_eq!(request.proposal_id, "prop-3");
                assert_eq!(request.reason, "insufficient evidence");
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn resume_message_wraps_the_expected_variant() {
        let message = messages::resume(
            "resume-1",
            v1::ResumeRunRequest {
                run_id: "run-8".to_string(),
            },
        );

        assert_eq!(message.request_id, "resume-1");
        match message.message {
            Some(v1::client_message::Message::Resume(request)) => {
                assert_eq!(request.run_id, "run-8");
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn update_budget_message_wraps_the_expected_variant() {
        let message = messages::update_budget(
            "budget-1",
            v1::UpdateBudgetRequest {
                run_id: "run-10".to_string(),
                budget: None,
            },
        );

        assert_eq!(message.request_id, "budget-1");
        match message.message {
            Some(v1::client_message::Message::UpdateBudget(request)) => {
                assert_eq!(request.run_id, "run-10");
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn unsubscribe_message_wraps_the_expected_variant() {
        let message = messages::unsubscribe(
            "unsub-1",
            v1::UnsubscribeRequest {
                job_id: Some("job-1".to_string()),
                run_id: None,
                correlation_id: None,
            },
        );

        assert_eq!(message.request_id, "unsub-1");
        match message.message {
            Some(v1::client_message::Message::Unsubscribe(request)) => {
                assert_eq!(request.job_id.as_deref(), Some("job-1"));
                assert!(request.run_id.is_none());
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn resume_from_message_wraps_the_expected_variant() {
        let message = messages::resume_from(
            "resume-from-1",
            v1::ResumeFromSequenceRequest {
                sequence: 100,
                allow_snapshot: true,
            },
        );

        assert_eq!(message.request_id, "resume-from-1");
        match message.message {
            Some(v1::client_message::Message::ResumeFrom(request)) => {
                assert_eq!(request.sequence, 100);
                assert!(request.allow_snapshot);
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn ping_message_wraps_the_expected_variant() {
        let message = messages::ping(
            "ping-1",
            v1::Ping {
                client_time_ns: 1_700_000_000_000,
            },
        );

        assert_eq!(message.request_id, "ping-1");
        match message.message {
            Some(v1::client_message::Message::Ping(request)) => {
                assert_eq!(request.client_time_ns, 1_700_000_000_000);
            }
            other => panic!("unexpected message variant: {other:?}"),
        }
    }

    #[test]
    fn client_error_invalid_endpoint_display() {
        let err = ClientError::InvalidEndpoint("bad uri".to_string());
        assert_eq!(err.to_string(), "invalid endpoint: bad uri");
    }

    #[test]
    fn client_error_status_from_conversion() {
        let status = tonic::Status::not_found("missing");
        let err: ClientError = status.into();
        assert!(err.to_string().contains("missing"));
    }
}
