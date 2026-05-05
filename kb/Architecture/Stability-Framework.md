---
name: Stability Framework Architecture
description: Production-grade stability testing pipeline with automated regression detection
type: architecture
source: mixed
---

# Stability Testing Framework

Converge stability testing transforms ad-hoc manual validation into systematic, recurring observation. The framework integrates performance baselines, chaos resilience, and memory profiling into a repeatable CI/CD pipeline.

## Framework Pillars

### 1. Performance Baselines (EXP-001, EXP-002, EXP-003)

**Purpose**: Establish latency floor and detect regressions

**Implementation**:
- **File**: `crates/core/benches/engine_bench.rs` (4 benchmarks)
- **Tool**: Criterion (JSON output to `target/criterion/`)
- **Metrics**: p50, p95, p99, mean, stddev (microseconds)
- **Extraction**: `extract-criterion-baseline.py` (moved to `runway` repo)
- **Storage**: `kb/Baselines/trends.csv` (historical tracking)

**Benchmarks**:
| Name | Suggestors | Purpose |
|------|-----------|---------|
| `engine_single_cycle` | 1 seed | Minimum latency (floor) |
| `engine_multi_suggestor/1` | 1 seed + 1 proposer | Single cascade |
| `engine_multi_suggestor/5` | 1 seed + 5 proposers | Light fan-out |
| `engine_multi_suggestor/20` | 1 seed + 20 proposers | Heavy fan-out |
| `engine_budget_pressure` | Infinite proposer, 99-cycle budget | Near-ceiling behavior |
| `engine_large_context` | 1000 pre-seeded facts + 1 proposer | Scaling with context size |

**Regression Thresholds**:
- **p99 latency**: Alert if > 1100µs or > baseline_mean × 2
- **Mean latency**: Alert if > baseline × 1.05 (5% margin)
- **StdDev**: Alert if > baseline × 1.20 (20% margin, variability increase)

### 2. Chaos Testing (EXP-005)

**Purpose**: Validate engine resilience under failure conditions

**Implementation**:
- **File**: `crates/core/tests/chaos.rs` (9 tests, 355 lines)
- **Gating**: `#[ignore = "chaos"]` (not in default suite)
- **Execution**: `cargo test --test chaos -- --include-ignored`

**Chaos Vectors**:
| Test | Condition | Expected Outcome |
|------|-----------|------------------|
| `chaos_panic_on_cycle_2_others_complete` | Panic injection on cycle 2 | Engine detects panic; 4 good seeds still process |
| `chaos_malformed_overconfident_proposal` | Confidence > 1.0 | Rejected gracefully |
| `chaos_malformed_null_byte_id` | ID contains `\0` | Rejected gracefully |
| `chaos_malformed_empty_id` | ID is empty string | Rejected gracefully |
| `chaos_malformed_whitespace_id` | ID is whitespace only | Rejected gracefully |
| `chaos_malformed_giant_content` | Content > 1MB | Rejected gracefully |
| `chaos_latency_variance_preserves_determinism` | 0ms vs 10ms injected latency | Same output both cases |
| `chaos_mixed_panic_and_malformed` | Panic + malformed proposals combined | Both handled independently |

**Resilience Score**: (tests_handled_gracefully / total_tests) × 100  
**Target**: > 85% (7/8 tests pass; 1 expected panic)

### 3. Soak Testing (Long-Running Stability)

**Purpose**: Validate stability across 100-10,000 cycles

**Implementation**:
- **File**: `crates/core/tests/soak.rs` (3 tests + memory profiling)
- **Gating**: `#[ignore = "soak"]`
- **Environment**: `SOAK_CYCLES`, `SOAK_CONCURRENCY`, `SOAK_ITERATIONS`, `SOAK_MEMORY_ITERATIONS`

**Soak Tests**:
| Test | Focus | Metric |
|------|-------|--------|
| `soak_engine_high_cycle_count` | Convergence stability | Cycles, fact count |
| `soak_concurrent_engine_runs` | Parallel interference | All runs converge, same output |
| `soak_throughput_latencies` | Latency under load | p50, p95, p99 distribution |
| `soak_memory_stability` | Memory leaks | RSS growth < 50MB per 100 iterations |

