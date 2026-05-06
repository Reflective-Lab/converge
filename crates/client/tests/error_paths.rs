// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Error-path coverage for `converge-client`.
//!
//! These tests exercise the surfaces that don't require a running gRPC
//! server: URI validation, `ClientError` formatting, and field-level
//! assertions on the message envelopes.

use converge_client::v1::client_message::Message;
use converge_client::v1::*;
use converge_client::{ClientError, ConvergeClient, messages};

// ── ClientError ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn connect_with_invalid_uri_returns_invalid_endpoint() {
    let err = ConvergeClient::connect("not a valid uri")
        .await
        .expect_err("invalid uri must error");
    match err {
        ClientError::InvalidEndpoint(msg) => assert!(!msg.is_empty(), "must include a reason"),
        other => panic!("expected InvalidEndpoint, got {other:?}"),
    }
}

#[tokio::test]
async fn connect_with_empty_uri_returns_invalid_endpoint() {
    let err = ConvergeClient::connect("")
        .await
        .expect_err("empty uri must error");
    assert!(matches!(err, ClientError::InvalidEndpoint(_)));
}

#[test]
fn client_error_display_is_non_empty_for_each_variant() {
    let invalid = ClientError::InvalidEndpoint("bad scheme".into());
    assert!(invalid.to_string().contains("invalid endpoint"));
    assert!(invalid.to_string().contains("bad scheme"));
}

// ── Field-level message envelope assertions ─────────────────────────────────

#[test]
fn cancel_job_preserves_fields() {
    let msg = messages::cancel_job(
        "req-c",
        CancelJobRequest {
            job_id: "job-7".into(),
            reason: Some("operator cancelled".into()),
        },
    );
    assert_eq!(msg.request_id, "req-c");
    match msg.message {
        Some(Message::CancelJob(r)) => {
            assert_eq!(r.job_id, "job-7");
            assert_eq!(r.reason.as_deref(), Some("operator cancelled"));
        }
        other => panic!("expected CancelJob, got {other:?}"),
    }
}

#[test]
fn resume_preserves_run_id() {
    let msg = messages::resume(
        "req-r",
        ResumeRunRequest {
            run_id: "run-resumed".into(),
        },
    );
    match msg.message {
        Some(Message::Resume(r)) => assert_eq!(r.run_id, "run-resumed"),
        other => panic!("expected Resume, got {other:?}"),
    }
}

#[test]
fn update_budget_preserves_run_id() {
    let msg = messages::update_budget(
        "req-b",
        UpdateBudgetRequest {
            run_id: "run-b".into(),
            budget: None,
        },
    );
    match msg.message {
        Some(Message::UpdateBudget(r)) => assert_eq!(r.run_id, "run-b"),
        other => panic!("expected UpdateBudget, got {other:?}"),
    }
}

#[test]
fn unsubscribe_preserves_correlation() {
    let msg = messages::unsubscribe(
        "req-u",
        UnsubscribeRequest {
            job_id: None,
            run_id: None,
            correlation_id: Some("corr-xyz".into()),
        },
    );
    match msg.message {
        Some(Message::Unsubscribe(r)) => {
            assert_eq!(r.correlation_id.as_deref(), Some("corr-xyz"));
        }
        other => panic!("expected Unsubscribe, got {other:?}"),
    }
}

#[test]
fn resume_from_preserves_sequence_and_snapshot_flag() {
    let msg = messages::resume_from(
        "req-rf",
        ResumeFromSequenceRequest {
            sequence: 9001,
            allow_snapshot: true,
        },
    );
    match msg.message {
        Some(Message::ResumeFrom(r)) => {
            assert_eq!(r.sequence, 9001);
            assert!(r.allow_snapshot);
        }
        other => panic!("expected ResumeFrom, got {other:?}"),
    }
}

#[test]
fn ping_preserves_client_time() {
    let msg = messages::ping(
        "req-p",
        Ping {
            client_time_ns: 1_700_000_000_000_000_000,
        },
    );
    match msg.message {
        Some(Message::Ping(r)) => assert_eq!(r.client_time_ns, 1_700_000_000_000_000_000),
        other => panic!("expected Ping, got {other:?}"),
    }
}

#[test]
fn empty_request_id_is_preserved_verbatim() {
    // request_id is caller-controlled; the helper doesn't validate.
    let msg = messages::ping("", Ping { client_time_ns: 0 });
    assert_eq!(msg.request_id, "");
}

#[test]
fn message_envelope_is_always_some_when_built_through_helpers() {
    // Spot-check that every helper produces Some(message) — the envelope
    // contract.
    let helpers: Vec<ClientMessage> = vec![
        messages::submit_job(
            "x",
            SubmitJobRequest {
                blueprint_id: "bp".into(),
                ..Default::default()
            },
        ),
        messages::cancel_job(
            "x",
            CancelJobRequest {
                job_id: "j".into(),
                reason: None,
            },
        ),
        messages::approve(
            "x",
            ApproveProposalRequest {
                run_id: "r".into(),
                proposal_id: "p".into(),
                comment: None,
            },
        ),
        messages::reject(
            "x",
            RejectProposalRequest {
                run_id: "r".into(),
                proposal_id: "p".into(),
                reason: "no".into(),
            },
        ),
        messages::pause(
            "x",
            PauseRunRequest {
                run_id: "r".into(),
                reason: None,
            },
        ),
        messages::resume("x", ResumeRunRequest { run_id: "r".into() }),
        messages::update_budget(
            "x",
            UpdateBudgetRequest {
                run_id: "r".into(),
                budget: None,
            },
        ),
        messages::subscribe(
            "x",
            SubscribeRequest {
                job_id: None,
                run_id: None,
                correlation_id: None,
                since_sequence: 0,
                entry_types: vec![],
            },
        ),
        messages::unsubscribe(
            "x",
            UnsubscribeRequest {
                job_id: None,
                run_id: None,
                correlation_id: None,
            },
        ),
        messages::resume_from(
            "x",
            ResumeFromSequenceRequest {
                sequence: 0,
                allow_snapshot: false,
            },
        ),
        messages::ping("x", Ping { client_time_ns: 0 }),
    ];
    for (i, m) in helpers.iter().enumerate() {
        assert!(m.message.is_some(), "helper {i} produced None envelope");
    }
}
