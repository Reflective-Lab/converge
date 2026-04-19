// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Integration tests for Converge Runtime HTTP handlers.
//!
//! These tests use axum's test utilities to test the full HTTP request/response cycle.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use converge_runtime::{handlers, state::AppState};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;

// =============================================================================
// Test Helpers
// =============================================================================

/// Create a test router with default state.
fn test_router() -> Router {
    handlers::router(AppState::new())
}

/// Helper to parse JSON response body.
async fn body_json(body: Body) -> Value {
    let bytes = body.collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

/// Helper to get response body as string.
async fn body_string(body: Body) -> String {
    let bytes = body.collect().await.unwrap().to_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

// =============================================================================
// Health Endpoint Tests
// =============================================================================

#[tokio::test]
async fn test_health_endpoint_returns_ok() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_string(response.into_body()).await;
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn test_health_endpoint_method_get() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_health_endpoint_post_not_allowed() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// =============================================================================
// Ready Endpoint Tests
// =============================================================================

#[tokio::test]
async fn test_ready_endpoint_returns_ok() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_ready_endpoint_returns_json() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response.into_body()).await;
    assert_eq!(body["status"], "ready");
    assert!(body.get("services").is_some());
}

#[tokio::test]
async fn test_ready_endpoint_services_engine_ok() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response.into_body()).await;
    assert_eq!(body["services"]["engine"], "ok");
}

// =============================================================================
// Jobs Endpoint Tests
// =============================================================================

#[tokio::test]
async fn test_jobs_endpoint_accepts_post() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"context":{}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_jobs_endpoint_empty_context() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_jobs_endpoint_returns_job_response() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"context":null}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response.into_body()).await;
    assert!(body.get("cycles").is_some());
    assert!(body.get("converged").is_some());
    assert!(body.get("metadata").is_some());
    assert!(body.get("context_summary").is_some());
}

#[tokio::test]
async fn test_jobs_endpoint_returns_metadata() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response.into_body()).await;
    let metadata = &body["metadata"];
    assert!(metadata.get("cycles").is_some());
    assert!(metadata.get("converged").is_some());
    assert!(metadata.get("duration_ms").is_some());
}

#[tokio::test]
async fn test_jobs_endpoint_invalid_json() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from("not valid json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_jobs_endpoint_get_not_allowed() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// =============================================================================
// Validate Rules Endpoint Tests
// =============================================================================

#[tokio::test]
async fn test_validate_rules_endpoint_accepts_post() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/validate-rules")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"content":"Feature: Test\n  Scenario: Example\n    When test\n    Then result"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Endpoint is a placeholder stub — returns 500 until validation is restored
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_validate_rules_returns_stub_error() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/validate-rules")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content":"Feature: Test"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_validate_rules_empty_content() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/validate-rules")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content":""}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Empty content may return OK (with validation issues) or error
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR,
        "Expected 200 or 500, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_validate_rules_with_llm_false() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/validate-rules")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content":"Feature: Test","use_llm":false}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Endpoint is a placeholder stub — returns 500 regardless of use_llm
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

// =============================================================================
// Templates Endpoint Tests
// =============================================================================

#[tokio::test]
async fn test_list_templates_endpoint() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/templates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = body_json(response.into_body()).await;
    assert!(body.is_array());
}

#[tokio::test]
async fn test_list_templates_contains_defaults() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/templates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_json(response.into_body()).await;
    let templates = body.as_array().unwrap();
    // Should have at least one default template
    assert!(!templates.is_empty());
}

#[tokio::test]
async fn test_get_template_not_found() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/templates/nonexistent-template")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_template_returns_pack_config() {
    let app = test_router();

    // First, get the list of templates to find a valid name
    let list_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/templates")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let templates = body_json(list_response.into_body()).await;
    let templates = templates.as_array().unwrap();

    if !templates.is_empty() {
        let first_template_name = templates[0]["name"].as_str().unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/v1/templates/{}", first_template_name))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_json(response.into_body()).await;
        assert!(body.get("name").is_some());
        assert!(body.get("agents").is_some());
        assert!(body.get("budget").is_some());
    }
}

// =============================================================================
// Template Job Execution Tests
// =============================================================================

#[tokio::test]
async fn test_execute_template_job_not_found() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/templates/jobs")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"pack":"nonexistent","overrides":{"seeds":[],"budget":null}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// =============================================================================
// Store Endpoints (GCP feature disabled)
// =============================================================================

#[tokio::test]
async fn test_store_create_job_requires_gcp() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/store/jobs")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"user_id":"test-user"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Without GCP feature, should return error
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_store_get_job_requires_gcp() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/store/jobs/some-job-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Without GCP feature, should return error
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

// =============================================================================
// 404 Tests
// =============================================================================

#[tokio::test]
async fn test_unknown_endpoint_returns_404() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/unknown/endpoint")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_api_unknown_version_returns_404() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v999/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// =============================================================================
// Content-Type Tests
// =============================================================================

#[tokio::test]
async fn test_ready_returns_json_content_type() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let content_type = response.headers().get("content-type").unwrap();
    assert!(content_type.to_str().unwrap().contains("application/json"));
}

// =============================================================================
// Concurrent Request Tests
// =============================================================================

#[tokio::test]
async fn test_concurrent_health_requests() {
    use std::sync::Arc;

    let app = Arc::new(test_router());

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let app = app.clone();
            tokio::spawn(async move {
                let response = (*app)
                    .clone()
                    .oneshot(
                        Request::builder()
                            .uri("/health")
                            .body(Body::empty())
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                response.status()
            })
        })
        .collect();

    for handle in handles {
        let status = handle.await.unwrap();
        assert_eq!(status, StatusCode::OK);
    }
}

#[tokio::test]
async fn test_concurrent_job_requests() {
    use std::sync::Arc;

    let app = Arc::new(test_router());

    let handles: Vec<_> = (0..5)
        .map(|_| {
            let app = app.clone();
            tokio::spawn(async move {
                let response = (*app)
                    .clone()
                    .oneshot(
                        Request::builder()
                            .method("POST")
                            .uri("/api/v1/jobs")
                            .header("content-type", "application/json")
                            .body(Body::from(r#"{}"#))
                            .unwrap(),
                    )
                    .await
                    .unwrap();
                response.status()
            })
        })
        .collect();

    for handle in handles {
        let status = handle.await.unwrap();
        assert_eq!(status, StatusCode::OK);
    }
}
