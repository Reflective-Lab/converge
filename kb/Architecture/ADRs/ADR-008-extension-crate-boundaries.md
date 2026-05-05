---
source: codex
---
# ADR-008: Extension Crate Boundaries

**Status**: Accepted
**Date**: 2026-05-05

## Context

The Converge workspace currently contains several crates that are valuable but
not foundational: provider adapters, analytics/ML suggestors, Cedar policy
implementation, knowledge stores, search/fetch/feed integrations, and other
service-facing code.

These crates increase compile time, pull in volatile dependency trees, and
force unrelated release cycles to move with the kernel. They also blur the
conceptual boundary: Converge should define governed execution and stable
contracts, while extension crates implement those contracts.

## Decision

Converge 3.8 treats provider, analytics, policy, and knowledge implementation
crates as extension candidates.

The foundation repository keeps:

- kernel execution, promotion, HITL, invariants, and integrity
- `converge-pack` authoring contracts
- semantic model contracts
- in-process and remote client/protocol contracts
- small provider/tool/policy/knowledge port contracts where downstreams need a
  stable API

Extension crates or external repositories own:

- concrete provider adapters and HTTP clients
- API keys, SDK churn, retries, redirects, and service-specific payloads
- analytics/ML pipelines and heavy data dependencies
- Cedar or other policy engine implementations
- knowledge stores, vector indexes, feed/search/fetch integrations, and
  ingestion adapters

The dependency rule is one-way: extensions depend on Converge contracts;
Converge contracts do not depend on extensions.

## Boundary Guidance

- **Provider:** provider capability contracts stay in the foundation; concrete
  LLM/search/fetch/feed adapters move out.
- **Analytics:** event/observer contracts may stay; analytics suggestors,
  training, storage, and ML dependencies move out.
- **Policy:** policy gate interfaces and authorization vocabulary stay; Cedar
  policy engine wiring moves out.
- **Knowledge:** recall/evidence vocabulary may stay; vector stores, agentic
  memory, ingestion, and retrieval implementations move out.

## Consequences

- The foundation builds faster and has a smaller security review surface.
- Extensions can release independently when provider APIs, model SDKs, policy
  engines, or storage systems change.
- Compile-pass tests become more important: each extension must prove it can
  implement the public contracts without importing Converge internals.
- v3.8 must avoid introducing new APIs that assume these implementations remain
  in the foundation repository.

See also: [[Planning/v3.8 Foundation]], [[Architecture/API Surfaces]],
[[Architecture/ADRs/ADR-007-provider-tool-contracts]].
