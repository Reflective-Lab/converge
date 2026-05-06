---
source: mixed
---
# ADR-001: Canonical Public Crates

## Status

Accepted

## Context

Converge historically exposed a broad mixed surface through `converge-core` and
`converge-traits`.

That surface mixed together:

- pack authoring
- provider capability routing
- semantic model types
- kernel embedding
- implementation details

This made external consumers too dependent on internal code shape. It also allowed
the wrong abstractions to become public by accident.

We control the known downstreams and do not need to preserve an oversized public
surface for unknown consumers.

## Decision

Converge defines six canonical external contracts:

1. pack authoring
2. provider capability routing
3. semantic model
4. in-process kernel embedding
5. remote Rust client
6. remote wire protocol

These contracts map to the following canonical public crates:

- `converge-pack`
- `converge-provider`
- `converge-model`
- `converge-kernel`
- `converge-protocol`
- `converge-client`

And one canonical wire protocol:

- protobuf package `converge.v1`

`converge-traits` is demoted to a deprecated compatibility facade and is not a
canonical public crate.

`converge-core` is an implementation crate. It may still be published for a
transitional period, but it is not the intended stable external contract.

## Consequences

### Positive

- public responsibilities are explicit
- dependency direction is clear
- downstreams can be migrated to narrow surfaces
- Rust consumers can depend on a real client SDK instead of a CLI crate
- semantic breaks can happen inside the correct public crates instead of through
  accidental re-exports

### Negative

- migration work is required in controlled downstreams
- transitional duplication exists while old facades still compile

### Required follow-up

- no new downstream code may import `converge-traits`
- no new downstream code should import `converge-core` for pack authoring or
  provider routing concerns
- no new downstream Rust code should import `converge-remote` as its API
- `converge-protocol` owns the Rust packaging of `converge.v1`
- `converge-client` owns the idiomatic remote Rust SDK
