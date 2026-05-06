---
source: mixed
---
# ADR-003: Pack Authoring Contract

## Status

Accepted

## Context

The current authoring contract is centered on `Suggestor`, `Context`, `Fact`, and
`AgentEffect`.

That contract is too permissive:

- pack code can emit authoritative facts
- authoring concerns are mixed with kernel semantics
- the trait shape does not reflect the intended role split between suggestion,
  validation, and acceptance

We want pack code to be pure, deterministic, hexagonal, and hard to misuse.

## Decision

`converge-pack` is the canonical Rust authoring surface, but its long-term shape is
role-based rather than legacy-agent-based.

The intended authoring roles are:

- `Suggestor`: reads context and returns draft proposals only
- `Validator`: evaluates draft proposals and returns validation outcomes
- `Invariant`: checks structural, semantic, or acceptance guarantees
- `CriterionEvaluator`: application/kernel boundary for evaluating success criteria
- `Pack`: a registration boundary bundling suggestors, validators, and invariants

### Pack authoring rules

1. pack code is pure and deterministic
2. pack code receives read-only context views only
3. pack code returns values; it does not mutate context
4. pack code must not construct authoritative facts
5. pack code must not depend on runtime, storage, network, or application crates
6. pack code may depend on `converge-model` for semantic types and
   `converge-provider` only when provider routing requirements are part of the
   pack's boundary

### Transitional note

The current `Suggestor` trait in `converge-pack` is a transitional compatibility shape.
It will be replaced or narrowed so that suggestion code cannot emit authoritative
facts.

## Consequences

### Positive

- the authoring contract matches the axioms
- pack code becomes easier to reason about and test
- the compiler can enforce more of the governance model

### Negative

- existing packs and downstream modules will need migration from `Suggestor`
  to the role-based contract

### Required follow-up

- introduce the new role traits in `converge-pack`
- migrate built-in packs to the new contract
- remove legacy direct-fact authoring once migration is complete