**Configuration**:
- **CI defaults**: 1000 cycles, 100 concurrency, 200 iterations (memory: 100)
- **Local profiling**: `SOAK_CYCLES=10000 SOAK_ITERATIONS=1000 cargo test --test soak -- --include-ignored`

### 4. Property-Based Validation (EXP-002)

**Purpose**: Discover validation gaps; generate evidence for ADRs

**Implementation**:
- **File**: `crates/core/tests/context_properties.rs` (9 tests)
- **Evidence tests**: Document current behavior that should be tightened

**Validation Gaps Documented**:
1. Empty IDs accepted (should reject)
2. Null bytes in IDs accepted (security gap)
3. Newlines in IDs accepted (escaping gap)
4. Unbounded ID length (should enforce max 128)
5. Whitespace-only IDs accepted (should reject)
6. Uppercase letters accepted (should enforce lowercase)

**Purpose**: These tests become acceptance criteria for ADR-006 (Typed-ID newtype)

### 5. Supply Chain Security

**Purpose**: Block vulnerable dependencies before merge

**Implementation**:
- **Tool**: `cargo audit` (blocking, no warnings allowed)
- **Tool**: `cargo deny` (license + advisory checking)
- **File**: `.github/workflows/security.yml` (runs on every push)
- **File**: `deny.toml` (allowlist + configuration)

**Gate**: Must pass before PR merge

### 6. Regression Gate (ADR-004 Enforcement)

**Purpose**: Prevent public API contract violations

**Implementation**:
- **Test suite**: `crates/core/tests/compile_fail.rs`
- **Tests**:
  - `converge-pack` proposal-only check
  - `converge-core::Fact` read-only validation
  - Kernel authority gating
  - Protocol observation-first guarantee

**Gate**: Runs in CI before test suite

## Execution Pipeline

### Automated (Scheduled)

**Weekly Stability Workflow** (`.github/workflows/stability.yml`)

Runs every Monday 6am UTC (`0 6 * * 1`):

```
regression-gate (ADR-004)
  ↓
bench-compile → bench-run (with extraction + regression check)
  ↓
soak (cycles=1000, concurrency=100, iterations=200, memory_iterations=100)
  ↓
live-endpoints (if secrets available)
  ↓
security-blocking (cargo audit --deny warnings)
  ↓
deny (cargo deny check)
```

**Output Artifacts**:
- `bench-reports/` — Criterion HTML results
- `baseline-data/` — Latest baseline JSON, summary markdown, trends CSV
- `soak-memory-logs/` — Memory profiling output (if captured)

### Manual (Developer)

**Single-Command Execution**:
```bash
# Run all stability checks (except soak, which takes ~2min)
just lint
cargo test --all-targets

# Run soak tests locally
SOAK_ITERATIONS=100 cargo test --test soak -- --include-ignored

# Profile memory with 1000 iterations
SOAK_MEMORY_ITERATIONS=1000 cargo test --test soak soak_memory_stability -- --include-ignored --nocapture

# Run chaos tests
cargo test --test chaos -- --include-ignored --nocapture

# Run benchmarks locally
cargo bench -p converge-core
```

**Regression Detection** (after benchmark):
```bash
python3 scripts/extract-criterion-baseline.py
cat kb/Baselines/latest-summary.md
```

## Experiment Tracking

Hypotheses are tracked in `kb/Experiments/` with evidence collected via automated runs:

| ID | Hypothesis | Evidence Source | Status |
|----|-----------|-----------------|--------|
| EXP-001 | p99 latency < 1ms (single-cycle) | `engine_bench.rs:bench_engine_single_cycle` | Running |
| EXP-002 | ID validation gaps exist; typed-ID needed | `context_properties.rs` (6 gap tests) | Running |
| EXP-004 | Memory stable under sustained load | `soak.rs:soak_memory_stability` | Running |
| EXP-005 | Engine resilient under chaos | `chaos.rs` (9 chaos vectors) | Running |

Each experiment has:
- **Hypothesis** — Falsifiable claim
- **Test case** — Code that validates hypothesis
- **Acceptance criteria** — Pass/fail thresholds
- **Evidence baseline** — Initial measurements
- **Related** — Links to dependent ADRs or future work

## Baseline Storage

**Location**: `kb/Baselines/`

