---
tags: [architecture, extensions, ports, providers]
source: mixed
date: 2026-05-05
---
# Extension Topology

`/Users/kpernyer/dev/extensions` is the canonical home for reusable Converge
extensions. These repositories keep implementation-heavy, SDK-facing, or
source-specific code outside the foundation while still depending on stable
Converge contracts.

> Every extension repo must satisfy
> [[Standards/Extension Release Checklist]] before it ships in a Converge
> release line.

## Decision

Converge keeps universal contracts. Extensions implement those contracts, and
may define extension-local ports when the domain is real but not foundational.

The dependency rule stays one-way:

```text
converge contracts <- extensions <- products / deployments
```

Converge does not depend on extension repositories. Products and deployment
layers wire extension crates into runnable systems.

## Port, Provider, Backend

- **Port:** a trait plus request/response DTOs and semantic value types. A port
  may live in Converge when it is universal, or in an extension when it is
  source-specific.
- **Provider / adapter:** a concrete implementation of a port. It imports the
  contract and external SDKs. It does not define truth.
- **Backend / capability:** the operational provider layer used by capability
  routing. It declares what it can do and is selected by requirements.
- **Suggestor:** the purposeful layer. It uses ports/backends to create
  proposals, but Converge still owns promotion.

If a source-specific extension port becomes broadly reusable across extension
families, promote the contract upstream. Keep the implementation downstream.

## Canonical Extension Homes

| Extension | Home | Owns |
|---|---|---|
| `embassy` | `/Users/kpernyer/dev/extensions/embassy` | External-party connector ports and adapters where the foreign system identity is part of the contract. LinkedIn is the first example. |
| `manifold` | `/Users/kpernyer/dev/extensions/manifold` | Generic provider/tool adapters where the vendor should be hidden behind capabilities: LLM, search, fetch, feed, embeddings, and external tool backends. |
| `mnemos` | `/Users/kpernyer/dev/extensions/mnemos` | Knowledge, recall, retrieval, vector stores, and memory suggestors. |
| `prism` | `/Users/kpernyer/dev/extensions/prism` | Analytics, ML pipelines, feature extraction, inference, training, and monitoring suggestors. |
| `arbiter` | `/Users/kpernyer/dev/extensions/arbiter` | Policy engines, Cedar wiring, policy suggestors, and authorization implementation. |
| `ferrox` | `/Users/kpernyer/dev/extensions/ferrox` | Native optimization solver integrations and solver adapters. |

Storage adapters follow the same rule. Reusable Rust adapters live under
extensions; operational service wiring lives in Runway or product repositories.
See [[Architecture/Storage Boundary]].

## Provider and Tool Naming

Provider/tool contracts own the clean domain names. Adapter implementations
carry qualifiers that describe what they implement.

For the 3.8.x line, `converge-provider` is the published provider contract.
Generic provider/tool implementations live in Manifold and use names such as
`OpenAiChatAdapter`, `BraveSearchAdapter`, `HttpFeedAdapter`, and
`OpenApiToolAdapter`. The former `converge-provider-adapters` staging crate
has been removed.

The Converge-side dependency break is in place: runtime and kernel no longer
depend on adapter implementations, and `converge-provider` owns the
host-supplied `ChatBackendRegistry` contract. See [[Planning/Manifold Provider
Tool Migration]].

## Embassy vs Manifold

Use `embassy` when the external party is semantically part of the API.

Examples:

- LinkedIn profile lookup.
- Professional-network company or person enrichment.
- A source-specific business connector whose request/response shape carries
  terms, provenance, identity, or compliance semantics from that source.

Use `manifold` when the provider is interchangeable behind a generic
capability.

Examples:

- Web fetch.
- Web search.
- Feed retrieval.
- LLM chat.
- Embedding generation.

The smell test:

- If a Formation can ask for a capability without naming the vendor, the
  implementation belongs in `manifold`.
- If the contract must say `LinkedInProfile`, `LinkedInCompany`, or another
  source-shaped semantic type, it belongs in `embassy`.

## Embassy Current State

As of 2026-05-05:

- `embassy-pack` (`/Users/kpernyer/dev/extensions/embassy/crates/pack`) owns
  the cross-port connector contract: call context, provenanced observations,
  and shared helpers.
- `embassy-linkedin`
  (`/Users/kpernyer/dev/extensions/embassy/crates/linkedin`) is the first
  source-specific connector port. It owns the LinkedIn trait,
  request/response types, and stub provider.
- `cargo check` passes in the embassy workspace.

Organism's old `organism-intelligence::linkedin` module was removed during the
extraction. End-to-end Organism checking was blocked by a pre-existing Atelier
path mismatch (`organism-domain` vs `atelier-domain`), not by the LinkedIn
extraction.

## Next Connector Notes

- Provenance helper types still live in `organism-intelligence` because OCR,
  vision, patent, and similar modules still use them. When the next port moves,
  switch those callers to `embassy-pack` and delete the local copy.
- LinkedIn URL parsing in Organism social extraction is a different layer:
  social extraction through web content, not the LinkedIn connector port. Move
  that only when an `embassy-social` connector exists.
- `linkedin_research` packs and `"linkedin"` capability strings are domain-pack
  concerns. They may consume Embassy, but they are not the port itself.

## Runtime Assembly

`runway` and product repositories assemble extensions into runnable systems.
They own Docker, processes, secrets, deployment topology, and environment
selection. They do not become upstream homes for reusable Converge contracts.

See also: [[Architecture/Plug Boundary]], [[Architecture/Ports]],
[[Architecture/Providers]], [[Architecture/Storage Boundary]],
[[Architecture/ADRs/ADR-008-extension-crate-boundaries]],
[[Planning/v3.8 Foundation]].
