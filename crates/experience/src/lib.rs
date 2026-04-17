// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! # Experience Store Implementations
//!
//! This crate provides concrete implementations of the `ExperienceStore` trait.
//! The first implementation is in-memory for tests and local development.

mod validate;

#[cfg(feature = "surrealdb")]
mod surrealdb_store;

#[cfg(feature = "lancedb")]
mod lancedb_store;

#[cfg(feature = "lancedb")]
pub use lancedb_store::{LanceDbConfig, LanceDbExperienceStore, SimilarEvent, VectorEvent};
#[cfg(feature = "surrealdb")]
pub use surrealdb_store::{SurrealDbConfig, SurrealDbExperienceStore};
pub use validate::validate_envelope;

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use converge_core::{
    ArtifactKind, EventQuery, ExperienceEvent, ExperienceEventEnvelope, ExperienceEventKind,
    ExperienceStore, ExperienceStoreError, ExperienceStoreResult, LifecycleEvent, ReplayTrace,
    TimeRange,
};
use serde::{Deserialize, Serialize};

/// In-memory experience store (dev/test).
#[derive(Debug, Default)]
pub struct InMemoryExperienceStore {
    events: RwLock<Vec<ExperienceEventEnvelope>>,
    trace_links: RwLock<HashMap<String, ReplayTrace>>,
    next_event_id: AtomicU64,
}

impl InMemoryExperienceStore {
    /// Create a new empty in-memory store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn next_id(&self) -> String {
        let id = self.next_event_id.fetch_add(1, Ordering::Relaxed);
        format!("evt-{}", id)
    }

    fn record_trace_link(&self, event: &ExperienceEvent) {
        if let ExperienceEvent::ReplayTraceRecorded {
            trace_link_id,
            trace_link,
        } = event
        {
            let mut map = self.trace_links.write().expect("trace link lock poisoned");
            map.insert(trace_link_id.clone(), trace_link.clone());
        }
    }
}

impl ExperienceStore for InMemoryExperienceStore {
    fn append_event(&self, event: ExperienceEventEnvelope) -> ExperienceStoreResult<()> {
        validate_envelope(&event)?;
        self.record_trace_link(&event.event);
        let mut events = self
            .events
            .write()
            .map_err(|_| ExperienceStoreError::StorageError {
                message: "events lock poisoned".to_string(),
            })?;
        events.push(event);
        Ok(())
    }

    fn query_events(
        &self,
        query: &EventQuery,
    ) -> ExperienceStoreResult<Vec<ExperienceEventEnvelope>> {
        let events = self
            .events
            .read()
            .map_err(|_| ExperienceStoreError::StorageError {
                message: "events lock poisoned".to_string(),
            })?;

        let filtered = events
            .iter()
            .filter(|event| event_matches_query(event, query))
            .cloned()
            .collect::<Vec<_>>();

        if let Some(limit) = query.limit {
            Ok(filtered.into_iter().take(limit).collect())
        } else {
            Ok(filtered)
        }
    }

    fn write_artifact_state_transition(
        &self,
        artifact_id: &str,
        artifact_kind: ArtifactKind,
        event: LifecycleEvent,
    ) -> ExperienceStoreResult<()> {
        let payload = ExperienceEvent::ArtifactStateTransitioned {
            artifact_id: artifact_id.to_string(),
            artifact_kind,
            event,
        };
        let envelope = ExperienceEventEnvelope::new(self.next_id(), payload);
        self.append_event(envelope)
    }

    fn get_trace_link(&self, trace_link_id: &str) -> ExperienceStoreResult<Option<ReplayTrace>> {
        let map = self
            .trace_links
            .read()
            .map_err(|_| ExperienceStoreError::StorageError {
                message: "trace link lock poisoned".to_string(),
            })?;
        Ok(map.get(trace_link_id).cloned())
    }
}

// ============================================================================
// ExperienceEventObserver bridge
// ============================================================================

