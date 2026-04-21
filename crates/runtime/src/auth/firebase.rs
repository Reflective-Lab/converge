//! Firebase ID token validation.
//!
//! Validates ID tokens issued by Firebase Auth using Google's public keys.
//! Supports passkey authentication via Firebase.

use super::identity::UserIdentity;
use super::jwt::JwtError;
use crate::semantic::RoleId;
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation, decode, decode_header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Google's public key endpoint for Firebase tokens.
const GOOGLE_CERTS_URL: &str =
    "https://www.googleapis.com/robot/v1/metadata/x509/securetoken@system.gserviceaccount.com";

/// Default cache duration for public keys (1 hour).
const DEFAULT_KEY_CACHE_DURATION: Duration = Duration::from_secs(3600);

/// Firebase ID token claims.
#[derive(Debug, Serialize, Deserialize)]
pub struct FirebaseClaims {
    /// Subject - the user's Firebase UID.
    pub sub: String,

    /// Issuer - must be `https://securetoken.google.com/<project-id>`.
    pub iss: String,

    /// Audience - must be the Firebase project ID.
    pub aud: String,

    /// Expiration time (Unix timestamp).
    pub exp: u64,

    /// Issued at (Unix timestamp).
    pub iat: u64,

    /// Authentication time (Unix timestamp).
    pub auth_time: u64,

    /// User ID (same as sub).
    pub user_id: Option<String>,

    /// Email address (if available).
    pub email: Option<String>,

    /// Whether email is verified.
    pub email_verified: Option<bool>,

    /// Phone number (if available).
    pub phone_number: Option<String>,

    /// Display name.
    pub name: Option<String>,

    /// Profile picture URL.
    pub picture: Option<String>,

    /// Firebase-specific claims.
    pub firebase: Option<FirebaseSpecificClaims>,
}

/// Firebase-specific claims within the ID token.
#[derive(Debug, Serialize, Deserialize)]
pub struct FirebaseSpecificClaims {
    /// Sign-in provider (e.g., "password", "google.com", "phone").
    pub sign_in_provider: Option<String>,

    /// Second factor sign-in provider (for MFA).
    pub sign_in_second_factor: Option<String>,

    /// Tenant ID (for multi-tenancy).
    pub tenant: Option<String>,

    /// Identity provider data.
    pub identities: Option<HashMap<String, Vec<String>>>,
}

/// Cached public keys with expiration.
struct KeyCache {
    keys: HashMap<String, DecodingKey>,
    expires_at: Instant,
}

/// Configuration for Firebase ID token validation.
#[derive(Debug, Clone)]
pub struct FirebaseConfig {
    /// Firebase project ID.
    pub project_id: String,

    /// Key cache duration.
    pub key_cache_duration: Duration,
}

impl FirebaseConfig {
    /// Create a new Firebase config with the given project ID.
    pub fn new(project_id: impl Into<String>) -> Self {
        Self {
            project_id: project_id.into(),
            key_cache_duration: DEFAULT_KEY_CACHE_DURATION,
        }
    }

    /// Set custom key cache duration.
    #[must_use]
    pub fn with_cache_duration(mut self, duration: Duration) -> Self {
        self.key_cache_duration = duration;
        self
    }
}

/// Firebase ID token validator.
///
/// Validates tokens using Google's public keys with automatic key rotation support.
pub struct FirebaseValidator {
    config: FirebaseConfig,
    http_client: reqwest::Client,
    key_cache: RwLock<Option<KeyCache>>,
}

impl FirebaseValidator {
    /// Create a new Firebase validator.
    pub fn new(config: FirebaseConfig) -> Self {
        Self {
            config,
            http_client: reqwest::Client::new(),
            key_cache: RwLock::new(None),
        }
    }

    /// Validate a Firebase ID token and extract user identity.
    pub async fn validate(&self, token: &str) -> Result<UserIdentity, JwtError> {
        debug!("Validating Firebase ID token");

        // Get the key ID from the token header
        let header = decode_header(token).map_err(|e| JwtError::Malformed(e.to_string()))?;

        let kid = header
            .kid
            .ok_or_else(|| JwtError::Malformed("missing kid in token header".to_string()))?;

        // Ensure we're using RS256
        if header.alg != Algorithm::RS256 {
            return Err(JwtError::Malformed(format!(
                "invalid algorithm: expected RS256, got {:?}",
                header.alg
            )));
        }

        // Get the decoding key
        let decoding_key = self.get_decoding_key(&kid).await?;

        // Set up validation
        let expected_issuer = format!("https://securetoken.google.com/{}", self.config.project_id);
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[&expected_issuer]);
        validation.set_audience(&[&self.config.project_id]);
        validation.validate_exp = true;

