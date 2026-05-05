---
source: llm
---
# KB Entity Catalog

All major entities in the Converge knowledge base, organized by category.

## Architecture

- [Core Ideas](Architecture/Core%20Ideas.md) — durable principles for the next stable period
- [System Overview](Architecture/System%20Overview.md)
- [API Surfaces](Architecture/API%20Surfaces.md)
- [Golden Path Matrix](Architecture/Golden%20Path%20Matrix.md)
- [Crate Map](Architecture/Crate%20Map.md)
- [Dependency Rules](Architecture/Dependency%20Rules.md)
- [Engine Execution Model](Architecture/Engine%20Execution%20Model.md)
- [Extension Topology](Architecture/Extension%20Topology.md) — canonical extension homes and the Embassy vs Manifold split
- [Formation Building Review](Architecture/Formation%20Building%20Review.md)
- [Hexagonal Architecture](Architecture/Hexagonal%20Architecture.md)
- [Known Drift](Architecture/Known%20Drift.md)
- [Plug Boundary](Architecture/Plug%20Boundary.md) — load-bearing rule: Suggestor and Backend are two distinct layers
- [Ports](Architecture/Ports.md)
- [Providers](Architecture/Providers.md)
- [Purity Rules](Architecture/Purity%20Rules.md)
- [Security Review Plan](Architecture/Security%20Review%20Plan.md)
- [Storage Boundary](Architecture/Storage%20Boundary.md) — storage ports, database adapters, and Runway assembly
- [Transport Protocols](Architecture/Transport%20Protocols.md)
- [Type Protocol](Architecture/Type%20Protocol.md)

## ADRs

- [ADR-001: Canonical Public Crates](Architecture/ADRs/ADR-001-canonical-public-crates.md)
- [ADR-002: Truth Pipeline](Architecture/ADRs/ADR-002-truth-pipeline.md)
- [ADR-003: Pack Authoring Contract](Architecture/ADRs/ADR-003-pack-authoring-contract.md)
- [ADR-004: Contract Stabilization](Architecture/ADRs/ADR-004-contract-stabilization.md)
- [ADR-005: Type Ownership Boundaries](Architecture/ADRs/ADR-005-type-ownership-boundaries.md)
- [ADR-006: Promotion Authority Boundary](Architecture/ADRs/ADR-006-promotion-authority-boundary.md)
- [ADR-007: Provider and Tool Contract Names](Architecture/ADRs/ADR-007-provider-tool-contracts.md)
- [ADR-008: Extension Crate Boundaries](Architecture/ADRs/ADR-008-extension-crate-boundaries.md)

## Building

- [Capabilities](Building/Capabilities.md)
- [Context Keys](Building/Context%20Keys.md)
- [Crate Catalog](Building/Crate%20Catalog.md)
- [Deployment](Building/Deployment.md)
- [Getting Started](Building/Getting%20Started.md)
- [Streaming](Building/Streaming.md)
- [Writing Agents](Building/Writing%20Agents.md)

## Planning

- [Milestones](Planning/MILESTONES.md)
- [Formation Business Strategy](Planning/Formation%20Business%20Strategy.md)
- [Organism Formation Compiler Milestone Draft](Planning/Organism%20Formation%20Compiler%20Milestone%20Draft.md)
- [Layer API Hardening](Planning/LAYER-API-HARDENING.md)

## Concepts

- [Agents](Concepts/Agents.md)
- [Backends and Capabilities](Concepts/Backends%20and%20Capabilities.md)
- [Context and Facts](Concepts/Context%20and%20Facts.md)
- [Domain Packs](Concepts/Domain%20Packs.md)
- [Experience and Recall](Concepts/Experience%20and%20Recall.md)
- [Governed Artifacts](Concepts/Governed%20Artifacts.md)
- [HITL Gates](Concepts/HITL%20Gates.md)
- [Invariants](Concepts/Invariants.md)
- [Proposals and Promotion](Concepts/Proposals%20and%20Promotion.md)
- [Root Intent](Concepts/Root%20Intent.md)

## Philosophy

- [Convergence Explained](Philosophy/Convergence%20Explained.md)
- [Nine Axioms](Philosophy/Nine%20Axioms.md)
- [What Converge Is Not](Philosophy/What%20Converge%20Is%20Not.md)
- [Why Converge](Philosophy/Why%20Converge.md)

## Governance

- [How This Works](Governance/How%20This%20Works.md)
- [README](Governance/README.md) — full persona catalog
- [GATES](Governance/GATES.md) — gate framework and taxonomy
- [TEAM](Governance/TEAM.md) — authority tiers and roster

## Stack

### Language & Frameworks
- [Rust](Stack/Rust.md)
- [Tauri](Stack/Tauri.md)
- [Svelte](Stack/Svelte.md)

### LLM Inference
- [Burn](Stack/Burn.md)
- [Ollama](Stack/Ollama.md)
- [vLLM](Stack/vLLM.md)

### Data & Storage
- [SurrealDB](Stack/SurrealDB.md)
- [LanceDB](Stack/LanceDB.md)
- [Polars](Stack/Polars.md)

### Optimization
- [OR-Tools](Stack/OR-Tools.md)

## Integrations

- [External Services](Integrations/External%20Services.md)
- [MCP Tools](Integrations/MCP%20Tools.md)

## Workflow

- [Daily Journey](Workflow/Daily%20Journey.md)
- [Working with Claude](Workflow/Working%20with%20Claude.md)
- [Working with Codex](Workflow/Working%20with%20Codex.md)
- [Working with Gemini](Workflow/Working%20with%20Gemini.md)

## Algorithms

### Optimization
- [Hungarian Algorithm](Algorithms/Hungarian%20Algorithm.md) — O(n³) optimal assignment
- [Hopcroft-Karp Bipartite Matching](Algorithms/Hopcroft-Karp%20Bipartite%20Matching.md) — O(E√V) maximum matching
- [0-1 Knapsack](Algorithms/0-1%20Knapsack.md) — DP pseudo-polynomial optimal subset
- [Dijkstra Shortest Path](Algorithms/Dijkstra%20Shortest%20Path.md) — O((V+E) log V) SSSP
- [Max Flow](Algorithms/Max%20Flow.md) — Ford-Fulkerson / Edmonds-Karp
- [Min-Cost Flow](Algorithms/Min-Cost%20Flow.md) — successive shortest paths
- [Greedy Set Cover](Algorithms/Greedy%20Set%20Cover.md) — O(ln n) approximation
- [List Scheduling](Algorithms/List%20Scheduling.md) — EDD heuristic

### Analytics
- [Z-Score Anomaly Detection](Algorithms/Z-Score%20Anomaly%20Detection.md) — (x-μ)/σ threshold
- [Descriptive Statistics](Algorithms/Descriptive%20Statistics.md) — mean, median, variance, range
- [Linear Regression](Algorithms/Linear%20Regression.md) — y = w·x + b inference
- [Logistic Classification](Algorithms/Logistic%20Classification.md) — sigmoid decision boundary
- [Cosine Similarity](Algorithms/Cosine%20Similarity.md) — pairwise vector similarity
- [Exponential Smoothing](Algorithms/Exponential%20Smoothing.md) — SES forecasting
- [K-Means Clustering](Algorithms/K-Means%20Clustering.md) — Lloyd's algorithm
- [Weighted Multi-Criteria Ranking](Algorithms/Weighted%20Multi-Criteria%20Ranking.md) — normalize-weight-sum MCDM

## Ecosystem

- [Ecosystem](Ecosystem.md)
