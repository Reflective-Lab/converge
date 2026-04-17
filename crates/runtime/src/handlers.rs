// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! HTTP request handlers for Converge Runtime.

use axum::{
    Router,
    extract::{Json, Path, State},
    routing::{get, post},
};
use converge_core::{Context, ContextKey, Engine};
use serde::{Deserialize, Serialize};
use strum::IntoEnumIterator;
use tracing::{info, info_span};
use utoipa::ToSchema;

use crate::error::{RuntimeError, RuntimeErrorResponse};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormFieldInput {
    pub field_id: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub risk: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormPlanRequest {
    pub fields: Vec<FormFieldInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormPlanResponse {
    pub form_id: String,
    pub missing_fields: Vec<String>,
    pub high_risk_fields: Vec<String>,
    pub ready_for_submit: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormApprovalRequest {
    pub approvals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormApprovalResponse {
    pub form_id: String,
    pub approved: bool,
    pub approvals_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormExecuteRequest {
    pub plan: FormPlanResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FormExecuteResponse {
    pub form_id: String,
    pub receipt_id: String,
}

/// Request to create and run a job.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobRequest {
    /// Optional initial context data (for now, simplified).
    /// TODO: Replace with proper `RootIntent` when implemented.
    #[schema(example = json!({}))]
    pub context: Option<serde_json::Value>,
}

/// Response from a job execution.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobResponse {
    /// Execution metadata.
    pub metadata: JobMetadata,
    /// Number of cycles executed.
    pub cycles: u32,
    /// Whether convergence was reached.
    pub converged: bool,
    /// Final context summary (simplified for now).
    pub context_summary: ContextSummary,
}

/// Simplified context summary for API responses.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ContextSummary {
    /// Number of facts by key.
    pub fact_counts: std::collections::HashMap<String, usize>,
    /// Context version.
    pub version: u64,
}

/// Execution metadata.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct JobMetadata {
    /// Number of cycles executed.
    pub cycles: u32,
    /// Whether convergence was reached.
    pub converged: bool,
    /// Execution duration (milliseconds).
    pub duration_ms: u64,
}

fn field_is_high_risk(field_id: &str, risk: &Option<String>) -> bool {
    if matches!(risk.as_deref(), Some("high" | "critical")) {
        return true;
    }
    let lower = field_id.to_lowercase();
    ["ssn", "bank", "account", "passport", "tax", "salary"]
        .iter()
        .any(|keyword| lower.contains(keyword))
}

/// Health check endpoint.
///
/// Returns "ok" if the server is running.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Server is healthy", body = String)
    )
)]
pub async fn health() -> &'static str {
    "ok"
}

/// Readiness check endpoint.
///
/// Returns readiness status and service health.
#[utoipa::path(
    get,
    path = "/ready",
    tag = "health",
    responses(
        (status = 200, description = "Server is ready", body = serde_json::Value),
        (status = 503, description = "Server is not ready")
    )
)]
pub async fn ready() -> Result<Json<serde_json::Value>, RuntimeError> {
    // TODO: Check dependencies (SurrealDB, etc.) when added
    Ok(Json(serde_json::json!({
        "status": "ready",
        "services": {
            "engine": "ok"
        }
    })))
}

