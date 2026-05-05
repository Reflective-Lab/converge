---
source: codex
---
# ADR-006: Promotion Authority Boundary

**Status**: Accepted; implemented in the v3.8 authority slice
**Date**: 2026-05-05

## Context

Converge's truth pipeline depends on a hard distinction:

- suggestors emit `ProposedFact`
- the engine validates and promotes
- only promoted values become authoritative `Fact`

The implementation hid authority constructors behind the `kernel-authority`
Cargo feature in `converge-pack`. That is not a real security or architecture
boundary. Cargo features are additive and unified across the dependency graph.
If one crate enables a feature, downstream crates in the same graph can observe
the expanded API.

## Decision

Promotion authority must not be exposed through a public Cargo feature in a
publishable contract crate.

`converge-pack` remains the authoring contract for suggestors, proposals,
context reads, and read-only fact projections. Authoritative promoted state
belongs inside `converge-core` behind a private promotion module.

The v3.8 mechanism is:

- remove `kernel-authority` from the public `converge-pack` contract
- keep proposal construction in `converge-pack`
- move promoted fact construction into `converge-core` as `pub(crate)` engine
  machinery
- expose admission through `converge-kernel`, with typed request and receipt
  values re-exported through `converge-model`
- expose read-only context facts as projections, not as values downstream can
  forge and feed back into the engine

The admission path stages `ProposedFact` values. It does not construct or
return authoritative facts. The engine promotion gate remains the only path from
proposal to promoted truth.

The desired end state is:

- downstream pack authors can construct `ProposedFact`
- downstream pack authors can read context fact projections
- downstream pack authors cannot construct authoritative promoted state
- the engine can construct promoted state only through the promotion path
- compile-fail tests prove misuse with both `converge-pack` and the embedding
  surface in the dependency graph

## Rejected Alternatives

### Non-publishable `converge-engine-authority` Crate

A private authority crate would make local dependency graphs look clean, but it
does not work for the published crate line. If `converge-core` depends on an
unpublished path crate, `converge-core` cannot be published normally. If the
authority crate is published, downstream crates can depend on it too. That only
moves the leak.

### Sealed Witness Across Crates

A witness such as `PromotionAuthority` cannot safely bridge the boundary between
`converge-pack` and `converge-core`. If `converge-pack` keeps the witness
private enough that downstream code cannot construct it, `converge-core` cannot
construct it either. If it is public enough for `converge-core`, it is public
enough for downstream code once Cargo resolves the same package.

### Public Feature Gate

The current `kernel-authority` feature is the drift being removed. Cargo feature
unification is not an authority boundary.

## Consequences

- The public `kernel-authority` feature is removed from `converge-pack`.
- Downstream reads promoted state through `ContextFact`, a read-only projection.
- Durable storage rehydrates through `ContextSnapshot`, not fact constructors.
- This is a breaking crate-boundary change. That is acceptable for v3.8 because
  the release is explicitly a foundation release.
- Documentation and release notes must avoid saying feature gating is sufficient
  authority isolation.

See also: [[Architecture/Known Drift]], [[Concepts/Proposals and Promotion]].
