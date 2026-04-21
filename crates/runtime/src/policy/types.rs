//! Policy types and data structures.

use crate::semantic::{GrpcMethod, RoleId, Selector, ServiceId, UserId};
use converge_core::types::PolicyVersionId;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Policy evaluation errors.
#[derive(Debug, Error)]
pub enum PolicyError {
    /// No matching rule found.
    #[error("no matching rule for method {method} and principal {principal}")]
    NoMatch {
        method: GrpcMethod,
        principal: String,
    },

    /// Access denied by policy.
    #[error("access denied: {reason}")]
    Denied { reason: String },

    /// Invalid policy configuration.
    #[error("invalid policy: {0}")]
    Invalid(String),
}

/// Effect of a policy rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    /// Allow the action.
    Allow,
    /// Deny the action.
    #[default]
    Deny,
}

impl std::fmt::Display for Effect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Effect::Allow => write!(f, "allow"),
            Effect::Deny => write!(f, "deny"),
        }
    }
}

/// Principal specification for matching.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Principal {
    /// Roles that match this rule.
    #[serde(default)]
    pub roles: Selector<RoleId>,

    /// Services that match this rule.
    #[serde(default)]
    pub services: Selector<ServiceId>,

    /// Specific user IDs that match.
    #[serde(default)]
    pub users: Selector<UserId>,
}

impl Principal {
    /// Check if this principal matches any criteria.
    pub fn is_empty(&self) -> bool {
        self.roles.is_empty() && self.services.is_empty() && self.users.is_empty()
    }

    /// Check if a user with given roles/service matches this principal.
    pub fn matches(
        &self,
        user_id: Option<&UserId>,
        roles: &[RoleId],
        service_id: &ServiceId,
    ) -> bool {
        if self.is_empty() {
            return false;
        }

        if user_id.is_some_and(|uid| self.users.matches(uid)) {
            return true;
        }

        if self.roles.matches_any(roles.iter()) {
            return true;
        }

        self.services.matches(service_id)
    }
}

/// A policy rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Rule name for logging/debugging.
    pub name: String,

    /// Effect when this rule matches.
    pub effect: Effect,

    /// Principal specification.
    #[serde(default)]
    pub principals: Principal,

    /// Methods this rule applies to.
    #[serde(default)]
    pub methods: Selector<GrpcMethod>,

    /// Optional condition expression (future use).
    #[serde(default)]
    pub condition: Option<String>,

    /// Rule priority (higher = evaluated first).
    #[serde(default)]
    pub priority: i32,
}

impl Rule {
    /// Check if this rule matches the given request.
    pub fn matches(
        &self,
        method: &GrpcMethod,
        user_id: Option<&UserId>,
        roles: &[RoleId],
        service_id: &ServiceId,
    ) -> bool {
        let method_matches = self.methods.is_empty() || self.methods.matches(method);

        if !method_matches {
            return false;
        }

        self.principals.matches(user_id, roles, service_id)
    }
}

/// Complete policy configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    /// Default effect when no rules match.
    #[serde(default)]
    pub default_effect: Effect,

    /// Policy rules (evaluated in priority order).
    #[serde(default)]
    pub rules: Vec<Rule>,

    /// Policy version for tracking changes.
    #[serde(default)]
    pub version: Option<PolicyVersionId>,

    /// Policy description.
    #[serde(default)]
    pub description: Option<String>,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            default_effect: Effect::Deny,
            rules: Vec::new(),
            version: None,
            description: None,
        }
    }
}

impl Policy {
    /// Create an allow-all policy (for development).
    pub fn allow_all() -> Self {
        Self {
            default_effect: Effect::Allow,
            rules: Vec::new(),
            version: Some("dev".into()),
            description: Some("Development allow-all policy".to_string()),
        }
    }

    /// Create a deny-all policy.
    pub fn deny_all() -> Self {
        Self::default()
    }

    /// Get rules sorted by priority (highest first).
    pub fn sorted_rules(&self) -> Vec<&Rule> {
        let mut rules: Vec<&Rule> = self.rules.iter().collect();
        rules.sort_by(|a, b| b.priority.cmp(&a.priority));
        rules
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_display() {
        assert_eq!(Effect::Allow.to_string(), "allow");
        assert_eq!(Effect::Deny.to_string(), "deny");
    }

    #[test]
    fn test_effect_default() {
        assert_eq!(Effect::default(), Effect::Deny);
    }

    #[test]
    fn test_principal_empty() {
        let p = Principal::default();
        assert!(p.is_empty());
    }

    #[test]
    fn test_principal_matches_role() {
        let p = Principal {
            roles: Selector::exact(vec!["admin".into()]),
            ..Default::default()
        };
        assert!(p.matches(None, &["admin".into()], &ServiceId::new("some-service")));
        assert!(!p.matches(None, &["user".into()], &ServiceId::new("some-service")));
    }

    #[test]
    fn test_principal_matches_service() {
        let p = Principal {
            services: Selector::exact(vec!["api-gateway".into()]),
            ..Default::default()
        };
        assert!(p.matches(None, &[], &ServiceId::new("api-gateway")));
        assert!(!p.matches(None, &[], &ServiceId::new("other-service")));
    }

    #[test]
    fn test_principal_matches_user() {
        let p = Principal {
            users: Selector::exact(vec!["user-123".into()]),
            ..Default::default()
        };
        assert!(p.matches(
            Some(&UserId::new("user-123")),
            &[],
            &ServiceId::new("some-service")
        ));
        assert!(!p.matches(
            Some(&UserId::new("user-456")),
            &[],
            &ServiceId::new("some-service")
        ));
        assert!(!p.matches(None, &[], &ServiceId::new("some-service")));
    }

    #[test]
    fn test_principal_matches_wildcard() {
        let p = Principal {
            roles: Selector::any(),
            ..Default::default()
        };
        assert!(p.matches(None, &["anything".into()], &ServiceId::new("any-service")));
    }

    #[test]
    fn test_rule_matches_method() {
        let rule = Rule {
            name: "test".to_string(),
            effect: Effect::Allow,
            principals: Principal {
                roles: Selector::exact(vec!["admin".into()]),
                ..Default::default()
            },
            methods: Selector::exact(vec!["/test/Method".into()]),
            condition: None,
            priority: 0,
        };

        assert!(rule.matches(
            &GrpcMethod::new("/test/Method"),
            None,
            &["admin".into()],
            &ServiceId::new("svc")
        ));
        assert!(!rule.matches(
            &GrpcMethod::new("/test/Other"),
            None,
            &["admin".into()],
            &ServiceId::new("svc")
        ));
    }

    #[test]
    fn test_sorted_rules_descending_priority() {
        let policy = Policy {
            rules: vec![
                Rule {
                    name: "low".to_string(),
                    effect: Effect::Allow,
                    principals: Principal {
                        roles: Selector::any(),
                        ..Default::default()
                    },
                    methods: Selector::any(),
                    condition: None,
                    priority: 1,
                },
                Rule {
                    name: "high".to_string(),
                    effect: Effect::Deny,
                    principals: Principal {
                        roles: Selector::any(),
                        ..Default::default()
                    },
                    methods: Selector::any(),
                    condition: None,
                    priority: 10,
                },
            ],
            ..Default::default()
        };

        let sorted = policy.sorted_rules();
        assert_eq!(sorted[0].name, "high");
        assert_eq!(sorted[1].name, "low");
    }
}
