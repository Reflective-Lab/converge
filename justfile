# Converge Agent OS — Development Commands
# Install: brew install just  |  cargo install just
# Usage:   just --list

set dotenv-load := true

# ── Build ──────────────────────────────────────────────────────────────

# Build workspace (release)
build:
    cargo build --release

# Build workspace (fast iteration)
build-quick:
    cargo build --profile quick-release

# Build for CI
build-ci:
    cargo build --workspace --profile ci

# Check workspace without producing release artifacts
check:
    cargo check --workspace

# ── Test ───────────────────────────────────────────────────────────────

# Run tests (default members)
test:
    cargo test --all-targets

# Run all tests including analytics, llm, runtime
test-all:
    cargo test --all-targets --workspace

# Run tests for a specific crate
test-crate crate:
    cargo test -p {{crate}} --all-targets

# Guard test file placement so Rust test files do not live in dead ad hoc directories
test-layout:
    #!/usr/bin/env bash
    set -euo pipefail
    bad="$(find crates -type f \
        \( -name '*proptest*.rs' -o -name '*property*.rs' -o -name '*negative*.rs' \) \
        ! -path '*/src/*' ! -path '*/tests/*' -print)"
    if [ -n "${bad}" ]; then
        echo "Non-standard Rust test files must live under src/ or tests/:"
        echo "${bad}"
        exit 1
    fi

# Run a single test by name
test-one name:
    cargo test --all-targets -- {{name}}

# Run benchmarks (compile only)
test-bench:
    cargo bench --workspace --no-run

# Run benchmarks (with execution)
test-bench-run:
    cargo bench --workspace

# Run soak tests (long-running stability tests)
test-soak:
    cargo test --workspace -- --include-ignored soak

# Run quick local soak tests
test-soak-quick:
    SOAK_CYCLES=100 SOAK_CONCURRENCY=20 SOAK_ITERATIONS=50 cargo test -p converge-core --test soak -- --include-ignored soak --nocapture

# Run CI-grade soak tests
test-soak-ci:
    SOAK_CYCLES=1000 SOAK_CONCURRENCY=100 SOAK_ITERATIONS=200 cargo test -p converge-core --test soak -- --include-ignored soak --nocapture

# Bounded release-grade soak run.
#
# Purpose: long-running stability validation for v3.8 release gating.
# Scales SOAK_* counts from a target wall-clock budget so the run is
# bounded and reproducible across machines.
#
# Configure with SOAK_DURATION_MIN (default 5). Output:
#   target/soak/soak-<UTC>.log   (full nocapture log)
#   target/soak/latest.log       (symlink to most recent run)
soak:
    #!/usr/bin/env bash
    set -euo pipefail
    duration_min="${SOAK_DURATION_MIN:-5}"
    out_dir="target/soak"
    mkdir -p "${out_dir}"
    stamp="$(date -u +%Y%m%dT%H%M%SZ)"
    log="${out_dir}/soak-${stamp}.log"
    # Heuristic scaling: 5 min ~ CI-grade (cycles=1000, concurrency=100, iter=200).
    # Linearly scale cycles & iterations to duration; concurrency is fixed.
    cycles=$(awk -v d="${duration_min}" 'BEGIN { printf "%d", 200 * d }')
    iterations=$(awk -v d="${duration_min}" 'BEGIN { printf "%d", 40 * d }')
    concurrency=100
    echo "soak: duration=${duration_min}min cycles=${cycles} concurrency=${concurrency} iterations=${iterations}" | tee "${log}"
    SOAK_CYCLES="${cycles}" \
    SOAK_CONCURRENCY="${concurrency}" \
    SOAK_ITERATIONS="${iterations}" \
    cargo test -p converge-core --test soak -- --include-ignored soak --nocapture 2>&1 | tee -a "${log}"
    ln -sf "soak-${stamp}.log" "${out_dir}/latest.log"
    echo "soak: log → ${log}"

# Security regression gate for policy, runtime, and public control surfaces
sec-gate:
    cargo check --workspace
    cargo test -p converge-runtime --lib
    cargo test -p converge-pack --test compile_fail
    cargo test -p converge-core --test compile_fail --test truth_pipeline --test negative --test properties
    cargo test -p converge-client --test messages

# Alias used by SECURITY.md and release checklists
security-gate: sec-gate

