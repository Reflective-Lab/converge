//! gRPC interceptors for authentication and authorization.
//!
//! Provides request-level processing for:
//! - Extracting service identity from mTLS client certificates
//! - Validating JWT tokens and extracting user identity
//! - Attaching verified identity to request extensions

#[allow(clippy::result_large_err)] // tonic::Status is the idiomatic gRPC error type
mod auth;

pub use auth::{AuthInterceptor, AuthInterceptorLayer};
