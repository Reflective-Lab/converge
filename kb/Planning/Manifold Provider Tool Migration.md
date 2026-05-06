---
tags: [planning, providers, tools, manifold, extensions]
source: codex
date: 2026-05-06
---
# Manifold Provider Tool Migration

This is the implementation plan for the remaining provider/tool extraction
after the v3.8.1 naming decision.

## Decision

Converge owns provider and tool **contracts**. Manifold owns generic provider
and tool **adapter implementations**.

The 3.8.x published contract crate is `converge-provider`. That is the real
domain name for the API. Implementations add adapter qualifiers.

`converge-provider-adapters` is therefore not a long-term foundation crate. It
is a temporary non-publishable in-repo adapter staging crate to drain into
Manifold.

## Target Dependency Shape

```text
converge-provider          # provider/tool contracts, selection DTOs
        ^
        |
manifold                   # long-term generic adapter home
        ^
        |
products / runway          # runtime assembly, credentials, environment wiring
```

Converge runtime must not import Manifold. Products or Runway wire adapters
into a runtime.

Interim state: `converge-provider-adapters` holds today's in-repo
OpenAI/Anthropic/search/tool adapters until this plan drains them into
Manifold. It is not part of the target dependency shape, not a default
workspace member, and not a publishable foundation crate.

## Adapter Naming Rule

Implementation names must say what they implement:

| Contract | Manifold implementation name pattern |
|---|---|
| `ChatBackend` | `OpenAiChatAdapter`, `AnthropicChatAdapter`, `GeminiChatAdapter`, `MistralChatAdapter` |
| `WebSearchBackend` | `BraveSearchAdapter`, `TavilySearchAdapter` |
| `WebFetchBackend` | `HttpFetchAdapter` |
| `FeedFetchBackend` | `HttpFeedAdapter` |
| tool conversion / execution | `OpenApiToolAdapter`, `GraphQlToolAdapter` |

Avoid giving implementations the clean domain names. The clean names belong to
contracts and capability vocabulary.

## Converge-Side Coupling

The first blocker was the foundation importing adapter selection:

- CLOSED: `ChatBackendSelectionConfig` moved to `converge-provider`.
- CLOSED: `converge-runtime` now depends on `converge-provider`, not
  `converge-provider-adapters`.
- CLOSED: `ProviderSelectionSuggestor` moved to `converge-kernel`, so the
  grouped formation surface no longer imports adapter code.
- CLOSED: `ChatBackendRegistry`, `RegisteredChatBackend`, and
  `ResolvedChatBackend` live in `converge-provider`. The registry holds
  already-constructed handles supplied by host assembly.

The remaining coupling is inside the staging crate:

- `crates/provider-adapters/src/llm/selection.rs` imports concrete vendor
  backend types such as `AnthropicBackend`, `OpenAiBackend`, `GeminiBackend`,
  and `MistralBackend`.
- The first physical LLM copy is present in
  `/Users/kpernyer/dev/extensions/manifold/crates/manifold/src/llm` and
  compiles behind Manifold's `llm-all` feature against the local Converge
  3.8.1 patch. This is intentional short-term duplication until staging is
  removed or converted to a compatibility facade.

That is now staging-only coupling. Moving vendor backends next should preserve
the same registry contract: Manifold constructs handles, products or Runway
register them, and Converge selects only through `converge-provider`.

## Migration Slices

1. **Contract extraction inside Converge. — CLOSED**
   Move selection input types needed by runtime from
   `converge-provider-adapters` into `converge-provider`:
   `ChatBackendSelectionConfig` and any typed environment or override values
   that are contract-level. `SelectedChatBackend` remains adapter-side until
   the host-supplied registry contract lands.

2. **Registry boundary. — CLOSED**
   Define a provider registry/activation contract in `converge-provider`
   that can hold `Arc<dyn DynChatBackend>` or capability-specific handles
   supplied by the host. Selection returns an already-registered capability
   handle, never instantiates a vendor type.

3. **Runtime decoupling. — CLOSED**
   Change `converge-runtime` to depend on `converge-provider` for LLM
   configuration and selection contracts. Runtime no longer constructs an
   adapter `ProviderRegistry`; products or deployment assembly must supply
   adapters.

4. **Move adapters to Manifold.**
   Move concrete modules out of `crates/provider-adapters` into Manifold:
   LLM adapters first, then search/fetch/feed, then OpenAPI/GraphQL tools.
   Manifold imports `converge-provider` and implements the contracts.
   LLM adapters are copied and verified in Manifold; the next cleanup is
   deleting or facading the in-repo staging definitions.

5. **Downstream proof.**
   Add compile-pass examples/tests showing:
   Manifold implements the provider contracts without importing Converge
   internals, and a product/runtime assembly can register Manifold adapters
   without Converge depending on Manifold.

6. **Retire staging crate.**
   Once all generic adapters are out, either empty/deprecate
   `converge-provider-adapters` for the 3.8 line. Do not leave Converge with a
   mixed contract and implementation crate.

## Explicit Non-Goals

- Do not move source-specific connectors such as LinkedIn into Manifold; those
  belong in `embassy`.
- Do not make `converge-runtime` import Manifold.
- Do not add new provider/tool feature flags in Converge as an extraction
  bridge.
- Do not let a Suggestor import a concrete adapter type. Suggestors ask for
  capabilities; runtime or product assembly resolves adapters.

## Acceptance Criteria

- `converge-runtime` no longer imports `converge-provider-adapters` for
  provider selection. ✓
- `converge-kernel` no longer imports `converge-provider-adapters` for
  `ProviderSelectionSuggestor`. ✓
- `converge-provider-adapters` is non-publishable and excluded from default
  workspace members. ✓
- `converge-provider` exposes a host-supplied chat registry that returns
  already-registered handles. ✓
- Concrete LLM/search/fetch/feed/tool adapter types are no longer defined in
  the Converge foundation.
- Manifold compiles against published Converge contracts.
- Compile-pass tests prove the dependency direction:
  `converge-provider <- manifold <- product/runtime assembly`.
- KB pages describe `converge-provider` as the contract and
  `converge-provider-adapters` as the temporary implementation holder.

See also: [[Architecture/ADRs/ADR-007-provider-tool-contracts]],
[[Architecture/ADRs/ADR-008-extension-crate-boundaries]],
[[Architecture/Extension Topology]], [[Architecture/Plug Boundary]].
