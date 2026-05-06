---
source: codex
---
# ADR-007: Provider and Tool Contract Names

**Status**: Accepted
**Date**: 2026-05-05

## Context

The previous workspace shape gave the implementation crate the clean name
`converge-provider` while the stable contract was named
`converge-provider-api`. That was backwards for long-lived architecture. The
clean name should identify the domain contract; implementation crates should
say what they implement.

Provider and tool integrations also carry volatile concerns: HTTP clients,
redirect behavior, SSRF policy, API keys, SDK churn, retries, rate limits, and
service-specific payloads. Those concerns should not sit next to the pure
Converge kernel longer than necessary.

## Decision

Contracts get the real domain names. Implementations add adapter qualifiers.

For provider and tool boundaries:

- the contract crate owns traits, request/response DTOs, capability vocabulary,
  and typed boundary values
- adapter crates own concrete HTTP clients, provider SDKs, credentials,
  service-specific parsing, and operational retry behavior
- the contract must not depend on `converge-pack` merely to reuse unrelated
  authoring value types
- external provider/tool implementations should move to adapter crates or a
  separate repository as part of the v3.8 foundation work

The contract crate must own `converge-provider`. Any in-repo implementation
holder must use an adapter-qualified name.

## v3.8.1 Resolution

The naming rule is settled in code.

The 3.8.x line treats `converge-provider` as the published provider contract.
It is the stable Rust package that downstreams import for `ChatBackend`,
`Backend`, `Capability`, `BackendRequirements`, `BackendSelector`, and
provider selection DTOs.

The clean long-lived domain name belongs to the contract, not to adapter
implementations. Therefore the temporary in-repo implementation staging crate
is named `converge-provider-adapters`. It is non-publishable, excluded from
default workspace members, and will be emptied or retired as generic adapters
move to `manifold`.

Target end state:

- `converge-provider` is the clean provider/capability contract name.
- `converge-provider-adapters` remains only a temporary in-repo implementation
  holder until generic adapters move to Manifold. Foundation crates do not
  depend on it.
- Generic adapter implementations live in `manifold` with implementation
  qualifiers such as `OpenAiChatAdapter`, `AnthropicChatAdapter`,
  `BraveSearchAdapter`, `HttpFetchAdapter`, `HttpFeedAdapter`,
  `OpenApiToolAdapter`, and `GraphQlToolAdapter`.
- Source-specific connector ports live in `embassy`, not `manifold`, when the
  external party is part of the semantic contract.

The implementation move is tracked as the manifold provider/tool migration.

## Consequences

- Feed, fetch, search, chat, embedding, and tool execution contracts should be
  reviewed as ports before new implementations are added.
- `converge-provider` must remain contract-only. Adapter code belongs in
  `converge-provider-adapters` temporarily, then Manifold.
- Provider extraction is not only packaging cleanup; it is a security and
  correctness boundary.

See also: [[Architecture/API Surfaces]], [[Planning/v3.8 Foundation]].
