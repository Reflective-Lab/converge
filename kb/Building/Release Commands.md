---
source: llm
type: reference
---

# Release Commands

Canonical reference for the four release-grade gates introduced in v3.8.
Every command is idempotent, runs from a fresh checkout, and produces
artefacts under `target/` (or `kb/Baselines/`) suitable for archiving.

| Command | Purpose | Output | CI binding |
|---|---|---|---|
| `just security-audit` | Supply-chain audit (advisories, licenses, bans, sources) | `target/security/` | `.github/workflows/security.yml`, `stability.yml` |
| `just coverage` | Workspace line coverage | `target/coverage/` | `.github/workflows/coverage.yml` |
| `just performance-profile` | Criterion bench baseline / regression compare | `target/criterion/`, `kb/Baselines/` | `.github/workflows/stability.yml` |
| `just soak` | Bounded long-running stability validation | `target/soak/` | `.github/workflows/stability.yml` |

The four commands compose into the v3.8 release gate. They map onto four of
the six pillars in [[Architecture/Stability-Framework]] (security,
benchmarks, soak, plus a coverage rail that supplements the framework).

## `just security-audit`

**Purpose.** Blocking pre-release supply-chain check.

**Command.**
```bash
just security-audit
```

**What it runs.**
- `cargo audit` (RUSTSEC advisories) — JSON to `target/security/audit.json`,
  human form appended to `target/security/summary.txt`.
- `cargo deny check` (advisories + licenses + bans + sources) — output to
  `target/security/deny.txt` and the same summary.

The `--ignore` list mirrors `.github/workflows/security.yml`. When that
list changes, update both in lockstep.

**Output.**
```
target/security/audit.json     cargo-audit JSON report
target/security/deny.txt       cargo-deny human report
target/security/summary.txt    combined human summary (the file you read)
```

**Baseline policy.** No saved baseline; the gate is "no new findings
beyond the ignore list". Adding to the ignore list requires an explanation
in `deny.toml` (advisory crate path, why it's transitive-only, owner).

**CI binding.** `security` workflow (PR + push + dispatch) and
`stability` workflow (weekly Monday) both invoke `cargo audit`/`cargo deny`
directly. The `just security-audit` recipe is the single source of truth
for the flag set; CI mirrors it.

**Tools required.** `cargo-audit`, `cargo-deny`. Both installed via
`cargo install` or `taiki-e/install-action` in CI.

## `just coverage`

**Purpose.** Workspace line coverage with three complementary artefacts.

**Command.**
```bash
just coverage
```

**What it runs.** `cargo llvm-cov` over the workspace (excluding
`converge-runtime`, matching CI), then renders three reports off the same
profraw data so JSON / LCOV / HTML stay consistent.

**Output.**
```
target/coverage/converge-coverage.json   summary JSON (used by the badge job)
target/coverage/lcov.info                LCOV (codecov, sonar, IDE plugins)
target/coverage/html/index.html          browsable HTML report
```

**Baseline policy.** No hard floor enforced locally. The badge job in
`.github/workflows/coverage.yml` publishes the percent to the dynamic
badge gist; trend is tracked there. Future floor will land in
`kb/Baselines/` once a reliable threshold is set.

**CI binding.** `.github/workflows/coverage.yml` runs `coverage-ci`
(legacy recipe that writes `coverage.json` at repo root for the badge
extractor). Local development uses `just coverage`.

**Tools required.** `cargo-llvm-cov`, `llvm-tools-preview` rustup
component. No root, no sudo, no tarpaulin.

## `just performance-profile`

**Purpose.** Capture Criterion baselines for the v3.8 release; on
subsequent runs, compare against the saved baseline.

**Command.**
```bash
just performance-profile                       # default baseline name "v3.8.0"
PERF_BASELINE=v3.8.1 just performance-profile  # save / compare a new name
```

**What it runs.** Criterion benches in the crates that ship them
(`converge-core`, `converge-optimization`). On the first run for a given
`PERF_BASELINE`, it `--save-baseline`s that name. On every subsequent run
it `--baseline` compares against it, so regressions show up in Criterion's
own diff output.

After the run, `scripts/extract-criterion-baseline.py` writes
`kb/Baselines/latest-baseline.json` and `latest-summary.md` for the
stability workflow regression check.

**Output.**
```
target/criterion/                          per-bench HTML + raw data
kb/Baselines/latest-baseline.json          extracted summary
kb/Baselines/latest-summary.md             human report
```

**Baseline policy.** `v3.8.0` is the release baseline; do not overwrite
without an ADR or release-note entry. Push a new `PERF_BASELINE` for each
release. The regression rule in `stability.yml` flags p99 > `max(mean × 2,
1100µs)`.

**CI binding.** `.github/workflows/stability.yml :: bench-run` runs the
benches weekly and uploads `target/criterion/` and `kb/Baselines/` as
artifacts.

**Tools required.** Criterion is a workspace dev-dep already (`crates/core`,
`crates/optimization`). No extra installs.

## `just soak`

**Purpose.** Time-bounded long-running stability run for v3.8 release
gating. Validates heap stability, concurrent execution, and latency
consistency at scale.

**Command.**
```bash
just soak                                # default 5 min
SOAK_DURATION_MIN=15 just soak           # extended run
```

**What it runs.** The existing `crates/core/tests/soak.rs` suite
(`#[ignore = "soak"]`-gated tests) with `SOAK_CYCLES`, `SOAK_CONCURRENCY`,
`SOAK_ITERATIONS` scaled from the wall-clock budget so the run is
bounded and reproducible. 5 min ≈ CI grade
(cycles=1000, concurrency=100, iterations=200).

**Output.**
```
target/soak/soak-<UTC>.log     full nocapture log of this run
target/soak/latest.log         symlink → most recent log
```

**Baseline policy.** Pass/fail is asserted inside the soak tests
themselves (heap growth bounds, latency stability). The log is the
artefact; archive it on release.

**CI binding.** `.github/workflows/stability.yml :: soak` runs the same
tests weekly with the CI-grade env vars.

**Tools required.** None beyond cargo + tokio (already workspace deps).

## Reproducing locally before a release

```bash
just security-audit
just coverage
just performance-profile
SOAK_DURATION_MIN=5 just soak
```

Archive the contents of `target/security/`, `target/coverage/`,
`target/criterion/`, `target/soak/`, and `kb/Baselines/` alongside the
release tag.
