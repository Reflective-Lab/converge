---
name: Stability Framework - Next Steps
description: Prioritized work to operationalize and extend the stability testing pipeline
type: planning
source: mixed
---

# Next Steps: Stability Framework Operationalization

The **6-pillar stability framework** is now implemented and committed. This document prioritizes the next work to operationalize it and drive actionable outcomes from evidence collection.

## Immediate (This Sprint)

### 1. **Establish Initial Baselines from Stability Run** 📊 Data Collection
**Effort**: 15 min (local) + CI time | **Impact**: Enables regression detection  
**Status**: Ready to execute

**What**:
1. Run benchmarks locally: `cargo bench -p converge-core`
2. Extract baseline: `python3 runway/ops/scripts/extract-criterion-baseline.py`
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

### 2. **Create Pre-Release Validation Script** 🚀 Quality Gate
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
python3 runway/ops/scripts/extract-criterion-baseline.py

echo "=== Pre-Release Check Complete ==="
echo "Review: cat kb/Baselines/latest-summary.md"
```

**Why**: 
- Ensures no regressions reach released versions
- Single command for release process
- Documents what "ship quality" means

---

### 3. **Update CHANGELOG with Stability Framework** 📝 Visibility
**Effort**: 20 min | **Impact**: Communicates infrastructure work

**What**:
Add to `CHANGELOG.md` under v3.3.x or Unreleased section:
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

### 4. **Test Stability Workflow Manually** ✅ Verification
**Effort**: 2-3 min (local) + CI time | **Impact**: Confirms automation works

**What**:
1. Verify benchmarks compile: `cargo bench --workspace --no-run`
2. Verify chaos tests compile: `cargo test --test chaos --no-run`
3. Verify soak tests compile: `cargo test --test soak --no-run`
4. Run context properties: `cargo test --test context_properties`
5. Spot-check extraction script: `python3 runway/ops/scripts/extract-criterion-baseline.py`

**Why**: 
- Catches any integration issues before weekly CI runs
- Validates Python extraction script
- Tests compile in CI environment

---

## Optional Enhancements (Backlog)

### 5. **Memory Profiling Dashboard** (Future)
- Track RSS over time in `trends.csv`
- Graph memory growth curve
- Alert on anomalies (e.g., 2x growth spike)

### 6. **jemalloc-ctl Integration** (Future)
- Per-allocation tracking (not just process RSS)
- Flame graphs showing allocation hot paths
- Per-test memory budgets

### 7. **Distributed Tracing** (Future)
- Opentelemetry spans for each Engine::run()
- Latency breakdown by phase (suggest → promote → add_fact)
- Trace export to Jaeger/Datadog

### 8. **ADR-006 Implementation** (Future)
- Once ADR is approved:
  - Implement `FactId` newtype in `converge-pack`
  - Migrate `Context` API to use `FactId`
  - Update all dependent crates
  - Run property tests; expect all to pass

---

## Decision: What's Your Priority?

**Recommended sequence** (balanced):
1. ✅ Initial baselines (15 min + time, enables regression detection)
2. ✅ CHANGELOG update (20 min, visibility)
3. ✅ Pre-release script (45 min, quality gate)
4. ✅ Manual verification (5 min, integration check)

**Total**: ~1.5 hours of focused work to close the loop on stability infrastructure.

---

## Files to Create/Update

| File | Action | Effort |
|------|--------|--------|
| `kb/Baselines/latest-baseline.json` | Populate (auto from bench run) | 5 min |
| `kb/Baselines/trends.csv` | Append first data point | 2 min |
| `kb/Experiments/EXP-001.md` | Update baseline table | 5 min |
| `kb/Experiments/EXP-004.md` | Update baseline table | 5 min |
| `scripts/pre-release.sh` | Create | 45 min |
| `CHANGELOG.md` | Update | 20 min |

---

## Success Criteria

✅ **Immediate**: All infrastructure operational with real baseline data  
✅ **Near-term**: Release process documented, pre-release check automated  
✅ **Outcome**: Team runs stability tests weekly, acts on evidence

---

**What would you like to prioritize first?**
