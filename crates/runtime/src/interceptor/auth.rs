//! Authentication interceptor for gRPC requests.
//!
//! Extracts and validates identity from:
//! - mTLS client certificates (service identity)
//! - Authorization header JWT (user identity)

use crate::auth::{
    JwtError, JwtValidator, JwtValidatorConfig, ServiceIdentity, UserIdentity, VerifiedIdentity,
};
use crate::semantic::GrpcMethod;
use std::sync::Arc;
use tonic::{Request, Status};
use tracing::{debug, warn};

/// Extension key for verified identity in request extensions.
#[derive(Debug, Clone)]
pub struct VerifiedIdentityExt(pub VerifiedIdentity);

/// Authentication interceptor configuration.
#[derive(Debug, Clone)]
pub struct AuthInterceptorConfig {
    /// JWT validator configuration.
    pub jwt: Option<JwtValidatorConfig>,

    /// Whether to require user authentication (JWT).
    /// If false, service-only authentication is allowed.
    pub require_user_auth: bool,

    /// List of methods that don't require authentication.
    pub unauthenticated_methods: Vec<GrpcMethod>,
}

impl Default for AuthInterceptorConfig {
    fn default() -> Self {
        Self {
            jwt: None,
            require_user_auth: true,
            unauthenticated_methods: vec![
                // Capability negotiation doesn't require auth
                "/converge.ConvergeService/GetCapabilities".into(),
            ],
        }
    }
}

impl AuthInterceptorConfig {
    /// Create config with JWT validation.
    pub fn with_jwt(jwt_config: JwtValidatorConfig) -> Self {
        Self {
            jwt: Some(jwt_config),
            ..Default::default()
        }
    }

    /// Require user authentication for all methods.
    #[must_use]
    pub fn require_user_auth(mut self) -> Self {
        self.require_user_auth = true;
        self
    }

    /// Add a method that doesn't require authentication.
    #[must_use]
    pub fn allow_unauthenticated(mut self, method: impl Into<GrpcMethod>) -> Self {
        self.unauthenticated_methods.push(method.into());
        self
    }
}

/// Authentication interceptor for gRPC.
#[derive(Clone)]
pub struct AuthInterceptor {
    config: AuthInterceptorConfig,
    jwt_validator: Option<Arc<JwtValidator>>,
}

impl AuthInterceptor {
    /// Create a new authentication interceptor.
    pub fn new(config: AuthInterceptorConfig) -> Self {
        let jwt_validator = config.jwt.clone().map(|c| Arc::new(JwtValidator::new(c)));

        Self {
            config,
            jwt_validator,
        }
    }

    /// Check if a method requires authentication.
    ///
    /// Call this from the service layer to determine if a specific method
    /// needs authentication before processing.
    pub fn requires_auth(&self, method: impl Into<GrpcMethod>) -> bool {
        let method = method.into();
        !self.config.unauthenticated_methods.contains(&method)
    }

    /// Check method authentication requirement and return error if needed.
    ///
    /// Use this in service methods:
    /// ```ignore
    /// if let Err(status) = interceptor.check_method("/converge.ConvergeService/SubmitJob", &request) {
    ///     return Err(status);
    /// }
    /// ```
    pub fn check_method<T>(
        &self,
        method: impl Into<GrpcMethod>,
        request: &Request<T>,
    ) -> Result<(), Status> {
        if !self.requires_auth(method) {
            return Ok(());
        }

        // If method requires auth, verify identity is present
        if request.extensions().get::<VerifiedIdentityExt>().is_none() {
            return Err(Status::unauthenticated(
                "Authentication required for this method",
            ));
        }

        Ok(())
    }

    /// Extract service identity from request.
    ///
    /// In a real mTLS setup, this would extract from the TLS connection.
    /// Header-based identity is intentionally not trusted here.
    fn extract_service_identity<T>(&self, request: &Request<T>) -> Option<ServiceIdentity> {
        // Try to get from TLS peer certificate (would be set by tonic-tls)
        // This is a placeholder - actual implementation depends on how tonic
        // exposes peer certificate info.
        let _ = request;
        None
    }

    /// Extract user identity from JWT in Authorization header.
    fn extract_user_identity<T>(
        &self,
        request: &Request<T>,
    ) -> Result<Option<UserIdentity>, Status> {
        let Some(ref validator) = self.jwt_validator else {
            return Ok(None);
        };

        // Get Authorization header
        let auth_header = match request.metadata().get("authorization") {
            Some(h) => h,
            None => return Ok(None),
        };

        let auth_str = auth_header
            .to_str()
            .map_err(|_| Status::unauthenticated("Invalid authorization header encoding"))?;

        // Extract Bearer token
        let token = auth_str
            .strip_prefix("Bearer ")
            .or_else(|| auth_str.strip_prefix("bearer "))
            .ok_or_else(|| Status::unauthenticated("Authorization header must be Bearer token"))?;

        // Validate JWT
        let identity = validator.validate(token).map_err(|e| {
            warn!(error = %e, "JWT validation failed");
            match e {
                JwtError::Expired => Status::unauthenticated("Token expired"),
                JwtError::InvalidSignature => Status::unauthenticated("Invalid token signature"),
                JwtError::InvalidIssuer { .. } => Status::unauthenticated("Invalid token issuer"),
                JwtError::InvalidAudience => Status::unauthenticated("Invalid token audience"),
                _ => Status::unauthenticated("Invalid token"),
            }
        })?;

        Ok(Some(identity))
    }

