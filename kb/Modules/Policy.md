---
tags: [modules, policy]
source: mixed
---
# Policy Module

> **Extracted to extension on 2026-05-05.** The Cedar policy engine, policy
> suggestors, and ed25519 delegation tokens now live in the **arbiter**
> extension (`~/dev/reflective/mosaic-extensions/arbiter`, formerly `converge-policy`).
> Foundation `converge-pack` keeps the gate trait and authorization
> vocabulary; concrete Cedar wiring lives in arbiter. See
> [[Architecture/Extension Topology]].

The arbiter extension provides Cedar-based authorization as Suggestors.
Policy enforcement happens INSIDE the convergence loop, not as an external
gate.

## Available Suggestors (3)

| Agent | Purpose | Reads | Writes |
|-------|---------|-------|--------|
| PolicyGateSuggestor | Evaluates Cedar policies against proposals | Strategies | Constraints (if denied) |
| DelegationVerifySuggestor | Validates delegation chains | Strategies | Constraints (if invalid) |
| FlowGateSuggestor | Enforces flow-level authorization | Strategies | Constraints (if blocked) |

## Usage in a Formation

```rust
use arbiter::PolicyGateSuggestor;

let policy = PolicyGateSuggestor::new(cedar_policy_set);
engine.register_suggestor(policy);
```

## How Policy Works in Convergence

1. Planning agent proposes a strategy
2. PolicyGateSuggestor's `accepts()` triggers (Strategies key modified)
3. `execute()` evaluates the strategy against Cedar policies
4. If denied → proposes a Constraint blocking the strategy
5. Planning agent sees the constraint, revises its proposal
6. Loop continues until a compliant strategy converges

This is fundamentally different from a pre-flight policy check:
- Policies see the full context (other facts, constraints)
- Policies can interact with other agents' outputs
- Policy violations feed back into planning (not just reject)
- Everything is recorded in the ExperienceStore

## Cedar Policy Language

Policies are written in Cedar:
```cedar
permit(
    principal in Role::"finance-approver",
    action == Action::"commit-expense",
    resource
) when {
    resource.amount <= 5000
};
```

## FlowGate Pattern

FlowGateSuggestor is the default for consequential transitions:
- Maps to `FlowGateAuthorizer` in converge-core
- Principal + Action + Resource model
- Deny-by-default
- All decisions auditable

See also: [[Architecture/Suggestor Contract]], [[Architecture/Formation Pattern]]
