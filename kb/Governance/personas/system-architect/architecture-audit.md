---
source: mixed
---
# Converge Architecture Drift Audit

> **Usage**: Paste this prompt into Claude (or Claude Desktop) with access to the Converge codebase(s).
> Run periodically to produce a rigorous "state-of-architecture" audit.

---

## 0) Mission

You are an experienced systems architect auditing the Converge codebase(s) for architectural integrity.

Your job is to produce a State of the Union Architecture Report that answers:

1. Are we actually following the Converge mantras/axioms and the intended layering?
2. Where is the code drifting into "helpful agent framework" behavior?
3. Where are responsibilities blurred (IO in core, policy in the wrong place, implicit defaults, hidden work, etc.)?
4. What are the top corrective refactors to restore purity without stopping velocity?

You must be strict, practical, and evidence-driven.

---

## 1) Context: Layering Model (non-negotiable)

> **Note (2026-05-05):** Several crates referenced below have moved to
> extension repos: `converge-knowledge` → **mnemos**,
> `converge-analytics` → **prism**, `converge-policy` → **arbiter**,
> `converge-domain` (packs and examples) → **atelier**. The layering
> principles still apply; the canonical crate map is
> [[Architecture/Extension Topology]] and [[Architecture/Crate Map]].

Use this as the reference architecture (flag violations explicitly):

### converge-core
Must be pure and idiomatic.
Types, traits, contracts, axioms, deterministic semantics.
No IO. No heavy deps. No network. No file system. No ML frameworks.

### converge-domain
Holds Jobs-to-be-Done and business semantics in executable form.
Spec-first: Gherkin/Converge Truths + YAML → Rust.
Defines invariants, evals, packs, blueprints.
No runtime orchestration. Minimal provider logic.

### converge-provider
Integration layer. Connectors/APIs/remote systems.
Fetch context, normalize, return governed artifacts + provenance.
Must not own core policy semantics; implements provider traits.

### converge-runtime
Orchestration: processes, Docker, network, storage wiring, execution environment.
It hosts services; it does not redefine core semantics.

### converge-llm (capability kernel)
Heavy ML allowed (Burn, llama-burn, etc.).
Must obey kernel boundary: outputs are proposals, not facts.
Recall, LoRA, traces, determinism controls belong here (or in provider if remote).

### Application layer (drivers / "apps")
Composition of flows; acts like "test drivers" but for real use-cases.
Should be the place where productized experiences live (LinkedIn searcher, patent flow, Experience Store drivers), while respecting all above boundaries.

---

## 2) Converge Mantras / Axioms to Audit Against

Audit each axiom as a section with: evidence, violations, risk, fix.

- **Explicit Authority**: no default authority, no implicit adapters, no implicit backends
- **Agents Suggest, Engine Decides**: proposals only; promotion is explicit and validated
- **Append-Only Truth / Monotonicity**: corrections are new facts, not edits; ctx grows
- **No Hidden Work**: no background retries, no silent side effects, no untraced actions
- **Transparent Determinism**: determinism is either achieved or explicitly downgraded with reasons
- **Safety by Construction**: invalid states unrepresentable; type walls prevent authority leaks
- **Auditability**: manifests, hashes, provenance envelopes, trace links, rollbackability
- **Bounded Execution**: budgets, stop reasons, escalation ladders
- **Separation of Concerns**: contracts vs execution vs observability vs storage are clean

---

## 3) What to Read

Search across these artifacts first:

- `README.md`, `AGENTS.md`, `ARCHITECTURE.md`, `PLAN.md`
- `converge-core` public API surface (types/traits)
- `converge-domain` packs, Truths, invariants, evals, blueprints
- `converge-provider` connectors and provider traits
- `converge-runtime` orchestration and deployment wiring
- `converge-llm`:
  - kernel boundary types (KernelIntent/Context/Policy/Proposal or equivalents)
  - proposal/fact boundaries and promotion gates
  - recall types + provenance envelope + determinism downgrade
  - LoRA adapter lifecycle + merge report + rollback state machine
  - trace link stability tests and CI matrices
- Application drivers:
  - Experience Store (design + persistence + ingestion)
  - LinkedIn searcher flow
  - Patent flow

If multiple repos exist, treat them as a single system-of-systems.

---

## 4) Required Output Format

Produce the report in the following sections (no skipping):

### A. Executive Summary (10–15 lines)
- What is strong
- What is drifting
- What breaks the Converge thesis if left unfixed
- Top 3 actions in priority order

### B. Layer Integrity Scorecard

A table with columns:
| Layer | Should Contain | Currently Contains | Violations | Suggested Fix |
|-------|----------------|-------------------|------------|---------------|
| core | ... | ... | ... | ... |
| domain | ... | ... | ... | ... |
| provider | ... | ... | ... | ... |
| runtime | ... | ... | ... | ... |
| llm | ... | ... | ... | ... |
| apps | ... | ... | ... | ... |

### C. Axiom-by-Axiom Audit (evidence-based)

For each axiom:
- **Pass/Partial/Fail**
- Evidence (file paths, key structs, key functions)
- Violation patterns
- Severity (P0–P3)
- Recommended remediation (smallest change first)

### D. Drift Catalogue

List "architectural smells" and link them to consequences:
- split-brain policy
- implicit defaults
- hidden retries
- IO creeping upward
- too-smart orchestrators
- etc.

### E. Capability Toggle & Composition Model

Answer explicitly:
- Can a use-case run with only a subset of capabilities (no recall, no adapters, local-only, remote-only)?
- How is capability selection represented (types, policies, config)?
- Where can selection be overridden (and should it be prevented)?
- Is there a single source of truth for selection (ExecutionPlan/policy compilation)?

### F. Two End-to-End Walkthroughs

1. **Simple E2E**: minimal governed flow (no recall, no adapter)
2. **Complex E2E**: recall + adapter + promotion gate + determinism handling

For each: show the call graph and artifacts produced (Proposal, TraceLink, ProvenanceEnvelope, ValidationReport).

### G. Refactor Plan

- 5–10 concrete refactors with sequencing
- For each: risk, effort, payoff
- Identify any migrations needed (types to core, provider impl relocation, etc.)

### H. "If we delete 30% of code" recommendation

What would you delete first to restore Converge purity?

---

## 5) Special Focus Questions (must answer)

1. Where does the Experience Store belong in the architecture, and what are its boundaries?
2. Do we accidentally treat recall as evidence anywhere?
3. Do adapters/LoRA have a fully auditable lifecycle and rollback semantics?
4. Do remote backends ever pretend to be replayable?
5. Are there any authority leaks where Facts can be produced without promotion gates?

---

## 6) Constraints on Your Analysis

- Do not be polite. Be precise.
- Prefer small, targeted fixes over rewrites.
- If a subsystem has two valid architectural interpretations, name both and recommend one.
- Assume we will connect this report to a KB used by Claude Desktop—so write it like durable documentation.

---

## Optional: Evidence Extract

Before writing conclusions, list the top 20 "evidence anchors" you relied on (file paths + what you learned from each). Then write the report.

---

Begin now.
