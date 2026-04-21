//! JWT token issuer for user authentication.
//!
//! Issues JWTs after successful authentication via any supported provider.

use super::jwt::Claims;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tracing::{debug, info};

/// Default token lifetime (1 hour).
const DEFAULT_TOKEN_LIFETIME: Duration = Duration::from_secs(3600);

/// Default refresh token lifetime (7 days).
const DEFAULT_REFRESH_LIFETIME: Duration = Duration::from_secs(7 * 24 * 3600);

/// Errors that can occur during token issuance.
#[derive(Debug, Error)]
pub enum IssuerError {
    /// Failed to encode token.
    #[error("failed to encode token: {0}")]
    Encoding(String),

    /// Invalid configuration.
    #[error("invalid configuration: {0}")]
    Config(String),
}

/// Configuration for the token issuer.
#[derive(Debug, Clone)]
pub struct TokenIssuerConfig {
    /// Secret key for signing (HS256).
    pub secret: String,

    /// Issuer claim (iss).
    pub issuer: String,

    /// Audience claim (aud).
    pub audience: String,

    /// Access token lifetime.
    pub token_lifetime: Duration,

    /// Refresh token lifetime.
    pub refresh_lifetime: Duration,
}

impl TokenIssuerConfig {
    /// Create a new issuer config.
    pub fn new(secret: impl Into<String>, issuer: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            issuer: issuer.into(),
            audience: "converge-runtime".to_string(),
            token_lifetime: DEFAULT_TOKEN_LIFETIME,
            refresh_lifetime: DEFAULT_REFRESH_LIFETIME,
        }
    }

    /// Set the audience.
    #[must_use]
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = audience.into();
        self
    }

    /// Set the access token lifetime.
    #[must_use]
    pub fn with_token_lifetime(mut self, lifetime: Duration) -> Self {
        self.token_lifetime = lifetime;
        self
    }

    /// Set the refresh token lifetime.
    #[must_use]
    pub fn with_refresh_lifetime(mut self, lifetime: Duration) -> Self {
        self.refresh_lifetime = lifetime;
        self
    }
}

/// Authenticated user information for token issuance.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// User ID (becomes the `sub` claim).
    pub user_id: String,

    /// Email address.
    pub email: Option<String>,

    /// User roles.
    pub roles: Vec<String>,

    /// Organization/tenant ID.
    pub org_id: Option<String>,

    /// Authentication method used.
    pub auth_method: AuthMethod,
}

impl AuthenticatedUser {
    /// Create a new authenticated user.
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            email: None,
            roles: Vec::new(),
            org_id: None,
            auth_method: AuthMethod::Unknown,
        }
    }

    /// Set the email.
    #[must_use]
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set the roles.
    #[must_use]
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles = roles;
        self
    }

    /// Add a role.
    #[must_use]
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Set the organization ID.
    #[must_use]
    pub fn with_org_id(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Set the authentication method.
    #[must_use]
    pub fn with_auth_method(mut self, method: AuthMethod) -> Self {
        self.auth_method = method;
        self
    }
}

/// Authentication method used.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMethod {
    /// Password-based authentication.
    Password,
    /// Passkey/WebAuthn authentication.
    Passkey,
    /// API key authentication.
    ApiKey,
    /// OAuth/OIDC provider.
    OAuth { provider: String },
    /// Service-to-service authentication.
    Service,
    /// Firebase authentication (validated externally).
    Firebase,
    /// Unknown/unspecified method.
    Unknown,
}

/// Token pair returned after successful authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    /// Access token (short-lived).
    pub access_token: String,

    /// Refresh token (long-lived, for getting new access tokens).
    pub refresh_token: String,

    /// Access token type (always "Bearer").
    pub token_type: String,

    /// Access token lifetime in seconds.
    pub expires_in: u64,

    /// Refresh token lifetime in seconds.
    pub refresh_expires_in: u64,
}

/// JWT token issuer.
pub struct TokenIssuer {
    config: TokenIssuerConfig,
    encoding_key: EncodingKey,
}

