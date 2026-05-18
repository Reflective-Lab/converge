// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Configuration management for Converge Runtime.

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;

/// Runtime configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// HTTP server configuration.
    pub http: HttpConfig,

    /// Security configuration (optional).
    #[serde(default)]
    pub security: Option<SecurityConfig>,

    /// NATS configuration (optional).
    #[serde(default)]
    pub nats: Option<NatsConfig>,

    /// Billing / Stripe configuration (optional, requires billing feature).
    #[cfg(feature = "billing")]
    #[serde(default)]
    pub billing: Option<BillingConfig>,
}

#[cfg(feature = "grpc")]
impl Config {
    /// Get gRPC configuration (only available when grpc feature is enabled).
    pub fn grpc(&self) -> GrpcConfig {
        GrpcConfig::default()
    }
}

/// HTTP server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpConfig {
    /// Bind address for HTTP server.
    pub bind: SocketAddr,
    /// Maximum request body size (bytes).
    pub max_body_size: usize,
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8080".parse().expect("valid default address"),
            max_body_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

/// gRPC server configuration (prepared, not implemented).
#[cfg(feature = "grpc")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GrpcConfig {
    /// Bind address for gRPC server.
    pub bind: SocketAddr,
}

#[cfg(feature = "grpc")]
impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:50051".parse().expect("valid default address"),
        }
    }
}

/// Security configuration for mTLS and authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityConfig {
    /// Identity source: "file" or "spiffe".
    #[serde(default = "default_identity_source")]
    pub identity_source: String,

    /// Path to server/client certificate (PEM format).
    pub cert_path: Option<PathBuf>,

    /// Path to private key (PEM format).
    pub key_path: Option<PathBuf>,

    /// Path to CA certificate(s) for peer verification (PEM format).
    pub ca_path: Option<PathBuf>,

    /// Service identifier (if not extracted from certificate).
    pub service_id: Option<String>,

    /// JWT configuration for token validation.
    #[serde(default)]
    pub jwt: Option<JwtConfig>,
}

fn default_identity_source() -> String {
    "file".to_string()
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            identity_source: default_identity_source(),
            cert_path: None,
            key_path: None,
            ca_path: None,
            service_id: None,
            jwt: None,
        }
    }
}

/// JWT validation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JwtConfig {
    /// Secret key for HS256 validation (base64 encoded or raw).
    pub secret: Option<String>,

    /// Expected issuer claim.
    pub issuer: Option<String>,

    /// Expected audience claim.
    pub audience: Option<String>,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            secret: None,
            issuer: None,
            audience: None,
        }
    }
}

/// NATS client configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NatsConfig {
    /// NATS server addresses.
    pub servers: Vec<String>,

    /// Connection name for monitoring.
    pub name: Option<String>,

    /// Whether to require TLS (default: true).
    #[serde(default = "default_require_tls")]
    pub require_tls: bool,

    /// Audit stream configuration.
    #[serde(default)]
    pub audit: Option<AuditConfig>,
}

fn default_require_tls() -> bool {
    true
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self {
            servers: vec!["nats://localhost:4222".to_string()],
            name: None,
            require_tls: true,
            audit: None,
        }
    }
}

/// Audit logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditConfig {
    /// JetStream stream name for audit events.
    #[serde(default = "default_audit_stream")]
    pub stream: String,

    /// Subject prefix for audit events.
    #[serde(default = "default_audit_subject_prefix")]
    pub subject_prefix: String,

    /// Retention period in days.
    #[serde(default = "default_audit_retention_days")]
    pub retention_days: u32,
}

fn default_audit_stream() -> String {
    "AUDIT".to_string()
}

fn default_audit_subject_prefix() -> String {
    "audit.runtime".to_string()
}

fn default_audit_retention_days() -> u32 {
    90
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            stream: default_audit_stream(),
            subject_prefix: default_audit_subject_prefix(),
            retention_days: default_audit_retention_days(),
        }
    }
}

/// Billing / Stripe configuration.
#[cfg(feature = "billing")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BillingConfig {
    /// Stripe API key (secret key, starts with `sk_`).
    pub stripe_api_key: String,
    /// Stripe webhook signing secret (starts with `whsec_`).
    pub stripe_webhook_secret: Option<String>,
    /// Stripe API base URL (default: `https://api.stripe.com/v1`).
    pub stripe_base_url: Option<String>,
    /// Stripe meter event name (default: `convergence_cycles`).
    #[serde(default = "default_meter_event_name")]
    pub meter_event_name: String,
    /// Credit cost per convergence cycle (default: 1).
    #[serde(default = "default_credits_per_cycle")]
    pub credits_per_cycle: u32,
}

