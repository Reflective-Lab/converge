//! Verified identity types.
//!
//! After authentication, requests carry a VerifiedIdentity that combines:
//! - ServiceIdentity: The calling service (from mTLS client cert)
//! - UserIdentity: The end user (from JWT claims)

use serde::{Deserialize, Serialize};

/// Identity of a service extracted from mTLS client certificate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceIdentity {
    /// Service identifier (from cert CN or SPIFFE SAN).
    pub service_id: String,

    /// Optional SPIFFE ID if using SPIFFE/SPIRE.
    pub spiffe_id: Option<String>,

    /// Certificate fingerprint for audit logging.
    pub cert_fingerprint: Option<String>,
}

impl ServiceIdentity {
    /// Create a new service identity.
    pub fn new(service_id: impl Into<String>) -> Self {
        Self {
            service_id: service_id.into(),
            spiffe_id: None,
            cert_fingerprint: None,
        }
    }

    /// Create a service identity from a SPIFFE ID.
    pub fn from_spiffe(spiffe_id: impl Into<String>) -> Self {
        let spiffe = spiffe_id.into();
        // Extract service name from spiffe://trust-domain/path/to/service
        let service_id = spiffe
            .strip_prefix("spiffe://")
            .and_then(|s| s.rsplit('/').next())
            .unwrap_or(&spiffe)
            .to_string();

        Self {
            service_id,
            spiffe_id: Some(spiffe),
            cert_fingerprint: None,
        }
    }

    /// Set the certificate fingerprint.
    #[must_use]
    pub fn with_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.cert_fingerprint = Some(fingerprint.into());
        self
    }
}

/// Identity of a user extracted from JWT claims.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserIdentity {
    /// User's unique identifier (sub claim).
    pub user_id: String,

    /// Optional email address.
    pub email: Option<String>,

    /// Roles assigned to the user.
    pub roles: Vec<String>,

    /// Optional organization/tenant ID.
    pub org_id: Option<String>,

    /// JWT ID for audit logging.
    pub jti: Option<String>,
}

impl UserIdentity {
    /// Create a new user identity.
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            email: None,
            roles: Vec::new(),
            org_id: None,
            jti: None,
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

    /// Set the organization ID.
    #[must_use]
    pub fn with_org_id(mut self, org_id: impl Into<String>) -> Self {
        self.org_id = Some(org_id.into());
        self
    }

    /// Set the JWT ID.
    #[must_use]
    pub fn with_jti(mut self, jti: impl Into<String>) -> Self {
        self.jti = Some(jti.into());
        self
    }

    /// Check if user has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

/// Combined verified identity from authentication.
///
/// This represents a fully authenticated request with:
/// - Service identity from mTLS (required for service-to-service)
/// - User identity from JWT (optional, present for user-initiated requests)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedIdentity {
    /// The calling service identity (from mTLS).
    pub service: ServiceIdentity,

    /// The end user identity (from JWT), if present.
    pub user: Option<UserIdentity>,

    /// Timestamp when this identity was verified.
    pub verified_at: u64,
}

impl VerifiedIdentity {
    /// Create a service-only identity (service-to-service call).
    pub fn service_only(service: ServiceIdentity) -> Self {
        Self {
            service,
            user: None,
            verified_at: Self::now(),
        }
    }

    /// Create an identity with both service and user (user-initiated request).
    pub fn with_user(service: ServiceIdentity, user: UserIdentity) -> Self {
        Self {
            service,
            user: Some(user),
            verified_at: Self::now(),
        }
    }

    /// Check if this is a user-initiated request.
    pub fn has_user(&self) -> bool {
        self.user.is_some()
    }

    /// Get the effective principal for authorization.
    ///
    /// Returns the user ID if present, otherwise the service ID.
    pub fn principal(&self) -> &str {
        self.user
            .as_ref()
            .map(|u| u.user_id.as_str())
            .unwrap_or(&self.service.service_id)
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_identity_new() {
        let svc = ServiceIdentity::new("my-service");
        assert_eq!(svc.service_id, "my-service");
        assert!(svc.spiffe_id.is_none());
    }

    #[test]
    fn test_service_identity_from_spiffe() {
        let svc = ServiceIdentity::from_spiffe("spiffe://example.org/ns/prod/sa/runtime");
        assert_eq!(svc.service_id, "runtime");
        assert_eq!(
            svc.spiffe_id,
            Some("spiffe://example.org/ns/prod/sa/runtime".to_string())
        );
    }

    #[test]
    fn test_user_identity_roles() {
        let user = UserIdentity::new("user-123").with_roles(vec!["admin".to_string()]);
        assert!(user.has_role("admin"));
        assert!(!user.has_role("viewer"));
    }

    #[test]
    fn test_verified_identity_principal() {
        let svc = ServiceIdentity::new("api-gateway");
        let user = UserIdentity::new("user-456");

        let service_only = VerifiedIdentity::service_only(svc.clone());
        assert_eq!(service_only.principal(), "api-gateway");

        let with_user = VerifiedIdentity::with_user(svc, user);
        assert_eq!(with_user.principal(), "user-456");
    }
}
