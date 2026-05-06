// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Integration tests for the public `InMemoryExperienceStore` paths that
//! inline tests under-cover: user events, query_records mixing engine and
//! user records, time-range and chain-id filters, and the StoreObserver
//! bridge.

use std::collections::HashMap;
use std::sync::Arc;

use converge_core::{
    BudgetResource, DecisionStep, EventQuery, ExperienceEvent, ExperienceEventEnvelope,
    ExperienceEventKind, ExperienceEventObserver, ExperienceRecord, ExperienceStore,
    OverrideTarget, TimeRange, Timestamp, UserExperienceEvent, UserExperienceEventEnvelope,
};
use converge_experience::{InMemoryExperienceStore, StoreObserver, summarize_events};

fn outcome(chain: &str) -> ExperienceEvent {
    ExperienceEvent::OutcomeRecorded {
        chain_id: chain.into(),
        step: DecisionStep::Planning,
        passed: true,
        stop_reason: None,
        latency_ms: None,
        tokens: None,
        cost_microdollars: None,
        backend: None,
        metadata: HashMap::new(),
    }
}

fn user_override(reason: &str) -> UserExperienceEvent {
    UserExperienceEvent::UserOverrideIssued {
        target: OverrideTarget::Constraint("budget".into()),
        actor: "operator-1".into(),
        policy_snapshot_hash: None,
        reason: reason.into(),
    }
}

// ── User-event paths ────────────────────────────────────────────────────────

#[test]
fn append_user_event_with_valid_envelope_succeeds() {
    let store = InMemoryExperienceStore::new();
    let env = UserExperienceEventEnvelope::new("user-evt-1", user_override("tighten"))
        .with_tenant("tenant-a")
        .with_correlation("corr-7");
    store.append_user_event(env).expect("append user event");

    let recs = store
        .query_records(&EventQuery::default())
        .expect("query records");
    assert_eq!(recs.len(), 1);
    assert!(matches!(recs[0], ExperienceRecord::User(_)));
}

#[test]
fn query_records_merges_engine_and_user_events() {
    let store = InMemoryExperienceStore::new();

    store
        .append_event(ExperienceEventEnvelope::new(
            "evt-engine-1",
            outcome("chain-a"),
        ))
        .unwrap();
    store
        .append_user_event(UserExperienceEventEnvelope::new(
            "evt-user-1",
            user_override("override budget"),
        ))
        .unwrap();

    let recs = store
        .query_records(&EventQuery::default())
        .expect("merged records");
    assert_eq!(recs.len(), 2, "must include both engine and user records");

    let engine_count = recs
        .iter()
        .filter(|r| matches!(r, ExperienceRecord::Engine(_)))
        .count();
    let user_count = recs
        .iter()
        .filter(|r| matches!(r, ExperienceRecord::User(_)))
        .count();
    assert_eq!(engine_count, 1);
    assert_eq!(user_count, 1);
}

#[test]
fn query_records_respects_limit() {
    let store = InMemoryExperienceStore::new();
    for i in 0..5 {
        store
            .append_event(ExperienceEventEnvelope::new(
                format!("evt-{i}"),
                outcome("c"),
            ))
            .unwrap();
        store
            .append_user_event(UserExperienceEventEnvelope::new(
                format!("user-{i}"),
                user_override("r"),
            ))
            .unwrap();
    }
    let q = EventQuery {
        limit: Some(3),
        ..Default::default()
    };
    let recs = store.query_records(&q).expect("limited query");
    assert_eq!(recs.len(), 3);
}

#[test]
fn user_event_query_filters_by_tenant() {
    let store = InMemoryExperienceStore::new();
    let a = UserExperienceEventEnvelope::new("u-a", user_override("a")).with_tenant("tenant-a");
    let b = UserExperienceEventEnvelope::new("u-b", user_override("b")).with_tenant("tenant-b");
    store.append_user_event(a).unwrap();
    store.append_user_event(b).unwrap();

    let q = EventQuery {
        tenant_id: Some("tenant-a".into()),
        ..Default::default()
    };
    let recs = store.query_records(&q).expect("filtered query");
    assert_eq!(recs.len(), 1);
    match &recs[0] {
        ExperienceRecord::User(env) => {
            assert_eq!(env.tenant_id.as_deref(), Some("tenant-a"));
        }
        ExperienceRecord::Engine(_) => panic!("expected user record"),
    }
}

#[test]
fn user_event_query_filters_by_correlation() {
    let store = InMemoryExperienceStore::new();
    store
        .append_user_event(
            UserExperienceEventEnvelope::new("u-a", user_override("a")).with_correlation("corr-x"),
        )
        .unwrap();
    store
        .append_user_event(
            UserExperienceEventEnvelope::new("u-b", user_override("b")).with_correlation("corr-y"),
        )
        .unwrap();

    let q = EventQuery {
        correlation_id: Some("corr-x".into()),
        ..Default::default()
    };
    let recs = store.query_records(&q).expect("filtered query");
    assert_eq!(recs.len(), 1);
}

