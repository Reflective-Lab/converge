//! Policy evaluation engine.

use super::types::{Effect, Policy, PolicyError};
use crate::auth::VerifiedIdentity;
use crate::semantic::{GrpcMethod, RoleId, ServiceId, UserId};
use std::sync::Arc;
use tracing::debug;

/// Policy engine for authorization decisions.
#[derive(Clone)]
pub struct PolicyEngine {
    policy: Arc<Policy>,
}

impl PolicyEngine {
    /// Create a new policy engine with the given policy.
    pub fn new(policy: Policy) -> Self {
        Self {
            policy: Arc::new(policy),
        }
    }

    /// Create a policy engine that allows all requests (development mode).
    pub fn allow_all() -> Self {
        Self::new(Policy::allow_all())
    }

    /// Create a policy engine that denies all requests.
    pub fn deny_all() -> Self {
        Self::new(Policy::deny_all())
    }

    /// Get a reference to the current policy.
    pub fn policy(&self) -> &Policy {
        &self.policy
    }

    /// Evaluate a request against the policy.
    ///
    /// Returns `Ok(())` if allowed, `Err(PolicyError::Denied)` if denied.
    pub fn evaluate(
        &self,
        method: impl Into<GrpcMethod>,
        identity: &VerifiedIdentity,
    ) -> Result<(), PolicyError> {
        let method = method.into();
        let user_id = identity.user.as_ref().map(|u| &u.user_id);
        let roles = identity
            .user
            .as_ref()
            .map(|u| u.roles.clone())
            .unwrap_or_default();
        let service_id = &identity.service.service_id;

        self.evaluate_raw(method, user_id, &roles, service_id)
    }

    /// Evaluate with raw parameters (for testing).
    pub fn evaluate_raw(
        &self,
        method: impl Into<GrpcMethod>,
        user_id: Option<&UserId>,
        roles: &[RoleId],
        service_id: &ServiceId,
    ) -> Result<(), PolicyError> {
        let method = method.into();
        debug!(
            method = %method,
            user_id = ?user_id,
            roles = ?roles,
            service_id = %service_id,
            "Evaluating policy"
        );

        // Evaluate rules in priority order
        for rule in self.policy.sorted_rules() {
            if rule.matches(&method, user_id, roles, service_id) {
                debug!(
                    rule = %rule.name,
                    effect = %rule.effect,
                    "Rule matched"
                );

                return match rule.effect {
                    Effect::Allow => Ok(()),
                    Effect::Deny => Err(PolicyError::Denied {
                        reason: format!("denied by rule '{}'", rule.name),
                    }),
                };
            }
        }

        // No rule matched, use default effect
        debug!(
            default_effect = %self.policy.default_effect,
            "No rule matched, using default"
        );

        match self.policy.default_effect {
            Effect::Allow => Ok(()),
            Effect::Deny => Err(PolicyError::NoMatch {
                method,
                principal: user_id
                    .map_or_else(|| service_id.to_string(), std::string::ToString::to_string),
            }),
        }
    }

    /// Check if a method is allowed without returning detailed error.
    pub fn is_allowed(&self, method: impl Into<GrpcMethod>, identity: &VerifiedIdentity) -> bool {
        self.evaluate(method, identity).is_ok()
    }

