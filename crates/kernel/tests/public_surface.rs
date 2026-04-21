use std::collections::HashMap;

use converge_kernel::{
    BudgetResource, DecisionStep, ExperienceEvent, ExperienceEventEnvelope, ExperienceEventKind,
    StopReason,
};

#[test]
fn kernel_reexports_stop_reason_for_outcome_events() {
    let envelope = ExperienceEventEnvelope::new(
        "evt-outcome",
        ExperienceEvent::OutcomeRecorded {
            chain_id: "chain:test".into(),
            step: DecisionStep::Planning,
            passed: false,
            stop_reason: Some(StopReason::CycleBudgetExhausted {
                cycles_executed: 5,
                limit: 5,
            }),
            latency_ms: Some(12),
            tokens: Some(34),
            cost_microdollars: None,
            backend: Some("kernel-test".into()),
            metadata: HashMap::new(),
        },
    );

    assert_eq!(envelope.event.kind(), ExperienceEventKind::OutcomeRecorded);

    match envelope.event {
        ExperienceEvent::OutcomeRecorded {
            stop_reason:
                Some(StopReason::CycleBudgetExhausted {
                    cycles_executed,
                    limit,
                }),
            ..
        } => {
            assert_eq!(cycles_executed, 5);
            assert_eq!(limit, 5);
        }
        other => panic!("unexpected event shape: {other:?}"),
    }
}

#[test]
fn kernel_reexports_budget_resource_for_budget_events() {
    let envelope = ExperienceEventEnvelope::new(
        "evt-budget",
        ExperienceEvent::BudgetExceeded {
            chain_id: "chain:test".into(),
            resource: BudgetResource::Tokens,
            limit: "100".into(),
            observed: Some("144".into()),
        },
    );

    assert_eq!(envelope.event.kind(), ExperienceEventKind::BudgetExceeded);

    match envelope.event {
        ExperienceEvent::BudgetExceeded {
            resource: BudgetResource::Tokens,
            limit,
            observed,
            ..
        } => {
            assert_eq!(limit, "100");
            assert_eq!(observed.as_deref(), Some("144"));
        }
        other => panic!("unexpected event shape: {other:?}"),
    }
}