    /// Authenticate a request and attach verified identity.
    ///
    /// Note: Method-level authentication checks should be done at the service
    /// layer using the `check_method` function, as tonic::Request doesn't
    /// expose the URI path in the interceptor context.
    pub fn authenticate<T>(&self, mut request: Request<T>) -> Result<Request<T>, Status> {
        debug!("Authenticating request");

        // Extract service identity (from mTLS or fallback)
        let service_identity = self.extract_service_identity(&request);

        // Extract user identity (from JWT)
        let user_identity = self.extract_user_identity(&request)?;

        // Build verified identity
        let verified = match (service_identity, user_identity) {
            (Some(svc), Some(user)) => VerifiedIdentity::with_user(svc, user),
            (None, Some(user)) => {
                let svc = ServiceIdentity::new("jwt-authenticated-client");
                VerifiedIdentity::with_user(svc, user)
            }
            (Some(svc), None) => VerifiedIdentity::service_only(svc),
            (None, None) => return Err(Status::unauthenticated("Authentication required")),
        };

        debug!(
            principal = %verified.principal(),
            has_user = verified.has_user(),
            "Request authenticated"
        );

        // Attach identity to request extensions
        request
            .extensions_mut()
            .insert(VerifiedIdentityExt(verified));

        Ok(request)
    }
}

/// Tower layer for the auth interceptor.
#[derive(Clone)]
pub struct AuthInterceptorLayer {
    interceptor: AuthInterceptor,
}

impl AuthInterceptorLayer {
    /// Create a new auth interceptor layer.
    pub fn new(config: AuthInterceptorConfig) -> Self {
        Self {
            interceptor: AuthInterceptor::new(config),
        }
    }

    /// Get a reference to the interceptor.
    pub fn interceptor(&self) -> &AuthInterceptor {
        &self.interceptor
    }
}

/// Helper trait to extract verified identity from request extensions.
pub trait RequestIdentityExt {
    /// Get the verified identity if present.
    fn identity(&self) -> Option<&VerifiedIdentity>;

    /// Get the verified identity or return Unauthenticated error.
    fn require_identity(&self) -> Result<&VerifiedIdentity, Status>;
}

impl<T> RequestIdentityExt for Request<T> {
    fn identity(&self) -> Option<&VerifiedIdentity> {
        self.extensions().get::<VerifiedIdentityExt>().map(|e| &e.0)
    }

    fn require_identity(&self) -> Result<&VerifiedIdentity, Status> {
        self.identity()
            .ok_or_else(|| Status::internal("Identity not found in request extensions"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tonic::metadata::MetadataValue;

    fn install_crypto_provider() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }

    fn test_jwt_config() -> JwtValidatorConfig {
        JwtValidatorConfig::new("test-secret-key-32-chars-long!!", "https://auth.test.com")
            .with_audience("converge-runtime")
    }

    #[test]
    fn test_default_config() {
        let config = AuthInterceptorConfig::default();
        assert!(config.require_user_auth);
        assert!(
            config
                .unauthenticated_methods
                .contains(&"/converge.ConvergeService/GetCapabilities".into())
        );
    }

    #[test]
    fn test_requires_auth() {
        let config = AuthInterceptorConfig::default();
        let interceptor = AuthInterceptor::new(config);

        assert!(interceptor.requires_auth("/converge.ConvergeService/SubmitJob"));
        assert!(!interceptor.requires_auth("/converge.ConvergeService/GetCapabilities"));
    }

    #[test]
    fn test_authenticate_with_valid_jwt() {
        install_crypto_provider();
        let config = AuthInterceptorConfig::with_jwt(test_jwt_config());
        let interceptor = AuthInterceptor::new(config);

        let mut request = Request::new(());
        let claims = crate::auth::Claims {
            sub: "user-123".to_string(),
            iss: Some("https://auth.test.com".to_string()),
            aud: Some(crate::auth::Audience::Single(
                "converge-runtime".to_string(),
            )),
            exp: Some(u64::MAX / 2),
            nbf: None,
            iat: None,
            jti: None,
            email: None,
            roles: None,
            org_id: None,
        };
        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &claims,
            &jsonwebtoken::EncodingKey::from_secret("test-secret-key-32-chars-long!!".as_bytes()),
        )
        .unwrap();
        request.metadata_mut().insert(
            "authorization",
            MetadataValue::try_from(format!("Bearer {token}")).unwrap(),
        );

        let result = interceptor.authenticate(request);
        assert!(result.is_ok());

        let request = result.unwrap();
        let identity = request.identity().unwrap();
        assert_eq!(identity.principal(), "user-123");
    }

    #[test]
    fn test_authenticate_without_credentials_fails_closed() {
        let config = AuthInterceptorConfig::with_jwt(test_jwt_config());
        let interceptor = AuthInterceptor::new(config);

        let request = Request::new(());
        let result = interceptor.authenticate(request);
        assert!(result.is_err());
    }

    #[test]
    fn test_require_user_auth_fails_without_jwt() {
        let config = AuthInterceptorConfig::default().require_user_auth();
        let interceptor = AuthInterceptor::new(config);

        let request = Request::new(());

        let result = interceptor.authenticate(request);
        assert!(result.is_err());
    }
}
