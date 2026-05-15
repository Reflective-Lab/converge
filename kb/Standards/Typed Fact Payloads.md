---
tags: [standards, facts, wire-contract]
source: codex
date: 2026-05-15
---
# Typed Fact Payloads

Converge facts and proposals are typed in process. JSON, protobuf, text, and
other serialized forms appear only at borders.

This is a hard contract. Do not pass a `String` that "probably contains JSON"
between suggestors.

## Rule

In-process fact flow uses `FactPayload`.

```rust
pub trait FactPayload {
    const FAMILY: &'static str;
    const VERSION: u16;
}
```

Each semantic fact family owns one Rust payload type per frozen schema version.
`(family, version)` maps to exactly one Rust type. If the shape changes, create
`V2` with `VERSION = 2`; do not silently upgrade V1 in the registry.

`ContextKey` is the destination namespace for a fact instance. `FactPayload::FAMILY`
and `FactPayload::VERSION` identify the payload schema. These are different
axes and must not be collapsed.

## Public Wire Contract

`WireProposedFact` and `WireContextFact` are the sanctioned materialization path
for borders:

- HTTP and gRPC
- CLI fixtures
- storage and replay
- audit export
- non-Rust clients
- Lattice or other node-to-node NATS traffic

The stable wire shape is:

```json
{
  "key": "Seeds",
  "id": "claim-1",
  "payload": {
    "family": "example.claim",
    "version": 1,
    "payload": {}
  },
  "confidence": 1.0,
  "provenance": "agent-or-border"
}
```

The `PayloadRegistry` decodes wire payloads at the border. Unknown
`(family, version)` fails closed.

## Replay Policy

Replay depends on stable history. The registry must not rewrite old payloads
into newer types implicitly.

Allowed:

- register `ClaimV1` and `ClaimV2` separately
- write an explicit suggestor that reads `ClaimV1` and emits `ClaimV2`
- run a deliberate one-shot archive rewrite when old string snapshots are known
  to be active and must be carried forward

Forbidden:

- deserialize V1 and return V2 from the registry
- accept unknown families as "best effort"
- use `serde_json::Value` as a normal in-process payload for a known family

Old string-content snapshots are not a continuing internal format. Active
archives must be rewritten through the wire registry path or handled by a
storage-border decoder that emits a named payload family.

## Provenance

Provenance stays uniform. Payload typing does not change provenance semantics.

Provenance answers "where did this come from?" Payload typing answers "what
schema is this value?" These are independent.

## Execution Identity

When a fact is produced by a solver, policy analyzer, native backend, model
runtime, or other evidence-producing engine, use `ExecutionIdentity`.

Use the embedded `ExecutionIdentity` field when the payload is itself an
execution report, such as an SMT report. Use `ExecutionIdentityEvidence` when
the produced payload must remain domain-generic, such as a `FormationPlan`.

`ExecutionIdentityEvidence` links to the subject by:

- `subject_key`
- `subject_id`
- `subject_family`
- `subject_version`

This keeps backend/build/runtime audit metadata out of domain contracts while
still making replay and evidence review uniform across extensions.

## Migration Bar

For each fact family:

1. Define a named payload type.
2. Implement `FactPayload` with a stable family and version.
3. Validate closed domains in the type or in `validate`.
4. Read with `ctx.get(...).payload::<T>()` or `require_payload::<T>()`.
5. Emit with `ProposedFact::new(key, id, typed_payload, provenance)`.
6. Serialize only through `WireProposedFact` or `WireContextFact` at borders.

The current `converge-pack::ProposedFact` constructor enforces this immediately:
raw strings no longer satisfy the payload contract.

Generic `PackSuggestor` is not an exception. It accepts `PackInputPayload` and
emits `PackPlanPayload`; domain-specific suggestors should still define
domain-specific payloads when the schema is known outside the pack.

See also: [[Architecture/Type Protocol]], [[Architecture/API Surfaces]],
[[Concepts/Context and Facts]], [[Concepts/Proposals and Promotion]]