    /// Update the policy (for hot reloading).
    pub fn update_policy(&mut self, policy: Policy) {
        self.policy = Arc::new(policy);
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::deny_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{ServiceIdentity, UserIdentity};
    use crate::policy::types::{Principal, Rule};
    use crate::semantic::Selector;

    fn make_identity(service: &str, user: Option<(&str, Vec<&str>)>) -> VerifiedIdentity {
        let service = ServiceIdentity::new(service);
        match user {
            Some((uid, roles)) => {
                let user = UserIdentity::new(uid)
                    .with_roles(roles.into_iter().map(RoleId::from).collect());
                VerifiedIdentity::with_user(service, user)
            }
            None => VerifiedIdentity::service_only(service),
        }
    }

    #[test]
    fn test_allow_all_policy() {
        let engine = PolicyEngine::allow_all();
        let identity = make_identity("test-service", None);
        assert!(engine.evaluate("/any/Method", &identity).is_ok());
    }

    #[test]
    fn test_deny_all_policy() {
        let engine = PolicyEngine::deny_all();
        let identity = make_identity("test-service", None);
        assert!(engine.evaluate("/any/Method", &identity).is_err());
    }

    #[test]
    fn test_role_based_allow() {
        let policy = Policy {
            default_effect: Effect::Deny,
            rules: vec![Rule {
                name: "admin-access".to_string(),
                effect: Effect::Allow,
                principals: Principal {
                    roles: Selector::exact(vec!["admin".into()]),
                    ..Default::default()
                },
                methods: Selector::any(),
                condition: None,
                priority: 0,
            }],
            ..Default::default()
        };

        let engine = PolicyEngine::new(policy);

        // Admin should be allowed
        let admin = make_identity("svc", Some(("user-1", vec!["admin"])));
        assert!(engine.evaluate("/test/Method", &admin).is_ok());

        // User without admin role should be denied
        let user = make_identity("svc", Some(("user-2", vec!["user"])));
        assert!(engine.evaluate("/test/Method", &user).is_err());
    }

    #[test]
    fn test_service_based_allow() {
        let policy = Policy {
            default_effect: Effect::Deny,
            rules: vec![Rule {
                name: "gateway-access".to_string(),
                effect: Effect::Allow,
                principals: Principal {
                    services: Selector::exact(vec!["api-gateway".into()]),
                    ..Default::default()
                },
                methods: Selector::any(),
                condition: None,
                priority: 0,
            }],
            ..Default::default()
        };

        let engine = PolicyEngine::new(policy);

        let gateway = make_identity("api-gateway", None);
        assert!(engine.evaluate("/test/Method", &gateway).is_ok());

        let other = make_identity("other-service", None);
        assert!(engine.evaluate("/test/Method", &other).is_err());
    }

    #[test]
    fn test_method_specific_rule() {
        let policy = Policy {
            default_effect: Effect::Deny,
            rules: vec![Rule {
                name: "read-access".to_string(),
                effect: Effect::Allow,
                principals: Principal {
                    roles: Selector::exact(vec!["user".into()]),
                    ..Default::default()
                },
                methods: Selector::exact(vec![
                    "/converge.Service/GetJob".into(),
                    "/converge.Service/GetCapabilities".into(),
                ]),
                condition: None,
                priority: 0,
            }],
            ..Default::default()
        };

        let engine = PolicyEngine::new(policy);
        let user = make_identity("svc", Some(("user-1", vec!["user"])));

        // Allowed methods
        assert!(engine.evaluate("/converge.Service/GetJob", &user).is_ok());
        assert!(
            engine
                .evaluate("/converge.Service/GetCapabilities", &user)
                .is_ok()
        );

        // Denied method
        assert!(
            engine
                .evaluate("/converge.Service/SubmitJob", &user)
                .is_err()
        );
    }

    #[test]
    fn test_priority_ordering() {
        let policy = Policy {
            default_effect: Effect::Allow,
            rules: vec![
                Rule {
                    name: "deny-all".to_string(),
                    effect: Effect::Deny,
                    principals: Principal {
                        roles: Selector::any(),
                        ..Default::default()
                    },
                    methods: Selector::any(),
                    condition: None,
                    priority: 1, // Lower priority
                },
                Rule {
                    name: "allow-admin".to_string(),
                    effect: Effect::Allow,
                    principals: Principal {
                        roles: Selector::exact(vec!["admin".into()]),
                        ..Default::default()
                    },
                    methods: Selector::any(),
                    condition: None,
                    priority: 100, // Higher priority - evaluated first
                },
            ],
            ..Default::default()
        };

        let engine = PolicyEngine::new(policy);

        // Admin matches higher priority rule first
        let admin = make_identity("svc", Some(("user-1", vec!["admin"])));
        assert!(engine.evaluate("/test/Method", &admin).is_ok());

        // Regular user matches lower priority deny rule
        let user = make_identity("svc", Some(("user-2", vec!["user"])));
        assert!(engine.evaluate("/test/Method", &user).is_err());
    }

    #[test]
    fn test_explicit_deny_rule() {
        let policy = Policy {
            default_effect: Effect::Allow,
            rules: vec![Rule {
                name: "deny-dangerous".to_string(),
                effect: Effect::Deny,
                principals: Principal {
                    roles: Selector::any(),
                    ..Default::default()
                },
                methods: Selector::exact(vec!["/admin/DeleteAll".into()]),
                condition: None,
                priority: 100,
            }],
            ..Default::default()
        };

        let engine = PolicyEngine::new(policy);
        let admin = make_identity("svc", Some(("user-1", vec!["admin"])));

        // Dangerous method denied even for admin
        assert!(engine.evaluate("/admin/DeleteAll", &admin).is_err());

        // Other methods allowed by default
        assert!(engine.evaluate("/safe/Method", &admin).is_ok());
    }

    #[test]
    fn test_is_allowed() {
        let engine = PolicyEngine::allow_all();
        let identity = make_identity("svc", None);
        assert!(engine.is_allowed("/any/Method", &identity));

        let engine = PolicyEngine::deny_all();
        assert!(!engine.is_allowed("/any/Method", &identity));
    }

    #[test]
    fn test_update_policy() {
        let mut engine = PolicyEngine::deny_all();
        let identity = make_identity("svc", None);

        assert!(engine.evaluate("/test", &identity).is_err());

        engine.update_policy(Policy::allow_all());
        assert!(engine.evaluate("/test", &identity).is_ok());
    }

    // Negative tests
    #[test]
    fn test_no_match_error_details() {
        let engine = PolicyEngine::deny_all();
        let identity = make_identity("test-service", Some(("user-123", vec![])));

        let err = engine.evaluate("/some/Method", &identity).unwrap_err();
        match err {
            PolicyError::NoMatch { method, principal } => {
                assert_eq!(method, "/some/Method");
                assert_eq!(principal, "user-123");
            }
            _ => panic!("Expected NoMatch error"),
        }
    }

    #[test]
    fn test_denied_error_details() {
        let policy = Policy {
            default_effect: Effect::Allow,
            rules: vec![Rule {
                name: "explicit-deny".to_string(),
                effect: Effect::Deny,
                principals: Principal {
                    services: Selector::any(),
                    ..Default::default()
                },
                methods: Selector::exact(vec!["/blocked".into()]),
                condition: None,
                priority: 0,
            }],
            ..Default::default()
        };

        let engine = PolicyEngine::new(policy);
        let identity = make_identity("any-service", None);

        let err = engine.evaluate("/blocked", &identity).unwrap_err();
        match err {
            PolicyError::Denied { reason } => {
                assert!(reason.contains("explicit-deny"));
            }
            _ => panic!("Expected Denied error"),
        }
    }
}
