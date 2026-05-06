---
tags: [architecture, philosophy, load-bearing]
source: mixed
date: 2026-05-05
---
# Core Ideas

The durable principles that define Converge for the next stable period.
v3.8 is a foundation release: the goal is fewer, stronger concepts that are
boring to depend on for years. These are the ideas that everything else —
ADRs, ports, extension repos, naming rules — exists to protect. If a future
change makes any of these harder to enforce, that change has to justify the
loss.

Five load-bearing rules, plus three implementation rules that operationalize
them.

## 1. Two pluggable layers, never collapsed

Converge has two distinct plug points: the **Suggestor** layer (purposeful
contributors that read context and emit proposals) and the **Backend /
Capability** layer (operational adapters that declare what they can do). A
Formation composes Suggestors with declared capability requirements; the
runtime resolves capabilities to backends at activation. A Suggestor must
never import an adapter type, and a Backend must never see authoring types
like `Context` or `ProposedFact`.

**Cost of violating:** the engine loses its ability to attribute truth, vendor
shape leaks into the kernel, and swapping a provider becomes a kernel change.
See [[Architecture/Plug Boundary]].

## 2. Promotion authority lives behind a real boundary

Pack authors emit `ProposedFact`. Context readers see the read-only
`ContextFact` projection. Authoritative `Fact` construction is held by the
engine and is not exposed through a Cargo feature, a public constructor, or
any path that downstream feature unification can reach. The promotion gate is
the only place a proposal becomes truth.

**Cost of violating:** any crate in the dependency graph that turns on a
feature can forge truth, the convergence loop's invariant chain breaks, and
governance becomes advisory. See [[Architecture/ADRs/ADR-006-promotion-authority-boundary]].

## 3. Contracts get the real names; implementations carry adapter words

A long-lived contract crate owns the clean domain name. Implementation crates
add qualifiers like `http`, `openai`, `surreal`, or `runtime`. The
`converge-provider` contract now owns the clean provider name; generic
provider/tool implementations live in Manifold. The contract must not import
unrelated value types from neighboring crates just to borrow vocabulary.

**Cost of violating:** every rename later breaks downstream pins, the contract
crate accretes incidental dependencies, and the support boundary becomes
ambiguous. See [[Architecture/ADRs/ADR-007-provider-tool-contracts]].

## 4. Implementation-heavy capabilities live in extensions

Foundation owns universal contracts. Vector stores, ML pipelines, policy
engines, source-specific connectors, native solvers, and vendor SDKs live in
extension repositories under `~/dev/extensions/*` (mnemos, prism, arbiter,
ferrox, embassy, manifold). The dependency arrow is one-way: foundation
contracts ← extensions ← products. Foundation never imports an extension.

**Cost of violating:** the foundation's release cadence gets dragged behind
every vendor SDK upgrade, security review surface explodes, and "boring to
depend on" stops being achievable. See [[Architecture/Extension Topology]],
[[Architecture/ADRs/ADR-008-extension-crate-boundaries]].

## 5. External I/O lives outside the kernel

The kernel owns the convergence loop, promotion, invariants, and the run
integrity proof. It does not own sockets, message buses, process lifecycle,
provider SDKs, retries, redirects, or feed/fetch/search implementations.
Those live in clearly adapter-qualified crates or in extensions. Hardening
(SSRF, oversized response, unsafe redirect, unbounded limit) is the adapter's
job, not the kernel's.

**Cost of violating:** the kernel grows a security review surface it can't
sustain, every transport CVE becomes a kernel issue, and pure logic becomes
untestable in isolation. See [[Architecture/Hexagonal Architecture]],
[[Architecture/Purity Rules]].

## 6. Semantic values carry their meaning

Confidence, score, ratio, limit, timeout, status, hash, identifier, and URL
values cross the public contract as typed domains, not raw `f64` / `u64` /
`String`. Property tests are not a substitute for making impossible states
unrepresentable. Wildcard selectors at the runtime perimeter use explicit
selector types, not magic `"*"` strings.

**Cost of violating:** invariants migrate from compile-time to runtime,
downstream callers reinvent validation, and "0.0..=1.0" drift accumulates
silently across releases. See [[Architecture/Type Protocol]].

## 7. Suggestors only propose; effects are finalized

`AgentEffect` is a finalized, proposal-only output. Incremental construction
belongs to `AgentEffectBuilder`. Once a Suggestor returns an effect, no other
crate — pack author, runtime, or test harness — mutates it before promotion.
Authoritative state changes only flow through the engine's promotion gate.

**Cost of violating:** mutation racing against promotion turns the engine into
a coordinator instead of a kernel; replay traces stop being trustworthy
evidence of what actually happened.

## 8. The KB is part of the product

Stale crate names, moved responsibilities, old release plans, and weak facts
are removed or demoted in the same release that moves the code. A KB that
disagrees with the workspace is a release defect, not a documentation chore.
Every kb/ page carries provenance (`source:`); every change appends to
[[LOG]].

**Cost of violating:** new contributors and downstream consumers calibrate to
fiction, and the foundation's promise of being boring to depend on becomes a
lie of omission.

## See also

- [[Architecture/Plug Boundary]] — the two-layer rule in detail
- [[Architecture/Extension Topology]] — where extension code lives and why
- [[Architecture/Hexagonal Architecture]] — ports and adapters big picture
- [[Architecture/ADRs/README]] — binding architecture decisions
- [[Architecture/ADRs/ADR-006-promotion-authority-boundary]]
- [[Architecture/ADRs/ADR-007-provider-tool-contracts]]
- [[Architecture/ADRs/ADR-008-extension-crate-boundaries]]
- [[Planning/v3.8 Foundation]] — the release this page distills