# Release-grade dependency security audit.
#
# Purpose: blocking pre-release supply-chain check.
# Runs cargo-audit (RUSTSEC advisories) plus cargo-deny
# (advisories, licenses, bans, sources).
# Output:
#   target/security/audit.json   (cargo-audit JSON)
#   target/security/deny.txt     (cargo-deny human report)
#   target/security/summary.txt  (combined human summary)
security-audit:
    #!/usr/bin/env bash
    set -uo pipefail
    out_dir="target/security"
    mkdir -p "${out_dir}"
    summary="${out_dir}/summary.txt"
    : > "${summary}"
    echo "── cargo-audit ──────────────────────────────" | tee -a "${summary}"
    cargo audit --json \
        --ignore RUSTSEC-2023-0089 \
        --ignore RUSTSEC-2024-0384 \
        --ignore RUSTSEC-2024-0436 \
        --ignore RUSTSEC-2025-0012 \
        --ignore RUSTSEC-2025-0134 \
        --ignore RUSTSEC-2025-0141 \
        > "${out_dir}/audit.json" || true
    cargo audit --deny warnings \
        --ignore RUSTSEC-2023-0089 \
        --ignore RUSTSEC-2024-0384 \
        --ignore RUSTSEC-2024-0436 \
        --ignore RUSTSEC-2025-0012 \
        --ignore RUSTSEC-2025-0134 \
        --ignore RUSTSEC-2025-0141 \
        2>&1 | tee -a "${summary}"
    audit_human_status=${PIPESTATUS[0]}
    echo "" | tee -a "${summary}"
    echo "── cargo-deny ───────────────────────────────" | tee -a "${summary}"
    cargo deny check 2>&1 | tee "${out_dir}/deny.txt" | tee -a "${summary}"
    deny_status=${PIPESTATUS[0]}
    echo "" | tee -a "${summary}"
    echo "audit→${out_dir}/audit.json  deny→${out_dir}/deny.txt  summary→${summary}"
    if [ "${audit_human_status}" -ne 0 ] || [ "${deny_status}" -ne 0 ]; then
        exit 1
    fi

# Smoke-test local runtime
test-smoke url="http://127.0.0.1:8080":
    bash scripts/smoke-test.sh {{url}}

# ── Lint & Format ─────────────────────────────────────────────────────

# Check formatting, clippy, and test layout hygiene
lint: _coverage-summary test-layout
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings

# Auto-fix lint issues
fix-lint:
    cargo clippy --fix --allow-staged --allow-dirty --allow-no-vcs
    cargo fmt

# Format only
fmt:
    cargo fmt

# Show test coverage by crate (unit tests per source file)
_coverage-summary:
    #!/usr/bin/env bash
    echo "──────────────────────────────────────────────"
    echo "Test Coverage Summary"
    echo "──────────────────────────────────────────────"
    (
      for crate in crates/*/; do
        crate_name=$(basename "$crate")
        unit_count=$(find "$crate/src" -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; 2>/dev/null | wc -l)
        integration_count=$(ls "$crate/tests"/*.rs 2>/dev/null | wc -l)
        bench_count=$(ls "$crate/benches"/*.rs 2>/dev/null | wc -l)
        src_files=$(find "$crate/src" -name "*.rs" 2>/dev/null | wc -l)

        if [ "$unit_count" -gt 0 ] || [ "$integration_count" -gt 0 ] || [ "$bench_count" -gt 0 ]; then
          printf "%-25s unit=%2d integration=%d bench=%d (src: %d files)\n" \
            "$crate_name" "$unit_count" "$integration_count" "$bench_count" "$src_files"
        fi
      done | sort -t= -k2 -rn
    )
    echo "──────────────────────────────────────────────"
    crates_with_tests=$(find crates -path "*/tests/*.rs" -o -path "*/benches/*.rs" | cut -d/ -f2 | sort -u | wc -l)
    total_crates=$(ls -1d crates/*/ | wc -l)
    echo "Test coverage: $crates_with_tests/$total_crates crates have tests"
    echo "──────────────────────────────────────────────"

# ── Docs ───────────────────────────────────────────────────────────────

# Generate workspace docs
doc:
    cargo doc --no-deps --workspace