#[cfg(feature = "billing")]
fn default_meter_event_name() -> String {
    "convergence_cycles".to_string()
}

#[cfg(feature = "billing")]
fn default_credits_per_cycle() -> u32 {
    1
}

impl Config {
    /// Load configuration from environment and files.
    ///
    /// Environment variables:
    /// - `HTTP_PORT` or `PORT` - HTTP server port (default: 8080)
    /// - `HTTP_BIND` - Full bind address (default: 127.0.0.1:8080)
    /// - `SECURITY_CERT_PATH`, `SECURITY_KEY_PATH`, `SECURITY_CA_PATH` - TLS paths
    /// - `SECURITY_SERVICE_ID` - explicit service identifier
    /// - `JWT_SECRET`, `JWT_ISSUER`, `JWT_AUDIENCE` - JWT validation config
    /// - `STRIPE_API_KEY` - Stripe secret key (billing feature)
    /// - `STRIPE_WEBHOOK_SECRET` - Stripe webhook signing secret (billing feature)
    pub fn load() -> anyhow::Result<Self> {
        let http_bind = if let Ok(bind) = std::env::var("HTTP_BIND") {
            bind.parse()?
        } else if let Ok(port) = std::env::var("HTTP_PORT").or_else(|_| std::env::var("PORT")) {
            format!("127.0.0.1:{port}").parse()?
        } else {
            "127.0.0.1:8080".parse()?
        };

        Ok(Self {
            http: HttpConfig {
                bind: http_bind,
                ..HttpConfig::default()
            },
            security: load_security_config(),
            nats: None,
            #[cfg(feature = "billing")]
            billing: std::env::var("STRIPE_API_KEY")
                .ok()
                .map(|key| BillingConfig {
                    stripe_api_key: key,
                    stripe_webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
                    stripe_base_url: std::env::var("STRIPE_BASE_URL").ok(),
                    meter_event_name: std::env::var("STRIPE_METER_EVENT_NAME")
                        .unwrap_or_else(|_| default_meter_event_name()),
                    credits_per_cycle: std::env::var("STRIPE_CREDITS_PER_CYCLE")
                        .ok()
                        .and_then(|v| v.parse().ok())
                        .unwrap_or_else(default_credits_per_cycle),
                }),
        })
    }
}

