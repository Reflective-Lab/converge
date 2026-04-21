// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Property-based and validation tests for Context and ID validation.
//!
//! These tests discover validation gaps and generate evidence for the typed-ID ADR.
//! Documents current behavior that should be tightened in a future ADR.

use converge_core::{AgentEffect, ContextKey, ContextState, Engine, ProposedFact, Suggestor};

// ─── Helper: Suggestor that proposes a single fact ───────────────────────────

struct SingleProposalSuggestor {
    key: ContextKey,
    id: String,
    content: String,
}

#[async_trait::async_trait]
impl Suggestor for SingleProposalSuggestor {
    fn name(&self) -> &str {
        "SingleProposal"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[]
    }

    fn accepts(&self, _ctx: &dyn converge_core::Context) -> bool {
        true
    }

    async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
        AgentEffect::with_proposal(ProposedFact::new(
            self.key,
            self.id.as_str(),
            &self.content,
            self.name(),
        ))
    }
}

// ─── Evidence Tests: Current Validation Gaps ─────────────────────────────────

/// Document current behavior: valid ID roundtrips through engine unchanged.
#[tokio::test]
async fn evidence_valid_id_roundtrips() {
    let mut engine = Engine::new();
    let test_id = "my-fact-1".to_string();
    let test_content = "test content".to_string();

    engine.register_suggestor(SingleProposalSuggestor {
        key: ContextKey::Seeds,
        id: test_id.clone(),
        content: test_content.clone(),
    });

    let result = engine.run(ContextState::new()).await;
    assert!(result.is_ok(), "engine should run successfully");

    let ctx = result.unwrap().context;
    let facts = ctx.get(ContextKey::Seeds);
    assert!(!facts.is_empty(), "fact should exist after engine run");

    let fact = facts
        .iter()
        .find(|f| f.id == test_id)
        .expect("should find fact");
    assert_eq!(&fact.id, &test_id, "ID should roundtrip unchanged");
    assert_eq!(
        &fact.content, &test_content,
        "content should roundtrip unchanged"
    );
}

/// Document current behavior: empty IDs are accepted (should be rejected).
#[tokio::test]
async fn evidence_empty_id_not_rejected() {
    let mut engine = Engine::new();
    engine.register_suggestor(SingleProposalSuggestor {
        key: ContextKey::Seeds,
        id: String::new(),
        content: "content".to_string(),
    });

    let result = engine.run(ContextState::new()).await;
    // Currently accepts empty ID; this should fail after typed-ID validation
    assert!(
        result.is_ok(),
        "Empty ID currently accepted (validation gap for ADR evidence)"
    );
}

/// Document current behavior: null bytes in ID are accepted (should be rejected).
#[tokio::test]
async fn evidence_null_byte_in_id_not_rejected() {
    let mut engine = Engine::new();
    let malformed_id = "valid-id\0injection".to_string();
    engine.register_suggestor(SingleProposalSuggestor {
        key: ContextKey::Seeds,
        id: malformed_id,
        content: "content".to_string(),
    });

    let result = engine.run(ContextState::new()).await;
    // Currently accepts null bytes; this is a security gap
    assert!(
        result.is_ok(),
        "Null byte in ID currently accepted (security gap)"
    );
}

/// Document current behavior: newlines in ID are accepted (should be rejected).
#[tokio::test]
async fn evidence_newline_in_id_not_rejected() {
    let mut engine = Engine::new();
    let malformed_id = "valid-id\nescaping".to_string();
    engine.register_suggestor(SingleProposalSuggestor {
        key: ContextKey::Seeds,
        id: malformed_id,
        content: "content".to_string(),
    });

    let result = engine.run(ContextState::new()).await;
    // Currently accepts newlines; might cause issues in logging/serialization
    assert!(
        result.is_ok(),
        "Newline in ID currently accepted (escaping gap)"
    );
}

/// Document current behavior: unbounded ID length is accepted (should have max).
#[tokio::test]
async fn evidence_very_long_id_not_rejected() {
    let mut engine = Engine::new();
    let oversized_id = "a".repeat(10_000);
    engine.register_suggestor(SingleProposalSuggestor {
        key: ContextKey::Seeds,
        id: oversized_id,
        content: "content".to_string(),
    });

    let result = engine.run(ContextState::new()).await;
    // Currently accepts very long IDs; should enforce max length (e.g., 256 chars)
    assert!(
        result.is_ok(),
        "Very long ID (10K chars) currently accepted (no max-length validation)"
    );
}

/// Document current behavior: whitespace-only IDs are accepted (should be rejected).
#[tokio::test]
async fn evidence_whitespace_only_id_not_rejected() {
    let mut engine = Engine::new();
    engine.register_suggestor(SingleProposalSuggestor {
        key: ContextKey::Seeds,
        id: "   ".to_string(),
        content: "content".to_string(),
    });

    let result = engine.run(ContextState::new()).await;
    // Currently accepts whitespace-only IDs
    assert!(
        result.is_ok(),
        "Whitespace-only ID currently accepted (validation gap)"
    );
}

/// Document current behavior: uppercase letters in ID are accepted (should be rejected).
#[tokio::test]
async fn evidence_uppercase_in_id_not_rejected() {
    let mut engine = Engine::new();
    engine.register_suggestor(SingleProposalSuggestor {
        key: ContextKey::Seeds,
        id: "MyFactID".to_string(),
        content: "content".to_string(),
    });

    let result = engine.run(ContextState::new()).await;
    // Currently accepts uppercase; spec requires lowercase
    assert!(
        result.is_ok(),
        "Uppercase in ID currently accepted (case normalization gap)"
    );
}

/// Document current behavior: different keys are independent namespaces.
#[tokio::test]
async fn evidence_different_keys_independent() {
    let mut engine = Engine::new();
    let test_id = "fact-1".to_string();

    engine.register_suggestor(SingleProposalSuggestor {
        key: ContextKey::Seeds,
        id: test_id.clone(),
        content: "seeds content".to_string(),
    });

    let result = engine.run(ContextState::new()).await;
    assert!(result.is_ok());

    let ctx = result.unwrap().context;
    assert_eq!(
        ctx.get(ContextKey::Seeds).len(),
        1,
        "Seeds should have 1 fact"
    );
    assert_eq!(
        ctx.get(ContextKey::Hypotheses).len(),
        0,
        "Hypotheses should be empty"
    );

    // Same ID in different key should succeed
    let mut engine2 = Engine::new();
    engine2.register_suggestor(SingleProposalSuggestor {
        key: ContextKey::Hypotheses,
        id: test_id.clone(),
        content: "hypotheses content".to_string(),
    });

    let result2 = engine2.run(ctx).await;
    assert!(result2.is_ok());
    let ctx2 = result2.unwrap().context;
    assert_eq!(
        ctx2.get(ContextKey::Seeds).len(),
        1,
        "Seeds still has 1 fact"
    );
    assert_eq!(
        ctx2.get(ContextKey::Hypotheses).len(),
        1,
        "Hypotheses now has 1 fact"
    );
}

/// Verify context version increments with each new fact.
#[tokio::test]
async fn regression_context_version_increments() {
    let mut engine = Engine::new();
    engine.register_suggestor(SingleProposalSuggestor {
        key: ContextKey::Seeds,
        id: "fact-1".to_string(),
        content: "content".to_string(),
    });

    let initial_ctx = ContextState::new();
    let initial_version = initial_ctx.version();

    let result = engine.run(initial_ctx).await;
    assert!(result.is_ok());

    let ctx = result.unwrap().context;
    assert!(
        ctx.version() > initial_version,
        "version should increment after adding fact"
    );
}
