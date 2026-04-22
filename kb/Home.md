---
tags: [moc]
source: mixed
---
# Converge Suggestor OS

Knowledge base for the Converge platform — a correctness-first, context-driven multi-suggestor runtime.

**Start here:** [[Ecosystem]] for the helicopter view, then [[Philosophy/Why Converge]] and [[Philosophy/Nine Axioms]].

**Meta:** [[INDEX]] — entity catalog | [[LOG]] — mutation log

## Ecosystem

- [[Ecosystem]] — the five-layer stack: Helm, Axiom, Organism, Converge, Providers

## Philosophy

- [[Philosophy/Why Converge]] — the problem and the promise
- [[Philosophy/Nine Axioms]] — the inviolable rules
- [[Philosophy/What Converge Is Not]] — drift prevention, anti-patterns
- [[Philosophy/Convergence Explained]] — how fixed-point execution works

## Concepts

- [[Concepts/Agents]] — the suggestor contract
- [[Concepts/Context and Facts]] — shared state and typed evidence
- [[Concepts/Proposals and Promotion]] — the governance boundary
- [[Concepts/Invariants]] — executable guarantees
- [[Concepts/Root Intent]] — scoping a convergence run
- [[Concepts/Backends and Capabilities]] — provider abstraction
- [[Concepts/Formations]] — self-assembling teams and structured formation requests
- [[Concepts/Domain Packs]] — pre-built suggestor collections
- [[Concepts/HITL Gates]] — human-in-the-loop decisions
- [[Concepts/Experience and Recall]] — learning from past runs
- [[Concepts/Governed Artifacts]] — lifecycle management for outputs

## Building with Converge

- [[Building/Getting Started]] — toolchain, first run, quick reference
- [[Building/Capabilities]] — what the platform already provides
- [[Building/Writing Agents]] — implement the `Suggestor` trait
- [[Building/Context Keys]] — partitioning evidence
- [[Building/Streaming]] — real-time convergence callbacks
- [[Building/Crate Catalog]] — all published crates

## Architecture

- [[Architecture/System Overview]] — system design, dependency graph, and runtime shape
- [[Architecture/Hexagonal Architecture]] — ports and adapters, the big picture
- [[Architecture/API Surfaces]] — the six public crates and who uses what
- [[Architecture/Golden Path Matrix]] — the first dependency each next layer should reach for
- [[Architecture/ADRs/README]] — binding architecture decisions
- [[Architecture/Known Drift]] — where code diverges from axioms (and how we fix it)
- [[Architecture/Ports]] — trait definitions (the boundary)
- [[Architecture/Providers]] — adapter implementations (the outside world)
- [[Architecture/Transport Protocols]] — gRPC, SSE, REST, WebSocket
- [[Architecture/Lean Packaging and Embedding]] — runtime shell variants and embedding baseline
- [[Architecture/Crate Map]] — workspace layout and layering
- [[Architecture/Engine Execution Model]] — the 8-phase cycle
- [[Architecture/Dependency Rules]] — what imports what and why
- [[Architecture/Type Protocol]] — the rules for core types
- [[Architecture/Purity Rules]] — what belongs in converge-core
- [[Architecture/Security Review Plan]] — post-restructuring test plan

## Integrations

- [[Integrations/MCP Tools]] — Model Context Protocol for agent tooling
- [[Integrations/External Services]] — mocking and service injection

## Stack

### Language & Frameworks
- [[Stack/Rust]] — language conventions
- [[Stack/Tauri]] — desktop packaging
- [[Stack/Svelte]] — frontend UI

### LLM Inference
- [[Stack/Burn]] — local inference framework (CUDA, Metal, CPU)
- [[Stack/Ollama]] — local LLM server
- [[Stack/vLLM]] — production GPU serving

### Data & Storage
- [[Stack/SurrealDB]] — experience store (document + relational)
- [[Stack/LanceDB]] — vector-indexed experience store and search
- [[Stack/Polars]] — data analytics and metrics

### Optimization
- [[Stack/OR-Tools]] — constraint programming and solvers

## Workflow

- [[Workflow/Daily Journey]] — start-to-finish cheat sheet
- [[Workflow/Git Strategy]] — branch, worktree, release, and remote hygiene rules
- [[Workflow/Working with Claude]] — skills vs Justfile
- [[Workflow/Working with Codex]] — plain-language workflow equivalents
- [[Workflow/Working with Gemini]] — native tools + shared scripts

## Planning

- [[Planning/MILESTONES]] — scheduled delivery across releases
- [[Planning/LAYER-API-HARDENING]] — move up the stack one layer at a time

## Experiments

- [[Experiments/INDEX]] — hypothesis-driven development with evidence logging
- [[Experiments/LOG]] — mutation log of all experiments

## Audits

- [[Architecture/Audits/2026-04-11 Drift Report]] — baseline: philosophy vs implementation
- [[Architecture/Audits/2026-04-11 KB Accuracy Report]] — baseline: documentation vs code
- [[Architecture/Audits/2026-04-11 Drift Rerun]] — current state after contract hardening
- [[Architecture/Audits/2026-04-11 KB Accuracy Rerun]] — current state after KB correction pass
- [[Architecture/Audits/2026-04-11 Security Review]] — runtime and policy control-surface audit

Re-run these audits when the public contract or control surfaces change.

## Governance

- [[Governance/How This Works]] — 24 personas, phase contracts, evals, strategic gates
- [[Governance/README]] — full persona catalog
- [[Governance/GATES]] — gate framework and taxonomy
- [[Governance/TEAM]] — authority tiers and roster