use converge_core::ExperienceEventObserver;

/// Bridges engine events to any `ExperienceStore` implementation.
///
/// Pass this to `Engine::set_event_observer()` to capture all convergence
/// events into a store for audit, debugging, and downstream consumption.
///
/// Works with `InMemoryExperienceStore`, `SurrealDbExperienceStore`,
/// `LanceDbExperienceStore`, or any custom `ExperienceStore` impl.
pub struct StoreObserver<S: ExperienceStore = InMemoryExperienceStore> {
    store: Arc<S>,
    next_id: AtomicU64,
}

impl<S: ExperienceStore> StoreObserver<S> {
    /// Create a new observer that appends to the given store.
    #[must_use]
    pub fn new(store: Arc<S>) -> Self {
        Self {
            store,
            next_id: AtomicU64::new(0),
        }
    }

    /// Access the underlying store (e.g., to query events after a run).
    #[must_use]
    pub fn store(&self) -> &Arc<S> {
        &self.store
    }
}

impl<S: ExperienceStore + 'static> ExperienceEventObserver for StoreObserver<S> {
    fn on_event(&self, event: &ExperienceEvent) {
        let id = format!("evt-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let envelope = ExperienceEventEnvelope::new(id, event.clone());
        let _ = self.store.append_event(envelope);
    }
}

/// Human-readable label for an experience event kind.
#[must_use]
pub fn event_kind_label(kind: ExperienceEventKind) -> &'static str {
    match kind {
        ExperienceEventKind::ProposalCreated => "proposal_created",
        ExperienceEventKind::ProposalValidated => "proposal_validated",
        ExperienceEventKind::FactPromoted => "fact_promoted",
        ExperienceEventKind::RecallExecuted => "recall_executed",
        ExperienceEventKind::ReplayTraceRecorded => "trace_link_recorded",
        ExperienceEventKind::ReplayabilityDowngraded => "replayability_downgraded",
        ExperienceEventKind::ArtifactStateTransitioned => "artifact_state_transitioned",
        ExperienceEventKind::ArtifactRollbackRecorded => "artifact_rollback_recorded",
        ExperienceEventKind::BackendInvoked => "backend_invoked",
        ExperienceEventKind::OutcomeRecorded => "outcome_recorded",
        ExperienceEventKind::BudgetExceeded => "budget_exceeded",
        ExperienceEventKind::PolicySnapshotCaptured => "policy_snapshot_captured",
        ExperienceEventKind::HypothesisResolved => "hypothesis_resolved",
    }
}

/// Generic summary of a set of experience events.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceEventSummary {
    pub total_events: usize,
    pub proposal_created: usize,
    pub proposal_validated: usize,
    pub fact_promoted: usize,
    pub recall_executed: usize,
    pub trace_link_recorded: usize,
    pub replayability_downgraded: usize,
    pub artifact_state_transitioned: usize,
    pub artifact_rollback_recorded: usize,
    pub backend_invoked: usize,
    pub outcome_recorded: usize,
    pub budget_exceeded: usize,
    pub policy_snapshot_captured: usize,
    pub hypothesis_resolved: usize,
    pub by_kind: BTreeMap<String, usize>,
}

