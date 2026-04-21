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
    bad="$(find crates examples -type f \
        \( -name '*proptest*.rs' -o -name '*property*.rs' -o -name '*negative*.rs' \) \
        ! -path '*/src/*' ! -path '*/tests/*' -print)"
    if [ -n "${bad}" ]; then
        echo "Non-standard Rust test files must live under src/ or tests/:"
        echo "${bad}"
        exit 1
    fi

# Run the feature-gated WASM property suite explicitly
test-runtime-wasm:
    cargo test -p converge-runtime --features wasm-runtime wasm_property_tests

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

# Security regression gate for policy, runtime, and public control surfaces
sec-gate:
    cargo check --workspace
    cargo test -p converge-policy
    cargo test -p converge-runtime --lib
    cargo test -p converge-pack --test compile_fail
    cargo test -p converge-core --test compile_fail --test truth_pipeline --test negative --test properties
    cargo test -p converge-client --test messages

# Smoke-test local runtime
test-smoke url="http://127.0.0.1:8080":
    bash ops/scripts/smoke-test.sh {{url}}

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

# Open docs in browser
doc-open:
    cargo doc --no-deps --workspace --open

# ── Publish ────────────────────────────────────────────────────────────

# Publishable crates in dependency order
_publishable := "converge-pack converge-provider-api converge-core converge-policy converge-model converge-kernel converge-protocol converge-client converge-storage converge-provider converge-experience converge-knowledge ortools-sys converge-optimization converge-domain converge-analytics"

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
    bash ops/scripts/dev-up.sh {{mode}}

# Stop local runtime or compose stack
dev-down mode="auto":
    bash ops/scripts/dev-down.sh {{mode}}

# ── Examples ───────────────────────────────────────────────────────────

# Run an example (e.g., just example hello-convergence)
example name:
    cargo run -p example-{{name}}

# List all examples
examples:
    @echo "Available examples:"
    @ls -1 examples/ | grep -v README

# ── Git ────────────────────────────────────────────────────────────────

# Create a worktree for parallel work (e.g., just git-worktree fix-auth)
git-worktree branch:
    git worktree add ../converge-{{branch}} -b {{branch}}
    @echo "Worktree ready at ../converge-{{branch}}"
    @echo "When done: just git-worktree-rm {{branch}}"

# Remove a worktree
git-worktree-rm branch:
    git worktree remove ../converge-{{branch}}
    @echo "Worktree removed. Branch '{{branch}}' still exists — delete with: git branch -d {{branch}}"

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
git-status:
    @cargo test --workspace -- --quiet 2>&1 | tail -5
    @echo "---"
    @git log --oneline -5

# Repo state and recent commits
git-sync:
    @git status --short
    @echo "---"
    @git log --oneline -5

# ── Clean ──────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# ── Workflow ──────────────────────────────────────────────────────────

# Session opener — build + test health
wow-focus:
    @cargo build --workspace
    @cargo test --workspace --lib -- --quiet
    @echo "✓ workspace healthy"

# ── Info ───────────────────────────────────────────────────────────────

# Show workspace crate dependency graph
deps:
    @echo "Canonical public crates:"
    @echo "  converge-pack"
    @echo "  converge-provider-api"
    @echo "  converge-model"
    @echo "  converge-kernel"
    @echo "  converge-protocol"
    @echo "  converge-client"
    @echo "---"
    @echo "Internal workspace crates:"
    @echo "  converge-core"
    @echo "  converge-provider"
    @echo "  converge-domain"
    @echo "  converge-policy"
    @echo "  converge-optimization"
    @echo "  converge-analytics"
    @echo "  converge-knowledge"
    @echo "  converge-experience"
    @echo "  converge-runtime"
    @echo "  converge-storage"
    @echo "  ortools-sys"
