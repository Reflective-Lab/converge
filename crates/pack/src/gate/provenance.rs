//! Provenance information for audit trail

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Provenance information for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEnvelope {
    /// Input data hash (for integrity verification)
    pub input_hash: String,
    /// Timestamp when problem was submitted
    #[serde(with = "system_time_serde")]
    pub submitted_at: SystemTime,
    /// Source system that created this problem
    pub source_system: String,
    /// User or service that submitted
    pub submitted_by: String,
    /// Correlation ID for distributed tracing
    pub correlation_id: String,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl Default for ProvenanceEnvelope {
    fn default() -> Self {
        Self {
            input_hash: String::new(),
            submitted_at: SystemTime::now(),
            source_system: String::new(),
            submitted_by: String::new(),
            correlation_id: String::new(),
            metadata: serde_json::Value::Null,
        }
    }
}

impl ProvenanceEnvelope {
    /// Create with minimal info
    pub fn new(source_system: impl Into<String>, submitted_by: impl Into<String>) -> Self {
        Self {
            source_system: source_system.into(),
            submitted_by: submitted_by.into(),
            ..Default::default()
        }
    }

    /// Set correlation ID
    pub fn with_correlation_id(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = id.into();
        self
    }

    /// Set input hash
    pub fn with_input_hash(mut self, hash: impl Into<String>) -> Self {
        self.input_hash = hash.into();
        self
    }

    /// Set metadata
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Link to kernel trace for replay/audit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelTraceLink {
    /// Trace ID
    pub trace_id: String,
    /// Whether this is for replay or audit-only
    pub mode: TraceMode,
    /// URL or path to trace data
    pub location: String,
}

impl Default for KernelTraceLink {
    fn default() -> Self {
        Self {
            trace_id: String::new(),
            mode: TraceMode::AuditOnly,
            location: String::new(),
        }
    }
}

impl KernelTraceLink {
    /// Create audit-only trace link
    pub fn audit_only(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            mode: TraceMode::AuditOnly,
            location: String::new(),
        }
    }

    /// Create replayable trace link
    pub fn replayable(trace_id: impl Into<String>, location: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            mode: TraceMode::Replayable,
            location: location.into(),
        }
    }

    /// Check if this trace supports replay
    pub fn is_replayable(&self) -> bool {
        self.mode == TraceMode::Replayable
    }
}

/// Trace mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceMode {
    /// Full replay capability
    Replayable,
    /// Audit/logging only
    AuditOnly,
}

/// Replay envelope for solver reproducibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayEnvelope {
    /// Hash of input data
    pub input_hash: String,
    /// Hash of output data
    pub output_hash: String,
    /// Seed used for this run
    pub seed: u64,
    /// Solver version
    pub solver_version: String,
    /// Pack version
    pub pack_version: String,
    /// Library version
    pub library_version: String,
}

impl ReplayEnvelope {
    /// Create for current library version
    pub fn new(
        input_hash: impl Into<String>,
        output_hash: impl Into<String>,
        seed: u64,
        solver_version: impl Into<String>,
        pack_version: impl Into<String>,
    ) -> Self {
        Self {
            input_hash: input_hash.into(),
            output_hash: output_hash.into(),
            seed,
            solver_version: solver_version.into(),
            pack_version: pack_version.into(),
            library_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Create a minimal replay envelope for testing
    pub fn minimal(seed: u64) -> Self {
        Self {
            input_hash: String::new(),
            output_hash: String::new(),
            seed,
            solver_version: "test".to_string(),
            pack_version: "test".to_string(),
            library_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Serde support for SystemTime as unix timestamp
mod system_time_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    pub fn serialize<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let duration = time.duration_since(UNIX_EPOCH).unwrap_or(Duration::ZERO);
        duration.as_secs_f64().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SystemTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = f64::deserialize(deserializer)?;
        Ok(UNIX_EPOCH + Duration::from_secs_f64(secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provenance_builder() {
        let prov = ProvenanceEnvelope::new("test-system", "test-user")
            .with_correlation_id("corr-123")
            .with_input_hash("sha256:abc");

        assert_eq!(prov.source_system, "test-system");
        assert_eq!(prov.submitted_by, "test-user");
        assert_eq!(prov.correlation_id, "corr-123");
        assert_eq!(prov.input_hash, "sha256:abc");
    }

    #[test]
    fn test_trace_link() {
        let audit = KernelTraceLink::audit_only("trace-001");
        assert!(!audit.is_replayable());

        let replay = KernelTraceLink::replayable("trace-002", "/traces/002.json");
        assert!(replay.is_replayable());
        assert_eq!(replay.location, "/traces/002.json");
    }

    #[test]
    fn test_replay_envelope() {
        let envelope = ReplayEnvelope::new("input-hash", "output-hash", 42, "greedy-v1", "1.0.0");
        assert_eq!(envelope.seed, 42);
        assert_eq!(envelope.solver_version, "greedy-v1");
    }

    #[test]
    fn test_serde_roundtrip() {
        let prov = ProvenanceEnvelope::new("system", "user");
        let json = serde_json::to_string(&prov).unwrap();
        let restored: ProvenanceEnvelope = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.source_system, prov.source_system);
    }
}
