//! JWT validation for user authentication.
//!
//! Validates JWTs issued by the auth service and extracts user claims.

use super::identity::UserIdentity;
use crate::semantic::RoleId;
use jsonwebtoken::{DecodingKey, TokenData, Validation, decode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::{debug, warn};

/// Errors that can occur during JWT validation.
#[derive(Debug, Error)]
pub enum JwtError {
    /// Token has expired.
    #[error("token expired")]
    Expired,

    /// Token signature is invalid.
    #[error("invalid signature")]
    InvalidSignature,

    /// Token is malformed.
    #[error("malformed token: {0}")]
    Malformed(String),

    /// Required claim is missing.
    #[error("missing required claim: {0}")]
    MissingClaim(String),

    /// Token issuer doesn't match expected.
    #[error("invalid issuer: expected {expected}, got {actual}")]
    InvalidIssuer { expected: String, actual: String },

    /// Token audience doesn't match expected.
    #[error("invalid audience")]
    InvalidAudience,

    /// Token is not yet valid (nbf claim).
    #[error("token not yet valid")]
    NotYetValid,
}

/// Configuration for JWT validation.
#[derive(Debug, Clone)]
pub struct JwtValidatorConfig {
    /// Secret key for HMAC validation (HS256/HS384/HS512).
    /// In production, prefer RS256 with public key.
    pub secret: String,

    /// Expected token issuer (iss claim).
    pub issuer: String,

    /// Expected audience (aud claim).
    pub audience: String,

    /// Leeway in seconds for expiration checks.
    pub leeway_secs: u64,

    /// Whether to require the exp claim.
    pub require_exp: bool,
}

impl JwtValidatorConfig {
    /// Create a new JWT validator config.
    pub fn new(secret: impl Into<String>, issuer: impl Into<String>) -> Self {
        Self {
            secret: secret.into(),
            issuer: issuer.into(),
            audience: "converge-runtime".to_string(),
            leeway_secs: 60,
            require_exp: true,
        }
    }

    /// Set the expected audience.
    #[must_use]
    pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
        self.audience = audience.into();
        self
    }

    /// Set the leeway for expiration checks.
    #[must_use]
    pub fn with_leeway(mut self, secs: u64) -> Self {
        self.leeway_secs = secs;
        self
    }
}

/// Standard JWT claims we expect from the auth service.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// Subject - the user ID.
    pub sub: String,

    /// Issuer.
    pub iss: Option<String>,

    /// Audience.
    pub aud: Option<Audience>,

    /// Expiration time (Unix timestamp).
    pub exp: Option<u64>,

    /// Not before (Unix timestamp).
    pub nbf: Option<u64>,

    /// Issued at (Unix timestamp).
    pub iat: Option<u64>,

    /// JWT ID.
    pub jti: Option<String>,

    // Custom claims
    /// User's email.
    pub email: Option<String>,

    /// User's roles.
    pub roles: Option<Vec<String>>,

    /// Organization/tenant ID.
    pub org_id: Option<String>,
}

/// Audience can be a single string or array of strings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

impl Audience {
    fn contains(&self, target: &str) -> bool {
        match self {
            Audience::Single(s) => s == target,
            Audience::Multiple(v) => v.iter().any(|s| s == target),
        }
    }
}

/// JWT token validator.
pub struct JwtValidator {
    config: JwtValidatorConfig,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtValidator {
    /// Create a new JWT validator.
    pub fn new(config: JwtValidatorConfig) -> Self {
        let decoding_key = DecodingKey::from_secret(config.secret.as_bytes());

        let mut validation = Validation::default();
        validation.set_issuer(&[&config.issuer]);
        validation.set_audience(&[&config.audience]);
        validation.leeway = config.leeway_secs;
        validation.validate_exp = config.require_exp;

        Self {
            config,
            decoding_key,
            validation,
        }
    }

    /// Validate a JWT and extract user identity.
    pub fn validate(&self, token: &str) -> Result<UserIdentity, JwtError> {
        debug!("Validating JWT");

        // Decode and validate the token
        let token_data: TokenData<Claims> = decode(token, &self.decoding_key, &self.validation)
            .map_err(|e| {
                warn!(error = %e, "JWT validation failed");
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => JwtError::Expired,
                    jsonwebtoken::errors::ErrorKind::InvalidSignature => JwtError::InvalidSignature,
                    jsonwebtoken::errors::ErrorKind::InvalidIssuer => JwtError::InvalidIssuer {
                        expected: self.config.issuer.clone(),
                        actual: "unknown".to_string(),
                    },
                    jsonwebtoken::errors::ErrorKind::InvalidAudience => JwtError::InvalidAudience,
                    jsonwebtoken::errors::ErrorKind::ImmatureSignature => JwtError::NotYetValid,
                    _ => JwtError::Malformed(e.to_string()),
                }
            })?;

        let claims = token_data.claims;

        // Validate issuer matches (already done by jsonwebtoken, but double-check)
        if let Some(ref iss) = claims.iss {
            if iss != &self.config.issuer {
                return Err(JwtError::InvalidIssuer {
                    expected: self.config.issuer.clone(),
                    actual: iss.clone(),
                });
            }
        }

        // Validate audience contains our expected audience
        if let Some(ref aud) = claims.aud {
            if !aud.contains(&self.config.audience) {
                return Err(JwtError::InvalidAudience);
            }
        }

        // Build user identity from claims
        let mut identity = UserIdentity::new(claims.sub.as_str());

        if let Some(email) = claims.email {
            identity = identity.with_email(email);
        }

        if let Some(roles) = claims.roles {
            identity = identity.with_roles(roles.into_iter().map(RoleId::from).collect());
        }

        if let Some(org_id) = claims.org_id {
            identity = identity.with_org_id(org_id);
        }

        if let Some(jti) = claims.jti {
            identity = identity.with_jti(jti);
        }

        debug!(user_id = %identity.user_id, "JWT validated successfully");
        Ok(identity)
    }

