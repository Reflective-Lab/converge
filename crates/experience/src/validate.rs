// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Input validation for experience events at the write boundary.
//!
//! All stores must call [`validate_envelope`] before persisting an event.
//! This prevents malformed identifiers from reaching storage backends
//! (SurrealDB record IDs, index keys, etc.).

use converge_core::{
    ExperienceEventEnvelope, ExperienceStoreError, ExperienceStoreResult,
    UserExperienceEventEnvelope,
};

/// Maximum length for identifier fields (event_id, tenant_id, correlation_id).
const MAX_ID_LEN: usize = 256;

/// Maximum length for the occurred_at timestamp field.
const MAX_TIMESTAMP_LEN: usize = 64;

/// Validate an event envelope before writing to any store.
///
/// Checks:
/// - `event_id` is non-empty and contains only safe identifier characters
/// - `occurred_at` is non-empty and within length bounds
/// - `tenant_id` (if present) contains only safe identifier characters
/// - `correlation_id` (if present) contains only safe identifier characters
pub fn validate_envelope(envelope: &ExperienceEventEnvelope) -> ExperienceStoreResult<()> {
    validate_id("event_id", &envelope.event_id)?;

    if envelope.occurred_at.is_empty() {
        return Err(ExperienceStoreError::InvalidQuery {
            message: "occurred_at must not be empty".to_string(),
        });
    }
    if envelope.occurred_at.len() > MAX_TIMESTAMP_LEN {
        return Err(ExperienceStoreError::InvalidQuery {
            message: format!(
                "occurred_at exceeds maximum length of {MAX_TIMESTAMP_LEN} characters"
            ),
        });
    }

    if let Some(ref tenant_id) = envelope.tenant_id {
        validate_id("tenant_id", tenant_id)?;
    }
    if let Some(ref correlation_id) = envelope.correlation_id {
        validate_id("correlation_id", correlation_id)?;
    }

    Ok(())
}

/// Validate a user-side event envelope before writing to any store.
pub fn validate_user_envelope(envelope: &UserExperienceEventEnvelope) -> ExperienceStoreResult<()> {
    validate_id("event_id", &envelope.event_id)?;

    if envelope.occurred_at.is_empty() {
        return Err(ExperienceStoreError::InvalidQuery {
            message: "occurred_at must not be empty".to_string(),
        });
    }
    if envelope.occurred_at.len() > MAX_TIMESTAMP_LEN {
        return Err(ExperienceStoreError::InvalidQuery {
            message: format!(
                "occurred_at exceeds maximum length of {MAX_TIMESTAMP_LEN} characters"
            ),
        });
    }

    if let Some(ref tenant_id) = envelope.tenant_id {
        validate_id("tenant_id", tenant_id)?;
    }
    if let Some(ref correlation_id) = envelope.correlation_id {
        validate_id("correlation_id", correlation_id)?;
    }

    Ok(())
}

/// Validate a string identifier field.
///
/// Identifiers must be non-empty, within [`MAX_ID_LEN`], and contain only
/// alphanumeric characters, hyphens, underscores, or dots. This prevents
/// injection via SurrealDB record IDs or other storage key formats.
fn validate_id(field: &str, value: &str) -> ExperienceStoreResult<()> {
    if value.is_empty() {
        return Err(ExperienceStoreError::InvalidQuery {
            message: format!("{field} must not be empty"),
        });
    }
    if value.len() > MAX_ID_LEN {
        return Err(ExperienceStoreError::InvalidQuery {
            message: format!("{field} exceeds maximum length of {MAX_ID_LEN} characters"),
        });
    }
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.')
    {
        return Err(ExperienceStoreError::InvalidQuery {
            message: format!(
                "{field} contains invalid characters; only alphanumeric, '-', '_', '.' are allowed"
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use converge_core::types::ActorId;
    use converge_core::{
        DecisionStep, ExperienceEvent, ExperienceEventEnvelope, FactId, OverrideTarget,
        UserExperienceEvent, UserExperienceEventEnvelope,
    };

    use super::*;

    fn make_envelope(event_id: &str) -> ExperienceEventEnvelope {
        let event = ExperienceEvent::OutcomeRecorded {
            chain_id: "chain-1".into(),
            step: DecisionStep::Planning,
            passed: true,
            stop_reason: None,
            latency_ms: None,
            tokens: None,
            cost_microdollars: None,
            backend: None,
            metadata: Default::default(),
        };
        ExperienceEventEnvelope::new(event_id, event)
    }

    #[test]
    fn valid_envelope_passes() {
        let envelope = make_envelope("evt-001");
        assert!(validate_envelope(&envelope).is_ok());
    }

    #[test]
    fn valid_envelope_with_dots_and_underscores() {
        let envelope = make_envelope("evt_001.abc");
        assert!(validate_envelope(&envelope).is_ok());
    }

    #[test]
    fn empty_event_id_rejected() {
        let envelope = make_envelope("");
        let err = validate_envelope(&envelope).unwrap_err();
        assert!(err.to_string().contains("event_id must not be empty"));
    }

    #[test]
    fn event_id_with_slashes_rejected() {
        let envelope = make_envelope("../../admin:hack");
        let err = validate_envelope(&envelope).unwrap_err();
        assert!(err.to_string().contains("invalid characters"));
    }

    #[test]
    fn event_id_with_spaces_rejected() {
        let envelope = make_envelope("evt 001");
        let err = validate_envelope(&envelope).unwrap_err();
        assert!(err.to_string().contains("invalid characters"));
    }

    #[test]
    fn event_id_with_colon_rejected() {
        let envelope = make_envelope("table:record");
        let err = validate_envelope(&envelope).unwrap_err();
        assert!(err.to_string().contains("invalid characters"));
    }

    #[test]
    fn oversized_event_id_rejected() {
        let long_id = "a".repeat(MAX_ID_LEN + 1);
        let envelope = make_envelope(&long_id);
        let err = validate_envelope(&envelope).unwrap_err();
        assert!(err.to_string().contains("exceeds maximum length"));
    }

    #[test]
    fn tenant_id_with_injection_rejected() {
        let mut envelope = make_envelope("evt-1");
        envelope.tenant_id = Some("tenant'; DROP TABLE event;--".into());
        let err = validate_envelope(&envelope).unwrap_err();
        assert!(err.to_string().contains("tenant_id"));
        assert!(err.to_string().contains("invalid characters"));
    }

    #[test]
    fn correlation_id_with_injection_rejected() {
        let mut envelope = make_envelope("evt-1");
        envelope.correlation_id = Some("corr\x00id".into());
        let err = validate_envelope(&envelope).unwrap_err();
        assert!(err.to_string().contains("correlation_id"));
    }

    #[test]
    fn valid_tenant_and_correlation_pass() {
        let envelope = make_envelope("evt-1")
            .with_tenant("tenant-abc")
            .with_correlation("corr-xyz-123");
        assert!(validate_envelope(&envelope).is_ok());
    }

    #[test]
    fn user_envelope_rejects_malformed_event_id() {
        let envelope = UserExperienceEventEnvelope::new(
            "../../admin:hack",
            UserExperienceEvent::UserOverrideIssued {
                target: OverrideTarget::Fact(FactId::new("fact-1")),
                actor: ActorId::new("actor-1"),
                policy_snapshot_hash: None,
                reason: "operator correction".into(),
            },
        );
        let err = validate_user_envelope(&envelope).unwrap_err();
        assert!(err.to_string().contains("invalid characters"));
    }
}
