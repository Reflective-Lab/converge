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

# Run a single test by name
test-one name:
    cargo test --all-targets -- {{name}}

# Run benchmarks (compile only)
bench:
    cargo bench --workspace --no-run

# Run benchmarks (with execution)
bench-run:
    cargo bench --workspace

# Run soak tests (long-running stability tests)
soak:
    cargo test --workspace -- --include-ignored soak

# ── Lint & Format ─────────────────────────────────────────────────────

# Check formatting and clippy
lint: _coverage-summary
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

# ── Supply Chain ───────────────────────────────────────────────────────

# Audit dependencies (requires cargo-deny)
deny:
    cargo deny check

# Audit advisories only
deny-advisories:
    cargo deny check advisories

# Validate repository security/compliance documentation
compliance-check:
    bash ops/scripts/validate-security-docs.sh

# Security regression gate for policy, runtime, and public control surfaces
security-gate:
    cargo check --workspace
    cargo test -p converge-policy
    cargo test -p converge-runtime --lib
    cargo test -p converge-pack --test compile_fail
    cargo test -p converge-core --test compile_fail --test truth_pipeline --test negative --test properties
    cargo test -p converge-client --test messages

INFRA_ENV := "ops/infra/environments/prod/converge-runtime"

# Start local runtime, preferring native Rust if available
dev-up mode="auto":
    bash ops/scripts/dev-up.sh {{mode}}

# Stop local runtime or compose stack
dev-down mode="auto":
    bash ops/scripts/dev-down.sh {{mode}}

# Smoke-test local runtime
smoke-test url="http://127.0.0.1:8080":
    bash ops/scripts/smoke-test.sh {{url}}

# Deploy runtime to Google Cloud Run
deploy-cloud-run:
    bash ops/scripts/deploy-cloud-run.sh

# ── Infrastructure ─────────────────────────────────────────────────────

# Create the remote Terraform state bucket
infra-bootstrap-state:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${PROJECT_ID:?Set PROJECT_ID}"
    : "${REGION:=europe-west1}"
    : "${TF_STATE_BUCKET:?Set TF_STATE_BUCKET}"
    gcloud storage buckets describe "gs://${TF_STATE_BUCKET}" --project "${PROJECT_ID}" >/dev/null 2>&1 || \
      gcloud storage buckets create "gs://${TF_STATE_BUCKET}" --location "${REGION}" --project "${PROJECT_ID}"

# Initialize Terraform for hosted deployment
infra-init:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${TF_STATE_BUCKET:?Set TF_STATE_BUCKET}"
    cd {{INFRA_ENV}} && terraform init -backend-config="bucket=${TF_STATE_BUCKET}"

# Preview Terraform changes
infra-plan:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${PROJECT_ID:?Set PROJECT_ID}"
    : "${REGION:=europe-west1}"
    cd {{INFRA_ENV}} && terraform plan \
      -var "project_id=${PROJECT_ID}" \
      -var "region=${REGION}"

# Apply Terraform changes
infra-apply:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${PROJECT_ID:?Set PROJECT_ID}"
    : "${REGION:=europe-west1}"
    cd {{INFRA_ENV}} && terraform apply \
      -var "project_id=${PROJECT_ID}" \
      -var "region=${REGION}"

# Show Terraform outputs
infra-output:
    cd {{INFRA_ENV}} && terraform output

# Build and push the runtime image using Cloud Build
cloud-build tag="latest":
    #!/usr/bin/env bash
    set -euo pipefail
    : "${PROJECT_ID:?Set PROJECT_ID}"
    REPO=$(cd {{INFRA_ENV}} && terraform output -raw registry_url)
    gcloud builds submit \
      --tag "${REPO}/converge-runtime:{{tag}}" \
      --project "${PROJECT_ID}"

