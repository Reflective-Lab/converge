---
eval_id: system-architect-eval
owner: system-architect

intent:
  risk_prevented: architectural violations accumulating technical debt and making codebase unmaintainable
  outcome_ensured: layer boundaries, axiom compliance, and type safety verified under current checks

determinism:
  class: deterministic
  justification: |
    Static code pattern matching and dependency direction checks produce identical
    results for identical codebase state. No external mutable state dependencies.
    Same grep patterns, same file structure = same violations detected.

governance:
  may_block_alone: true
  may_contribute_to_block: true
  eligible_for:
    pr-merge:
      may_contribute: true
      may_block: true
    release-candidate:
      may_contribute: true
      may_block: true
    release-approval:
      may_contribute: true
      may_block: true
    production-deploy:
      may_contribute: true
      may_block: true
source: mixed
---

# System Architect Eval

> Quick validation of architecture integrity. Target: 10-15 minutes.

## Mission

Perform a rapid architecture check of the Converge codebase. Verify layer boundaries, axiom compliance, and type safety. Flag critical violations only.

---

## Criteria Checklist

> **Note (2026-05-05):** The layer rows below reference crates that have
> moved to extension repos: `converge-knowledge` → **mnemos**,
> `converge-analytics` → **prism**, `converge-policy` → **arbiter**,
> `converge-domain` → **atelier**. The boundary rules still apply; see
> [[Architecture/Extension Topology]] and [[Architecture/Crate Map]] for the
> current map.

### 1. Layer Boundaries (Critical)

Check each layer for violations:

| Layer | Check | Pass Criteria |
|-------|-------|---------------|
| converge-core | No IO, no network, no file system | Zero violations |
| converge-core | No heavy deps (ML frameworks, etc.) | Zero violations |
| converge-domain | No runtime orchestration | Zero violations |
| converge-provider | Implements traits, doesn't own policy | Zero violations |
| converge-runtime | Hosts, doesn't redefine semantics | Zero violations |

**Quick checks:**
```bash
# In converge-core, should find NO:
grep -r "std::fs" converge-core/src/
grep -r "std::net" converge-core/src/
grep -r "tokio::" converge-core/src/
grep -r "reqwest" converge-core/src/
```

### 2. Axiom Compliance (Critical)

| Axiom | Quick Check | Pass Criteria |
|-------|-------------|---------------|
| Explicit Authority | Search for `Default::default()` on authority types | None found |
| Agents Suggest, Engine Decides | Proposal types exist and are distinct from Facts | Types are separate |
| Append-Only Truth | No `mut` on audit/trace types | No mutations |
| No Hidden Work | No background spawns without tracing | All traced |
| Safety by Construction | Proposal→Fact requires validation | Type-enforced |

### 3. Type Safety (Critical)

| Check | What to Look For | Pass Criteria |
|-------|------------------|---------------|
| Proposal/Fact separation | Cannot create Fact without going through promotion | Type wall exists |
| Authority boundaries | Authority is explicit parameter, not ambient | No implicit authority |
| Provenance tracking | Artifacts include provenance | ProvenanceEnvelope used |

### 4. Dependency Direction

| Check | Pass Criteria |
|-------|---------------|
| Core depends on nothing internal | Only std/external deps |
| Domain depends only on core | No provider/runtime deps |
| Provider depends on core+domain | No runtime deps |
| Runtime depends on all | OK |

---

## Output Format

```markdown
# System Architect Eval Report

## Summary
- **Status**: PASS / PARTIAL / FAIL
- **Critical Violations**: [count]
- **Warnings**: [count]
- **Run Date**: [date]

## Layer Boundary Check

| Layer | Status | Violations |
|-------|--------|------------|
| converge-core | ✓/✗ | [list if any] |
| converge-domain | ✓/✗ | [list if any] |
| converge-provider | ✓/✗ | [list if any] |
| converge-runtime | ✓/✗ | [list if any] |
| converge-llm | ✓/✗ | [list if any] |

## Axiom Compliance

| Axiom | Status | Evidence |
|-------|--------|----------|
| Explicit Authority | ✓/✗ | |
| Agents Suggest | ✓/✗ | |
| Append-Only | ✓/✗ | |
| No Hidden Work | ✓/✗ | |
| Safety by Construction | ✓/✗ | |

## Type Safety

| Check | Status | Notes |
|-------|--------|-------|
| Proposal/Fact wall | ✓/✗ | |
| Authority explicit | ✓/✗ | |
| Provenance tracked | ✓/✗ | |

## Critical Issues
[List any FAIL items with file:line references]

## Warnings
[List any concerning patterns]

## Verdict

[ ] PASS - Ready for release
[ ] PARTIAL - Minor issues, proceed with caution
[ ] FAIL - Block release, critical violations found
```

---

## Pass/Fail Thresholds

| Threshold | Result |
|-----------|--------|
| Zero critical violations | PASS |
| 1-2 minor violations, no critical | PARTIAL |
| Any critical violation | FAIL |

**Critical violations:**
- IO in converge-core
- Fact creation without promotion gate
- Implicit authority
- Audit mutation

**Minor violations:**
- Suboptimal dependency direction (not wrong, but could be cleaner)
- Missing provenance on non-critical paths