/// Summarize experience events into reusable telemetry counts.
#[must_use]
pub fn summarize_events(events: &[ExperienceEventEnvelope]) -> ExperienceEventSummary {
    let mut summary = ExperienceEventSummary {
        total_events: events.len(),
        ..ExperienceEventSummary::default()
    };

    for envelope in events {
        let kind = envelope.event.kind();
        *summary
            .by_kind
            .entry(event_kind_label(kind).to_string())
            .or_insert(0) += 1;

        match kind {
            ExperienceEventKind::ProposalCreated => summary.proposal_created += 1,
            ExperienceEventKind::ProposalValidated => summary.proposal_validated += 1,
            ExperienceEventKind::FactPromoted => summary.fact_promoted += 1,
            ExperienceEventKind::RecallExecuted => summary.recall_executed += 1,
            ExperienceEventKind::ReplayTraceRecorded => summary.trace_link_recorded += 1,
            ExperienceEventKind::ReplayabilityDowngraded => summary.replayability_downgraded += 1,
            ExperienceEventKind::ArtifactStateTransitioned => {
                summary.artifact_state_transitioned += 1
            }
            ExperienceEventKind::ArtifactRollbackRecorded => {
                summary.artifact_rollback_recorded += 1
            }
            ExperienceEventKind::BackendInvoked => summary.backend_invoked += 1,
            ExperienceEventKind::OutcomeRecorded => summary.outcome_recorded += 1,
            ExperienceEventKind::BudgetExceeded => summary.budget_exceeded += 1,
            ExperienceEventKind::PolicySnapshotCaptured => summary.policy_snapshot_captured += 1,
            ExperienceEventKind::HypothesisResolved => summary.hypothesis_resolved += 1,
        }
    }

    summary
}

fn event_matches_query(event: &ExperienceEventEnvelope, query: &EventQuery) -> bool {
    if let Some(ref tenant_id) = query.tenant_id {
        if event.tenant_id.as_deref() != Some(tenant_id.as_str()) {
            return false;
        }
    }

    if let Some(ref correlation_id) = query.correlation_id {
        if event.correlation_id.as_deref() != Some(correlation_id.as_str()) {
            return false;
        }
    }

    if !query.kinds.is_empty() && !query.kinds.contains(&event.event.kind()) {
        return false;
    }

    if let Some(ref chain_id) = query.chain_id {
        if event_chain_id(&event.event) != Some(chain_id.as_str()) {
            return false;
        }
    }

    if let Some(ref range) = query.time_range {
        if !matches_time_range(event.occurred_at.as_str(), range) {
            return false;
        }
    }

    true
}

fn event_chain_id(event: &ExperienceEvent) -> Option<&str> {
    match event {
        ExperienceEvent::ProposalCreated { chain_id, .. } => Some(chain_id.as_str()),
        ExperienceEvent::ProposalValidated { chain_id, .. } => Some(chain_id.as_str()),
        ExperienceEvent::OutcomeRecorded { chain_id, .. } => Some(chain_id.as_str()),
        ExperienceEvent::BudgetExceeded { chain_id, .. } => Some(chain_id.as_str()),
        _ => None,
    }
}

fn matches_time_range(timestamp: &str, range: &TimeRange) -> bool {
    let start_ok = range
        .start
        .as_deref()
        .is_none_or(|start| timestamp >= start);
    let end_ok = range.end.as_deref().is_none_or(|end| timestamp <= end);
    start_ok && end_ok
}

#[cfg(test)]
mod tests {
    use converge_core::{DecisionStep, ExperienceEventEnvelope, ExperienceEventKind};

    use super::*;

    #[test]
    fn query_filters_by_tenant_and_kind() {
        let store = InMemoryExperienceStore::new();
        let event = ExperienceEvent::OutcomeRecorded {
            chain_id: "chain-1".to_string(),
            step: DecisionStep::Planning,
            passed: true,
            stop_reason: None,
            latency_ms: None,
            tokens: None,
            cost_microdollars: None,
            backend: None,
            metadata: Default::default(),
        };
        let envelope = ExperienceEventEnvelope::new("evt-1", event).with_tenant("tenant-a");
        store.append_event(envelope).expect("append event");

        let query = EventQuery {
            tenant_id: Some("tenant-a".to_string()),
            kinds: vec![ExperienceEventKind::OutcomeRecorded],
            ..Default::default()
        };
        let results = store.query_events(&query).expect("query events");
        assert_eq!(results.len(), 1);

        let query_other = EventQuery {
            tenant_id: Some("tenant-b".to_string()),
            kinds: vec![ExperienceEventKind::OutcomeRecorded],
            ..Default::default()
        };
        let results = store.query_events(&query_other).expect("query events");
        assert!(results.is_empty());
    }