# Release-grade workspace coverage report.
#
# Purpose: LCOV + HTML + summary JSON for v3.8 release gating.
# Runs cargo-llvm-cov over the workspace (excludes runtime/example crates,
# matching CI). No root/sudo required. Output:
#   target/coverage/converge-coverage.json  (machine-readable summary)
#   target/coverage/lcov.info               (LCOV for codecov/sonar)
#   target/coverage/html/index.html         (browsable report)
coverage:
    #!/usr/bin/env bash
    set -euo pipefail
    out_dir="target/coverage"
    mkdir -p "${out_dir}/html"
    common=(--workspace --exclude converge-runtime --lib --tests
        --ignore-filename-regex '(^|/)(tests|benches|examples)/')
    cargo llvm-cov clean --workspace
    # Drop trybuild scratch — it pins absolute paths to crates that may
    # have moved (e.g. atelier/prism/arbiter extractions). Trybuild
    # regenerates these on next run.
    rm -rf target/tests/trybuild
    # Collect once, render from the same data. The --exclude applies during
    # collection; `report` reads the already-filtered profraw set.
    cargo llvm-cov "${common[@]}" --no-report
    cargo llvm-cov report \
        --json --summary-only --output-path "${out_dir}/converge-coverage.json"
    cargo llvm-cov report \
        --lcov --output-path "${out_dir}/lcov.info"
    cargo llvm-cov report \
        --html --output-dir "${out_dir}/html"
    pct=$(python3 -c "import json; d=json.load(open('${out_dir}/converge-coverage.json')); print(f\"{d['data'][0]['totals']['lines']['percent']:.1f}\")")
    echo "coverage: ${pct}%  json→${out_dir}/converge-coverage.json  lcov→${out_dir}/lcov.info  html→${out_dir}/html/index.html"

# Generate CI coverage JSON at repo root (legacy path used by coverage.yml)
coverage-ci:
    cargo llvm-cov --workspace --exclude converge-runtime --lib --tests --ignore-filename-regex '(^|/)(tests|benches|examples)/' --json --summary-only --output-path coverage.json

# Release-grade Criterion benchmark profile.
#
# Purpose: capture a comparable v3.8 perf baseline.
# Runs Criterion across crates that ship benches (core, optimization).
# Saves a baseline named PERF_BASELINE (default "v3.8.0"). Set
# PERF_MODE=compare to compare against an existing baseline instead.
# Output:
#   target/criterion/                       (per-bench HTML + raw data)
#   kb/Baselines/latest-baseline.json       (extracted summary)
#   kb/Baselines/latest-summary.md          (human report)
performance-profile:
    #!/usr/bin/env bash
    set -euo pipefail
    name="${PERF_BASELINE:-v3.8.0}"
    benches=(
        "converge-core:engine_bench:"
        "converge-optimization:assignment:"
        "converge-optimization:graph:"
        "converge-optimization:cp_comparison:sat"
    )
    mode="${PERF_MODE:-save}"
    case "${mode}" in
        save) mode_flag="--save-baseline" ;;
        compare) mode_flag="--baseline" ;;
        *)
            echo "PERF_MODE must be 'save' or 'compare' (got '${mode}')" >&2
            exit 2
            ;;
    esac
    echo "performance-profile: ${mode_flag} ${name}"
    for target in "${benches[@]}"; do
        IFS=":" read -r crate bench features <<< "${target}"
        echo "── ${crate}/${bench} ──"
        if [ -n "${features}" ]; then
            cargo bench -p "${crate}" --features "${features}" --bench "${bench}" -- "${mode_flag}" "${name}"
        else
            cargo bench -p "${crate}" --bench "${bench}" -- "${mode_flag}" "${name}"
        fi
    done
    if [ -f scripts/extract-criterion-baseline.py ]; then
        python3 scripts/extract-criterion-baseline.py || \
            echo "warn: baseline extraction failed (non-fatal)"
    fi
    echo "performance-profile: criterion→target/criterion/  summary→kb/Baselines/latest-summary.md"

# Legacy aliases (pre-v3.8 names) — keep until consumers migrate
perf-baseline: performance-profile
perf-profile:
    cargo bench -p converge-core -- --profile-time 10

# Open docs in browser
doc-open:
    cargo doc --no-deps --workspace --open

# ── Publish ────────────────────────────────────────────────────────────

# Publishable crates in dependency order
_publishable := "converge-pack converge-provider converge-protocol converge-core converge-model converge-storage converge-experience converge-optimization converge-kernel converge-client"

