---
source: mixed
---
# ADR-005: Type Ownership Boundaries

**Status**: Accepted
**Date**: 2026-04-12

## Context

The workspace accumulated type duplication from an incomplete v0.2 migration — duplicate prompt-only provider traits, duplicate `Invariant` traits, four `FinishReason` enums, two `TraceLink` enums, and copy-pasted prompt types. v3.2 removed most duplication; v3.3 finished the move to canonical `ChatBackend` / `EmbedBackend` request types.

This ADR prevents recurrence by declaring which crate owns which types.

## Decision

### Rule 1: Every public type has exactly one canonical definition

If a type appears in two crates, one must re-export from the other. No copy-paste, no "local copies for purity." If a crate needs a type it doesn't own, it depends on and re-exports the owner.

### Rule 2: Ownership follows the dependency graph (leaves own, roots consume)

| Crate | Owns | Re-exports from |
|---|---|---|
| `converge-pack` | `Suggestor`, `Context`, `ContextKey`, `AgentEffect`, `Fact`, `ProposedFact`, `ValidationError` | — |
| `converge-provider` | `Backend`, `BackendKind`, `Capability`, `BackendError`, `BackendSelector`, `BackendRequirements` | — |
| `converge-core` | `Engine`, `ConvergeResult`, `Budget`, `ExperienceStore`, `ExperienceEvent`, `CriterionEvaluator`, `CriterionResult`, `Invariant`, `InvariantClass`, `InvariantResult`, `ChatBackend`, `DynChatBackend`, `EmbedBackend`, `DynEmbedBackend`, `ChatRequest`, `ChatResponse`, `ChatMessage`, `ChatRole`, `ToolDefinition`, `ToolCall`, `ResponseFormat`, `LlmError`, `TokenUsage`, `AgentRequirements`, `ReplayTrace`, `BackendRequest`, `BackendResponse`, `IntentId`, `RootIntent`, `AgentPrompt`, `PromptFormat` | `converge-pack::*` |
| `converge-model` | — (curated re-exports only) | `converge-core`, `converge-pack` |
| `converge-kernel` | — (curated re-exports only) | `converge-core`, `converge-pack` |
| `converge-protocol` | Generated `converge.v1` types | — |
| `converge-client` | `ConvergeClient`, `ClientError`, `StoreObserver` | `converge-protocol` |
| `converge-provider` | Provider contracts, capability routing, chat registry, selection DTOs | none |
| `manifold` | Generic provider impls (LLM chat, storage, search/fetch/feed/tools as migrated), model catalog | `converge-provider`, other Converge contracts |
| `converge-experience` | `InMemoryExperienceStore`, `StoreObserver`, store impls | `converge-core::ExperienceStore` |
| `converge-llm` | `LlmBackend` (local inference), `LlmRouter`, `ProviderAgent`, `LlmAgentConfig`, `LlmError` (kernel), `FinishReason` (inference), `KernelProposal` | `converge-core::kernel_boundary::*`, `converge-core::prompt::*` |

### Rule 3: Name collisions require disambiguation

When two types serve different purposes but would collide on name:
- The more general type keeps the short name
- The specialized type gets a prefix

Examples already applied in v3.2:
- `TraceLink` (OTel operational) vs `ReplayTrace` (kernel audit fingerprint)
- `IntentId` (canonical in `types::frame`) vs deleted duplicate in `root_intent`
- `FinishReason` in `inference` (local engine stops) vs `FinishReason` in `core::llm` (remote provider stops) — different enums, different variants, correctly separate

### Rule 4: Re-export modules preserve the owner's API

When crate B re-exports from crate A:
- B may alias (`pub use A::Foo as Bar`) but must not change the type
- B must not add methods to A's types (use extension traits if needed)
- B's doc comments should reference A as the canonical home

### Rule 5: New types require ownership declaration

Any PR adding a `pub struct`, `pub enum`, or `pub trait` to a publishable crate must state which crate owns it. If the type already exists elsewhere, the PR must re-export instead of redefining.

## Enforcement

1. **Code review**: PRs adding `pub trait` or `pub struct` to publishable crates must declare ownership
2. **Compile/API tests**: public crates use the canonical owner for each surface (`converge-provider` for chat contracts, `converge-pack` for authoring contracts); no local copies of request/response types
3. **`just lint` must pass**: no `#[allow(deprecated)]` without a tracking comment

## Consequences

- No more "local copies for purity" — if you need a type, depend on it
- Slightly more explicit dependency edges, but the dep graph is already well-structured
- New contributors have a clear reference for where types live
- Future transport adapters must follow the same ownership rules
