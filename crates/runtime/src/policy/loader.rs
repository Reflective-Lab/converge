//! Policy loading from YAML files.

use super::types::Policy;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info};

/// Errors that can occur when loading policies.
#[derive(Debug, Error)]
pub enum PolicyLoaderError {
    /// File not found.
    #[error("policy file not found: {0}")]
    NotFound(String),

    /// I/O error reading file.
    #[error("failed to read policy file: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error.
    #[error("failed to parse policy YAML: {0}")]
    Parse(#[from] serde_yaml::Error),

    /// Policy validation error.
    #[error("policy validation failed: {0}")]
    Validation(String),
}

/// Policy loader for YAML configuration files.
pub struct PolicyLoader;

impl PolicyLoader {
    /// Load a policy from a YAML file.
    pub fn load_file(path: impl AsRef<Path>) -> Result<Policy, PolicyLoaderError> {
        let path = path.as_ref();

        if !path.exists() {
            return Err(PolicyLoaderError::NotFound(path.display().to_string()));
        }

        info!(path = %path.display(), "Loading policy file");

        let content = std::fs::read_to_string(path)?;
        Self::load_yaml(&content)
    }

    /// Load a policy from a YAML string.
    pub fn load_yaml(yaml: &str) -> Result<Policy, PolicyLoaderError> {
        let policy: Policy = serde_yaml::from_str(yaml)?;

        // Validate the policy
        Self::validate(&policy)?;

        debug!(
            rules = policy.rules.len(),
            default_effect = %policy.default_effect,
            "Policy loaded successfully"
        );

        Ok(policy)
    }

    /// Validate a policy for common errors.
    fn validate(policy: &Policy) -> Result<(), PolicyLoaderError> {
        for rule in &policy.rules {
            // Rule must have a name
            if rule.name.is_empty() {
                return Err(PolicyLoaderError::Validation(
                    "rule must have a name".to_string(),
                ));
            }

            // Rule should have at least one method or be explicit about matching all
            // (empty methods means no match, which is probably not intended)
            if rule.methods.is_empty() && !rule.principals.is_empty() {
                return Err(PolicyLoaderError::Validation(format!(
                    "rule '{}' has principals but no methods - use '*' to match all methods",
                    rule.name
                )));
            }
        }

        Ok(())
    }

    /// Load policy from file or return a default policy if file doesn't exist.
    pub fn load_or_default(path: impl AsRef<Path>) -> Policy {
        match Self::load_file(path) {
            Ok(policy) => policy,
            Err(PolicyLoaderError::NotFound(_)) => {
                info!("No policy file found, using default deny-all policy");
                Policy::default()
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to load policy, using default");
                Policy::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_POLICY_YAML: &str = r#"
default_effect: deny
version: "1.0"
description: "Test policy"
rules:
  - name: admin-all-access
    effect: allow
    principals:
      roles:
        - admin
    methods:
      - "*"

  - name: user-read-access
    effect: allow
    principals:
      roles:
        - user
    methods:
      - "/converge.Service/GetJob"
      - "/converge.Service/GetCapabilities"
    priority: 10

  - name: service-to-service
    effect: allow
    principals:
      services:
        - api-gateway
        - scheduler
    methods:
      - "*"
"#;

    #[test]
    fn test_load_valid_yaml() {
        let policy = PolicyLoader::load_yaml(VALID_POLICY_YAML).unwrap();

        assert_eq!(policy.default_effect, super::super::types::Effect::Deny);
        assert_eq!(policy.rules.len(), 3);
        assert_eq!(policy.version, Some("1.0".into()));
    }

    #[test]
    fn test_load_minimal_yaml() {
        let yaml = r#"
default_effect: allow
rules: []
"#;
        let policy = PolicyLoader::load_yaml(yaml).unwrap();
        assert_eq!(policy.default_effect, super::super::types::Effect::Allow);
        assert!(policy.rules.is_empty());
    }

    #[test]
    fn test_load_empty_yaml_uses_defaults() {
        let yaml = "{}";
        let policy = PolicyLoader::load_yaml(yaml).unwrap();
        assert_eq!(policy.default_effect, super::super::types::Effect::Deny);
    }

    #[test]
    fn test_validation_empty_rule_name() {
        let yaml = r#"
rules:
  - name: ""
    effect: allow
    methods:
      - "*"
"#;
        let result = PolicyLoader::load_yaml(yaml);
        assert!(matches!(result, Err(PolicyLoaderError::Validation(_))));
    }

    #[test]
    fn test_validation_principals_without_methods() {
        let yaml = r#"
rules:
  - name: bad-rule
    effect: allow
    principals:
      roles:
        - admin
    methods: []
"#;
        let result = PolicyLoader::load_yaml(yaml);
        assert!(matches!(result, Err(PolicyLoaderError::Validation(_))));
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = PolicyLoader::load_file("/nonexistent/policy.yaml");
        assert!(matches!(result, Err(PolicyLoaderError::NotFound(_))));
    }

    #[test]
    fn test_load_or_default_nonexistent() {
        let policy = PolicyLoader::load_or_default("/nonexistent/policy.yaml");
        assert_eq!(policy.default_effect, super::super::types::Effect::Deny);
    }

    #[test]
    fn test_invalid_yaml() {
        let yaml = "not: valid: yaml: here";
        let result = PolicyLoader::load_yaml(yaml);
        assert!(matches!(result, Err(PolicyLoaderError::Parse(_))));
    }

    #[test]
    fn test_rule_priority_parsing() {
        let yaml = r#"
rules:
  - name: high-priority
    effect: allow
    principals:
      roles: ["*"]
    methods: ["*"]
    priority: 100
  - name: low-priority
    effect: deny
    principals:
      roles: ["*"]
    methods: ["*"]
    priority: 1
"#;
        let policy = PolicyLoader::load_yaml(yaml).unwrap();
        assert_eq!(policy.rules[0].priority, 100);
        assert_eq!(policy.rules[1].priority, 1);
    }

    #[test]
    fn test_condition_parsing() {
        let yaml = r#"
rules:
  - name: conditional
    effect: allow
    principals:
      roles: ["admin"]
    methods: ["*"]
    condition: "request.time > 9 && request.time < 17"
"#;
        let policy = PolicyLoader::load_yaml(yaml).unwrap();
        assert_eq!(
            policy.rules[0].condition,
            Some("request.time > 9 && request.time < 17".to_string())
        );
    }
}
