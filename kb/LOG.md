---
source: llm
---
# KB Mutation Log

| Date | File | Change | Author |
|------|------|--------|--------|
| 2026-05-05 | Architecture/Known Drift.md; Planning/MILESTONES.md; Planning/v3.8 Foundation.md | atelier and arbiter extractions: crates/domain + examples/* moved to ~/dev/atelier (showcase repo); crates/policy moved to ~/dev/extensions/arbiter (Cedar engine extension). manifold deferred — see notes for the runtime/llm/selection coupling that blocks a clean split. | mixed |
| 2026-05-05 | Planning/MILESTONES.md; Planning/v3.8 Foundation.md | v3.8 low-hanging security and extraction updates: feed/web fetch hardening marked complete; ferrox relocated to ~/dev/extensions/ferrox | codex |
| 2026-05-05 | Architecture/ADRs/ADR-006-promotion-authority-boundary.md; Planning/Authority Slice.md; Architecture/API Surfaces.md; Architecture/Embedding Quick Start.md; Architecture/Known Drift.md; Architecture/Type Protocol.md; Concepts/Context and Facts.md; Architecture/System Overview.md; Architecture/Security Review Plan.md; Planning/MILESTONES.md | v3.8 authority slice: removed public kernel-authority, introduced read-only ContextFact and verified ContextSnapshot rehydration, recorded compile-fail proof set and Helms migration path | mixed |
| 2026-05-05 | Architecture/Crate Map.md; Building/Crate Catalog.md; Architecture/System Overview.md; Architecture/API Surfaces.md; Concepts/Domain Packs.md; Modules/Domain Packs.md; Building/DEVELOPMENT.md; Architecture/Known Drift.md; Planning/MILESTONES.md; Planning/v3.8 Foundation.md; Algorithms/*.md | prism extraction: removed converge-analytics from foundation; analytics suggestors now live in ~/dev/extensions/prism. Locked names for remaining extractions: manifold, arbiter, atelier, plus ferrox relocation. | mixed |
| 2026-05-05 | Architecture/Crate Map.md; Building/Crate Catalog.md; Architecture/System Overview.md; Architecture/API Surfaces.md; Architecture/Suggestor Catalog.md; Concepts/Domain Packs.md; Modules/Domain Packs.md; Building/DEVELOPMENT.md; Architecture/Known Drift.md; Planning/MILESTONES.md; Planning/v3.8 Foundation.md | mnemos extraction: removed converge-knowledge from foundation; knowledge suggestors now live in ~/dev/extensions/mnemos | mixed |
| 2026-05-05 | Architecture/Plug Boundary.md; INDEX.md | New load-bearing page: Suggestor and Backend are two distinct pluggable layers; Formation composes Suggestors with declared capability requirements, never adapter types | mixed |
| 2026-04-23 | History/CHANGELOG.md; Architecture/Formation Building Review.md | Prepared v3.7.3 release notes and clarified Organism-owned correlation observer path for HITL experience events | llm |
| 2026-04-23 | Planning/Organism Formation Compiler Milestone Draft.md; ~/.codex/memories/converge-organism-handoff-2026-04-23.md | Added final cross-layer handoff for building applications across Axiom, Organism, and Converge | llm |
| 2026-04-23 | Planning/Formation Business Strategy.md | Business strategy companion: go-to-market wedges, trust graduation curve, institutional decision memory moat, commercial framing | mixed |
| 2026-04-23 | Architecture/Formation Building Review.md | Two-pass architecture review: formation compiler, three catalogs, capability discovery, HITL→Cedar graduation, OpenClaw semantics | mixed |
| 2026-04-22 | Algorithms/Problem Taxonomy.md | Created analytics and ML problem taxonomy cheat-sheet | mixed |
| 2026-04-15 | Integrations/Response Formats.md; Architecture/Known Drift.md | Clarified that provider contract now enforces output format but not exact schema shape | llm |
| 2026-04-15 | Ecosystem.md; Home.md | Reframed the ecosystem around Helm, Axiom, Organism, Converge, and Providers | llm |
| 2026-04-13 | Architecture/Crate Map.md | Added auth, consensus, ledger, nats, observability crates | llm |
| 2026-04-13 | Architecture/System Overview.md | Added 5 missing crates | llm |
| 2026-04-13 | INDEX.md | Created entity catalog | llm |
| 2026-04-13 | LOG.md | Created mutation log | llm |
| 2026-04-13 | Multiple files | Added source: frontmatter per workspace convention | llm |
