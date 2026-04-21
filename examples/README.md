# Converge Examples

Each subdirectory under `examples/` is a standalone crate that compiles as part
of the workspace.

## Quick Start

```bash
just examples
just example hello-convergence
just example formation-mixed
```

Most example package names follow the directory directly:

- `just example hello-convergence`
- `cargo run -p example-hello-convergence`

## Current Example Set

| Example | What it shows | Run |
|---|---|---|
| [hello-convergence](hello-convergence/) | Minimal convergence loop and fixed-point execution | `just example hello-convergence` |
| [custom-agent](custom-agent/) | A custom `Suggestor` implementation | `just example custom-agent` |
| [custom-provider](custom-provider/) | A provider adapter implementing the provider API | `just example custom-provider` |
| [meeting-scheduler](meeting-scheduler/) | Multi-step convergence over scheduling constraints | `just example meeting-scheduler` |
| [expense-approval](expense-approval/) | Governed approval flow with policy decisions in the loop | `just example expense-approval` |
| [vendor-selection](vendor-selection/) | Multi-criteria selection with default Cedar-backed flow gating | `just example vendor-selection` |
| [loan-application](loan-application/) | Domain suggestors, policy checks, and HITL escalation | `just example loan-application` |
| [formation-mixed](formation-mixed/) | One loop mixing optimization, policy, and LLM-style reasoning | `just example formation-mixed` |

## Contract Notes

- All examples use the current `Context` / `ContextState` naming. There is no `ContextView`.
- Examples seed runs with `ProposedFact` or `ContextState::add_input(...)`. They do not construct authoritative `Fact` values.
- Heterogeneous behavior still enters through one trait: `Suggestor`.

## For Consumers

Start with `hello-convergence`, then read `formation-mixed`.

That pairing shows the stable contract clearly:

- `Engine`
- `Suggestor`
- `AgentEffect`
- `ProposedFact`
- `Context` / `ContextState`

Everything else is composition on top.

## Three Things To Know Before You Build

**Converge reasons; something else acts.**
The convergence loop produces a `ConvergeResult` — a stable context of promoted facts. Executing side effects (sending emails, calling APIs, provisioning infrastructure) happens *after* the run, driven by the result. Suggestors are pure reasoners. Do not call external systems inside `execute()`.

**`ContextKey` is a semantic protocol, not your domain model.**
The nine keys (`Seeds`, `Signals`, `Evaluations`, `Strategies`, `Constraints`, `Hypotheses`, `Proposals`, `Competitors`, `Diagnostic`) are semantic roles in a convergence run — like HTTP methods. Your domain concepts map *to* these roles via content and id conventions, not by adding new keys. A supply chain suggestor that evaluates suppliers puts its output in `Evaluations`; a clinical trial suggestor that scores protocols does the same. They compose because they share the same protocol.

**Use structured ids for sub-key namespacing.**
Multiple suggestors can write to the same key in one run. Distinguish their outputs by id convention: `"price:vendor-a"`, `"risk:vendor-a"`, `"compliance:vendor-a"`. The examples show this pattern throughout. The compiler won't enforce the convention, but consistent ids make reads predictable and auditable.
