//! Shared HTTP authentication middleware for Axum routes.

use axum::{
    extract::Request,
    http::{HeaderMap, header::AUTHORIZATION},
    middleware::Next,
    response::Response,
};

#[cfg(feature = "firebase")]
use crate::auth::{FirebaseConfig, FirebaseValidator};
use crate::auth::{JwtValidator, JwtValidatorConfig};
use crate::error::RuntimeError;

use crate::auth::UserIdentity;

/// Authenticated user stored in request extensions.
#[derive(Debug, Clone)]
pub struct AuthenticatedUser(pub UserIdentity);

enum AuthMode {
    Jwt(JwtValidatorConfig),
    #[cfg(feature = "firebase")]
    Firebase(FirebaseConfig),
}

fn configured_auth_mode() -> Result<AuthMode, RuntimeError> {
    configured_auth_mode_from_lookup(|key| std::env::var(key).ok())
}

fn configured_auth_mode_from_lookup(
    mut get_var: impl FnMut(&str) -> Option<String>,
) -> Result<AuthMode, RuntimeError> {
    #[cfg(feature = "firebase")]
    if let Some(project_id) = get_var("FIREBASE_PROJECT_ID")
        .or_else(|| get_var("GOOGLE_CLOUD_PROJECT"))
        .or_else(|| get_var("GCP_PROJECT_ID"))
    {
        let project_id = project_id.trim();
        if !project_id.is_empty() {
            return Ok(AuthMode::Firebase(FirebaseConfig::new(project_id)));
        }
    }

    let secret = get_var("JWT_SECRET").ok_or_else(|| {
        RuntimeError::Config(
            "Authentication is not configured. Set JWT_SECRET and JWT_ISSUER, or enable Firebase auth with FIREBASE_PROJECT_ID.".to_string(),
        )
    })?;
    let issuer = get_var("JWT_ISSUER").ok_or_else(|| {
        RuntimeError::Config(
            "Authentication is not configured. Set JWT_SECRET and JWT_ISSUER, or enable Firebase auth with FIREBASE_PROJECT_ID.".to_string(),
        )
    })?;
    let audience = get_var("JWT_AUDIENCE").unwrap_or_else(|| "converge-runtime".to_string());

    let secret = secret.trim();
    let issuer = issuer.trim();
    let audience = audience.trim();

    if secret.is_empty() || issuer.is_empty() || audience.is_empty() {
        return Err(RuntimeError::Config(
            "JWT auth requires non-empty JWT_SECRET, JWT_ISSUER, and JWT_AUDIENCE values."
                .to_string(),
        ));
    }

    Ok(AuthMode::Jwt(
        JwtValidatorConfig::new(secret, issuer).with_audience(audience),
    ))
}

fn sanitize_token(token: &str) -> Result<String, RuntimeError> {
    let trimmed = token.trim().to_string();
    if trimmed.is_empty() {
        return Err(RuntimeError::Authentication(
            "Missing bearer token.".to_string(),
        ));
    }
    Ok(trimmed)
}

fn extract_bearer_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("bearer "))
        })
        .map(std::string::ToString::to_string)
}

pub(crate) async fn authenticate_request(
    headers: &HeaderMap,
) -> Result<UserIdentity, RuntimeError> {
    let token = sanitize_token(
        &extract_bearer_from_headers(headers)
            .ok_or_else(|| RuntimeError::Authentication("Missing bearer token.".to_string()))?,
    )?;

    validate_token(&token).await
}

pub(crate) async fn validate_token(token: &str) -> Result<UserIdentity, RuntimeError> {
    validate_token_with_mode(configured_auth_mode()?, token).await
}

async fn validate_token_with_mode(
    mode: AuthMode,
    token: &str,
) -> Result<UserIdentity, RuntimeError> {
    match mode {
        AuthMode::Jwt(config) => JwtValidator::new(config)
            .validate(token)
            .map_err(|e| RuntimeError::Authentication(format!("Invalid JWT: {e}"))),
        #[cfg(feature = "firebase")]
        AuthMode::Firebase(config) => FirebaseValidator::new(config)
            .validate(token)
            .await
            .map_err(|e| RuntimeError::Authentication(format!("Invalid Firebase token: {e}"))),
    }
}

pub async fn require_auth(mut request: Request, next: Next) -> Result<Response, RuntimeError> {
    let identity = authenticate_request(request.headers()).await?;
    request.extensions_mut().insert(AuthenticatedUser(identity));
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{Audience, Claims};
    use crate::semantic::RoleId;
    use jsonwebtoken::{EncodingKey, Header, encode};

    fn install_crypto_provider() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }

    #[test]
    fn bearer_header_extraction_rejects_missing_scheme() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "not-a-bearer".parse().unwrap());
        assert!(extract_bearer_from_headers(&headers).is_none());
    }

    #[tokio::test]
    async fn missing_auth_configuration_fails_closed() {
        let result = configured_auth_mode_from_lookup(|_| None);
        assert!(matches!(result, Err(RuntimeError::Config(_))));
    }

    #[tokio::test]
    async fn missing_authorization_header_is_rejected() {
        let headers = HeaderMap::new();
        let result = authenticate_request(&headers).await;
        assert!(matches!(result, Err(RuntimeError::Authentication(_))));
    }

    fn make_token(secret: &str, issuer: &str, audience: &str, subject: &str) -> String {
        install_crypto_provider();
        let claims = Claims {
            sub: subject.to_string(),
            iss: Some(issuer.to_string()),
            aud: Some(Audience::Single(audience.to_string())),
            exp: Some(u64::MAX / 2),
            nbf: None,
            iat: None,
            jti: None,
            email: None,
            roles: Some(vec!["admin".to_string()]),
            org_id: None,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn valid_jwt_authenticates() {
        let token = make_token(
            "test-secret-key-32-chars-long!!",
            "https://auth.test.com",
            "converge-runtime",
            "user-123",
        );

        let mode = AuthMode::Jwt(
            JwtValidatorConfig::new("test-secret-key-32-chars-long!!", "https://auth.test.com")
                .with_audience("converge-runtime"),
        );
        let identity = validate_token_with_mode(mode, &token).await.unwrap();
        assert_eq!(identity.user_id, "user-123");
        assert!(identity.has_role(&RoleId::new("admin")));
    }

    #[tokio::test]
    async fn invalid_jwt_is_rejected() {
        let token = make_token(
            "different-secret-key-32-chars-long",
            "https://auth.test.com",
            "converge-runtime",
            "user-123",
        );

        let mode = AuthMode::Jwt(
            JwtValidatorConfig::new("test-secret-key-32-chars-long!!", "https://auth.test.com")
                .with_audience("converge-runtime"),
        );
        let result = validate_token_with_mode(mode, &token).await;
        assert!(matches!(result, Err(RuntimeError::Authentication(_))));
    }
}
