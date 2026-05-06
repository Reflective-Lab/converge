# Development

## Prerequisites

- Rust 1.94+ (`rustup update`)
- [just](https://github.com/casey/just) (`brew install just`)
- Optional: [jj (Jujutsu)](https://martinvonz.github.io/jj/) for version control
- Optional: CUDA / Vulkan / WGPU for GPU acceleration
- Optional: [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) for supply chain auditing

## Quick Start

```bash
git clone https://github.com/Reflective-Lab/converge.git
cd converge

just build-quick   # fast iteration build
just test          # run tests
just lint          # format + clippy
```

## Running Examples

```bash
just examples                        # list all examples
just example hello-convergence       # run the hello-convergence example
just example custom-agent            # implement your own agent
just example meeting-scheduler       # domain pack with constraint agents
```

See [examples/README.md](examples/README.md) for the full list.

## Workspace Structure

```
crates/
├── pack/          # Canonical pack authoring contract
├── provider/      # Canonical provider capability contract
├── model/         # Curated semantic model surface
├── kernel/        # Curated in-process embedding API
├── protocol/      # Generated wire contract (converge.v1)
├── client/        # Canonical remote Rust SDK
├── core/          # Convergence engine (implementation)
├── experience/    # Event-sourced audit store
├── optimization/  # Native optimization and constraint solving
├── storage/       # Object storage abstraction
└── runtime/       # HTTP/gRPC execution service
```

See [kb/Architecture/System Overview.md](kb/Architecture/System%20Overview.md) for the full
dependency graph and [kb/Architecture/API Surfaces.md](kb/Architecture/API%20Surfaces.md) for
the canonical public contracts.

## Build Profiles

| Profile | Use case | Command |
|---------|----------|---------|
| `quick-release` | Daily development | `just build-quick` |
| `ci` | GitHub Actions | `just build-ci` |
| `release` | Production / publish | `just build` |

## Git Workflow

### Using git worktrees for parallel work

Worktrees let you work on multiple branches simultaneously without stashing.
Each worktree is a separate checkout sharing the same `.git` directory.

```bash
# Create a worktree for a feature branch
just worktree fix-auth
# → creates ../converge-fix-auth/ on branch fix-auth

# Work in the worktree
cd ../converge-fix-auth
just test

# When done, clean up
just worktree-rm fix-auth
```

This is especially useful for:
- Running tests on one branch while developing on another
- Code review checkouts without disrupting your working tree
- Parallel agent work (each agent gets its own worktree)

### Using jj (Jujutsu) for version control

[Jujutsu](https://martinvonz.github.io/jj/) is a Git-compatible VCS with
better ergonomics for stacking changes and conflict resolution.

```bash
# Initialize jj in an existing git repo
jj git init --colocate

# Basic workflow
jj new -m "feat: add custom provider"   # start a new change
# ... edit files ...
jj status                                # see what changed
jj diff                                  # review changes
jj squash                                # fold into parent

# Stacking changes (jj's strength)
jj new -m "test: add provider tests"     # stack another change
jj log                                   # see the change graph

# Push to GitHub
jj git push
```

Key advantages over plain git:
- **No staging area** — every file save is automatically tracked
- **First-class conflicts** — resolve at your pace, not during rebase
- **Change stacking** — easy to reorder, split, and squash changes
- **Undo anything** — `jj undo` works for any operation

```bash
# Quick jj commands via just
just jj-status
just jj-log
just jj-new "feat: add something"
just jj-squash
just jj-push
```

## Release Gates

The four release-grade commands for v3.8 are documented in
[Release Commands](Release%20Commands.md):

```bash
just security-audit         # supply-chain audit → target/security/
just coverage               # workspace coverage → target/coverage/
just performance-profile    # criterion baseline → target/criterion/, kb/Baselines/
just soak                   # bounded soak run  → target/soak/
```

Each command is idempotent, runs from a fresh checkout, and writes
archivable artefacts for the release tag. See the reference doc for
output layout, baseline policy, and CI bindings.

## Supply Chain Security

```bash
# Audit dependencies for vulnerabilities and license compliance
just sec-deny

# Advisories only (faster)
just sec-deny-advisories

# Blocking release-candidate audit (full report → target/security/)
just security-audit
```

## Publishing to crates.io

Publishable crates in dependency order (see [ADR-001](kb/Architecture/ADRs/ADR-001-canonical-public-crates.md)):

1. `converge-pack`
2. `converge-provider`
3. `converge-protocol`
4. `converge-core`
5. `converge-model`
6. `converge-storage`
7. `converge-experience`
8. `converge-optimization`
9. `converge-kernel`
10. `converge-client`

```bash
# Validate readiness
just publish-dry-run
```
