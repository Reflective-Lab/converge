---
source: codex
---
# ADR-007: Provider and Tool Contract Names

**Status**: Accepted
**Date**: 2026-05-05

## Context

The current workspace gives the implementation crate the clean name
`converge-provider` while the stable contract is named `converge-provider-api`.
That is backwards for long-lived architecture. The clean name should identify
the domain contract; implementation crates should say what they implement.

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

The current `converge-provider-api` name is transitional. The v3.8 migration
plan should either rename/re-home the contract or document a compatibility
path toward the naming rule.

## Consequences

- Feed, fetch, search, chat, embedding, and tool execution contracts should be
  reviewed as ports before new implementations are added.
- `converge-provider` should not remain both the clean domain name and the
  implementation crate.
- Provider extraction is not only packaging cleanup; it is a security and
  correctness boundary.

See also: [[Architecture/API Surfaces]], [[Planning/v3.8 Foundation]].
