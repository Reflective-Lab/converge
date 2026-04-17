---
name: Stability Framework - Next Steps
description: Prioritized work to operationalize and extend the stability testing pipeline
type: planning
source: human
---

# Next Steps: Stability Framework Operationalization

The **6-pillar stability framework** is now implemented and committed. This document prioritizes the next work to operationalize it and drive actionable outcomes from evidence collection.

## Immediate (This Sprint)

### 1. **Create deny.toml for Supply Chain Security** ⚡ High Impact
**Effort**: 30 min | **Impact**: Blocks vulnerable dependencies  
**Status**: Planned (mentioned in ADR-004, not yet implemented)

**What**:
- Create `deny.toml` at repo root
- Configure `cargo deny check` to block licenses and advisories
- Add to CI as blocking gate (already in security.yml, needs config)

**Why**: 
- No license compliance checking currently
- Advisories not enforced (only cargo audit, which warns)
- Risk: transitive vulnerable deps can slip through

**Deliverable**:
- `deny.toml` with:
  - License allowlist (MIT, Apache-2.0, MPL-2.0)
  - Advisory deny rules (CVSS > 5.0)
  - Known exceptions (if any)
- CI blocks on `cargo deny check` failure

---

### 2. **Draft ADR-005: Typed-ID Newtype Design** 🎯 Actionable Outcome
**Effort**: 1-2 hours | **Impact**: Prevents runtime ID validation errors  
**Status**: Evidence gathered (EXP-002 documents 6 validation gaps)

**What**:
- Propose `FactId` newtype wrapping `String`
- Validation: `[a-z][a-z0-9:_-]{0,127}` (max 128 chars)
- Update `Context::add_input()` to accept `FactId`
- Implement in `converge-pack` (public contract)

**Why**: 
- EXP-002 discovered 6 validation gaps (empty IDs, null bytes, newlines, unbounded length, whitespace, uppercase)
- Type system prevents invalid IDs at compile time
- Matches experience validation layer pattern

**Deliverable**:
- `kb/Standards/ADR/ADR-005-typed-id.md`
  - Status: Proposed (for social review)
  - Context: Why IDs need validation
  - Decision: Newtype wrapper + validation in constructor
  - Consequences: API change (breaking for internal crates)

---

### 3. **Establish Initial Baselines from Stability Run** 📊 Data Collection
**Effort**: 15 min (local) + CI time | **Impact**: Enables regression detection  
**Status**: Ready to execute

**What**:
1. Run benchmarks locally: `cargo bench -p converge-core`
2. Extract baseline: `python3 ops/scripts/extract-criterion-baseline.py`
3. Record memory soak: `SOAK_MEMORY_ITERATIONS=1000 cargo test --test soak soak_memory_stability -- --include-ignored --nocapture`
4. Document results in EXP-001, EXP-004 baseline tables

**Why**: 
- Baselines enable regression detection
- Without initial measurement, thresholds are untested
- Establishes "known good" state

**Deliverable**:
- `kb/Baselines/latest-baseline.json` populated
- `kb/Baselines/trends.csv` first data point
- `kb/Experiments/EXP-001.md`: baseline measurements table filled
- `kb/Experiments/EXP-004.md`: baseline measurements table filled

---

## Near-Term (Next 1-2 Weeks)

### 4. **Create Pre-Release Validation Script** 🚀 Quality Gate
**Effort**: 45 min | **Impact**: Prevents releases with regressions

**What**:
Create `scripts/pre-release.sh`:
```bash
#!/bin/bash
set -euo pipefail

echo "=== Pre-Release Stability Check ==="

# Lint
echo "→ Lint & format"
just lint

# Unit tests
echo "→ Unit tests"
cargo test --all-targets

# Quick soak (100 iterations)
echo "→ Soak tests (100 iterations)"
SOAK_ITERATIONS=100 cargo test --test soak -- --include-ignored

# Benchmarks + regression check
echo "→ Benchmarks"
cargo bench -p converge-core --no-run
cargo bench -p converge-core -- --profile-time 10
python3 ops/scripts/extract-criterion-baseline.py

echo "=== Pre-Release Check Complete ==="
echo "Review: cat kb/Baselines/latest-summary.md"
```

**Why**: 
- Ensures no regressions reach released versions
- Single command for release process
- Documents what "ship quality" means