    /// Extract claims without full validation (for debugging).
    /// WARNING: Do not use for authorization decisions.
    pub fn decode_without_validation(&self, token: &str) -> Result<Claims, JwtError> {
        let token_data: TokenData<Claims> = jsonwebtoken::dangerous::insecure_decode(token)
            .map_err(|e| JwtError::Malformed(e.to_string()))?;

        Ok(token_data.claims)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::RoleId;
    use jsonwebtoken::{EncodingKey, Header, encode};

    fn install_crypto_provider() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }

    fn create_test_token(claims: &Claims, secret: &str) -> String {
        install_crypto_provider();
        encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    fn test_config() -> JwtValidatorConfig {
        JwtValidatorConfig::new("test-secret-key-32-chars-long!!", "https://auth.test.com")
            .with_audience("test-service")
    }

    fn valid_claims() -> Claims {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Claims {
            sub: "user-123".to_string(),
            iss: Some("https://auth.test.com".to_string()),
            aud: Some(Audience::Single("test-service".to_string())),
            exp: Some(now + 3600),
            nbf: Some(now - 60),
            iat: Some(now),
            jti: Some("jwt-456".to_string()),
            email: Some("user@example.com".to_string()),
            roles: Some(vec!["admin".to_string(), "user".to_string()]),
            org_id: Some("org-789".to_string()),
        }
    }

    #[test]
    fn test_validate_valid_token() {
        let config = test_config();
        let validator = JwtValidator::new(config.clone());
        let token = create_test_token(&valid_claims(), &config.secret);

        let identity = validator.validate(&token).unwrap();
        assert_eq!(identity.user_id, "user-123");
        assert_eq!(identity.email, Some("user@example.com".to_string()));
        assert!(identity.has_role(&RoleId::new("admin")));
        assert_eq!(identity.org_id, Some("org-789".into()));
    }

    #[test]
    fn test_validate_expired_token() {
        let config = test_config();
        let validator = JwtValidator::new(config.clone());

        let mut claims = valid_claims();
        claims.exp = Some(0); // Expired

        let token = create_test_token(&claims, &config.secret);
        let result = validator.validate(&token);

        assert!(matches!(result, Err(JwtError::Expired)));
    }

    #[test]
    fn test_validate_wrong_issuer() {
        let config = test_config();
        let validator = JwtValidator::new(config.clone());

        let mut claims = valid_claims();
        claims.iss = Some("https://wrong-issuer.com".to_string());

        let token = create_test_token(&claims, &config.secret);
        let result = validator.validate(&token);

        assert!(matches!(result, Err(JwtError::InvalidIssuer { .. })));
    }

    #[test]
    fn test_validate_wrong_secret() {
        let config = test_config();
        let validator = JwtValidator::new(config);

        let token = create_test_token(&valid_claims(), "wrong-secret-key-32-chars-long!!");
        let result = validator.validate(&token);

        assert!(matches!(result, Err(JwtError::InvalidSignature)));
    }

    #[test]
    fn test_audience_contains() {
        let single = Audience::Single("service-a".to_string());
        assert!(single.contains("service-a"));
        assert!(!single.contains("service-b"));

        let multiple = Audience::Multiple(vec!["service-a".to_string(), "service-b".to_string()]);
        assert!(multiple.contains("service-a"));
        assert!(multiple.contains("service-b"));
        assert!(!multiple.contains("service-c"));
    }
}