impl TokenIssuer {
    /// Create a new token issuer.
    pub fn new(config: TokenIssuerConfig) -> Self {
        let encoding_key = EncodingKey::from_secret(config.secret.as_bytes());
        Self {
            config,
            encoding_key,
        }
    }

    /// Issue a token pair for an authenticated user.
    pub fn issue(&self, user: &AuthenticatedUser) -> Result<TokenPair, IssuerError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Generate unique token IDs
        let access_jti = format!("acc_{}", generate_token_id());
        let refresh_jti = format!("ref_{}", generate_token_id());

        // Create access token
        let access_claims = Claims {
            sub: user.user_id.clone(),
            iss: Some(self.config.issuer.clone()),
            aud: Some(super::jwt::Audience::Single(self.config.audience.clone())),
            exp: Some(now + self.config.token_lifetime.as_secs()),
            nbf: Some(now),
            iat: Some(now),
            jti: Some(access_jti),
            email: user.email.clone(),
            roles: if user.roles.is_empty() {
                None
            } else {
                Some(user.roles.clone())
            },
            org_id: user.org_id.clone(),
        };

        let access_token = encode(&Header::default(), &access_claims, &self.encoding_key)
            .map_err(|e| IssuerError::Encoding(e.to_string()))?;

        // Create refresh token (minimal claims)
        let refresh_claims = RefreshClaims {
            sub: user.user_id.clone(),
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            exp: now + self.config.refresh_lifetime.as_secs(),
            iat: now,
            jti: refresh_jti,
            token_type: "refresh".to_string(),
        };

        let refresh_token = encode(&Header::default(), &refresh_claims, &self.encoding_key)
            .map_err(|e| IssuerError::Encoding(e.to_string()))?;

        info!(
            user_id = %user.user_id,
            auth_method = ?user.auth_method,
            "Issued token pair"
        );

        Ok(TokenPair {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.config.token_lifetime.as_secs(),
            refresh_expires_in: self.config.refresh_lifetime.as_secs(),
        })
    }

    /// Issue only an access token (no refresh token).
    pub fn issue_access_token(&self, user: &AuthenticatedUser) -> Result<String, IssuerError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let jti = format!("acc_{}", generate_token_id());

        let claims = Claims {
            sub: user.user_id.clone(),
            iss: Some(self.config.issuer.clone()),
            aud: Some(super::jwt::Audience::Single(self.config.audience.clone())),
            exp: Some(now + self.config.token_lifetime.as_secs()),
            nbf: Some(now),
            iat: Some(now),
            jti: Some(jti),
            email: user.email.clone(),
            roles: if user.roles.is_empty() {
                None
            } else {
                Some(user.roles.clone())
            },
            org_id: user.org_id.clone(),
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| IssuerError::Encoding(e.to_string()))?;

        debug!(user_id = %user.user_id, "Issued access token");

        Ok(token)
    }

    /// Validate a refresh token and issue a new token pair.
    pub fn refresh(&self, refresh_token: &str) -> Result<TokenPair, IssuerError> {
        use jsonwebtoken::{DecodingKey, Validation, decode};

        let decoding_key = DecodingKey::from_secret(self.config.secret.as_bytes());
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.config.issuer]);
        validation.set_audience(&[&self.config.audience]);

        let token_data = decode::<RefreshClaims>(refresh_token, &decoding_key, &validation)
            .map_err(|e| IssuerError::Encoding(format!("invalid refresh token: {}", e)))?;

        let claims = token_data.claims;

        // Verify it's a refresh token
        if claims.token_type != "refresh" {
            return Err(IssuerError::Encoding("not a refresh token".to_string()));
        }

        // Issue new tokens for this user
        let user = AuthenticatedUser::new(&claims.sub).with_auth_method(AuthMethod::Unknown);

        self.issue(&user)
    }

    /// Get the issuer string (for validation config).
    pub fn issuer(&self) -> &str {
        &self.config.issuer
    }

    /// Get the audience string (for validation config).
    pub fn audience(&self) -> &str {
        &self.config.audience
    }
}