    #[test]
    fn summarize_events_counts_all_kinds() {
        let events = vec![
            ExperienceEventEnvelope::new(
                "evt-1",
                ExperienceEvent::FactPromoted {
                    proposal_id: "proposal-1".into(),
                    fact_id: "fact-1".into(),
                    promoted_by: "agent-1".into(),
                    reason: "promoted".into(),
                    requires_human: false,
                },
            ),
            ExperienceEventEnvelope::new(
                "evt-2",
                ExperienceEvent::OutcomeRecorded {
                    chain_id: "chain-1".to_string(),
                    step: DecisionStep::Planning,
                    passed: true,
                    stop_reason: Some("converged".into()),
                    latency_ms: None,
                    tokens: None,
                    cost_microdollars: None,
                    backend: Some("converge-engine".into()),
                    metadata: Default::default(),
                },
            ),
            ExperienceEventEnvelope::new(
                "evt-3",
                ExperienceEvent::BudgetExceeded {
                    chain_id: "chain-1".to_string(),
                    resource: "engine-budget".into(),
                    limit: "max_cycles (20)".into(),
                    observed: None,
                },
            ),
        ];

        let summary = summarize_events(&events);
        assert_eq!(summary.total_events, 3);
        assert_eq!(summary.fact_promoted, 1);
        assert_eq!(summary.outcome_recorded, 1);
        assert_eq!(summary.budget_exceeded, 1);
        assert_eq!(summary.by_kind.get("fact_promoted"), Some(&1));
        assert_eq!(summary.by_kind.get("outcome_recorded"), Some(&1));
        assert_eq!(summary.by_kind.get("budget_exceeded"), Some(&1));
    }

    #[test]
    fn trace_link_is_recorded_and_retrievable() {
        let store = InMemoryExperienceStore::new();
        let trace_link = ReplayTrace::Remote(converge_core::RemoteReplayTrace {
            provider_name: "anthropic".to_string(),
            provider_model_id: "claude".to_string(),
            request_fingerprint: "req".to_string(),
            response_fingerprint: "resp".to_string(),
            temperature: 0.0,
            top_p: 1.0,
            max_tokens: 512,
            provider_metadata: HashMap::new(),
            retried: false,
            retry_reasons: vec![],
            replayability: converge_core::Replayability::BestEffort,
        });
        let event = ExperienceEvent::ReplayTraceRecorded {
            trace_link_id: "trace-1".to_string(),
            trace_link: trace_link.clone(),
        };
        let envelope = ExperienceEventEnvelope::new("evt-2", event);
        store.append_event(envelope).expect("append event");

        let fetched = store
            .get_trace_link("trace-1")
            .expect("get trace")
            .expect("trace exists");
        assert_eq!(fetched.replayability(), trace_link.replayability());
    }

    #[test]
    fn store_rejects_malformed_event_id_at_write_boundary() {
        let store = InMemoryExperienceStore::new();
        let event = ExperienceEvent::OutcomeRecorded {
            chain_id: "chain-1".to_string(),
            step: DecisionStep::Planning,
            passed: true,
            stop_reason: None,
            latency_ms: None,
            tokens: None,
            cost_microdollars: None,
            backend: None,
            metadata: Default::default(),
        };
        // SurrealDB record ID injection attempt
        let envelope = ExperienceEventEnvelope::new("../../admin:hack", event);
        let result = store.append_event(envelope);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("invalid characters")
        );
    }

    #[test]
    fn store_rejects_malformed_tenant_id() {
        let store = InMemoryExperienceStore::new();
        let event = ExperienceEvent::OutcomeRecorded {
            chain_id: "chain-1".to_string(),
            step: DecisionStep::Planning,
            passed: true,
            stop_reason: None,
            latency_ms: None,
            tokens: None,
            cost_microdollars: None,
            backend: None,
            metadata: Default::default(),
        };
        let envelope = ExperienceEventEnvelope::new("evt-1", event)
            .with_tenant("tenant'; DROP TABLE event;--");
        let result = store.append_event(envelope);
        assert!(result.is_err());
    }

    #[test]
    fn store_accepts_valid_event() {
        let store = InMemoryExperienceStore::new();
        let event = ExperienceEvent::OutcomeRecorded {
            chain_id: "chain-1".to_string(),
            step: DecisionStep::Planning,
            passed: true,
            stop_reason: None,
            latency_ms: None,
            tokens: None,
            cost_microdollars: None,
            backend: None,
            metadata: Default::default(),
        };
        let envelope = ExperienceEventEnvelope::new("evt-valid-123", event)
            .with_tenant("tenant-a")
            .with_correlation("corr-xyz");
        assert!(store.append_event(envelope).is_ok());
    }
}

