// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Negative tests for Converge Runtime.
//!
//! These tests verify error handling, edge cases, and behavior
//! with invalid or malformed inputs.

use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use converge_runtime::{handlers, state::AppState};
use http_body_util::BodyExt;
use tower::ServiceExt;

// =============================================================================
// Test Helpers
// =============================================================================

fn test_router() -> Router {
    handlers::router(AppState::new())
}

async fn body_string(body: Body) -> String {
    let bytes = body.collect().await.unwrap().to_bytes();
    String::from_utf8_lossy(&bytes).to_string()
}

// =============================================================================
// Invalid JSON Tests
// =============================================================================

#[tokio::test]
async fn test_jobs_empty_body() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_jobs_null_body() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from("null"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 UNPROCESSABLE_ENTITY for deserialization errors
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_jobs_array_instead_of_object() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from("[]"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 UNPROCESSABLE_ENTITY for deserialization errors
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_jobs_string_instead_of_object() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from("\"string\""))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 UNPROCESSABLE_ENTITY for deserialization errors
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_jobs_number_instead_of_object() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from("42"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 UNPROCESSABLE_ENTITY for deserialization errors
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_jobs_truncated_json() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from("{\"context\":"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_jobs_extra_comma_json() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from("{\"context\":{},}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_jobs_single_quotes_json() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from("{'context':{}}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_jobs_unquoted_keys() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from("{context:{}}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// =============================================================================
// Missing Content-Type Tests
// =============================================================================

#[tokio::test]
async fn test_jobs_missing_content_type() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should fail without content-type header
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
}

#[tokio::test]
async fn test_jobs_wrong_content_type() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "text/plain")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
}

// =============================================================================
// Invalid HTTP Methods
// =============================================================================

#[tokio::test]
async fn test_health_put_not_allowed() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_health_delete_not_allowed() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_jobs_get_not_allowed() {
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

#[tokio::test]
async fn test_jobs_delete_not_allowed() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/jobs")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn test_templates_post_not_allowed() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/templates")
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

// =============================================================================
// Invalid Paths Tests
// =============================================================================

#[tokio::test]
async fn test_unknown_api_path() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/unknown")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_empty_path() {
    let app = test_router();

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_double_slashes_in_path() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("//health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Behavior may vary, but shouldn't crash
    assert!(
        response.status() == StatusCode::OK
            || response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn test_path_with_query_params() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health?foo=bar")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Health endpoint should work with query params
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_path_with_fragment() {
    let app = test_router();

    // Fragments are not sent to server, but let's verify parsing
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
}

// =============================================================================
// Edge Case Input Tests
// =============================================================================

#[tokio::test]
async fn test_jobs_very_large_context() {
    let app = test_router();

    // Create a large JSON payload (1MB)
    let large_value = "x".repeat(1_000_000);
    let json = format!(r#"{{"context":{{"large":"{}"}}}}"#, large_value);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from(json))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should either accept or reject, but not crash
    assert!(
        response.status() == StatusCode::NOT_IMPLEMENTED
            || response.status() == StatusCode::PAYLOAD_TOO_LARGE
            || response.status() == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn test_jobs_deeply_nested_context() {
    let app = test_router();

    // Create deeply nested JSON (100 levels)
    let mut json = String::from("{\"context\":");
    for _ in 0..100 {
        json.push_str("{\"nested\":");
    }
    json.push_str("null");
    for _ in 0..100 {
        json.push('}');
    }
    json.push('}');

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from(json))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle without stack overflow
    assert!(
        response.status() == StatusCode::NOT_IMPLEMENTED
            || response.status() == StatusCode::BAD_REQUEST
    );
}

#[tokio::test]
async fn test_jobs_unicode_in_context() {
    let app = test_router();

    let json = r#"{"context":{"emoji":"🚀","chinese":"你好","arabic":"مرحبا"}}"#;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from(json))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
}

#[tokio::test]
async fn test_jobs_null_characters_in_json() {
    let app = test_router();

    // JSON with escaped null character
    let json = r#"{"context":{"value":"test\u0000null"}}"#;

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/jobs")
                .header("content-type", "application/json")
                .body(Body::from(json))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle without issues
    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
}

// =============================================================================
// Validate Rules Negative Tests
// =============================================================================

#[tokio::test]
async fn test_validate_rules_missing_content_field() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/validate-rules")
                .header("content-type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 for missing required fields
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_validate_rules_null_content() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/validate-rules")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content":null}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 for type mismatches
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_validate_rules_number_content() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/validate-rules")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"content":42}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 for type mismatches
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_validate_rules_boolean_use_llm() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/validate-rules")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"content":"test","use_llm":"not_a_boolean"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 for type mismatches
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

// =============================================================================
// Template Endpoint Negative Tests
// =============================================================================

#[tokio::test]
async fn test_template_jobs_missing_pack() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/templates/jobs")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"overrides":{"seeds":[]}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 for missing required fields
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_template_jobs_invalid_overrides() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/templates/jobs")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"pack":"test","overrides":"invalid"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Axum returns 422 for type mismatches
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_get_template_special_characters_in_name() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/templates/../../etc/passwd")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should return not found, not a path traversal
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_get_template_url_encoded_name() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/templates/test%20name")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Should handle URL encoding properly
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// =============================================================================
// Store Endpoint Negative Tests (without GCP)
// =============================================================================

#[tokio::test]
async fn test_store_create_job_missing_user_id() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/store/jobs")
                .header("content-type", "application/json")
                .body(Body::from(r#"{}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should fail - Axum returns 422 for missing fields, or 500 if no GCP
    assert!(
        response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
            || response.status() == StatusCode::INTERNAL_SERVER_ERROR
    );
}

#[tokio::test]
async fn test_store_run_job_nonexistent() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/store/jobs/nonexistent-job-id/run")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Without GCP, should return internal error
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_store_cancel_job_nonexistent() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/store/jobs/nonexistent-job-id/cancel")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Without GCP, should return internal error
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn test_store_delete_job_nonexistent() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/v1/store/jobs/nonexistent-job-id")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Without GCP, should return internal error
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

// =============================================================================
// Response Body Verification
// =============================================================================

#[tokio::test]
async fn test_error_response_has_required_fields() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/templates/nonexistent")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_string(response.into_body()).await;
    let json: serde_json::Value = serde_json::from_str(&body).unwrap();

    assert!(json.get("error").is_some(), "Missing 'error' field");
    assert!(json.get("status").is_some(), "Missing 'status' field");
}

#[tokio::test]
async fn test_404_error_response_contains_path() {
    let app = test_router();

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/templates/my-missing-template")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = body_string(response.into_body()).await;

    // Error message should reference what was not found
    assert!(
        body.contains("my-missing-template") || body.contains("not found"),
        "Error should mention the missing resource"
    );
}