# Apply Terraform with the selected runtime image tag
deploy-runtime tag="latest":
    #!/usr/bin/env bash
    set -euo pipefail
    : "${PROJECT_ID:?Set PROJECT_ID}"
    : "${REGION:=europe-west1}"
    cd {{INFRA_ENV}} && terraform apply \
      -var "project_id=${PROJECT_ID}" \
      -var "region=${REGION}" \
      -var "runtime_image_tag={{tag}}"

# Create a Secret Manager secret if missing
secret-create name:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${PROJECT_ID:?Set PROJECT_ID}"
    gcloud secrets describe {{name}} --project "${PROJECT_ID}" >/dev/null 2>&1 || \
      gcloud secrets create {{name}} --replication-policy="automatic" --project "${PROJECT_ID}"

# Add a new Secret Manager version from a local file
secret-put-file name path:
    #!/usr/bin/env bash
    set -euo pipefail
    : "${PROJECT_ID:?Set PROJECT_ID}"
    gcloud secrets versions add {{name}} \
      --data-file="{{path}}" \
      --project "${PROJECT_ID}"

# ── Examples ───────────────────────────────────────────────────────────

# Run an example (e.g., just example hello-convergence)
example name:
    cargo run -p example-{{name}}

# List all examples
examples:
    @echo "Available examples:"
    @ls -1 examples/ | grep -v README

# ── Git Workflow ───────────────────────────────────────────────────────

# Create a worktree for parallel work (e.g., just worktree fix-auth)
worktree branch:
    git worktree add ../converge-{{branch}} -b {{branch}}
    @echo "Worktree ready at ../converge-{{branch}}"
    @echo "When done: just worktree-rm {{branch}}"

# Remove a worktree
worktree-rm branch:
    git worktree remove ../converge-{{branch}}
    @echo "Worktree removed. Branch '{{branch}}' still exists — delete with: git branch -d {{branch}}"

# List active worktrees
worktrees:
    git worktree list

# ── jj (Jujutsu) Workflow ─────────────────────────────────────────────

# Show jj status
jj-status:
    jj status

# Create a new change
jj-new desc:
    jj new -m "{{desc}}"

# Show the change log
jj-log:
    jj log --limit 20

# Squash current change into parent
jj-squash:
    jj squash

# Push to git remote
jj-push:
    jj git push

# ── Clean ──────────────────────────────────────────────────────────────

# Remove build artifacts
clean:
    cargo clean

# ── Workflow ──────────────────────────────────────────────────────────

# Session opener — build + test health
focus:
    @cargo build --workspace
    @cargo test --workspace --lib -- --quiet
    @echo "✓ workspace healthy"

# Build health, test results
status:
    @cargo test --workspace -- --quiet 2>&1 | tail -5
    @echo "---"
    @git log --oneline -5

# ── Info ───────────────────────────────────────────────────────────────

# Show workspace crate dependency graph
deps:
    @echo "Dependency graph (leaf → root):"
    @echo "  converge-pack            (no internal deps)"
    @echo "  converge-provider-api    (no internal deps)"
    @echo "  converge-protocol        (no internal deps)"
    @echo "  converge-traits          -> pack, provider-api (compatibility only)"
    @echo "  converge-core            -> pack"
    @echo "  converge-model           -> core, pack"
    @echo "  converge-kernel          -> core, pack"
    @echo "  converge-client          -> protocol"
    @echo "  converge-provider        → core, pack, provider-api"
    @echo "  converge-domain          → core, provider"
    @echo "  converge-experience      → core"
    @echo "  converge-knowledge       (standalone, optional in app)"
    @echo "  ortools-sys              (no deps, FFI)"
    @echo "  converge-optimization    → ortools-sys (optional)"
    @echo "  converge-analytics       → core, domain, provider"
    @echo "  converge-llm             → core, domain, provider (optional)"
    @echo "  converge-policy          → core"
    @echo "  converge-axiom            → core, provider"
    @echo "  converge-remote          → client, protocol"
    @echo "  converge-runtime         → core, provider, protocol"
    @echo "  converge-application     → core, provider, domain, knowledge"
