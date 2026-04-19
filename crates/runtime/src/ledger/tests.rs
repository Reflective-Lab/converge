// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Integration tests for LedgerClient.
//!
//! These tests verify the client interface and serialization.
//! For full integration tests with a running ledger server, use:
//!
//! ```bash
//! # Start the ledger server (in converge-ledger)
//! iex -S mix
//!
//! # Run integration tests
//! cargo test --features grpc -- --ignored
//! ```

use super::*;
use std::collections::HashMap;

#[test]
fn test_append_request_serialization() {
    let request = AppendRequest {
        context_id: "test-context".to_string(),
        key: "facts".to_string(),
        payload: b"test payload".to_vec(),
        metadata: HashMap::new(),
    };

    assert_eq!(request.context_id, "test-context");
    assert_eq!(request.key, "facts");
    assert_eq!(request.payload, b"test payload");
}

#[test]
fn test_get_request_with_filters() {
    let request = GetRequest {
        context_id: "my-context".to_string(),
        key: "intents".to_string(),
        after_sequence: 42,
        limit: 100,
    };

    assert_eq!(request.context_id, "my-context");
    assert_eq!(request.key, "intents");
    assert_eq!(request.after_sequence, 42);
    assert_eq!(request.limit, 100);
}

#[test]
fn test_snapshot_request() {
    let request = SnapshotRequest {
        context_id: "snapshot-context".to_string(),
    };

    assert_eq!(request.context_id, "snapshot-context");
}

#[test]
fn test_load_request() {
    let request = LoadRequest {
        context_id: "new-context".to_string(),
        snapshot: b"binary snapshot data".to_vec(),
        fail_if_exists: true,
    };

    assert_eq!(request.context_id, "new-context");
    assert!(request.fail_if_exists);
}

#[test]
fn test_watch_request() {
    let request = WatchRequest {
        context_id: "watch-context".to_string(),
        key: "traces".to_string(),
        from_sequence: 10,
    };

    assert_eq!(request.context_id, "watch-context");
    assert_eq!(request.key, "traces");
    assert_eq!(request.from_sequence, 10);
}

#[test]
fn test_entry_structure() {
    let entry = Entry {
        id: "entry-123".to_string(),
        key: "facts".to_string(),
        payload: b"test data".to_vec(),
        sequence: 42,
        appended_at_ns: 1_234_567_890_000_000_000,
        metadata: HashMap::from([
            ("agent_id".to_string(), "agent-001".to_string()),
            ("cycle".to_string(), "5".to_string()),
        ]),
    };

    assert_eq!(entry.id, "entry-123");
    assert_eq!(entry.sequence, 42);
    assert_eq!(
        entry.metadata.get("agent_id"),
        Some(&"agent-001".to_string())
    );
}

#[test]
fn test_snapshot_metadata() {
    let metadata = SnapshotMetadata {
        created_at_ns: 1_234_567_890_000_000_000,
        entry_count: 100,
        version: 2,
    };

    assert_eq!(metadata.entry_count, 100);
    assert_eq!(metadata.version, 2);
}

#[test]
fn test_ledger_error_types() {
    // Test InvalidResponse error
    let err = LedgerError::InvalidResponse("missing field".to_string());
    assert!(err.to_string().contains("invalid response"));
    assert!(err.to_string().contains("missing field"));
}

// Integration tests that require a running ledger server
// Run with: cargo test --features grpc -- --ignored

#[tokio::test]
#[ignore = "requires running ledger server"]
async fn integration_test_connect() {
    let result = LedgerClient::connect("http://localhost:50051").await;
    assert!(result.is_ok(), "Failed to connect: {:?}", result.err());
}

#[tokio::test]
#[ignore = "requires running ledger server"]
async fn integration_test_append_and_get() {
    let client = LedgerClient::connect("http://localhost:50051")
        .await
        .expect("Failed to connect");

    // Append an entry
    let entry = client
        .append("integration-test", "facts", b"test payload")
        .await
        .expect("Failed to append");

    assert!(!entry.id.is_empty());
    assert_eq!(entry.key, "facts");
    assert_eq!(entry.payload, b"test payload");
    assert!(entry.sequence > 0);

    // Get entries
    let response = client
        .get("integration-test", Some("facts".to_string()), None, None)
        .await
        .expect("Failed to get entries");

    assert!(!response.entries.is_empty());
    assert!(response.latest_sequence >= entry.sequence);
}

#[tokio::test]
#[ignore = "requires running ledger server"]
async fn integration_test_snapshot_and_load() {
    let client = LedgerClient::connect("http://localhost:50051")
        .await
        .expect("Failed to connect");

    // Create some entries
    for i in 0..5 {
        client
            .append("snapshot-test", "facts", format!("entry {}", i).as_bytes())
            .await
            .expect("Failed to append");
    }

    // Create snapshot
    let snapshot = client
        .snapshot("snapshot-test")
        .await
        .expect("Failed to create snapshot");

    assert!(!snapshot.snapshot.is_empty());
    assert!(snapshot.sequence >= 5);

    // Load into new context
    let load_result = client
        .load("snapshot-test-restored", snapshot.snapshot, true)
        .await
        .expect("Failed to load snapshot");

    assert_eq!(load_result.entries_restored, 5);
}
