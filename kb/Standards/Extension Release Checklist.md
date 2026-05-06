---
name: Extension Release Checklist
description: Canonical engineering standard every Converge extension repo must meet before a release.
type: standard
source: mixed
date: 2026-05-06
---
# Extension Release Checklist

The seven extension repos under `~/dev/extensions/*` (atelier, mnemos, prism,
arbiter, embassy, ferrox, manifold) live outside the foundation but
participate in the same release. This page is the **single source of truth**
for the engineering habits each one must demonstrate before it ships
alongside Converge.

If a future Converge release tightens a standard, the rule is updated here
first. Extensions sync.

## Why this exists

Foundation owns universal contracts. Extensions own implementations.
Foundation cannot guarantee the quality of implementations it does not
control — but it *can* publish the bar, and it *can* refuse to certify a
release where an extension has not cleared the bar. This checklist is that
bar.

The cost of not enforcing it is exactly what we paid in v3.7: silent drift,
stale crate names, untested solver packs, leaked vendor types, and CI
green-but-meaningless because the gates were never wired.

## The eight pillars

Every extension repo must satisfy all eight before tagging a release.

### 1. Surface hygiene

- One canonical public crate per repo (the one users `Cargo.toml` against).
  All other crates have `publish = false`.
- Adapter-qualified names per
  [[Architecture/ADRs/ADR-007-provider-tool-contracts]]: the contract gets
  the clean name; the implementation carries the qualifier
  (`mnemos-lance`, `prism-burn`, `arbiter-cedar`, …).
- No re-exports of foundation internals. Public types come from
  `converge-pack`, `converge-model`, or `converge-provider` only.
- Workspace-level `[lints] workspace = true` in every crate; `unsafe`
  forbidden.
- Edition 2024, rust-version pinned to the same MSRV as foundation
  (currently 1.94).

### 2. Compile gates

All four green on PR + push, no exceptions:

```bash
cargo check --workspace
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Wire as `just check`, `just lint`, `just test` so contributors don't have to
remember the flag set.

### 3. The four release-grade gates

These are the same four that gate the foundation release. Each writes
archivable artefacts under `target/` or `kb/Baselines/` and is idempotent.

| Gate | Recipe | Output | CI |
|---|---|---|---|
| Supply chain | `just security-audit` | `target/security/` | `.github/workflows/security.yml` |
| Coverage | `just coverage` | `target/coverage/` | `.github/workflows/coverage.yml` |
| Performance | `just performance-profile` | `target/criterion/`, `kb/Baselines/` | `.github/workflows/stability.yml` |
| Soak | `just soak` | `target/soak/` | `.github/workflows/stability.yml` |

The recipes mirror the foundation recipes byte for byte (with crate name
substitution). Copy from `~/dev/templates/converge-extension/Justfile`
rather than handcrafting. When the foundation tightens a recipe, every
extension picks up the change in its next sync.

### 4. Coverage floor

- Workspace line coverage: **≥ 80%**.
- Per crate: **≥ 80%**, except for crates explicitly excluded for
  technical reasons (transport servers, CLI shells) — and those exclusions
  are listed in the extension's `kb/Architecture/Coverage Policy.md` with
  a one-paragraph justification each.
- **No regression.** Once a crate is above 80%, it cannot drop below the
  previously-recorded percentage for that release line. The badge job
  publishes the percent; the stability workflow fails the build on
  regression.
- New code added to a crate already at ≥ 80% must keep it there. New
  crates start at ≥ 80% on first release.

### 5. Test layout (mirror of foundation)

- `crates/<crate>/tests/` for integration tests.
- `*property*.rs` / `*negative*.rs` / `*proptest*.rs` only under `src/` or
  `tests/` — never in ad-hoc directories.
- One end-to-end smoke test per shipped pack, exercising the full
  `Pack::solve` lifecycle (validate_inputs → solve → plan_as<T>).
- `trybuild` compile-fail suite for any sealed type or kernel-authority
  surface (per
  [[Architecture/ADRs/ADR-006-promotion-authority-boundary]]).
- One `#[ignore = "soak"]` test per long-running invariant the extension
  claims to hold (heap stability, latency stability, replay determinism).

