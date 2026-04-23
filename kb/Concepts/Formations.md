---
tags: [concepts, formations]
source: mixed
---
# Formations

Formations are self-assembling or pre-arranged teams of `Suggestor`s that run
inside one Converge loop.

## What a Formation Is

A formation is:

- a set of suggestors
- a seed context
- a budget
- optionally a provider pool and a suggestor catalog

It is not a separate pipeline stage. It is one convergence loop with a chosen
team.

## Stable Pattern

The public rule is simple:

- semantics in `converge-model`
- authoring in `converge-pack`
- runnable machinery in `converge-kernel`

For embedders, the grouped offering API is `converge_kernel::formation`.

## Structured Boundary

The canonical formation contract begins at structured requests:

- `FormationRequest`
- `ProviderRequest`

Everything upstream of those is optional policy or application logic.

- If intent already arrives structured, a seeder can write the requests directly.
- If intent arrives loose, an upstream suggestor can compile it into the requests.

Both are valid. The structured requests are the stable handoff.

## What Converge Owns vs What It Does Not

Converge owns:

- running the loop
- promoting proposals
- built-in formation and provider matching suggestors
- deterministic execution and stop semantics

Converge does not own:

- deciding which formation shape is best for an intent
- tournamenting multiple formations
- learning which shapes work best over time

Those belong in upper layers such as Organism.

For the current Organism-layer architecture guidance — descriptor contracts,
formation compilers, tournaments, HITL→Cedar graduation, and OpenClaw guard
rails — see [[Architecture/Formation Building Review]].

## Code Pattern

See [[Architecture/Formation Pattern]] for the actual embedding recipe.
