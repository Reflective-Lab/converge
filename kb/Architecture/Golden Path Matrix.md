---
tags: [architecture]
source: mixed
---
# Golden Path Matrix

This page is the stack-wide source of truth for the **first dependency you should
reach for**, the **next layer it is meant for**, and the **internal crates you
should avoid** unless you are deliberately working on internals.

The goal is simple: **make the right thing easy to use**.

## Stack Model

| Layer | Purpose | Public surface | Typical consumers |
|---|---|---|---|
| Providers | External capabilities | provider SDKs behind adapter traits | adapter maintainers, controlled product integrations |
| Converge | Governance, authority, convergence, promotion | `converge-pack`, `converge-provider`, `converge-model`, `converge-kernel`, `converge-client`, `converge-protocol` | Organism, Axiom, Helm, Wolfgang, hackathon, other apps |
| Organism | Intent interpretation, planning, debate, simulation, reusable organizational workflows | `organism-pack`, `organism-runtime`, `organism-intelligence`, `organism-notes`, `organism-domain` | Helm and other apps that need reusable reasoning above Converge |
| Axiom | Truth authoring, validation, simulation, compilation | Axiom CLI/library + produced WASM/manifests | Helm, truth authors, CI pipelines |
| Helm | Operator-facing control surface | product UI and app APIs | humans |

## Converge Layer

| Need | Start here | Add when needed | Avoid by default |
|---|---|---|---|
| Author packs and invariants | `converge-pack` | `converge-model` for governed result types | `converge-core` |
| Consume provider capability contracts | `converge-provider` | Manifold for ready-made generic adapters | `converge-core` |
| Embed governed execution in-process | `converge-kernel` | `converge-model`, `converge-pack` | `converge-core`, `converge-runtime` |
| Read governed semantic outputs | `converge-model` | `converge-kernel` if you also execute | `converge-core` |
| Call a deployed runtime | `converge-client` | `converge-protocol` for typed wire access | runtime internals |
| Implement provider adapters | `converge-provider` | `converge-provider` if contributing inside Converge | app-local vendor SDK code spread across repos |

Rules:

- `converge-core` is the constitutional engine crate. It is not the default next-layer import.
- `converge-provider` is the canonical capability contract for chat and routing vocabulary.
- Manifold is the implementation layer for ready-made generic adapters, env helpers, and provider registry behavior.

## Organism Layer

| Need | Start here | Add when needed | Avoid by default |
|---|---|---|---|
| Use Organism planning and intent semantics | `organism-pack` | `organism-runtime` | phase crates such as `organism-planning` |
| Embed Organism with Converge | `organism-runtime` | `organism-domain` for built-in packs | direct phase-crate assembly |
| Use reusable world-facing capabilities | `organism-intelligence` | `organism-notes` for vault flows | rebuilding OCR/web/social locally |
| Use reusable organizational workflows | `organism-domain` | `organism-pack`, `organism-runtime` | app-local copies of the same pack logic |

Rules:

- Apps above Organism should start with `organism-pack` + `organism-runtime`.
- Reach for `organism-intelligence`, `organism-notes`, and `organism-domain` only when the app really needs those reusable capabilities.
- Do not treat `organism-intent`, `organism-planning`, `organism-adversarial`, `organism-simulation`, or `organism-learning` as the default app-facing surface.

## Axiom Layer

| Need | Start here | Add when needed | Avoid by default |
|---|---|---|---|
| Validate and compile truths | Axiom CLI/library | `converge-provider` + `converge-provider` for live LLM-backed validation | `converge-core` |
| Consume Axiom output in Converge | produced WASM + manifest ABI | `converge-model` for truth metadata if needed | direct dependence on Axiom internals from runtime code |

Rules:

- Axiom is the truth-definition and validation layer.
- Axiom consumes provider capability contracts; it does not need the Converge engine crate for chat vocabulary.
- Converge consumes Axiom artifacts, not Axiom authority.

## Helm And Product Layers

| Need | Start here | Add when needed | Avoid by default |
|---|---|---|---|
| Operator-facing truth authoring and validation | Axiom | Organism for reusable reasoning, Converge for execution | direct lower-layer internals |
| Governed execution in a product | `converge-kernel` or `converge-client` | `converge-model`, `converge-pack`, `converge-provider`, `converge-provider` | `converge-core` |
| Reusable reasoning above execution | `organism-pack` + `organism-runtime` | `organism-domain`, `organism-intelligence`, `organism-notes` | Organism phase crates |

Rules:

- Helm is the control surface, not the semantic foundation.
- Product repos compose Converge, Organism, and Axiom. They do not redefine those layers.
- If a capability is generic and missing, add it to the correct lower layer rather than rebuilding it in the app.

## Named Consumers

| Consumer | Golden path |
|---|---|
| Organism | `converge-pack` + `converge-kernel` + `converge-model` + `converge-client` |
| Axiom | `converge-provider` + `converge-provider` |
| Helms | Axiom + `organism-pack`/`organism-runtime` + `converge-kernel`/`converge-model`/`converge-pack` |
| Wolfgang | `converge-kernel` + `converge-model` + `converge-provider` + `converge-provider`; Organism only when a reusable capability clearly beats product-local code |
| Hackathon participant-facing code | `converge-pack` + `converge-kernel` + `converge-provider` + `converge-provider` + `organism-pack`/`organism-runtime` |

## Hard Rule

Each layer should expose the **smallest stable surface** that the next layer
needs.

- Do not make apps learn `converge-core` when `converge-kernel` or `converge-provider` is enough.
- Do not make Organism consumers learn phase crates when `organism-pack` or `organism-runtime` is enough.
- Do not make Helm consumers learn architecture internals when Axiom or product APIs are enough.

## References

- [[Architecture/API Surfaces]]
- [[Architecture/Dependency Rules]]
- [[Architecture/ADRs/ADR-007-provider-tool-contracts]]
- [[Planning/LAYER-API-HARDENING]]