### 6. CI (GitHub Actions)

Four workflows, each mirrored from converge:

- `ci.yml` — check, test, lint, format on PR + push to main.
- `coverage.yml` — `cargo-llvm-cov`, push percent to a dynamic-badges
  gist, surface as a README badge.
- `security.yml` — `cargo-audit` (blocking) + `gitleaks` on PR + push.
- `stability.yml` — weekly Monday cron: regression-gate, bench-run, soak,
  audit, deny.

The `stability.yml` regression-check fails the build if the recorded
coverage percent drops below the floor *or* below the last release-line
high-water mark.

### 7. Provenance

The extension's `kb/` mirrors the foundation structure. Every page carries
`source:` frontmatter (`human` / `llm` / `mixed`). Every change appends
to a `kb/LOG.md` mutation log.

Required pages:

- `kb/Home.md` — Obsidian moc index.
- `kb/INDEX.md` — entity catalog.
- `kb/LOG.md` — mutation log.
- `kb/Architecture/` — surface diagrams, port boundaries, ADRs.
- `kb/Building/` — getting-started, release-commands.
- `kb/History/CHANGELOG.md` — release notes.
- `kb/Planning/MILESTONES.md` — scheduled delivery.

### 8. Versioning

- Single workspace version, bumped per release.
- Floor versions tracked in `~/dev/CLAUDE.md` (the Platform Floor table).
  Bumping a foundation floor that an extension depends on requires a
  matching extension release within the same window.
- While the platform head is unreleased, consumers redirect crates.io to
  local paths via `[patch.crates-io]` at the bottom of their workspace
  `Cargo.toml` (canonical block lives in
  `~/dev/templates/converge-engagement/Cargo.toml`).
- Release tags follow `vMAJOR.MINOR.PATCH`. Performance baselines are
  named after the release tag (`PERF_BASELINE=v1.0.0 just
  performance-profile`).

## The release ritual

A clean release is exactly these five commands, all green, in order:

```bash
just security-audit                            # 1. clean supply chain
just coverage                                  # 2. ≥ 80% per crate, no regression
PERF_BASELINE=v$(cargo pkgid | sed -E 's/.*#//') just performance-profile  # 3. baseline locked
SOAK_DURATION_MIN=5 just soak                  # 4. stability proven
just lint && cargo test --workspace            # 5. green
```

Archive `target/security/`, `target/coverage/`, `target/criterion/`,
`target/soak/`, and `kb/Baselines/` alongside the release tag.

## What this checklist is not

- Not a substitute for the four published ADRs (006/007/008 are binding).
- Not a guarantee against bugs — only against the absence of evidence.
- Not optional for repos that "feel small". The bar is the same for a
  five-file connector port and a ten-thousand-line solver bridge.
- Not negotiable per release. If an extension cannot clear it, it does
  not ship in this release line.

## How to apply this to an existing extension

1. Copy `~/dev/templates/converge-extension/Justfile` to the extension
   root and replace the crate name placeholders.
2. Copy `~/dev/templates/converge-extension/.github/workflows/*.yml`
   into the extension's `.github/workflows/` directory.
3. Copy `~/dev/templates/converge-extension/deny.toml` to the
   extension root, with the foundation's ignore list as the starting
   point.
4. Run the five-command release ritual locally. Address every red gate
   before opening the release PR.
5. Add `kb/Standards/Extension Release Checklist Compliance.md` to the
   extension's `kb/`, recording which crates have current per-crate
   coverage and any approved exclusions.
6. Append the change to the extension's `kb/LOG.md`.

## See also

- [[Architecture/Extension Topology]] — where each extension lives and why
- [[Architecture/Core Ideas]] — the durable principles this checklist defends
- [[Architecture/ADRs/ADR-006-promotion-authority-boundary]]
- [[Architecture/ADRs/ADR-007-provider-tool-contracts]]
- [[Architecture/ADRs/ADR-008-extension-crate-boundaries]]
- [[Building/Release Commands]] — foundation reference for the four gates