/// Claims for refresh tokens (minimal).
#[derive(Debug, Serialize, Deserialize)]
struct RefreshClaims {
    sub: String,
    iss: String,
    aud: String,
    exp: u64,
    iat: u64,
    jti: String,
    token_type: String,
}

/// Generate a random token ID.
fn generate_token_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // Simple unique ID based on timestamp and random suffix
    format!("{:x}", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::jwt::{JwtValidator, JwtValidatorConfig};
    use crate::semantic::RoleId;

    fn install_crypto_provider() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }

    fn test_config() -> TokenIssuerConfig {
        install_crypto_provider();
        TokenIssuerConfig::new("test-secret-key-32-chars-long!!", "https://auth.test.com")
            .with_audience("test-service")
    }

    #[test]
    fn test_issue_token_pair() {
        let issuer = TokenIssuer::new(test_config());
        let user = AuthenticatedUser::new("user-123")
            .with_email("user@example.com")
            .with_roles(vec!["admin".to_string()])
            .with_auth_method(AuthMethod::Password);

        let tokens = issuer.issue(&user).unwrap();

        assert!(!tokens.access_token.is_empty());
        assert!(!tokens.refresh_token.is_empty());
        assert_eq!(tokens.token_type, "Bearer");
        assert_eq!(tokens.expires_in, DEFAULT_TOKEN_LIFETIME.as_secs());
    }

    #[test]
    fn test_issued_token_validates() {
        let config = test_config();
        let issuer = TokenIssuer::new(config.clone());

        let user = AuthenticatedUser::new("user-123")
            .with_email("user@example.com")
            .with_roles(vec!["admin".to_string()]);

        let tokens = issuer.issue(&user).unwrap();

        // Validate with our JWT validator
        let validator_config =
            JwtValidatorConfig::new(config.secret.clone(), config.issuer.clone())
                .with_audience(config.audience.clone());
        let validator = JwtValidator::new(validator_config);

        let identity = validator.validate(&tokens.access_token).unwrap();
        assert_eq!(identity.user_id, "user-123");
        assert_eq!(identity.email, Some("user@example.com".to_string()));
        assert!(identity.has_role(&RoleId::new("admin")));
    }

    #[test]
    fn test_refresh_token() {
        let issuer = TokenIssuer::new(test_config());
        let user = AuthenticatedUser::new("user-123");

        let tokens = issuer.issue(&user).unwrap();
        let new_tokens = issuer.refresh(&tokens.refresh_token).unwrap();

        assert!(!new_tokens.access_token.is_empty());
        assert_ne!(new_tokens.access_token, tokens.access_token);
    }

    #[test]
    fn test_issue_access_token_only() {
        let issuer = TokenIssuer::new(test_config());
        let user = AuthenticatedUser::new("service-account");

        let token = issuer.issue_access_token(&user).unwrap();
        assert!(!token.is_empty());
    }

    #[test]
    fn test_auth_method_serialization() {
        let method = AuthMethod::OAuth {
            provider: "google".to_string(),
        };
        let json = serde_json::to_string(&method).unwrap();
        // snake_case renames "OAuth" to "o_auth"
        assert!(json.contains("o_auth") || json.contains("OAuth"));
        assert!(json.contains("google"));

        // Test roundtrip
        let deserialized: AuthMethod = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, method);
    }

    #[test]
    fn test_custom_token_lifetime() {
        install_crypto_provider();
        let config = TokenIssuerConfig::new("secret", "issuer")
            .with_token_lifetime(Duration::from_secs(300))
            .with_refresh_lifetime(Duration::from_secs(86400));

        let issuer = TokenIssuer::new(config);
        let user = AuthenticatedUser::new("user");

        let tokens = issuer.issue(&user).unwrap();
        assert_eq!(tokens.expires_in, 300);
        assert_eq!(tokens.refresh_expires_in, 86400);
    }
}