/// Build a form fill plan from provided field inputs.
#[utoipa::path(
    post,
    path = "/api/v1/forms/{form_id}/plan",
    tag = "forms",
    request_body = FormPlanRequest,
    responses(
        (status = 200, description = "Form plan generated", body = FormPlanResponse),
        (status = 400, description = "Invalid request", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
pub async fn plan_form(
    Path(form_id): Path<String>,
    Json(request): Json<FormPlanRequest>,
) -> Result<Json<FormPlanResponse>, RuntimeError> {
    let missing_fields = request
        .fields
        .iter()
        .filter(|field| field.required && field.value.as_deref().unwrap_or("").is_empty())
        .map(|field| field.field_id.clone())
        .collect::<Vec<_>>();

    let high_risk_fields = request
        .fields
        .iter()
        .filter(|field| field_is_high_risk(&field.field_id, &field.risk))
        .map(|field| field.field_id.clone())
        .collect::<Vec<_>>();

    let ready_for_submit = missing_fields.is_empty() && high_risk_fields.is_empty();
    Ok(Json(FormPlanResponse {
        form_id,
        missing_fields,
        high_risk_fields,
        ready_for_submit,
    }))
}

/// Record approvals for a form plan.
#[utoipa::path(
    post,
    path = "/api/v1/forms/{form_id}/approve",
    tag = "forms",
    request_body = FormApprovalRequest,
    responses(
        (status = 200, description = "Approvals recorded", body = FormApprovalResponse),
        (status = 400, description = "Invalid request", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
pub async fn approve_form(
    Path(form_id): Path<String>,
    Json(request): Json<FormApprovalRequest>,
) -> Result<Json<FormApprovalResponse>, RuntimeError> {
    let approvals_count = request.approvals.len();
    Ok(Json(FormApprovalResponse {
        form_id,
        approved: approvals_count > 0,
        approvals_count,
    }))
}

/// Execute a form submission (stubbed for now).
#[utoipa::path(
    post,
    path = "/api/v1/forms/{form_id}/execute",
    tag = "forms",
    request_body = FormExecuteRequest,
    responses(
        (status = 200, description = "Form executed", body = FormExecuteResponse),
        (status = 400, description = "Invalid request", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
pub async fn execute_form(
    Path(form_id): Path<String>,
    Json(_request): Json<FormExecuteRequest>,
) -> Result<Json<FormExecuteResponse>, RuntimeError> {
    let receipt_id = format!("receipt:{}", uuid::Uuid::new_v4());
    Ok(Json(FormExecuteResponse {
        form_id,
        receipt_id,
    }))
}

/// Prometheus metrics endpoint.
///
/// Returns metrics in Prometheus text exposition format.
#[cfg(feature = "metrics")]
#[utoipa::path(
    get,
    path = "/metrics",
    tag = "health",
    responses(
        (status = 200, description = "Prometheus metrics", content_type = "text/plain")
    )
)]
pub async fn metrics() -> (
    axum::http::StatusCode,
    [(axum::http::header::HeaderName, &'static str); 1],
    String,
) {
    let metrics_text = crate::metrics::render();
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        metrics_text,
    )
}

/// Handle job submission.
///
/// Submits a new job to the Converge engine and runs it until convergence.
#[utoipa::path(
    post,
    path = "/api/v1/jobs",
    tag = "jobs",
    request_body = JobRequest,
    responses(
        (status = 200, description = "Job completed successfully", body = JobResponse),
        (status = 400, description = "Invalid request", body = RuntimeErrorResponse),
        (status = 422, description = "Invariant violation", body = RuntimeErrorResponse),
        (status = 413, description = "Budget exhausted", body = RuntimeErrorResponse),
        (status = 409, description = "Conflict detected", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn handle_job(
    Json(request): Json<JobRequest>,
) -> Result<Json<JobResponse>, RuntimeError> {
    let _span = info_span!("handle_job");
    let _guard = _span.enter();
    info!("Received job request");

    let start = std::time::Instant::now();

    // Extract request data
    let context_data = request.context.clone();

    // Drop the span guard before await (it's not Send)
    drop(_guard);

    let mut engine = Engine::new();

    // TODO: Register agents based on request or configuration
    // For now, create a minimal engine

    // Create context from request or use empty
    // TODO: Properly deserialize RootIntent and create Context
    // For now, use empty context
    let _context_data = context_data;
    let context = Context::new();

    let result = engine.run(context).await.map_err(RuntimeError::Converge)?;

    let duration = start.elapsed();

    // Build context summary
    let fact_counts: std::collections::HashMap<String, usize> = ContextKey::iter()
        .map(|key| {
            let count = result.context.get(key).len();
            (format!("{key:?}"), count)
        })
        .collect();

    let context_summary = ContextSummary {
        fact_counts,
        version: result.context.version(),
    };

    info!(
        cycles = result.cycles,
        converged = result.converged,
        duration_ms = duration.as_millis(),
        "Job completed"
    );

    Ok(Json(JobResponse {
        metadata: JobMetadata {
            cycles: result.cycles,
            converged: result.converged,
            duration_ms: duration.as_millis() as u64,
        },
        cycles: result.cycles,
        converged: result.converged,
        context_summary,
    }))
}

/// Request to validate Converge Rules.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidateRulesRequest {
    /// The Converge Rules content (Gherkin format).
    #[schema(example = "Feature: Test\n  Scenario: Example\n    When something\n    Then result")]
    pub content: String,
    /// Optional file name for reporting.
    #[schema(example = "rules.feature")]
    pub file_name: Option<String>,
    /// Whether to use LLM for deep validation.
    #[serde(default)]
    #[schema(example = false)]
    pub use_llm: bool,
}

/// A single validation issue.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidationIssueResponse {
    /// Location of the issue.
    pub location: String,
    /// Issue category.
    pub category: String,
    /// Severity level.
    pub severity: String,
    /// Issue message.
    pub message: String,
    /// Suggested fix.
    pub suggestion: Option<String>,
}

/// Response from rules validation.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValidateRulesResponse {
    /// Whether the rules are valid.
    pub is_valid: bool,
    /// Number of scenarios checked.
    pub scenario_count: usize,
    /// Validation issues found.
    pub issues: Vec<ValidationIssueResponse>,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
}

/// Validate Converge Rules.
///
/// Validates Gherkin-format business rules for convention compliance,
/// compilability, and business sense.
#[utoipa::path(
    post,
    path = "/api/v1/validate-rules",
    tag = "validation",
    request_body = ValidateRulesRequest,
    responses(
        (status = 200, description = "Validation completed", body = ValidateRulesResponse),
        (status = 400, description = "Invalid request", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn validate_rules(
    Json(_request): Json<ValidateRulesRequest>,
) -> Result<Json<ValidateRulesResponse>, RuntimeError> {
    // Gherkin validation moved to organism-application; this endpoint is a
    // placeholder until the axiom crate is restored or replaced.
    Err(RuntimeError::Config(
        "Rule validation is not yet available in converge-runtime".to_string(),
    ))
}

// =============================================================================
// Firestore-backed Job API (when gcp feature is enabled)
// =============================================================================

/// Request to create a persisted job.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateJobRequest {
    /// User ID (will be authenticated in production).
    #[schema(example = "user-123")]
    pub user_id: String,
    /// Optional seed facts as JSON.
    #[schema(example = json!({"key": "value"}))]
    pub seeds: Option<serde_json::Value>,
}

/// Response with created job details.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateJobResponse {
    /// The created job ID.
    pub id: String,
    /// Job status.
    pub status: String,
    /// Created timestamp.
    pub created_at: String,
}

/// Response with job details.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetJobResponse {
    /// Job ID.
    pub id: String,
    /// User ID.
    pub user_id: String,
    /// Job status.
    pub status: String,
    /// Number of cycles executed.
    pub cycles: u32,
    /// Seed facts.
    pub seeds: Option<serde_json::Value>,
    /// Final context (if converged).
    pub context: Option<serde_json::Value>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Created timestamp.
    pub created_at: String,
    /// Updated timestamp.
    pub updated_at: String,
}

/// Create a new job (persisted to Firestore).
///
/// Creates a job record in Firestore and returns the job ID.
#[utoipa::path(
    post,
    path = "/api/v1/store/jobs",
    tag = "store",
    request_body = CreateJobRequest,
    responses(
        (status = 201, description = "Job created successfully", body = CreateJobResponse),
        (status = 503, description = "Database not available", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn create_job(
    State(state): State<AppState>,
    Json(request): Json<CreateJobRequest>,
) -> Result<(axum::http::StatusCode, Json<CreateJobResponse>), RuntimeError> {
    info!(user_id = %request.user_id, "Creating new job");

    #[cfg(feature = "gcp")]
    {
        use crate::db::{Job, JobStatus};

        let db = state
            .db
            .as_ref()
            .ok_or_else(|| RuntimeError::Config("Database not available".to_string()))?;

        // Create job record
        let mut job = Job::new(&request.user_id);
        job.seeds = request.seeds;

        let job_id = db
            .jobs
            .create(&job)
            .await
            .map_err(|e| RuntimeError::Config(format!("Failed to create job: {e}")))?;

        info!(job_id = %job_id, "Job created in Firestore");

        Ok((
            axum::http::StatusCode::CREATED,
            Json(CreateJobResponse {
                id: job_id,
                status: format!("{:?}", JobStatus::Pending).to_lowercase(),
                created_at: job.created_at.to_rfc3339(),
            }),
        ))
    }

    #[cfg(not(feature = "gcp"))]
    {
        let _ = state;
        let _ = request;
        Err(RuntimeError::Config(
            "Firestore not available (compile with --features gcp)".to_string(),
        ))
    }
}

/// Get a job by ID.
///
/// Retrieves job details from Firestore.
#[utoipa::path(
    get,
    path = "/api/v1/store/jobs/{job_id}",
    tag = "store",
    params(
        ("job_id" = String, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "Job found", body = GetJobResponse),
        (status = 404, description = "Job not found", body = RuntimeErrorResponse),
        (status = 503, description = "Database not available", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn get_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<GetJobResponse>, RuntimeError> {
    info!(job_id = %job_id, "Getting job");

    #[cfg(feature = "gcp")]
    {
        let db = state
            .db
            .as_ref()
            .ok_or_else(|| RuntimeError::Config("Database not available".to_string()))?;

        let job = db
            .jobs
            .get(&job_id)
            .await
            .map_err(|e| RuntimeError::Config(format!("Failed to get job: {e}")))?;

        let job = job.ok_or_else(|| RuntimeError::NotFound(format!("Job {job_id} not found")))?;

        Ok(Json(GetJobResponse {
            id: job.id.unwrap_or_default(),
            user_id: job.user_id,
            status: format!("{:?}", job.status).to_lowercase(),
            cycles: job.cycles,
            seeds: job.seeds,
            context: job.context,
            error: job.error,
            created_at: job.created_at.to_rfc3339(),
            updated_at: job.updated_at.to_rfc3339(),
        }))
    }

    #[cfg(not(feature = "gcp"))]
    {
        let _ = state;
        let _ = job_id;
        Err(RuntimeError::Config(
            "Firestore not available (compile with --features gcp)".to_string(),
        ))
    }
}

/// Response from running a job.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RunJobResponse {
    /// Job ID.
    pub id: String,
    /// Final job status.
    pub status: String,
    /// Number of cycles executed.
    pub cycles: u32,
    /// Whether convergence was reached.
    pub converged: bool,
    /// Execution duration in milliseconds.
    pub duration_ms: Option<u64>,
    /// Error message (if failed).
    pub error: Option<String>,
}

/// Run a pending job.
///
/// Executes a pending job through the Converge engine and updates its status.
#[utoipa::path(
    post,
    path = "/api/v1/store/jobs/{job_id}/run",
    tag = "store",
    params(
        ("job_id" = String, Path, description = "Job ID to run")
    ),
    responses(
        (status = 200, description = "Job completed", body = RunJobResponse),
        (status = 404, description = "Job not found", body = RuntimeErrorResponse),
        (status = 409, description = "Job not in pending state", body = RuntimeErrorResponse),
        (status = 503, description = "Database not available", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn run_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<RunJobResponse>, RuntimeError> {
    info!(job_id = %job_id, "Running job");

    #[cfg(feature = "gcp")]
    {
        use crate::db::JobStatus;

        let db = state
            .db
            .as_ref()
            .ok_or_else(|| RuntimeError::Config("Database not available".to_string()))?;

        // Get the job
        let mut job = db
            .jobs
            .get(&job_id)
            .await
            .map_err(|e| RuntimeError::Config(format!("Failed to get job: {e}")))?
            .ok_or_else(|| RuntimeError::NotFound(format!("Job {job_id} not found")))?;

        // Check job is pending
        if job.status != JobStatus::Pending {
            return Err(RuntimeError::Conflict(format!(
                "Job {} is not pending (status: {:?})",
                job_id, job.status
            )));
        }

        // Mark as running
        job.start();
        db.jobs
            .update(&job_id, &job)
            .await
            .map_err(|e| RuntimeError::Config(format!("Failed to update job: {e}")))?;

        info!(job_id = %job_id, "Job started");

        // Run the engine
        let seeds = job.seeds.clone();
        let max_cycles = job.max_cycles;

        use converge_core::Budget;

        let budget = Budget {
            max_cycles,
            ..Budget::default()
        };
        let mut engine = Engine::with_budget(budget);

        // TODO: Register agents based on job configuration
        // For now, create a minimal engine

        // Create context from seeds
        let _seeds = seeds;
        let context = Context::new();

        let result = engine.run(context).await.map_err(RuntimeError::Converge);

        // Update job based on result
        match result {
            Ok(run_result) => {
                // Build context summary for storage
                let context_summary: std::collections::HashMap<String, usize> = ContextKey::iter()
                    .map(|key| {
                        let count = run_result.context.get(key).len();
                        (format!("{key:?}"), count)
                    })
                    .collect();

                job.complete(
                    serde_json::to_value(&context_summary).unwrap_or_default(),
                    run_result.cycles,
                );
                db.jobs
                    .update(&job_id, &job)
                    .await
                    .map_err(|e| RuntimeError::Config(format!("Failed to update job: {e}")))?;

                info!(job_id = %job_id, cycles = run_result.cycles, "Job converged");

                Ok(Json(RunJobResponse {
                    id: job_id,
                    status: "converged".to_string(),
                    cycles: run_result.cycles,
                    converged: run_result.converged,
                    duration_ms: job.duration_ms,
                    error: None,
                }))
            }
            Err(e) => {
                let error_msg = format!("{e}");
                job.fail(&error_msg);
                db.jobs
                    .update(&job_id, &job)
                    .await
                    .map_err(|e| RuntimeError::Config(format!("Failed to update job: {e}")))?;

                tracing::warn!(job_id = %job_id, error = %error_msg, "Job failed");

                Ok(Json(RunJobResponse {
                    id: job_id,
                    status: "failed".to_string(),
                    cycles: job.cycles,
                    converged: false,
                    duration_ms: job.duration_ms,
                    error: Some(error_msg),
                }))
            }
        }
    }

    #[cfg(not(feature = "gcp"))]
    {
        let _ = state;
        let _ = job_id;
        Err(RuntimeError::Config(
            "Firestore not available (compile with --features gcp)".to_string(),
        ))
    }
}

/// Response from cancelling a job.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CancelJobResponse {
    /// Job ID.
    pub id: String,
    /// Final job status.
    pub status: String,
    /// Cancelled timestamp.
    pub cancelled_at: String,
}

/// Cancel a pending or running job.
///
/// Cancels a job that hasn't completed yet.
#[utoipa::path(
    post,
    path = "/api/v1/store/jobs/{job_id}/cancel",
    tag = "store",
    params(
        ("job_id" = String, Path, description = "Job ID to cancel")
    ),
    responses(
        (status = 200, description = "Job cancelled", body = CancelJobResponse),
        (status = 404, description = "Job not found", body = RuntimeErrorResponse),
        (status = 409, description = "Job already completed", body = RuntimeErrorResponse),
        (status = 503, description = "Database not available", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn cancel_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<Json<CancelJobResponse>, RuntimeError> {
    info!(job_id = %job_id, "Cancelling job");

    #[cfg(feature = "gcp")]
    {
        use crate::db::JobStatus;

        let db = state
            .db
            .as_ref()
            .ok_or_else(|| RuntimeError::Config("Database not available".to_string()))?;

        // Get the job
        let mut job = db
            .jobs
            .get(&job_id)
            .await
            .map_err(|e| RuntimeError::Config(format!("Failed to get job: {e}")))?
            .ok_or_else(|| RuntimeError::NotFound(format!("Job {job_id} not found")))?;

        // Check job can be cancelled (pending or running)
        match job.status {
            JobStatus::Pending | JobStatus::Running => {}
            _ => {
                return Err(RuntimeError::Conflict(format!(
                    "Job {} cannot be cancelled (status: {:?})",
                    job_id, job.status
                )));
            }
        }

        // Cancel the job
        job.cancel();
        db.jobs
            .update(&job_id, &job)
            .await
            .map_err(|e| RuntimeError::Config(format!("Failed to update job: {e}")))?;

        info!(job_id = %job_id, "Job cancelled");

        Ok(Json(CancelJobResponse {
            id: job_id,
            status: "cancelled".to_string(),
            cancelled_at: job.completed_at.map(|t| t.to_rfc3339()).unwrap_or_default(),
        }))
    }

    #[cfg(not(feature = "gcp"))]
    {
        let _ = state;
        let _ = job_id;
        Err(RuntimeError::Config(
            "Firestore not available (compile with --features gcp)".to_string(),
        ))
    }
}

/// Delete a job.
///
/// Permanently deletes a job from Firestore.
#[utoipa::path(
    delete,
    path = "/api/v1/store/jobs/{job_id}",
    tag = "store",
    params(
        ("job_id" = String, Path, description = "Job ID to delete")
    ),
    responses(
        (status = 204, description = "Job deleted"),
        (status = 404, description = "Job not found", body = RuntimeErrorResponse),
        (status = 503, description = "Database not available", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn delete_job(
    State(state): State<AppState>,
    Path(job_id): Path<String>,
) -> Result<axum::http::StatusCode, RuntimeError> {
    info!(job_id = %job_id, "Deleting job");

    #[cfg(feature = "gcp")]
    {
        let db = state
            .db
            .as_ref()
            .ok_or_else(|| RuntimeError::Config("Database not available".to_string()))?;

        // Check job exists
        let job = db
            .jobs
            .get(&job_id)
            .await
            .map_err(|e| RuntimeError::Config(format!("Failed to get job: {e}")))?;

        if job.is_none() {
            return Err(RuntimeError::NotFound(format!("Job {job_id} not found")));
        }

        // Delete the job
        db.jobs
            .delete(&job_id)
            .await
            .map_err(|e| RuntimeError::Config(format!("Failed to delete job: {e}")))?;

        info!(job_id = %job_id, "Job deleted");

        Ok(axum::http::StatusCode::NO_CONTENT)
    }

    #[cfg(not(feature = "gcp"))]
    {
        let _ = state;
        let _ = job_id;
        Err(RuntimeError::Config(
            "Firestore not available (compile with --features gcp)".to_string(),
        ))
    }
}

/// List jobs for a user.
///
/// Lists recent jobs for a user from Firestore.
#[utoipa::path(
    get,
    path = "/api/v1/store/users/{user_id}/jobs",
    tag = "store",
    params(
        ("user_id" = String, Path, description = "User ID")
    ),
    responses(
        (status = 200, description = "Jobs list", body = Vec<GetJobResponse>),
        (status = 503, description = "Database not available", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn list_user_jobs(
    State(state): State<AppState>,
    Path(user_id): Path<String>,
) -> Result<Json<Vec<GetJobResponse>>, RuntimeError> {
    info!(user_id = %user_id, "Listing jobs for user");

    #[cfg(feature = "gcp")]
    {
        let db = state
            .db
            .as_ref()
            .ok_or_else(|| RuntimeError::Config("Database not available".to_string()))?;

        let jobs = db
            .jobs
            .list_by_user(&user_id, 50)
            .await
            .map_err(|e| RuntimeError::Config(format!("Failed to list jobs: {e}")))?;

        let response: Vec<GetJobResponse> = jobs
            .into_iter()
            .map(|job| GetJobResponse {
                id: job.id.unwrap_or_default(),
                user_id: job.user_id,
                status: format!("{:?}", job.status).to_lowercase(),
                cycles: job.cycles,
                seeds: job.seeds,
                context: job.context,
                error: job.error,
                created_at: job.created_at.to_rfc3339(),
                updated_at: job.updated_at.to_rfc3339(),
            })
            .collect();

        Ok(Json(response))
    }

    #[cfg(not(feature = "gcp"))]
    {
        let _ = state;
        let _ = user_id;
        Err(RuntimeError::Config(
            "Firestore not available (compile with --features gcp)".to_string(),
        ))
    }
}

// =============================================================================
// Template API
// =============================================================================

use crate::templates::{PackConfig, PackJobRequest, PackSummary};

/// List all available packs.
///
/// Returns a summary of each pack including name, description, and agent count.
#[utoipa::path(
    get,
    path = "/api/v1/templates",
    tag = "templates",
    responses(
        (status = 200, description = "List of packs", body = Vec<PackSummary>)
    )
)]
#[axum::debug_handler]
pub async fn list_templates(State(state): State<AppState>) -> Json<Vec<PackSummary>> {
    let templates = state.templates.list();
    info!(count = templates.len(), "Listed templates");
    Json(templates)
}

/// Get a pack by name.
///
/// Returns the full pack configuration including agents and budget settings.
#[utoipa::path(
    get,
    path = "/api/v1/templates/{name}",
    tag = "templates",
    params(
        ("name" = String, Path, description = "Pack name")
    ),
    responses(
        (status = 200, description = "Pack found", body = PackConfig),
        (status = 404, description = "Pack not found", body = RuntimeErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn get_template(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<PackConfig>, RuntimeError> {
    let template = state
        .templates
        .get(&name)
        .ok_or_else(|| RuntimeError::NotFound(format!("Template not found: {name}")))?;

    info!(name = %name, "Retrieved template");
    Ok(Json((*template).clone()))
}

/// Execute a job using a pack.
///
/// Loads the specified pack, applies any overrides, and executes the convergence job.
/// This uses the `JobExecutor` to register domain agents from converge-domain.
#[utoipa::path(
    post,
    path = "/api/v1/templates/jobs",
    tag = "templates",
    request_body = PackJobRequest,
    responses(
        (status = 200, description = "Job completed successfully", body = JobResponse),
        (status = 404, description = "Pack not found", body = RuntimeErrorResponse),
        (status = 400, description = "Invalid request", body = RuntimeErrorResponse),
        (status = 422, description = "Invariant violation", body = RuntimeErrorResponse),
        (status = 413, description = "Budget exhausted", body = RuntimeErrorResponse),
        (status = 500, description = "Internal server error", body = RuntimeErrorResponse)
    )
)]
#[axum::debug_handler]
pub async fn execute_template_job(
    State(state): State<AppState>,
    Json(request): Json<PackJobRequest>,
) -> Result<Json<JobResponse>, RuntimeError> {
    use crate::execution::JobExecutor;
    use converge_core::Budget;

    let _span = info_span!("execute_template_job", pack = %request.pack);
    let _guard = _span.enter();

    // Get pack config to verify it exists and get settings
    let template = state
        .templates
        .get(&request.pack)
        .ok_or_else(|| RuntimeError::NotFound(format!("Pack not found: {}", request.pack)))?;

    info!(
        template = %template.name,
        agents = template.agents.len(),
        seeds = request.overrides.seeds.len(),
        "Executing template job with domain agents"
    );

    // Extract billing-relevant values before moving request
    #[cfg(all(feature = "billing", feature = "gcp"))]
    let max_cycles = request
        .overrides
        .budget
        .as_ref()
        .map(|b| b.max_cycles)
        .unwrap_or(template.budget.max_cycles);
    #[cfg(all(feature = "billing", feature = "gcp"))]
    let seed_user_id: Option<String> = request
        .overrides
        .seeds
        .iter()
        .find(|s| s.id == "user_id")
        .map(|s| s.content.clone());
    #[cfg(feature = "billing")]
    let seed_stripe_customer_id: Option<String> = request
        .overrides
        .seeds
        .iter()
        .find(|s| s.id == "stripe_customer_id")
        .map(|s| s.content.clone());

    // Pre-execution credit check (billing + gcp)
    #[cfg(all(feature = "billing", feature = "gcp"))]
    {
        if let Some(ref ledger) = state.credit_ledger {
            if let Some(ref user_id) = seed_user_id {
                let required = i64::from(max_cycles) * i64::from(state.credits_per_cycle);
                if !ledger
                    .has_sufficient_credits(user_id, required)
                    .await
                    .unwrap_or(false)
                {
                    let balance = ledger
                        .get_balance(user_id)
                        .await
                        .map(|b| b.balance)
                        .unwrap_or(0);
                    return Err(RuntimeError::Billing(format!(
                        "Insufficient credits: required {required}, available {balance}"
                    )));
                }
            }
        }
    }

    // Clone what we need for the blocking task
    let pack_id = request.pack.clone();
    let template = (*template).clone();
    let overrides = request.overrides.clone();

    // Drop the span guard before await
    drop(_guard);

    let budget = Budget {
        max_cycles: overrides
            .budget
            .as_ref()
            .map(|b| b.max_cycles)
            .unwrap_or(template.budget.max_cycles),
        max_facts: overrides
            .budget
            .as_ref()
            .map(|b| b.max_facts)
            .unwrap_or(template.budget.max_facts),
    };

    let mut builder = JobExecutor::builder()
        .with_pack(&pack_id)
        .with_pack_config(template)
        .with_seeds(overrides.seeds.clone())
        .with_budget(budget);

    if overrides.use_llm {
        builder = builder.with_real_llm();
    } else {
        builder = builder.with_mock_llm();
    }

    let result = builder.execute().await?;

    let context_summary = ContextSummary {
        fact_counts: result.fact_counts,
        version: result.context.version(),
    };

    info!(
        cycles = result.cycles,
        converged = result.converged,
        duration_ms = result.duration_ms,
        "Template job completed"
    );

    #[cfg(feature = "billing")]
    let actual_cycles = result.cycles;

    // Post-execution: deduct credits (billing + gcp)
    #[cfg(all(feature = "billing", feature = "gcp"))]
    {
        if let Some(ref ledger) = state.credit_ledger {
            if let Some(ref user_id) = seed_user_id {
                let deduct_amount = i64::from(actual_cycles) * i64::from(state.credits_per_cycle);
                let job_id = uuid::Uuid::new_v4().to_string();
                let idempotency_key = format!("job_{job_id}_deduct");

                if let Err(e) = ledger
                    .deduct(
                        user_id,
                        deduct_amount,
                        &format!("Job execution: {actual_cycles} cycles"),
                        Some(&job_id),
                        Some(&idempotency_key),
                    )
                    .await
                {
                    tracing::warn!(
                        user_id = %user_id,
                        error = %e,
                        "Failed to deduct credits post-execution"
                    );
                }
            }
        }
    }

    // Post-execution: report usage to Stripe (billing, fire-and-forget)
    #[cfg(feature = "billing")]
    {
        if let (Some(billing), Some(event_name)) = (state.billing_client(), &state.meter_event_name)
        {
            if let Some(ref customer_id) = seed_stripe_customer_id {
                let billing = billing.clone();
                let event_name = event_name.clone();
                let customer_id = customer_id.clone();
                let idempotency_key = format!("meter_{}_{}", uuid::Uuid::new_v4(), actual_cycles);

                tokio::spawn(async move {
                    if let Err(e) = billing
                        .report_usage(
                            &event_name,
                            &customer_id,
                            actual_cycles,
                            Some(&idempotency_key),
                        )
                        .await
                    {
                        tracing::warn!(
                            customer_id,
                            error = %e,
                            "Failed to report usage to Stripe"
                        );
                    }
                });
            }
        }
    }

    Ok(Json(JobResponse {
        metadata: JobMetadata {
            cycles: result.cycles,
            converged: result.converged,
            duration_ms: result.duration_ms,
        },
        cycles: result.cycles,
        converged: result.converged,
        context_summary,
    }))
}

/// Build the HTTP router.
pub fn public_router() -> Router<()> {
    let router = Router::new()
        .route("/health", get(health))
        .route("/ready", get(ready));

    #[cfg(feature = "metrics")]
    let router = router.route("/metrics", get(metrics));

    router
}

/// Build the protected HTTP router.
pub fn protected_router(state: AppState) -> Router<()> {
    let router = Router::new()
        .route("/api/v1/jobs", post(handle_job))
        .route("/api/v1/forms/{form_id}/plan", post(plan_form))
        .route("/api/v1/forms/{form_id}/approve", post(approve_form))
        .route("/api/v1/forms/{form_id}/execute", post(execute_form));

    let router = router
        .route("/api/v1/validate-rules", post(validate_rules))
        // Template endpoints
        .route("/api/v1/templates", get(list_templates))
        .route("/api/v1/templates/{name}", get(get_template))
        .route("/api/v1/templates/jobs", post(execute_template_job))
        // Firestore-backed endpoints
        .route("/api/v1/store/jobs", post(create_job))
        .route(
            "/api/v1/store/jobs/{job_id}",
            get(get_job).delete(delete_job),
        )
        .route("/api/v1/store/jobs/{job_id}/run", post(run_job))
        .route("/api/v1/store/jobs/{job_id}/cancel", post(cancel_job))
        .route("/api/v1/store/users/{user_id}/jobs", get(list_user_jobs))
        .with_state(state.clone());

    #[cfg(feature = "billing")]
    let router = router.merge(crate::billing::handlers::billing_router(state));

    router
}

/// Build the combined HTTP router.
pub fn router(state: AppState) -> Router<()> {
    public_router().merge(protected_router(state))
}
