// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Streaming callback for real-time fact emission.
//!
//! This module provides a `StreamingCallback` implementation that bridges
//! the converge-core engine to SSE/gRPC streaming endpoints.
//!
//! # Architecture
//!
//! ```text
//! Engine → StreamingCallback → EventSender → SSE/gRPC Channel
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use converge_core::{ContextFact, StreamingCallback};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

/// Event types emitted during convergence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamingEvent {
    /// A new fact was produced.
    Fact {
        sequence: u64,
        cycle: u32,
        key: String,
        id: String,
        content: String,
        timestamp_ns: u64,
    },
    /// A cycle started.
    CycleStart {
        sequence: u64,
        cycle: u32,
        timestamp_ns: u64,
    },
    /// A cycle ended.
    CycleEnd {
        sequence: u64,
        cycle: u32,
        facts_added: usize,
        timestamp_ns: u64,
    },
    /// Convergence completed.
    Converged {
        sequence: u64,
        cycles: u32,
        total_facts: usize,
        duration_ms: u64,
        timestamp_ns: u64,
    },
    /// Run halted due to error or invariant violation.
    Halted {
        sequence: u64,
        cycles: u32,
        reason: String,
        timestamp_ns: u64,
    },
}

/// Sender for streaming events.
pub type EventSender = mpsc::Sender<StreamingEvent>;

/// Receiver for streaming events.
pub type EventReceiver = mpsc::Receiver<StreamingEvent>;

/// Streaming callback that sends events to a channel.
///
/// This implements `StreamingCallback` from converge-core and forwards
/// all events to an async channel that can be consumed by SSE/gRPC handlers.
pub struct RuntimeStreamingCallback {
    /// Event sender.
    sender: EventSender,
    /// Monotonic sequence counter.
    sequence: AtomicU64,
    /// Current cycle.
    current_cycle: AtomicU32,
    /// Total facts emitted.
    fact_count: AtomicUsize,
    /// Start time (nanos).
    start_time_ns: u64,
}

impl RuntimeStreamingCallback {
    /// Create a new streaming callback with the given sender.
    pub fn new(sender: EventSender) -> Self {
        Self {
            sender,
            sequence: AtomicU64::new(0),
            current_cycle: AtomicU32::new(0),
            fact_count: AtomicUsize::new(0),
            start_time_ns: Self::now_ns(),
        }
    }

    /// Create a new streaming callback and return both the callback and receiver.
    pub fn channel(buffer_size: usize) -> (Arc<Self>, EventReceiver) {
        let (sender, receiver) = mpsc::channel(buffer_size);
        let callback = Arc::new(Self::new(sender));
        (callback, receiver)
    }

    /// Get the next sequence number.
    fn next_sequence(&self) -> u64 {
        self.sequence.fetch_add(1, Ordering::SeqCst)
    }

    /// Get current time in nanoseconds.
    fn now_ns() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }

    /// Get the total number of facts emitted.
    pub fn fact_count(&self) -> usize {
        self.fact_count.load(Ordering::SeqCst)
    }

    /// Get the current sequence number.
    pub fn current_sequence(&self) -> u64 {
        self.sequence.load(Ordering::SeqCst)
    }

    /// Emit a convergence completed event.
    pub fn emit_converged(&self, cycles: u32, total_facts: usize) {
        let duration_ms = (Self::now_ns() - self.start_time_ns) / 1_000_000;
        let event = StreamingEvent::Converged {
            sequence: self.next_sequence(),
            cycles,
            total_facts,
            duration_ms,
            timestamp_ns: Self::now_ns(),
        };
        let _ = self.sender.try_send(event);
    }

    /// Emit a halted event.
    pub fn emit_halted(&self, cycles: u32, reason: String) {
        let event = StreamingEvent::Halted {
            sequence: self.next_sequence(),
            cycles,
            reason,
            timestamp_ns: Self::now_ns(),
        };
        let _ = self.sender.try_send(event);
    }
}

impl StreamingCallback for RuntimeStreamingCallback {
    fn on_cycle_start(&self, cycle: u32) {
        self.current_cycle.store(cycle, Ordering::SeqCst);
        let event = StreamingEvent::CycleStart {
            sequence: self.next_sequence(),
            cycle,
            timestamp_ns: Self::now_ns(),
        };
        // Use try_send to avoid blocking the engine
        let _ = self.sender.try_send(event);
    }

