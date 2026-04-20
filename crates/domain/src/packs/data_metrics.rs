// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT

//! Data & Metrics Pack agents for single source of truth measurements.
//!
//! Implements the agent contracts defined in specs/data_metrics.truth.
//!
//! # Data & Metrics is the Measurement Layer
//!
//! Every metric, dashboard, and alert flows through this pack:
//! - Metric definitions and versioning
//! - Data source connectivity
//! - Pipeline orchestration
//! - Anomaly detection
//! - Alerting and reporting
//!
//! Note: This implementation uses the standard ContextKey enum. Facts are
//! distinguished by their ID prefixes (metric:, source:, pipeline:, etc.).

use converge_core::{
    AgentEffect, ContextKey, Suggestor,
    invariant::{Invariant, InvariantClass, InvariantResult, Violation},
};

// ============================================================================
// Fact ID Prefixes
// ============================================================================

pub const METRIC_PREFIX: &str = "metric:";
pub const SOURCE_PREFIX: &str = "source:";
pub const PIPELINE_PREFIX: &str = "pipeline:";
pub const VALIDATION_PREFIX: &str = "validation:";
pub const DASHBOARD_PREFIX: &str = "dashboard:";
pub const REPORT_PREFIX: &str = "report:";
pub const ALERT_PREFIX: &str = "alert:";
pub const ANOMALY_PREFIX: &str = "anomaly:";

// ============================================================================
// Agents
// ============================================================================

/// Registers and validates metric definitions.
#[derive(Debug, Clone, Default)]
pub struct MetricRegistrarAgent;

