---
type: planning
source: mixed
---
# Layer API Hardening

This plan sequences API hardening **one layer at a time**, moving upward through
the stack so downstream consumers can always stand on a stable lower boundary.

## Principle

Start at the lowest reusable layer, make its public surface exact, update the
next layer to use it correctly, then move up.

Sequence:

1. Converge
2. Organism
3. Axiom
4. Helm and product apps

The reference contract for that work is [[Architecture/Golden Path Matrix]].

## Phase 1: Converge

Goal:
- make the six Converge public crates the obvious and easiest import path

Work:
- finish capability-contract ownership under `converge-provider-api`
- remove downstream doc/examples that still teach `converge-core` as the first import path
- keep `converge-kernel`, `converge-model`, and `converge-pack` as the first surfaces for execution, reading, and authoring

Exit:
- downstream docs and examples prefer curated Converge surfaces
- internal crates are no longer the easiest things to copy from

## Phase 2: Organism

Goal:
- make `organism-pack` and `organism-runtime` the obvious app-facing Organism entrypoints

Work:
- keep phase crates internal-facing by default
- tighten the boundary for `organism-intelligence`, `organism-notes`, and `organism-domain`
- document which apps should add those optional surfaces and why

Exit:
- apps above Organism can stay on curated surfaces without custom wrapper layers

## Phase 3: Axiom

Goal:
- make Axiom clearly the truth-definition and validation layer, not an accidental Converge-internals consumer

Work:
- keep Axiom on `converge-provider-api` + `converge-provider` for live validation
- tighten the artifact boundary from Axiom to Converge
- document Helm as the operator-facing surface above Axiom

Exit:
- Axiom docs and code teach the narrow contract it actually needs

## Phase 4: Helm And Product Apps

Goal:
- make product repos compose lower layers cleanly instead of redefining them

Work:
- teach app builders the correct lower-layer surfaces first
- rename and document the operator-facing surface consistently
- keep product-specific UX, storage, and composition at the product layer

Exit:
- apps above the stack do not need internal lower-layer crates to make progress

## Rule Of Engagement

When a product repo needs a generic capability:

1. check the lower-layer public surface
2. if it exists, consume it
3. if it is missing but generic, add it to the correct lower layer
4. only keep it app-local if it is genuinely product-specific
