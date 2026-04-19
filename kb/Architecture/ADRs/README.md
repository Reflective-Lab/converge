# Architecture Decision Records

This directory contains binding architecture decisions for Converge.

Rules:

- ADRs define the canonical contract when older code comments or docs disagree.
- ADRs are append-only. Amend by adding a new ADR, not by silently rewriting old
  decisions after they have guided implementation.
- If an ADR requires a breaking change, the codebase should move toward the ADR.
  The ADR should not be weakened to fit accidental implementation.

Current sequence:

- `ADR-001` Canonical Public Crates
- `ADR-002` Truth Pipeline
- `ADR-003` Pack Authoring Contract
- `ADR-004` Contract Stabilization
- `ADR-005` Type Ownership Boundaries
- `ADR-006` Clock Injection for Kernel Determinism
- `ADR-007` Capability Contract Ownership