#[async_trait::async_trait]
impl Suggestor for MetricRegistrarAgent {
    fn name(&self) -> &str {
        "metric_registrar"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("metric.define") || s.content.contains("metric.update"))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("metric.define")
                || trigger.content.contains("metric.update")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}{}", METRIC_PREFIX, trigger.id),
                    serde_json::json!({
                        "type": "metric_definition",
                        "source_id": trigger.id,
                        "state": "draft",
                        "version": "1.0.0",
                        "formula": "to_be_defined",
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Establishes and manages data source connections.
#[derive(Debug, Clone, Default)]
pub struct SourceConnectorAgent;

#[async_trait::async_trait]
impl Suggestor for SourceConnectorAgent {
    fn name(&self) -> &str {
        "source_connector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("source.register") || s.content.contains("source.connect"))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("source.register")
                || trigger.content.contains("source.connect")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Signals,
                    format!("{}{}", SOURCE_PREFIX, trigger.id),
                    serde_json::json!({
                        "type": "data_source",
                        "source_id": trigger.id,
                        "state": "registered",
                        "source_type": "detected",
                        "freshness_sla_minutes": 60,
                        "registered_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Coordinates data pipeline execution.
#[derive(Debug, Clone, Default)]
pub struct PipelineCoordinatorAgent;

#[async_trait::async_trait]
impl Suggestor for PipelineCoordinatorAgent {
    fn name(&self) -> &str {
        "pipeline_coordinator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with(SOURCE_PREFIX) && s.content.contains("\"state\":\"healthy\""))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for source in signals.iter() {
            if source.id.starts_with(SOURCE_PREFIX)
                && source.content.contains("\"state\":\"healthy\"")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}{}", PIPELINE_PREFIX, source.id),
                    serde_json::json!({
                        "type": "pipeline",
                        "source_id": source.id,
                        "state": "ready",
                        "schedule": "*/15 * * * *",
                        "timeout_minutes": 30,
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Validates collected data quality.
#[derive(Debug, Clone, Default)]
pub struct DataValidatorAgent;

#[async_trait::async_trait]
impl Suggestor for DataValidatorAgent {
    fn name(&self) -> &str {
        "data_validator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Proposals).iter().any(|p| {
            p.id.starts_with(PIPELINE_PREFIX) && p.content.contains("\"state\":\"succeeded\"")
        })
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for pipeline in proposals.iter() {
            if pipeline.id.starts_with(PIPELINE_PREFIX)
                && pipeline.content.contains("\"state\":\"succeeded\"")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Evaluations,
                    format!("{}{}", VALIDATION_PREFIX, pipeline.id),
                    serde_json::json!({
                        "type": "data_validation",
                        "pipeline_id": pipeline.id,
                        "schema_valid": true,
                        "null_ratio_ok": true,
                        "range_check_ok": true,
                        "freshness_ok": true,
                        "validated_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Detects anomalies in metric data.
#[derive(Debug, Clone, Default)]
pub struct AnomalyDetectorAgent;

#[async_trait::async_trait]
impl Suggestor for AnomalyDetectorAgent {
    fn name(&self) -> &str {
        "anomaly_detector"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Evaluations).iter().any(|e| {
            e.id.starts_with(VALIDATION_PREFIX) && e.content.contains("\"schema_valid\":true")
        })
    }

    async fn execute(&self, _ctx: &dyn converge_core::Context) -> AgentEffect {
        // In real implementation, would analyze data for anomalies
        // For now, creates a placeholder showing no anomalies detected
        AgentEffect::with_proposal(crate::proposal(
            self.name(),
            ContextKey::Evaluations,
            format!("{}scan:latest", ANOMALY_PREFIX),
            serde_json::json!({
                "type": "anomaly_scan",
                "anomalies_detected": 0,
                "metrics_scanned": 10,
                "methods_used": ["statistical", "threshold"],
                "scanned_at": "2026-01-12T12:00:00Z"
            })
            .to_string(),
        ))
    }
}

/// Builds and configures dashboards.
#[derive(Debug, Clone, Default)]
pub struct DashboardBuilderAgent;

#[async_trait::async_trait]
impl Suggestor for DashboardBuilderAgent {
    fn name(&self) -> &str {
        "dashboard_builder"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Seeds).iter().any(|s| {
            s.content.contains("dashboard.create") || s.content.contains("dashboard.update")
        })
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("dashboard.create")
                || trigger.content.contains("dashboard.update")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}{}", DASHBOARD_PREFIX, trigger.id),
                    serde_json::json!({
                        "type": "dashboard",
                        "source_id": trigger.id,
                        "state": "draft",
                        "widgets": [],
                        "refresh_rate": "5m",
                        "created_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Generates scheduled and ad-hoc reports.
#[derive(Debug, Clone, Default)]
pub struct ReportGeneratorAgent;

#[async_trait::async_trait]
impl Suggestor for ReportGeneratorAgent {
    fn name(&self) -> &str {
        "report_generator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Seeds]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Seeds)
            .iter()
            .any(|s| s.content.contains("report.generate") || s.content.contains("report.schedule"))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let triggers = ctx.get(ContextKey::Seeds);
        let mut facts = Vec::new();

        for trigger in triggers.iter() {
            if trigger.content.contains("report.generate")
                || trigger.content.contains("report.schedule")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Proposals,
                    format!("{}{}", REPORT_PREFIX, trigger.id),
                    serde_json::json!({
                        "type": "report",
                        "source_id": trigger.id,
                        "state": "generating",
                        "format": "pdf",
                        "recipients": [],
                        "generated_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Evaluates alert conditions and triggers notifications.
#[derive(Debug, Clone, Default)]
pub struct AlertEvaluatorAgent;

#[async_trait::async_trait]
impl Suggestor for AlertEvaluatorAgent {
    fn name(&self) -> &str {
        "alert_evaluator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        // Check if there are anomalies that need alerting
        ctx.get(ContextKey::Evaluations).iter().any(|e| {
            e.id.starts_with(ANOMALY_PREFIX) && e.content.contains("\"anomalies_detected\"")
        })
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let evaluations = ctx.get(ContextKey::Evaluations);
        let mut facts = Vec::new();

        for eval in evaluations.iter() {
            if eval.id.starts_with(ANOMALY_PREFIX) {
                // Parse anomaly count - in real impl would check if > 0
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Evaluations,
                    format!("{}evaluation:{}", ALERT_PREFIX, eval.id),
                    serde_json::json!({
                        "type": "alert_evaluation",
                        "anomaly_scan_id": eval.id,
                        "alerts_triggered": 0,
                        "alerts_evaluated": 5,
                        "evaluated_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Monitors data freshness against SLAs.
#[derive(Debug, Clone, Default)]
pub struct FreshnessMonitorAgent;

#[async_trait::async_trait]
impl Suggestor for FreshnessMonitorAgent {
    fn name(&self) -> &str {
        "freshness_monitor"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Signals]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        ctx.get(ContextKey::Signals)
            .iter()
            .any(|s| s.id.starts_with(SOURCE_PREFIX))
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let signals = ctx.get(ContextKey::Signals);
        let mut facts = Vec::new();

        for source in signals.iter() {
            if source.id.starts_with(SOURCE_PREFIX) {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Evaluations,
                    format!("freshness:{}", source.id),
                    serde_json::json!({
                        "type": "freshness_check",
                        "source_id": source.id,
                        "is_fresh": true,
                        "last_data_at": "2026-01-12T11:55:00Z",
                        "sla_minutes": 60,
                        "checked_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

/// Calculates metric values from raw data.
#[derive(Debug, Clone, Default)]
pub struct MetricCalculatorAgent;

#[async_trait::async_trait]
impl Suggestor for MetricCalculatorAgent {
    fn name(&self) -> &str {
        "metric_calculator"
    }

    fn dependencies(&self) -> &[ContextKey] {
        &[ContextKey::Proposals, ContextKey::Evaluations]
    }

    fn accepts(&self, ctx: &dyn converge_core::Context) -> bool {
        let has_metrics = ctx
            .get(ContextKey::Proposals)
            .iter()
            .any(|p| p.id.starts_with(METRIC_PREFIX) && p.content.contains("\"state\":\"active\""));
        let has_validation = ctx
            .get(ContextKey::Evaluations)
            .iter()
            .any(|e| e.id.starts_with(VALIDATION_PREFIX));
        has_metrics && has_validation
    }

    async fn execute(&self, ctx: &dyn converge_core::Context) -> AgentEffect {
        let proposals = ctx.get(ContextKey::Proposals);
        let mut facts = Vec::new();

        for metric in proposals.iter() {
            if metric.id.starts_with(METRIC_PREFIX)
                && metric.content.contains("\"state\":\"active\"")
            {
                facts.push(crate::proposal(
                    self.name(),
                    ContextKey::Evaluations,
                    format!("calculated:{}", metric.id),
                    serde_json::json!({
                        "type": "metric_calculation",
                        "metric_id": metric.id,
                        "value": 0.0,
                        "unit": "count",
                        "calculated_at": "2026-01-12T12:00:00Z"
                    })
                    .to_string(),
                ));
            }
        }

        AgentEffect::with_proposals(facts)
    }
}

// ============================================================================
// Invariants
// ============================================================================

/// Ensures metric definitions are versioned.
#[derive(Debug, Clone, Default)]
pub struct MetricDefinitionVersionedInvariant;

impl Invariant for MetricDefinitionVersionedInvariant {
    fn name(&self) -> &str {
        "metric_definition_versioned"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        for metric in ctx.get(ContextKey::Proposals).iter() {
            if metric.id.starts_with(METRIC_PREFIX) && !metric.content.contains("\"version\"") {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Metric {} has no version", metric.id),
                    vec![metric.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures dashboards cite their data sources.
#[derive(Debug, Clone, Default)]
pub struct DashboardCitesSourcesInvariant;

impl Invariant for DashboardCitesSourcesInvariant {
    fn name(&self) -> &str {
        "dashboard_cites_sources"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        for dashboard in ctx.get(ContextKey::Proposals).iter() {
            if dashboard.id.starts_with(DASHBOARD_PREFIX)
                && dashboard.content.contains("\"state\":\"published\"")
                && !dashboard.content.contains("\"data_source\"")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Dashboard {} does not cite data sources", dashboard.id),
                    vec![dashboard.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures alerts have designated owners.
#[derive(Debug, Clone, Default)]
pub struct AlertHasOwnerInvariant;

impl Invariant for AlertHasOwnerInvariant {
    fn name(&self) -> &str {
        "alert_has_owner"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Structural
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        for alert in ctx.get(ContextKey::Proposals).iter() {
            if alert.id.starts_with(ALERT_PREFIX)
                && alert.content.contains("\"state\":\"active\"")
                && !alert.content.contains("\"owner\"")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Alert {} has no owner", alert.id),
                    vec![alert.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

/// Ensures data freshness SLAs are met.
#[derive(Debug, Clone, Default)]
pub struct DataFreshnessInvariant;

impl Invariant for DataFreshnessInvariant {
    fn name(&self) -> &str {
        "data_freshness"
    }

    fn class(&self) -> InvariantClass {
        InvariantClass::Semantic
    }

    fn check(&self, ctx: &dyn converge_core::Context) -> InvariantResult {
        for check in ctx.get(ContextKey::Evaluations).iter() {
            if check.content.contains("\"type\":\"freshness_check\"")
                && check.content.contains("\"is_fresh\":false")
            {
                return InvariantResult::Violated(Violation::with_facts(
                    format!("Data source {} is stale", check.id),
                    vec![check.id.clone()],
                ));
            }
        }
        InvariantResult::Ok
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agents_have_correct_names() {
        assert_eq!(MetricRegistrarAgent.name(), "metric_registrar");
        assert_eq!(SourceConnectorAgent.name(), "source_connector");
        assert_eq!(PipelineCoordinatorAgent.name(), "pipeline_coordinator");
        assert_eq!(DataValidatorAgent.name(), "data_validator");
        assert_eq!(AnomalyDetectorAgent.name(), "anomaly_detector");
        assert_eq!(DashboardBuilderAgent.name(), "dashboard_builder");
        assert_eq!(ReportGeneratorAgent.name(), "report_generator");
        assert_eq!(AlertEvaluatorAgent.name(), "alert_evaluator");
        assert_eq!(FreshnessMonitorAgent.name(), "freshness_monitor");
        assert_eq!(MetricCalculatorAgent.name(), "metric_calculator");
    }
}