# Dry-run publish to crates.io (validates readiness)
publish-dry-run:
    #!/usr/bin/env bash
    set -euo pipefail
    for crate in {{_publishable}}; do
        echo "--- dry-run: $crate ---"
        cargo publish --dry-run -p "$crate"
    done

# ── Security ───────────────────────────────────────────────────────────

# Audit dependencies (requires cargo-deny)
sec-deny:
    cargo deny check

# Audit advisories only
sec-deny-advisories:
    cargo deny check advisories

# ── Local Dev ──────────────────────────────────────────────────────────

# Start local runtime, preferring native Rust if available
dev-up mode="auto":
    bash scripts/dev-up.sh {{mode}}

# Stop local runtime or compose stack
dev-down mode="auto":
    bash scripts/dev-down.sh {{mode}}

# ── Git ────────────────────────────────────────────────────────────────

# List active worktrees
git-worktrees:
    git worktree list

# Report branch/worktree/release hygiene and remote cleanup candidates
git-hygiene:
    #!/usr/bin/env bash
    set -euo pipefail

    current_branch="$(git branch --show-current 2>/dev/null || true)"
    current_sha="$(git rev-parse --short HEAD)"
    latest_tag="$(git tag --sort=-creatordate --list 'v*' | head -n1 || true)"

    echo "──────────────────────────────────────────────"
    echo "Git Hygiene"
    echo "──────────────────────────────────────────────"
    printf "branch: %s\n" "${current_branch:-DETACHED}"
    printf "head:   %s\n" "${current_sha}"
    if [ -n "${latest_tag}" ]; then
        tag_sha="$(git rev-list -n 1 "${latest_tag}")"
        since_tag="$(git rev-list --count "${latest_tag}..HEAD")"
        printf "latest release tag: %s (%s)\n" "${latest_tag}" "$(git rev-parse --short "${tag_sha}")"
        printf "commits since tag:  %s\n" "${since_tag}"
    else
        echo "latest release tag: none"
    fi

    echo
    echo "Worktrees"
    echo "─────────"
    git worktree list

    echo
    echo "Local Branches"
    echo "──────────────"
    git branch -vv

    echo
    echo "Working Tree"
    echo "────────────"
    git status --short --branch

    if git show-ref --verify --quiet refs/remotes/origin/main; then
        echo
        echo "Merged Remote Branches (safe delete candidates)"
        echo "──────────────────────────────────────────────"
        merged=0
        while IFS= read -r branch; do
            [ -z "${branch}" ] && continue
            case "${branch}" in
                origin|origin/HEAD|origin/main) continue ;;
            esac
            if git merge-base --is-ancestor "${branch}" origin/main; then
                printf "%s\t%s\t%s\n" \
                    "${branch}" \
                    "$(git for-each-ref --format='%(committerdate:short)' "refs/remotes/${branch}")" \
                    "$(git log -1 --format=%s "${branch}")"
                merged=1
            fi
        done < <(git for-each-ref --format='%(refname:short)' refs/remotes/origin)
        if [ "${merged}" -eq 0 ]; then
            echo "none"
        fi

        echo
        echo "Unmerged Remote Branches (review or recreate)"
        echo "─────────────────────────────────────────────"
        unmerged=0
        while IFS= read -r branch; do
            [ -z "${branch}" ] && continue
            case "${branch}" in
                origin|origin/HEAD|origin/main) continue ;;
            esac
            if ! git merge-base --is-ancestor "${branch}" origin/main; then
                counts="$(git rev-list --left-right --count "${branch}...origin/main")"
                read -r ahead behind <<< "${counts}"
                printf "%s\tahead=%s\tbehind=%s\t%s\t%s\n" \
                    "${branch}" \
                    "${ahead}" \
                    "${behind}" \
                    "$(git for-each-ref --format='%(committerdate:short)' "refs/remotes/${branch}")" \
                    "$(git log -1 --format=%s "${branch}")"
                unmerged=1
            fi
        done < <(git for-each-ref --format='%(refname:short)' refs/remotes/origin)
        if [ "${unmerged}" -eq 0 ]; then
            echo "none"
        fi
    fi

# Build health and recent commits
status:
    @cargo test --workspace -- --quiet 2>&1 | tail -5
    @echo "---"
    @git log --oneline -5

