---
source: mixed
---
# Test Coverage Audit

> **Usage**: Run monthly to assess test coverage health and identify gaps.

> **Note (2026-05-05):** The crate table below lists crates that have moved
> to extension repos: `converge-knowledge` → **mnemos**, `converge-analytics`
> → **prism**, `converge-policy` → **arbiter**, `converge-domain` →
> **atelier**. Each extension repo runs its own coverage. See
> [[Architecture/Extension Topology]].

---

## Mission

You are a QA engineer auditing test coverage across the Converge codebase. Identify gaps, assess coverage quality (not just quantity), and prioritize testing improvements.

---

## 1) Coverage Metrics

### Quantitative Coverage

```bash
# Generate coverage report (example with cargo-tarpaulin)
cargo tarpaulin --out Html --output-dir coverage/

# Or with llvm-cov
cargo llvm-cov --html
```

| Crate | Line Coverage | Branch Coverage | Function Coverage |
|-------|---------------|-----------------|-------------------|
| converge-core | | | |
| converge-domain | | | |
| converge-provider | | | |
| converge-runtime | | | |
| converge-llm | | | |

### Coverage Trends

| Crate | Last Month | This Month | Trend |
|-------|------------|------------|-------|
| | | | ↑/↓/→ |

---

## 2) Coverage Quality Assessment

High coverage numbers don't mean good tests. Assess:

### Test Types Present

| Test Type | Present? | Count | Quality |
|-----------|----------|-------|---------|
| Unit tests | | | |
| Integration tests | | | |
| Property tests | | | |
| Contract tests | | | |
| E2E tests | | | |
| Fuzz tests | | | |

### Critical Path Coverage

| Critical Path | Tested? | Test Quality | Gaps |
|---------------|---------|--------------|------|
| Proposal creation | | | |
| Fact promotion | | | |
| Validation gates | | | |
| Audit trail recording | | | |
| Authority checking | | | |
| Determinism guarantees | | | |
| Error handling | | | |

### Edge Cases

| Area | Edge Cases Tested | Missing |
|------|-------------------|---------|
| Empty inputs | | |
| Maximum sizes | | |
| Concurrent access | | |
| Error conditions | | |
| Boundary values | | |

---

## 3) Converge-Specific Test Gaps

### Governance Invariants

| Invariant | Test Exists? | Test Type | Confidence |
|-----------|--------------|-----------|------------|
| Cannot create Fact without promotion | | | |
| Proposals are not Facts | | | |
| Authority must be explicit | | | |
| Audit trail is append-only | | | |
| Traces are immutable | | | |

### Determinism Testing

| Scenario | Tested? | Approach |
|----------|---------|----------|
| Same input → same output | | Property test |
| Replay produces same result | | Golden file |
| Downgrade is explicit | | Unit test |

### Provider Contract Testing

| Provider | Contract Tests | Isolation Tests |
|----------|----------------|-----------------|
| | | |

---

## 4) Test Health

### Flaky Tests

| Test | Flakiness Rate | Cause | Status |
|------|----------------|-------|--------|
| | | | |

### Slow Tests

| Test | Duration | Acceptable? | Optimization |
|------|----------|-------------|--------------|
| | | | |

### Test Maintainability

| Concern | Status | Issues |
|---------|--------|--------|
| Test readability | | |
| Test isolation | | |
| Setup/teardown | | |
| Mocking approach | | |

---

## 5) Required Output

### A. Coverage Summary

```markdown
## Coverage Report - [Date]

### Overall
- Line Coverage: X%
- Branch Coverage: X%
- Critical Path Coverage: X%

### By Crate
| Crate | Coverage | Change | Status |
|-------|----------|--------|--------|

### Health
- Flaky tests: [count]
- Slow tests (>1s): [count]
- Skipped tests: [count]
```

### B. Gap Analysis

| Priority | Gap | Risk if Not Tested | Effort to Fix |
|----------|-----|-------------------|---------------|
| P0 | | | |
| P1 | | | |
| P2 | | | |

### C. Recommended Test Additions

| Test | Type | Covers | Effort |
|------|------|--------|--------|
| | | | |

### D. Test Debt Register

| Debt Item | Age | Risk | Plan |
|-----------|-----|------|------|
| | | | |

### E. Action Items

1. [Immediate - blocking issues]
2. [This sprint - high priority gaps]
3. [Backlog - nice to have]

---

## Coverage Targets

| Category | Target | Rationale |
|----------|--------|-----------|
| Core types | >90% | Foundational, must be solid |
| Governance paths | 100% | These are our claims |
| Error handling | >80% | Failures should be tested |
| Happy paths | >80% | Basic functionality |
| Edge cases | >60% | Risk-based coverage |

---

## Constraints

- Focus on coverage quality, not just percentage
- Prioritize testing governance invariants over general code
- Identify tests that exist but don't actually test anything meaningful
- Flag areas where mocking might be hiding real bugs