fn load_security_config() -> Option<SecurityConfig> {
    let cert_path = std::env::var("SECURITY_CERT_PATH").ok().map(PathBuf::from);
    let key_path = std::env::var("SECURITY_KEY_PATH").ok().map(PathBuf::from);
    let ca_path = std::env::var("SECURITY_CA_PATH").ok().map(PathBuf::from);
    let service_id = std::env::var("SECURITY_SERVICE_ID").ok();
    let identity_source =
        std::env::var("SECURITY_IDENTITY_SOURCE").unwrap_or_else(|_| default_identity_source());

    let jwt_secret = std::env::var("JWT_SECRET")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let jwt_issuer = std::env::var("JWT_ISSUER")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let jwt_audience = std::env::var("JWT_AUDIENCE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());

    let jwt = if jwt_secret.is_some() || jwt_issuer.is_some() || jwt_audience.is_some() {
        Some(JwtConfig {
            secret: jwt_secret,
            issuer: jwt_issuer,
            audience: jwt_audience,
        })
    } else {
        None
    };

    let has_security = cert_path.is_some()
        || key_path.is_some()
        || ca_path.is_some()
        || service_id.is_some()
        || jwt.is_some()
        || identity_source != default_identity_source();

    has_security.then_some(SecurityConfig {
        identity_source,
        cert_path,
        key_path,
        ca_path,
        service_id,
        jwt,
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_unknown_field<T>(json: &str, field: &str)
    where
        T: serde::de::DeserializeOwned,
    {
        let err = match serde_json::from_str::<T>(json) {
            Ok(_) => panic!("expected unknown field error for {field:?}"),
            Err(err) => err,
        };
        let message = err.to_string();
        assert!(
            message.contains("unknown field"),
            "expected unknown field error, got: {message}"
        );
        assert!(
            message.contains(field),
            "expected error to mention {field:?}, got: {message}"
        );
    }

    // -------------------------------------------------------------------------
    // Unit Tests: HttpConfig
    // -------------------------------------------------------------------------

    #[test]
    fn test_http_config_default_bind_address() {
        let config = HttpConfig::default();
        assert_eq!(config.bind.port(), 8080);
        assert_eq!(config.bind.ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn test_http_config_default_max_body_size() {
        let config = HttpConfig::default();
        assert_eq!(config.max_body_size, 10 * 1024 * 1024); // 10 MB
    }

    #[test]
    fn test_http_config_debug() {
        let config = HttpConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("HttpConfig"));
        assert!(debug_str.contains("8080"));
    }

    #[test]
    fn test_http_config_clone() {
        let config = HttpConfig::default();
        let cloned = config.clone();
        assert_eq!(config.bind, cloned.bind);
        assert_eq!(config.max_body_size, cloned.max_body_size);
    }

    #[test]
    fn test_http_config_serialize() {
        let config = HttpConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("bind"));
        assert!(json.contains("max_body_size"));
    }

    #[test]
    fn test_http_config_deserialize() {
        let json = r#"{"bind":"127.0.0.1:3000","max_body_size":1024}"#;
        let config: HttpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.bind.port(), 3000);
        assert_eq!(config.bind.ip().to_string(), "127.0.0.1");
        assert_eq!(config.max_body_size, 1024);
    }

    // -------------------------------------------------------------------------
    // Unit Tests: Config
    // -------------------------------------------------------------------------

    #[test]
    fn test_config_debug() {
        let config = Config {
            http: HttpConfig::default(),
            security: None,
            nats: None,
            #[cfg(feature = "billing")]
            billing: None,
        };
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("HttpConfig"));
    }

    #[test]
    fn test_config_clone() {
        let config = Config {
            http: HttpConfig::default(),
            security: None,
            nats: None,
            #[cfg(feature = "billing")]
            billing: None,
        };
        let cloned = config.clone();
        assert_eq!(config.http.bind, cloned.http.bind);
    }

    #[test]
    fn test_config_serialize() {
        let config = Config {
            http: HttpConfig::default(),
            security: None,
            nats: None,
            #[cfg(feature = "billing")]
            billing: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("http"));
        assert!(json.contains("bind"));
    }

    #[test]
    fn test_config_deserialize() {
        let json = r#"{"http":{"bind":"0.0.0.0:9000","max_body_size":2048}}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert_eq!(config.http.bind.port(), 9000);
        assert_eq!(config.http.max_body_size, 2048);
    }

    // -------------------------------------------------------------------------
    // Unit Tests: GrpcConfig (when grpc feature is enabled)
    // -------------------------------------------------------------------------

    #[cfg(feature = "grpc")]
    #[test]
    fn test_grpc_config_default() {
        let config = GrpcConfig::default();
        assert_eq!(config.bind.port(), 50051);
        assert_eq!(config.bind.ip().to_string(), "127.0.0.1");
    }

    #[cfg(feature = "grpc")]
    #[test]
    fn test_grpc_config_debug() {
        let config = GrpcConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("GrpcConfig"));
        assert!(debug_str.contains("50051"));
    }

    #[cfg(feature = "grpc")]
    #[test]
    fn test_config_grpc_method() {
        let config = Config {
            http: HttpConfig::default(),
            security: None,
            nats: None,
            #[cfg(feature = "billing")]
            billing: None,
        };
        let grpc = config.grpc();
        assert_eq!(grpc.bind.port(), 50051);
    }

    // -------------------------------------------------------------------------
    // Negative Tests: Invalid configurations
    // -------------------------------------------------------------------------

    #[test]
    fn test_http_config_deserialize_invalid_bind() {
        let json = r#"{"bind":"not-an-address","max_body_size":1024}"#;
        let result: Result<HttpConfig, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_http_config_deserialize_negative_body_size() {
        // Note: usize doesn't support negative values, so JSON parsing will fail
        let json = r#"{"bind":"127.0.0.1:8080","max_body_size":-1}"#;
        let result: Result<HttpConfig, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_http_config_deserialize_missing_fields() {
        let json = r#"{"bind":"127.0.0.1:8080"}"#;
        let result: Result<HttpConfig, _> = serde_json::from_str(json);
        assert!(result.is_err()); // max_body_size is required
    }

    #[test]
    fn test_config_deserialize_empty_json() {
        let json = r#"{}"#;
        let result: Result<Config, _> = serde_json::from_str(json);
        assert!(result.is_err()); // http field is required
    }

    #[test]
    fn test_config_deserialize_unknown_field_fails_loudly() {
        let json = r#"{"http":{"bind":"0.0.0.0:9000","max_body_size":2048},"workers":4}"#;
        assert_unknown_field::<Config>(json, "workers");
    }

    #[test]
    fn test_http_config_deserialize_unknown_field_fails_loudly() {
        let json = r#"{"bind":"127.0.0.1:8080","max_body_size":1024,"timeout_ms":5000}"#;
        assert_unknown_field::<HttpConfig>(json, "timeout_ms");
    }

    #[test]
    fn test_security_config_deserialize_unknown_field_fails_loudly() {
        let json = r#"{"identity_source":"file","unexpected":true}"#;
        assert_unknown_field::<SecurityConfig>(json, "unexpected");
    }

    #[test]
    fn test_jwt_config_deserialize_unknown_field_fails_loudly() {
        let json = r#"{"issuer":"converge","scope":"admin"}"#;
        assert_unknown_field::<JwtConfig>(json, "scope");
    }

    #[test]
    fn test_nats_config_deserialize_unknown_field_fails_loudly() {
        let json = r#"{"servers":["nats://localhost:4222"],"token":"secret"}"#;
        assert_unknown_field::<NatsConfig>(json, "token");
    }

    #[test]
    fn test_audit_config_deserialize_unknown_field_fails_loudly() {
        let json = r#"{"stream":"AUDIT","subject_prefix":"audit.runtime","retention_days":90,"bucket":"cold"}"#;
        assert_unknown_field::<AuditConfig>(json, "bucket");
    }

    #[cfg(feature = "grpc")]
    #[test]
    fn test_grpc_config_deserialize_unknown_field_fails_loudly() {
        let json = r#"{"bind":"127.0.0.1:50051","reflection":true}"#;
        assert_unknown_field::<GrpcConfig>(json, "reflection");
    }

    #[cfg(feature = "billing")]
    #[test]
    fn test_billing_config_deserialize_unknown_field_fails_loudly() {
        let json = r#"{"stripe_api_key":"sk_test","currency":"usd"}"#;
        assert_unknown_field::<BillingConfig>(json, "currency");
    }

    // -------------------------------------------------------------------------
    // Round-trip Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_http_config_roundtrip() {
        let original = HttpConfig {
            bind: "192.168.1.1:4000".parse().unwrap(),
            max_body_size: 5000,
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: HttpConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(original.bind, restored.bind);
        assert_eq!(original.max_body_size, restored.max_body_size);
    }

    #[test]
    fn test_config_roundtrip() {
        let original = Config {
            http: HttpConfig {
                bind: "10.0.0.1:5555".parse().unwrap(),
                max_body_size: 12345,
            },
            security: None,
            nats: None,
            #[cfg(feature = "billing")]
            billing: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(original.http.bind, restored.http.bind);
        assert_eq!(original.http.max_body_size, restored.http.max_body_size);
    }

    // -------------------------------------------------------------------------
    // Edge Case Tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_http_config_ipv6_address() {
        let json = r#"{"bind":"[::1]:8080","max_body_size":1024}"#;
        let config: HttpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.bind.port(), 8080);
        assert!(config.bind.ip().is_ipv6());
    }

    #[test]
    fn test_http_config_zero_body_size() {
        let json = r#"{"bind":"127.0.0.1:8080","max_body_size":0}"#;
        let config: HttpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.max_body_size, 0);
    }

    #[test]
    fn test_http_config_large_body_size() {
        let json = r#"{"bind":"127.0.0.1:8080","max_body_size":1073741824}"#; // 1 GB
        let config: HttpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.max_body_size, 1073741824);
    }

    #[test]
    fn test_http_config_port_0() {
        let json = r#"{"bind":"127.0.0.1:0","max_body_size":1024}"#;
        let config: HttpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.bind.port(), 0); // OS assigns port
    }

    #[test]
    fn test_http_config_high_port() {
        let json = r#"{"bind":"127.0.0.1:65535","max_body_size":1024}"#;
        let config: HttpConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.bind.port(), 65535);
    }
}