# Repo state and recent commits
sync:
    @git status --short
    @echo "---"
    @git log --oneline -5

# ── Clean ──────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# ── Workflow ──────────────────────────────────────────────────────────

# Session opener — build + test health
focus:
    @just sync
    @echo "---"
    @cargo build --workspace
    @cargo test --workspace --lib -- --quiet
    @echo "✓ workspace healthy"

# Legacy aliases retained while docs and habits converge on focus/sync/status
wow-focus: focus
git-sync: sync
git-status: status

# Audit release artifact sizes for the lean-packaging milestone
size-audit:
    #!/usr/bin/env bash
    set -euo pipefail

    target_dir="${CARGO_TARGET_DIR:-/tmp/converge-size-audit}"
    runtime_bin="${target_dir}/release/converge-runtime"

    mib() {
        awk -v size="$1" 'BEGIN { printf "%.2f", size / (1024 * 1024) }'
    }

    dep_count() {
        cargo tree -p converge-runtime "$@" --prefix none | sort -u | wc -l | tr -d ' '
    }

    dir_size() {
        du -sh "$1" 2>/dev/null | awk '{print $1}'
    }

    runtime_size() {
        local label="$1"
        shift
        local start="${SECONDS}"
        set +e
        cargo build -p converge-runtime --release --target-dir "${target_dir}" "$@" >/dev/null
        local status="$?"
        set -e
        local elapsed="$((SECONDS - start))s"
        if [ "${status}" -ne 0 ]; then
            printf "%-10s %12s          %7s\n" "${label}" "build-fail" "${elapsed}"
            return 0
        fi
        local size
        size="$(wc -c < "${runtime_bin}")"
        printf "%-10s %12s bytes  %7s MiB  %7s\n" "${label}" "${size}" "$(mib "${size}")" "${elapsed}"
    }

    echo "──────────────────────────────────────────────"
    echo "Lean Packaging Audit"
    echo "──────────────────────────────────────────────"
    echo "target dir: ${target_dir}"
    echo
    echo "converge-runtime"
    echo "────────────────"
    runtime_size "minimal" --no-default-features
    runtime_size "standard"
    runtime_size "full" --all-features
    echo
    echo "dependency graph"
    echo "────────────────"
    printf "%-10s %12s unique lines\n" "minimal" "$(dep_count --no-default-features)"
    printf "%-10s %12s unique lines\n" "standard" "$(dep_count)"
    printf "%-10s %12s unique lines\n" "full" "$(dep_count --all-features)"

    echo
    echo "converge-kernel"
    echo "───────────────"
    kernel_start="${SECONDS}"
    cargo build -p converge-kernel --release --lib --target-dir "${target_dir}" >/dev/null
    kernel_elapsed="$((SECONDS - kernel_start))s"
    kernel_rlib="$(find "${target_dir}/release/deps" -name 'libconverge_kernel-*.rlib' | sort | tail -n1)"
    if [ -z "${kernel_rlib}" ]; then
        echo "kernel artifact not found"
        exit 1
    fi
    kernel_size="$(wc -c < "${kernel_rlib}")"
    printf "%-10s %12s bytes  %7s MiB  %7s\n" "release" "${kernel_size}" "$(mib "${kernel_size}")" "${kernel_elapsed}"
    printf "%-10s %12s unique lines\n" "deps" "$(cargo tree -p converge-kernel --prefix none | sort -u | wc -l | tr -d ' ')"

    echo
    echo "disk"
    echo "────"
    printf "%-14s %s\n" "audit target" "$(dir_size "${target_dir}")"
    printf "%-14s %s\n" "release deps" "$(dir_size "${target_dir}/release/deps")"
    if [ -d target ]; then
        printf "%-14s %s\n" "workspace target" "$(dir_size target)"
    fi

# ── Info ───────────────────────────────────────────────────────────────

# Show workspace crate dependency graph
deps:
    @echo "Canonical public crates:"
    @echo "  converge-pack"
    @echo "  converge-provider"
    @echo "  converge-model"
    @echo "  converge-kernel"
    @echo "  converge-protocol"
    @echo "  converge-client"
    @echo "---"
    @echo "Internal workspace crates:"
    @echo "  converge-core"
    @echo "  converge-optimization"
    @echo "  converge-experience"
    @echo "  converge-runtime"
    @echo "  converge-storage"