**Files**:
- `README.md` — Format specification, regression thresholds
- `latest-baseline.json` — Most recent run (timestamp, run_id, metrics)
- `latest-summary.md` — Human-readable table
- `trends.csv` — Historical data (date, benchmark, p50/p95/p99/mean/stddev)

**Example Trend Entry**:
```csv
date,run_id,benchmark,p50_us,p95_us,p99_us,mean_us,std_dev_us
2026-04-17,20260417-123456,engine_single_cycle,123.45,145.67,167.89,125.00,12.34
```

**Regression Detection** (from CI):
```python
if p99 > max(mean * 2, 1100):
    alert(f"Regression: {benchmark} p99={p99:.2f}µs (threshold: {threshold:.2f}µs)")
```

## Architecture Decisions

### Why These Pillars?

1. **Baselines** — Catch latency regressions before they compound
2. **Chaos** — Prove the engine doesn't crash under realistic failures
3. **Soak** — Detect memory leaks, scaling issues, fatigue failures
4. **Properties** — Generate evidence for architectural decisions (typed-ID)
5. **Security** — Prevent supply chain attacks and dependency vulnerabilities
6. **Contract** — Enforce public API stability (ADR-004)

### Why Weekly, Not Every Commit?

Soak tests (1000 cycles) take ~2 minutes. Running on every PR would slow CI. Weekly schedule:
- Catches issues early (before release)
- Doesn't block development velocity
- Produces historical trends for regression detection
- Allows local developers to run selectively

### Why Separate Test Files?

- **chaos.rs** — Resilience focused; different from functional tests
- **soak.rs** — Long-running; requires explicit `--include-ignored`
- **context_properties.rs** — Evidence collection for ADRs; documents gaps
- **engine_bench.rs** — Performance floor; separate from functional tests

Separation allows selective execution and clear ownership.

## Integration with Development

### Local Developer Workflow

Before committing:
```bash
just lint                           # Formatting + clippy
cargo test                          # Unit + integration tests
```

Before pushing to main:
```bash
cargo test --all-targets            # Full test suite
SOAK_ITERATIONS=100 cargo test --test soak -- --include-ignored  # Quick soak
```

Before release:
```bash
./scripts/pre-release.sh            # Full stability check (bench + soak + chaos)
```

### CI Gates

1. **On push to main**: `cargo fmt --check`, `cargo clippy`, `cargo test`
2. **On PR**: Same as push + `cargo deny check`
3. **Weekly**: Full stability pipeline (bench + soak + chaos + security)

### Documentation

- `AGENTS.md` — Agent system architecture
- `kb/Architecture/Engine.md` — Core convergence loop
- `kb/Architecture/Stability-Framework.md` — This document
- `kb/Experiments/EXP-*.md` — Individual hypotheses

## Performance Targets

Based on EXP-001 and benchmarks:

| Metric | Target | Status |
|--------|--------|--------|
| Single-cycle p99 | < 1 ms | Pending baseline |
| Multi-suggestor (5) p99 | < 2 ms | Pending baseline |
| Multi-suggestor (20) p99 | < 5 ms | Pending baseline |
| Throughput p95 latency | < 500µs | Pending baseline |
| Memory growth (1000 runs) | < 50 MB | Pending baseline |
| Chaos resilience score | > 85% | All tests pass (expected) |
| Convergence success rate | 100% | All tests pass |

## Future Enhancements

1. **jemalloc-ctl integration** — Per-allocation memory tracking
2. **Distributed tracing** — Span-level latency breakdown
3. **Custom metrics** — Engine-specific histograms (fact count, cycle count)
4. **Flame graphs** — CPU profiling under load
5. **ADR-006 implementation** — Typed-ID newtype (evidence ready in EXP-002)
6. **deny.toml hardening** — Supply chain risk assessment

## References

- `.github/workflows/stability.yml` — Scheduled execution
- `.github/workflows/security.yml` — Security gating
- `crates/core/benches/engine_bench.rs` — Performance baselines
- `crates/core/tests/chaos.rs` — Chaos test suite
- `crates/core/tests/soak.rs` — Long-running stability tests
- `crates/core/tests/context_properties.rs` — Property tests + ADR evidence
- Baseline extraction script: moved to `runway` repo
- `kb/Baselines/README.md` — Baseline format specification
- `kb/Experiments/` — Hypothesis tracking and evidence