// ── Engine-event filters under-covered by inline tests ──────────────────────

#[test]
fn engine_event_query_filters_by_chain_id() {
    let store = InMemoryExperienceStore::new();
    store
        .append_event(ExperienceEventEnvelope::new(
            "evt-1",
            outcome("chain-target"),
        ))
        .unwrap();
    store
        .append_event(ExperienceEventEnvelope::new(
            "evt-2",
            outcome("chain-other"),
        ))
        .unwrap();

    let q = EventQuery {
        chain_id: Some("chain-target".into()),
        ..Default::default()
    };
    let results = store.query_events(&q).expect("query events");
    assert_eq!(results.len(), 1);
}

#[test]
fn chain_id_filter_excludes_events_without_a_chain() {
    let store = InMemoryExperienceStore::new();
    // FactPromoted does not carry a chain_id
    store
        .append_event(ExperienceEventEnvelope::new(
            "evt-1",
            ExperienceEvent::FactPromoted {
                proposal_id: "p-1".into(),
                fact_id: "f-1".into(),
                promoted_by: "agent".into(),
                reason: "promo".into(),
                requires_human: false,
            },
        ))
        .unwrap();
    let q = EventQuery {
        chain_id: Some("chain-x".into()),
        ..Default::default()
    };
    let results = store.query_events(&q).expect("query events");
    assert!(
        results.is_empty(),
        "events without a chain_id must not match"
    );
}

#[test]
fn engine_event_query_filters_by_kinds_set() {
    let store = InMemoryExperienceStore::new();
    store
        .append_event(ExperienceEventEnvelope::new("evt-1", outcome("c")))
        .unwrap();
    store
        .append_event(ExperienceEventEnvelope::new(
            "evt-2",
            ExperienceEvent::BudgetExceeded {
                chain_id: "c".into(),
                resource: BudgetResource::EngineBudget,
                limit: "1".into(),
                observed: None,
            },
        ))
        .unwrap();

    let q = EventQuery {
        kinds: vec![ExperienceEventKind::BudgetExceeded],
        ..Default::default()
    };
    let results = store.query_events(&q).expect("query events");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].event.kind(), ExperienceEventKind::BudgetExceeded);
}

#[test]
fn time_range_filter_includes_only_events_inside_bounds() {
    let store = InMemoryExperienceStore::new();
    let early = ExperienceEventEnvelope::new("evt-early", outcome("c"));
    let early_ts: Timestamp = early.occurred_at.clone();

    store.append_event(early).unwrap();

    // The store assigns occurred_at automatically. We use that timestamp
    // to bound the range exactly so the appended event matches.
    let q = EventQuery {
        time_range: Some(TimeRange {
            start: Some(early_ts.clone()),
            end: Some(early_ts.clone()),
        }),
        ..Default::default()
    };
    let inside = store.query_events(&q).expect("query within bounds");
    assert_eq!(inside.len(), 1);

    // A range entirely after the event excludes it.
    let q_after = EventQuery {
        time_range: Some(TimeRange {
            start: Some(Timestamp::from("9999-01-01T00:00:00Z")),
            end: None,
        }),
        ..Default::default()
    };
    let outside = store.query_events(&q_after).expect("query after window");
    assert!(outside.is_empty());
}

// ── StoreObserver bridge ────────────────────────────────────────────────────

#[test]
fn store_observer_appends_engine_events_to_store() {
    let store = Arc::new(InMemoryExperienceStore::new());
    let observer = StoreObserver::new(Arc::clone(&store));

    // Two events through the observer.
    observer.on_event(&outcome("chain-a"));
    observer.on_event(&ExperienceEvent::BudgetExceeded {
        chain_id: "chain-b".into(),
        resource: BudgetResource::EngineBudget,
        limit: "1".into(),
        observed: None,
    });

    let stored = observer
        .store()
        .query_events(&EventQuery::default())
        .expect("query events");
    assert_eq!(stored.len(), 2);
    let summary = summarize_events(&stored);
    assert_eq!(summary.outcome_recorded, 1);
    assert_eq!(summary.budget_exceeded, 1);
}

#[test]
fn store_observer_id_is_unique_per_event() {
    let store = Arc::new(InMemoryExperienceStore::new());
    let observer = StoreObserver::new(Arc::clone(&store));
    for _ in 0..10 {
        observer.on_event(&outcome("c"));
    }
    let stored = store.query_events(&EventQuery::default()).expect("query");
    let mut ids: Vec<_> = stored
        .iter()
        .map(|e| e.event_id.as_str().to_string())
        .collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), 10, "every observed event must get a unique id");
}