    fn on_fact(&self, cycle: u32, fact: &ContextFact) {
        self.fact_count.fetch_add(1, Ordering::SeqCst);
        let event = StreamingEvent::Fact {
            sequence: self.next_sequence(),
            cycle,
            key: format!("{:?}", fact.key()),
            id: fact.id().to_string(),
            content: fact.content().to_string(),
            timestamp_ns: Self::now_ns(),
        };
        let _ = self.sender.try_send(event);
    }

    fn on_cycle_end(&self, cycle: u32, facts_added: usize) {
        let event = StreamingEvent::CycleEnd {
            sequence: self.next_sequence(),
            cycle,
            facts_added,
            timestamp_ns: Self::now_ns(),
        };
        let _ = self.sender.try_send(event);
    }
}

/// Convert a streaming event to SSE format.
impl StreamingEvent {
    /// Get the event type name for SSE.
    pub fn event_type(&self) -> &'static str {
        match self {
            StreamingEvent::Fact { .. } => "fact",
            StreamingEvent::CycleStart { .. } => "cycle_start",
            StreamingEvent::CycleEnd { .. } => "cycle_end",
            StreamingEvent::Converged { .. } => "converged",
            StreamingEvent::Halted { .. } => "halted",
        }
    }

    /// Get the sequence number.
    pub fn sequence(&self) -> u64 {
        match self {
            StreamingEvent::Fact { sequence, .. }
            | StreamingEvent::CycleStart { sequence, .. }
            | StreamingEvent::CycleEnd { sequence, .. }
            | StreamingEvent::Converged { sequence, .. }
            | StreamingEvent::Halted { sequence, .. } => *sequence,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use converge_core::{ContextKey, ContextState, Engine};

    async fn promoted_fact(key: ContextKey, id: &str, content: &str) -> ContextFact {
        let mut ctx = ContextState::new();
        let _ = ctx.add_input(key, id, content);
        Engine::new()
            .run(ctx)
            .await
            .expect("should promote test input")
            .context
            .get(key)
            .first()
            .expect("promoted fact should exist")
            .clone()
    }

    #[tokio::test]
    async fn test_streaming_callback_emits_facts() {
        let (callback, mut receiver) = RuntimeStreamingCallback::channel(100);

        let fact = promoted_fact(ContextKey::Seeds, "test-fact", "Test content").await;

        callback.on_fact(1, &fact);

        let event = receiver.recv().await.unwrap();
        match event {
            StreamingEvent::Fact {
                id, content, cycle, ..
            } => {
                assert_eq!(id, "test-fact");
                assert_eq!(content, "Test content");
                assert_eq!(cycle, 1);
            }
            _ => panic!("Expected Fact event"),
        }
    }

    #[tokio::test]
    async fn test_streaming_callback_emits_cycle_events() {
        let (callback, mut receiver) = RuntimeStreamingCallback::channel(100);

        callback.on_cycle_start(1);
        callback.on_cycle_end(1, 5);

        let start_event = receiver.recv().await.unwrap();
        assert!(matches!(
            start_event,
            StreamingEvent::CycleStart { cycle: 1, .. }
        ));

        let end_event = receiver.recv().await.unwrap();
        match end_event {
            StreamingEvent::CycleEnd {
                cycle, facts_added, ..
            } => {
                assert_eq!(cycle, 1);
                assert_eq!(facts_added, 5);
            }
            _ => panic!("Expected CycleEnd event"),
        }
    }

    #[tokio::test]
    async fn test_streaming_callback_sequence_increments() {
        let (callback, mut receiver) = RuntimeStreamingCallback::channel(100);

        callback.on_cycle_start(1);
        callback.on_cycle_start(2);
        callback.on_cycle_start(3);

        let e1 = receiver.recv().await.unwrap();
        let e2 = receiver.recv().await.unwrap();
        let e3 = receiver.recv().await.unwrap();

        assert_eq!(e1.sequence(), 0);
        assert_eq!(e2.sequence(), 1);
        assert_eq!(e3.sequence(), 2);
    }

    #[test]
    fn test_event_type() {
        let fact_event = StreamingEvent::Fact {
            sequence: 0,
            cycle: 1,
            key: "Seeds".to_string(),
            id: "test".to_string(),
            content: "content".to_string(),
            timestamp_ns: 0,
        };
        assert_eq!(fact_event.event_type(), "fact");

        let converged_event = StreamingEvent::Converged {
            sequence: 0,
            cycles: 5,
            total_facts: 10,
            duration_ms: 100,
            timestamp_ns: 0,
        };
        assert_eq!(converged_event.event_type(), "converged");
    }
}
