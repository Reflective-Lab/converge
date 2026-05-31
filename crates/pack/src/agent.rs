// Copyright 2024-2026 Reflective Labs

// SPDX-License-Identifier: MIT

//! The Suggestor trait — the contract all suggestor implementations satisfy.
//!
//! # Why this shape?
//!
//! Suggestors in Converge are not actors, not services, not workflow steps.
//! They are pure functions over context: given the current state of shared
//! context, a suggestor decides whether to act (`accepts`) and what to
//! contribute (`execute`).
//!
//! This design makes suggestors:
//! - **Deterministic**: same context → same decision.
//! - **Composable**: suggestors don't know about each other, only about context.
//! - **Testable**: mock the context, assert the effect.
//!
//! # Critical rules
//!
//! - `accepts()` must be **pure** — no side effects, no I/O, no mutations.
//! - `execute()` is **read-only** — it reads context and returns an effect.
//! - Suggestors **never call other suggestors** — all communication via shared context.
//! - **Idempotency is context-based** — check for existing contributions in
//!   context, not internal state. Internal `has_run` flags violate the
//!   "context is the only shared state" axiom.

use crate::context::{Context, ContextKey};
use crate::effect::AgentEffect;
use crate::fact::Provenance;

/// The core suggestor contract.
///
/// Every suggestor in the Converge ecosystem implements this trait — whether
/// it wraps an LLM, a policy engine, an optimizer, analytics, knowledge
/// retrieval, or a simple rule.
///
/// The engine calls `accepts()` to determine eligibility, then `execute()`
/// to collect effects. Effects are merged by the engine in deterministic
/// registration order via [`crate::types::SuggestorId`].
///
/// # Async
///
/// `execute()` is async, allowing suggestors to call LLM providers, search
/// backends, and other I/O without blocking. The engine awaits each
/// suggestor and controls concurrency — suggestors don't need to manage
/// their own parallelism.
///
/// # Thread Safety
///
/// Suggestors must be `Send + Sync` because the engine may execute eligible
/// suggestors concurrently in the future.
#[async_trait::async_trait]
pub trait Suggestor: Send + Sync {
    /// Human-readable name, used for logging and provenance.
    ///
    /// Must be unique within a convergence run. Deterministic execution order
    /// is derived from registration order, not lexical name sorting.
    fn name(&self) -> &str;

    /// Context keys this suggestor reads from.
    ///
    /// The engine uses this to determine when a suggestor becomes eligible:
    /// a suggestor is a candidate when at least one of its dependency keys
    /// has been modified since the last cycle.
    fn dependencies(&self) -> &[ContextKey];

    /// Pure predicate: should this suggestor execute given the current context?
    ///
    /// # Contract
    ///
    /// - Must be **pure**: no side effects, no I/O, no state mutation.
    /// - Must be **deterministic**: same context → same answer.
    /// - Must check **idempotency via context**: look for your own
    ///   contributions in context (both `Proposals` and target key),
    ///   not internal flags.
    fn accepts(&self, ctx: &dyn Context) -> bool;

    /// Produce effects given the current context.
    ///
    /// # Contract
    ///
    /// - **Read-only**: do not mutate context. Return effects instead.
    /// - Effects are collected by the engine and merged after all
    ///   eligible suggestors have executed.
    /// - For LLM suggestors: emit `ProposedFact` to `ContextKey::Proposals`,
    ///   not directly to the target key.
    async fn execute(&self, ctx: &dyn Context) -> AgentEffect;

    /// Typed provenance for this suggestor's emitted facts and execution span.
    ///
    /// Used by both the engine's tracing layer and by `ProposedFact::new`
    /// callers that want to cite the suggestor as the proposal's origin. The
    /// emitted proposal still remains the authoritative audit boundary: every
    /// [`ProposedFact`](crate::ProposedFact) must carry a non-empty
    /// [`Provenance`](crate::Provenance).
    ///
    /// The default impl derives the provenance from [`name`](Self::name),
    /// which is sufficient for span labels and for proposal provenance when
    /// the suggestor's identity is exhausted by its name. Fact-emitting
    /// suggestors that carry richer identity claims (versioned model,
    /// capability scope, source URI) should override this with their
    /// crate-canonical typed provenance constructor:
    ///
    /// ```ignore
    /// fn provenance(&self) -> Provenance { ARBITER_PROVENANCE.clone() }
    /// ```
    ///
    /// Empty provenance on an emitted proposal is a kernel error; the
    /// default impl above never produces empty provenance as long as
    /// `name()` is non-empty.
    fn provenance(&self) -> Provenance {
        Provenance::new(self.name())
    }

    /// Algorithmic complexity of this suggestor's core computation.
    ///
    /// Returns a short string describing time complexity and practical scale
    /// guidance. `None` means negligible / not applicable (e.g. pure LLM
    /// calls where latency is network-bound, not algorithmic).
    ///
    /// Examples: `"O(n³) — n = agents/tasks, practical for n ≤ 500"`
    fn complexity_hint(&self) -> Option<&'static str> {
        None
    }
}