---

### 5. **Update CHANGELOG with Stability Framework** 📝 Visibility
**Effort**: 20 min | **Impact**: Communicates infrastructure work

**What**:
Add to `CHANGELOG.md` under v3.3.0 section:
```markdown
### Added
- **Stability Testing Framework** — Production-grade continuous validation
  - Criterion benchmarks for Engine latency (p50/p95/p99)
  - Chaos test suite (9 vectors: panic injection, malformed proposals, latency variance)
  - Soak tests for memory stability (RSS growth < 50MB per 100 cycles)
  - Property tests documenting Context ID validation gaps (evidence for typed-ID ADR)
  - Automated weekly stability workflow with regression detection
  - Performance baseline tracking (`kb/Baselines/trends.csv`)
  - Hypothesis-driven experiment framework (`kb/Experiments/EXP-*.md`)
  - Supply chain security gating (cargo audit blocking)
```

**Why**: 
- Signals to stakeholders that quality infrastructure is improving
- Justifies time investment to non-technical audiences

---

### 6. **Test Stability Workflow Manually** ✅ Verification
**Effort**: 2-3 min (local) + CI time | **Impact**: Confirms automation works

**What**:
1. Verify benchmarks compile: `cargo bench --workspace --no-run`
2. Verify chaos tests compile: `cargo test --test chaos --no-run`
3. Verify soak tests compile: `cargo test --test soak --no-run`
4. Run context properties: `cargo test --test context_properties`
5. Spot-check extraction script: `python3 ops/scripts/extract-criterion-baseline.py`

**Why**: 
- Catches any integration issues before weekly CI runs
- Validates Python extraction script
- Tests compile in CI environment

---

## Optional Enhancements (Backlog)

### 7. **Memory Profiling Dashboard** (Future)
- Track RSS over time in `trends.csv`
- Graph memory growth curve
- Alert on anomalies (e.g., 2x growth spike)

### 8. **jemalloc-ctl Integration** (Future)
- Per-allocation tracking (not just process RSS)
- Flame graphs showing allocation hot paths
- Per-test memory budgets

### 9. **Distributed Tracing** (Future)
- Opentelemetry spans for each Engine::run()
- Latency breakdown by phase (suggest → promote → add_fact)
- Trace export to Jaeger/Datadog

### 10. **ADR-005 Implementation** (Future)
- Once ADR is approved:
  - Implement `FactId` newtype in `converge-pack`
  - Migrate `Context` API to use `FactId`
  - Update all dependent crates
  - Run property tests; expect all to pass

---

## Decision: What's Your Priority?

**Recommended sequence** (balanced):
1. ✅ deny.toml (30 min, unblocks supply chain security)
2. ✅ ADR-005 draft (90 min, actionable outcome from evidence)
3. ✅ Initial baselines (15 min + time, enables regression detection)
4. ✅ Update CHANGELOG (20 min, visibility)
5. ✅ Pre-release script (45 min, quality gate)

**Total**: ~3 hours of focused work to close the loop on stability infrastructure.

**Or, if you prefer faster value**:
1. Start with initial baselines (data collection)
2. Draft ADR-005 immediately (act on evidence while fresh)
3. Let deny.toml be a separate small task

**Or, if focused on supply chain**:
1. deny.toml first (security)
2. Pre-release script (quality gate)
3. CHANGELOG update (communication)

---

## Files to Create/Update

| File | Action | Effort |
|------|--------|--------|
| `deny.toml` | Create | 30 min |
| `kb/Standards/ADR/ADR-005-typed-id.md` | Create | 90 min |
| `kb/Experiments/EXP-001.md` | Update (add baselines) | 5 min |
| `kb/Experiments/EXP-004.md` | Update (add baselines) | 5 min |
| `scripts/pre-release.sh` | Create | 45 min |
| `CHANGELOG.md` | Update | 20 min |
| `.github/workflows/security.yml` | Update (if deny.toml additions needed) | 10 min |

---

## Success Criteria

✅ **Immediate**: All 3 pillars (infrastructure, evidence, automation) operational  
✅ **Near-term**: Baselines established, ADR drafted, release process documented  
✅ **Outcome**: Convergence team runs stability tests weekly, acts on evidence

---

**What would you like to prioritize first?**
