// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Converge Runtime Server
//!
//! Provides HTTP and gRPC interfaces for the Converge engine.

// Scaffolding code - allow pedantic lints during development
#![allow(unreachable_code)]
#![allow(dead_code)]
#![allow(clippy::needless_for_each)]
#![allow(clippy::unused_self)]
#![allow(clippy::unnecessary_wraps)]
#![allow(clippy::used_underscore_binding)]
#![allow(clippy::unused_async)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::no_effect_underscore_binding)]

// Use the library crate
use converge_runtime::config::Config;
use converge_runtime::http::HttpServer;
use converge_runtime::state::AppState;

#[cfg(feature = "gcp")]
use converge_runtime::db;
#[cfg(feature = "gcp")]
use converge_runtime::gcp;

#[cfg(feature = "grpc")]
use converge_runtime::grpc;

#[cfg(feature = "metrics")]
use converge_runtime::metrics;

#[cfg(feature = "telemetry")]
use converge_runtime::telemetry;

use anyhow::Result;
use tracing::info;
#[cfg(feature = "gcp")]
use tracing::warn;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Initialize tracing.
    init_tracing();

    info!("Starting Converge Runtime");

    // Initialize Prometheus metrics
    #[cfg(feature = "metrics")]
    {
        match metrics::init() {
            Ok(_) => info!("Prometheus metrics initialized"),
            Err(e) => tracing::warn!(error = %e, "Failed to initialize metrics"),
        }
    }

    // Load configuration
    let config = Config::load()?;
    info!(
        http_bind = %config.http.bind,
        http_max_body_size = config.http.max_body_size,
        security_configured = config.security.is_some(),
        "Configuration loaded"
    );

    #[cfg(feature = "auth")]
    {
        let firebase_configured = std::env::var("FIREBASE_PROJECT_ID")
            .or_else(|_| std::env::var("GOOGLE_CLOUD_PROJECT"))
            .or_else(|_| std::env::var("GCP_PROJECT_ID"))
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
        let jwt_configured = config
            .security
            .as_ref()
            .and_then(|security| security.jwt.as_ref())
            .is_some();
        info!(
            jwt_configured,
            firebase_configured, "Protected routes require configured authentication"
        );
    }

    // Initialize application state
    #[cfg(feature = "gcp")]
    let app_state = {
        let gcp_config = gcp::GcpConfig::from_env();
        info!(
            local_dev = gcp_config.is_local(),
            "GCP configuration loaded"
        );

        match db::Database::new(gcp_config).await {
            Ok(database) => {
                info!("Database connection established");
                AppState::with_database(database)
            }
            Err(e) => {
                warn!(error = %e, "Failed to connect to database, running without persistence");
                AppState::new()
            }
        }
    };

    #[cfg(not(feature = "gcp"))]
    let app_state = AppState::new();

    // Start HTTP server (always enabled)
    let http_server = HttpServer::new(config.http.clone(), app_state);
    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_server.start().await {
            tracing::error!(error = %e, "HTTP server failed");
        }
    });

    // TODO: Start gRPC server when grpc feature is enabled
    #[cfg(feature = "grpc")]
    {
        let grpc_config = config.grpc();
        let grpc_server = {
            #[cfg(feature = "security")]
            {
                if let Some(security_config) = config.security.clone() {
                    grpc::GrpcServer::with_security(grpc_config, security_config)
                } else {
                    grpc::GrpcServer::new(grpc_config)
                }
            }
            #[cfg(not(feature = "security"))]
            {
                grpc::GrpcServer::new(grpc_config)
            }
        };
        let grpc_handle = tokio::spawn(async move {
            if let Err(e) = grpc_server.start().await {
                tracing::error!(error = %e, "gRPC server failed");
            }
        });
        tokio::select! {
            _ = http_handle => {},
            _ = grpc_handle => {},
        }
        return Ok(());
    }

    // Default: just wait for HTTP server
    http_handle
        .await
        .map_err(|e| anyhow::anyhow!("HTTP server task failed: {e}"))?;

    Ok(())
}

// =============================================================================
// Initialization Functions
// =============================================================================

/// Initialize tracing subscriber with optional OpenTelemetry integration.
fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(false)
        .with_thread_ids(false);

    // Build the subscriber with all enabled layers
    let subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    // Add OpenTelemetry layer if enabled
    #[cfg(feature = "telemetry")]
    let subscriber = {
        match telemetry::init() {
            Ok(tracer) => {
                let otel_layer = telemetry::layer(tracer);
                subscriber.with(Some(otel_layer))
            }
            Err(e) => {
                eprintln!("Failed to initialize OpenTelemetry: {e}");
                subscriber.with(None::<tracing_opentelemetry::OpenTelemetryLayer<_, _>>)
            }
        }
    };

    subscriber.init();
}