#[cfg(test)]
mod proptest_tests {
    use super::*;
    use converge_core::{DecisionStep, ExperienceEventEnvelope, ExperienceEventKind};
    use proptest::prelude::*;

    /// Generate a valid identifier (alphanumeric + dash/underscore/dot, 1-64 chars).
    fn valid_id() -> impl Strategy<Value = String> {
        proptest::string::string_regex("[a-zA-Z0-9][a-zA-Z0-9._-]{0,63}").expect("valid regex")
    }

    /// Generate a valid chain ID.
    fn valid_chain_id() -> impl Strategy<Value = String> {
        valid_id().prop_map(|s| format!("chain-{s}"))
    }

    /// Generate a DecisionStep.
    fn arb_step() -> impl Strategy<Value = DecisionStep> {
        prop_oneof![
            Just(DecisionStep::Reasoning),
            Just(DecisionStep::Evaluation),
            Just(DecisionStep::Planning),
        ]
    }

    fn arb_metadata_key() -> impl Strategy<Value = String> {
        proptest::string::string_regex("[a-z][a-z0-9._-]{0,31}").expect("valid regex")
    }

    fn arb_metadata() -> impl Strategy<Value = std::collections::HashMap<String, String>> {
        proptest::collection::hash_map(arb_metadata_key(), valid_id(), 0..5)
    }

    /// Generate a simple OutcomeRecorded event (most common variant for store tests).
    fn arb_outcome_event() -> impl Strategy<Value = ExperienceEvent> {
        (valid_chain_id(), arb_step(), any::<bool>(), arb_metadata()).prop_map(
            |(chain_id, step, passed, metadata)| ExperienceEvent::OutcomeRecorded {
                chain_id,
                step,
                passed,
                stop_reason: None,
                latency_ms: None,
                tokens: None,
                cost_microdollars: None,
                backend: None,
                metadata,
            },
        )
    }

    /// Generate a BudgetExceeded event.
    fn arb_budget_event() -> impl Strategy<Value = ExperienceEvent> {
        (valid_chain_id(), valid_id()).prop_map(|(chain_id, resource)| {
            ExperienceEvent::BudgetExceeded {
                chain_id,
                resource,
                limit: "100".to_string(),
                observed: Some("150".to_string()),
            }
        })
    }

    /// Generate either an OutcomeRecorded or BudgetExceeded event.
    fn arb_event() -> impl Strategy<Value = ExperienceEvent> {
        prop_oneof![arb_outcome_event(), arb_budget_event(),]
    }

    /// Generate a valid envelope with optional tenant and correlation.
    fn arb_envelope() -> impl Strategy<Value = ExperienceEventEnvelope> {
        (
            valid_id(),
            arb_event(),
            proptest::option::of(valid_id()),
            proptest::option::of(valid_id()),
        )
            .prop_map(|(event_id, event, tenant, corr)| {
                let mut env = ExperienceEventEnvelope::new(event_id, event);
                if let Some(t) = tenant {
                    env = env.with_tenant(t);
                }
                if let Some(c) = corr {
                    env = env.with_correlation(c);
                }
                env
            })
    }