        // Decode and validate
        let token_data: TokenData<FirebaseClaims> = decode(token, &decoding_key, &validation)
            .map_err(|e| {
                warn!(error = %e, "Firebase token validation failed");
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                    jsonwebtoken::errors::ErrorKind::InvalidSignature => JwtError::InvalidSignature,
                    jsonwebtoken::errors::ErrorKind::InvalidIssuer => JwtError::InvalidIssuer {
                        expected: expected_issuer.clone(),
                        actual: "unknown".to_string(),
                    },
                    jsonwebtoken::errors::ErrorKind::InvalidAudience => JwtError::InvalidAudience,
                    jsonwebtoken::errors::ErrorKind::ImmatureSignature => JwtError::NotYetValid,
                    _ => JwtError::Malformed(e.to_string()),
                }
            })?;

        let claims = token_data.claims;

        // Additional validation: auth_time must be in the past
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if claims.auth_time > now {
            return Err(JwtError::NotYetValid);
        }

        // Build user identity
        let mut identity = UserIdentity::new(claims.sub.as_str());

        if let Some(email) = claims.email {
            identity = identity.with_email(email);
        }

        // Add sign-in provider as a role for policy decisions
        if let Some(ref firebase) = claims.firebase {
            if let Some(ref provider) = firebase.sign_in_provider {
                identity = identity.with_roles(vec![RoleId::new(format!("firebase:{}", provider))]);
            }
        }

        debug!(
            user_id = %identity.user_id,
            email = ?identity.email,
            "Firebase ID token validated successfully"
        );

        Ok(identity)
    }

    /// Get or refresh the decoding key for the given key ID.
    async fn get_decoding_key(&self, kid: &str) -> Result<DecodingKey, JwtError> {
        // Check cache first
        {
            let cache = self.key_cache.read().unwrap();
            if let Some(ref cached) = *cache {
                if Instant::now() < cached.expires_at {
                    if let Some(key) = cached.keys.get(kid) {
                        return Ok(key.clone());
                    }
                }
            }
        }

        // Fetch fresh keys
        self.refresh_keys().await?;

        // Try again from cache
        let cache = self.key_cache.read().unwrap();
        if let Some(ref cached) = *cache {
            if let Some(key) = cached.keys.get(kid) {
                return Ok(key.clone());
            }
        }

        Err(JwtError::Malformed(format!("unknown key ID: {}", kid)))
    }

    /// Fetch and cache public keys from Google.
    async fn refresh_keys(&self) -> Result<(), JwtError> {
        info!("Fetching Firebase public keys from Google");

        let response = self
            .http_client
            .get(GOOGLE_CERTS_URL)
            .send()
            .await
            .map_err(|e| JwtError::Malformed(format!("failed to fetch public keys: {}", e)))?;

        if !response.status().is_success() {
            return Err(JwtError::Malformed(format!(
                "failed to fetch public keys: HTTP {}",
                response.status()
            )));
        }

        // Parse cache-control header for expiration
        let cache_duration = response
            .headers()
            .get("cache-control")
            .and_then(|v| v.to_str().ok())
            .and_then(parse_max_age)
            .unwrap_or(self.config.key_cache_duration);

        // Parse the certificate map
        let certs: HashMap<String, String> = response
            .json()
            .await
            .map_err(|e| JwtError::Malformed(format!("failed to parse public keys: {}", e)))?;

        // Convert PEM certificates to decoding keys
        let mut keys = HashMap::new();
        for (kid, pem) in certs {
            let key = DecodingKey::from_rsa_pem(pem.as_bytes()).map_err(|e| {
                JwtError::Malformed(format!("invalid certificate for {}: {}", kid, e))
            })?;
            keys.insert(kid, key);
        }

        info!(
            key_count = keys.len(),
            cache_seconds = cache_duration.as_secs(),
            "Cached Firebase public keys"
        );

        // Update cache
        let mut cache = self.key_cache.write().unwrap();
        *cache = Some(KeyCache {
            keys,
            expires_at: Instant::now() + cache_duration,
        });

        Ok(())
    }
}

/// Parse max-age from Cache-Control header.
fn parse_max_age(cache_control: &str) -> Option<Duration> {
    for part in cache_control.split(',') {
        let part = part.trim();
        if part.starts_with("max-age=") {
            if let Ok(secs) = part[8..].parse::<u64>() {
                return Some(Duration::from_secs(secs));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firebase_config_new() {
        let config = FirebaseConfig::new("my-project");
        assert_eq!(config.project_id, "my-project");
        assert_eq!(config.key_cache_duration, DEFAULT_KEY_CACHE_DURATION);
    }

    #[test]
    fn test_firebase_config_with_cache_duration() {
        let config =
            FirebaseConfig::new("my-project").with_cache_duration(Duration::from_secs(1800));
        assert_eq!(config.key_cache_duration, Duration::from_secs(1800));
    }

    #[test]
    fn test_parse_max_age() {
        assert_eq!(
            parse_max_age("public, max-age=3600, must-revalidate"),
            Some(Duration::from_secs(3600))
        );
        assert_eq!(
            parse_max_age("max-age=7200"),
            Some(Duration::from_secs(7200))
        );
        assert_eq!(parse_max_age("no-cache"), None);
        assert_eq!(parse_max_age("max-age=invalid"), None);
    }

    #[test]
    fn test_firebase_claims_deserialize() {
        let json = r#"{
            "sub": "user123",
            "iss": "https://securetoken.google.com/my-project",
            "aud": "my-project",
            "exp": 1700000000,
            "iat": 1699996400,
            "auth_time": 1699996400,
            "email": "user@example.com",
            "email_verified": true,
            "firebase": {
                "sign_in_provider": "password",
                "identities": {
                    "email": ["user@example.com"]
                }
            }
        }"#;

        let claims: FirebaseClaims = serde_json::from_str(json).unwrap();
        assert_eq!(claims.sub, "user123");
        assert_eq!(claims.email, Some("user@example.com".to_string()));
        assert_eq!(claims.email_verified, Some(true));
        assert!(claims.firebase.is_some());
        assert_eq!(
            claims.firebase.unwrap().sign_in_provider,
            Some("password".to_string())
        );
    }
}
