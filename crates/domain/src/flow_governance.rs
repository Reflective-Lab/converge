use std::sync::Arc;

use converge_core::{
    AuthorityLevel, Context, ContextKey, FlowAction, FlowGateAuthorizer, FlowGateContext,
    FlowGateInput, FlowGatePrincipal, FlowGateResource, FlowPhase,
};
use converge_policy::{FLOW_GOVERNANCE_POLICY, PolicyEngine};

pub(crate) fn default_flow_authorizer() -> Arc<dyn FlowGateAuthorizer> {
    Arc::new(
        PolicyEngine::from_policy_str(FLOW_GOVERNANCE_POLICY)
            .expect("built-in flow governance Cedar policy should parse"),
    )
}

pub(crate) fn has_approval(
    ctx: &dyn Context,
    scope: &str,
    target_id: &str,
    required_role: &str,
) -> bool {
    let expected_id = format!("approval:{scope}:{target_id}");
    ctx.get(ContextKey::Proposals).iter().any(|fact| {
        fact.id == expected_id
            || (fact.id.starts_with("approval:")
                && fact.content.contains(target_id)
                && fact.content.contains(required_role))
    })
}

pub(crate) fn flow_input(
    principal_id: &str,
    authority: &str,
    domain: &str,
    resource_id: String,
    kind: &str,
    gates_passed: Vec<String>,
    amount: Option<i64>,
    human_approval_present: bool,
    required_gates_met: bool,
    action: FlowAction,
) -> FlowGateInput {
    FlowGateInput {
        principal: FlowGatePrincipal {
            id: principal_id.into(),
            authority: authority_level(authority),
            domains: vec![domain.into()],
            policy_version: Some("flow_governance_v1".into()),
        },
        resource: FlowGateResource {
            id: resource_id.into(),
            kind: kind.into(),
            phase: FlowPhase::Commitment,
            gates_passed: gates_passed.into_iter().map(Into::into).collect(),
        },
        action,
        context: FlowGateContext {
            commitment_type: Some(kind.into()),
            amount,
            human_approval_present: Some(human_approval_present),
            required_gates_met: Some(required_gates_met),
        },
    }
}

fn authority_level(authority: &str) -> AuthorityLevel {
    match authority {
        "advisory" => AuthorityLevel::Advisory,
        "supervisory" => AuthorityLevel::Supervisory,
        "participatory" => AuthorityLevel::Participatory,
        "sovereign" => AuthorityLevel::Sovereign,
        _ => panic!("unsupported authority: {authority}"),
    }
}

pub(crate) fn json_has_array_items(value: &serde_json::Value, key: &str) -> bool {
    value
        .get(key)
        .and_then(serde_json::Value::as_array)
        .is_some_and(|items| !items.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_has_array_items_returns_true_for_non_empty_array() {
        let val = serde_json::json!({"items": [1, 2, 3]});
        assert!(json_has_array_items(&val, "items"));
    }

    #[test]
    fn json_has_array_items_returns_false_for_empty_array() {
        let val = serde_json::json!({"items": []});
        assert!(!json_has_array_items(&val, "items"));
    }

    #[test]
    fn json_has_array_items_returns_false_for_missing_key() {
        let val = serde_json::json!({"other": 42});
        assert!(!json_has_array_items(&val, "items"));
    }

    #[test]
    fn json_has_array_items_returns_false_for_non_array() {
        let val = serde_json::json!({"items": "not an array"});
        assert!(!json_has_array_items(&val, "items"));
    }

    #[test]
    fn flow_input_constructs_correct_struct() {
        let input = flow_input(
            "agent:fin",
            "supervisory",
            "finance",
            "expense:001".into(),
            "expense",
            vec!["receipt".into()],
            Some(5000),
            true,
            true,
            FlowAction::Commit,
        );
        assert_eq!(input.principal.id, "agent:fin");
        assert_eq!(input.principal.authority, AuthorityLevel::Supervisory);
        assert_eq!(
            input
                .principal
                .domains
                .iter()
                .map(|d| d.as_str())
                .collect::<Vec<_>>(),
            vec!["finance"]
        );
        assert_eq!(
            input.principal.policy_version.as_deref(),
            Some("flow_governance_v1")
        );
        assert_eq!(input.resource.id, "expense:001");
        assert_eq!(input.resource.kind, "expense");
        assert_eq!(input.resource.phase, FlowPhase::Commitment);
        assert_eq!(
            input
                .resource
                .gates_passed
                .iter()
                .map(|g| g.as_str())
                .collect::<Vec<_>>(),
            vec!["receipt"]
        );
        assert_eq!(input.context.amount, Some(5000));
        assert_eq!(input.context.human_approval_present, Some(true));
        assert_eq!(input.context.required_gates_met, Some(true));
    }
}