    // Property 1: Every appended event is retrievable via unfiltered query.
    proptest! {
        #[test]
        fn append_query_roundtrip(events in proptest::collection::vec(arb_envelope(), 1..20)) {
            let store = InMemoryExperienceStore::new();
            let count = events.len();
            for e in events {
                store.append_event(e).expect("append should succeed");
            }
            let all = store.query_events(&EventQuery::default()).expect("query");
            prop_assert_eq!(all.len(), count);
        }
    }

    // Property 2: Events are returned in insertion order.
    proptest! {
        #[test]
        fn ordering_preserved(events in proptest::collection::vec(arb_envelope(), 2..15)) {
            let store = InMemoryExperienceStore::new();
            let ids: Vec<String> = events.iter().map(|e| e.event_id.clone()).collect();
            for e in events {
                store.append_event(e).expect("append");
            }
            let results = store.query_events(&EventQuery::default()).expect("query");
            let result_ids: Vec<String> = results.iter().map(|e| e.event_id.clone()).collect();
            prop_assert_eq!(ids, result_ids);
        }
    }

    // Property 3: Tenant isolation — querying tenant A never returns tenant B events.
    proptest! {
        #[test]
        fn tenant_isolation(
            a_events in proptest::collection::vec(arb_outcome_event(), 1..10),
            b_events in proptest::collection::vec(arb_outcome_event(), 1..10),
        ) {
            let store = InMemoryExperienceStore::new();
            let a_count = a_events.len();
            let b_count = b_events.len();

            for (i, e) in a_events.into_iter().enumerate() {
                let env = ExperienceEventEnvelope::new(format!("a-{i}"), e)
                    .with_tenant("tenant-alpha");
                store.append_event(env).expect("append a");
            }
            for (i, e) in b_events.into_iter().enumerate() {
                let env = ExperienceEventEnvelope::new(format!("b-{i}"), e)
                    .with_tenant("tenant-beta");
                store.append_event(env).expect("append b");
            }

            let qa = EventQuery { tenant_id: Some("tenant-alpha".to_string()), ..Default::default() };
            let qb = EventQuery { tenant_id: Some("tenant-beta".to_string()), ..Default::default() };
            let ra = store.query_events(&qa).expect("query a");
            let rb = store.query_events(&qb).expect("query b");

            prop_assert_eq!(ra.len(), a_count);
            prop_assert_eq!(rb.len(), b_count);
            prop_assert!(ra.iter().all(|e| e.tenant_id.as_deref() == Some("tenant-alpha")));
            prop_assert!(rb.iter().all(|e| e.tenant_id.as_deref() == Some("tenant-beta")));
        }
    }

    // Property 4: Kind filter returns only matching event kinds.
    proptest! {
        #[test]
        fn kind_filtering(
            outcomes in proptest::collection::vec(arb_outcome_event(), 1..10),
            budgets in proptest::collection::vec(arb_budget_event(), 1..10),
        ) {
            let store = InMemoryExperienceStore::new();
            let outcome_count = outcomes.len();
            let budget_count = budgets.len();

            for (i, e) in outcomes.into_iter().enumerate() {
                let env = ExperienceEventEnvelope::new(format!("out-{i}"), e);
                store.append_event(env).expect("append outcome");
            }
            for (i, e) in budgets.into_iter().enumerate() {
                let env = ExperienceEventEnvelope::new(format!("bud-{i}"), e);
                store.append_event(env).expect("append budget");
            }

            let q_out = EventQuery {
                kinds: vec![ExperienceEventKind::OutcomeRecorded],
                ..Default::default()
            };
            let q_bud = EventQuery {
                kinds: vec![ExperienceEventKind::BudgetExceeded],
                ..Default::default()
            };

            let r_out = store.query_events(&q_out).expect("query outcomes");
            let r_bud = store.query_events(&q_bud).expect("query budgets");

            prop_assert_eq!(r_out.len(), outcome_count);
            prop_assert_eq!(r_bud.len(), budget_count);
        }
    }

    // Property 5: Query limit caps result count.
    proptest! {
        #[test]
        fn limit_enforced(
            events in proptest::collection::vec(arb_envelope(), 5..20),
            limit in 1usize..5,
        ) {
            let store = InMemoryExperienceStore::new();
            for e in events {
                store.append_event(e).expect("append");
            }
            let q = EventQuery { limit: Some(limit), ..Default::default() };
            let results = store.query_events(&q).expect("query");
            prop_assert!(results.len() <= limit);
        }
    }

    // Property 6: Correlation ID filtering is precise.
    proptest! {
        #[test]
        fn correlation_filtering(
            target_events in proptest::collection::vec(arb_outcome_event(), 1..8),
            other_events in proptest::collection::vec(arb_outcome_event(), 1..8),
        ) {
            let store = InMemoryExperienceStore::new();
            let target_count = target_events.len();

            for (i, e) in target_events.into_iter().enumerate() {
                let env = ExperienceEventEnvelope::new(format!("t-{i}"), e)
                    .with_correlation("corr-target");
                store.append_event(env).expect("append target");
            }
            for (i, e) in other_events.into_iter().enumerate() {
                let env = ExperienceEventEnvelope::new(format!("o-{i}"), e)
                    .with_correlation("corr-other");
                store.append_event(env).expect("append other");
            }

            let q = EventQuery {
                correlation_id: Some("corr-target".to_string()),
                ..Default::default()
            };
            let results = store.query_events(&q).expect("query");
            prop_assert_eq!(results.len(), target_count);
            prop_assert!(results.iter().all(|e| e.correlation_id.as_deref() == Some("corr-target")));
        }
    }

    // Property 7: Metadata survives store roundtrip — arbitrary key-value pairs
    // appended to OutcomeRecorded events are preserved through write and query.
    proptest! {
        #[test]
        fn metadata_roundtrip(
            events in proptest::collection::vec(
                (valid_id(), arb_outcome_event()),
                1..10,
            ),
        ) {
            let store = InMemoryExperienceStore::new();
            let mut expected: Vec<(String, std::collections::HashMap<String, String>)> = Vec::new();

            for (id, event) in &events {
                let meta = match event {
                    ExperienceEvent::OutcomeRecorded { metadata, .. } => metadata.clone(),
                    _ => std::collections::HashMap::new(),
                };
                expected.push((id.clone(), meta));
                let env = ExperienceEventEnvelope::new(id.clone(), event.clone());
                store.append_event(env).expect("append");
            }

            let results = store.query_events(&EventQuery::default()).expect("query");
            prop_assert_eq!(results.len(), expected.len());

            for (result, (_, expected_meta)) in results.iter().zip(expected.iter()) {
                if let ExperienceEvent::OutcomeRecorded { metadata, .. } = &result.event {
                    prop_assert_eq!(metadata, expected_meta);
                }
            }
        }
    }

    // Property 8: Metadata serialization roundtrip — OutcomeRecorded with
    // arbitrary metadata survives JSON serialize/deserialize.
    proptest! {
        #[test]
        fn metadata_serde_roundtrip(meta in arb_metadata()) {
            let event = ExperienceEvent::OutcomeRecorded {
                chain_id: "chain-serde".to_string(),
                step: DecisionStep::Planning,
                passed: true,
                stop_reason: None,
                latency_ms: None,
                tokens: None,
                cost_microdollars: None,
                backend: None,
                metadata: meta.clone(),
            };
            let json = serde_json::to_string(&event).expect("serialize");
            let back: ExperienceEvent = serde_json::from_str(&json).expect("deserialize");
            if let ExperienceEvent::OutcomeRecorded { metadata, .. } = back {
                prop_assert_eq!(metadata, meta);
            } else {
                prop_assert!(false, "wrong variant after deserialize");
            }
        }
    }
}
