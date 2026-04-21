# Converge Agent OS

This is the canonical agent entrypoint — all agents (Claude, Codex, Gemini, or otherwise) start here. Long-form documentation lives in `kb/`.

## Philosophy

Converge is a correctness-first, context-driven multi-agent runtime. Read `kb/Philosophy/Nine Axioms.md` — those are non-negotiable.

We use strongly typed languages that compile to native code. Rust for the system. No virtual machines. No garbage collectors in the hot path. The compiler is the first reviewer. The type system is the first test suite.

## The Knowledgebase

`kb/` is an Obsidian vault. It is THE documentation for this project.

- Humans open it in Obsidian.
- AI agents read it with file tools.
- When you learn something, update the kb.
- When architecture changes, the kb changes first.

**Do NOT read the entire kb on startup.** Lazy-load:

1. Read `kb/Home.md` only when you need to find something (it's the index).
2. Follow ONE wikilink to the specific page you need.
3. Read that page. If it links to something else you need, follow that link.
4. Never bulk-read `kb/` — treat it like documentation you look up, not a preamble you memorize.

## Public API

Converge exposes six public crates. See `kb/Architecture/API Surfaces.md` for the full contract.

| Crate | Purpose |
|---|---|
| `converge-pack` | Author packs, suggestors, invariants |
| `converge-provider-api` | Backend identity, capability routing |
| `converge-model` | Curated semantic types |
| `converge-kernel` | In-process embedding API |
| `converge-protocol` | Generated `converge.v1` wire types |
| `converge-client` | Remote Rust SDK |

Everything else is internal. See `kb/Architecture/API Surfaces.md` for who should use what.

## Build

```bash
just build          # cargo build --release
just build-quick    # cargo build --profile quick-release
just test           # cargo test --all-targets
just test-all       # cargo test --all-targets --workspace
just lint           # cargo fmt --check && cargo clippy -- -D warnings
just fix-lint       # auto-fix lint issues
just doc            # cargo doc --no-deps --workspace
just focus          # Session opener — repo health + recent activity
just sync           # Team sync — PRs, issues, recent commits
just status         # Build health, test results
just git-hygiene    # Worktrees, branch state, latest release tag, cleanup candidates
just test-layout    # Guard non-standard Rust test file placement
just test-runtime-wasm # Run the feature-gated WASM property suite
```

## Rules

These are not suggestions.

- No `unsafe` code. Ever.
- Use typed enums, not strings with semantics.
- Closed numeric domains use validated types or private validated fields, not naked `f32`/`f64` with meaning.
- Known config schemas deserialize into typed values and use `serde(deny_unknown_fields)` unless open extension points are deliberate.
- Before adding a negative or property test for internal misuse, first ask whether the type system can make the misuse impossible. Keep those tests for public boundaries, wire formats, feature gates, and algorithmic invariants.
- Rust test files live under `src/` or `tests/`; source-tree test modules must be explicitly linked, not left as dead files.
- Agents emit proposals, not direct facts — Converge promotes them.
- Every mutation needs an Actor.
- `just lint` clean before considering work done.
- The root checkout stays on clean `main`; use a topic branch and preferably a dedicated worktree for any non-trivial change.
- `main` is the integration branch, not a scratch branch. One branch/worktree = one concern.
- Releases are defined by annotated tags, not by the current tip of `main`.
- Delete merged or abandoned remote branches; do not use remote refs as archival storage.
- Routine Dependabot cargo and GitHub Actions bumps are usually safe to auto-merge after green CI; majors or code-touching updates still need review.
- No feature flags. No backwards-compat shims. Change the code.
- No unnecessary abstractions. Three similar lines beat a premature helper.
- All deps use `workspace = true` — never inline versions in crate Cargo.tomls.
- Edition 2024, rust-version 1.94.

Type-strengthening contract: `kb/Architecture/Type Protocol.md`.

Git operating model: `kb/Workflow/Git Strategy.md`.

## Architecture

The kernel is pure. No I/O, no persistence, no hidden background work, and no non-determinism in `converge-core`.

Async trait boundaries are allowed when they stay runtime-agnostic. Executor ownership, task spawning, and network/runtime coupling stay outside `converge-core`.

The hexagonal boundary is enforced by crate dependencies:
- `converge-pack` and `converge-provider-api` are leaves (zero internal deps)
- `converge-core` depends on `converge-pack`
- Adapters depend on contracts, never the reverse

See `kb/Architecture/Hexagonal Architecture.md` for the full picture.

## Known Drift

The codebase has known gaps between axioms and implementation. These are tracked in `kb/Architecture/Known Drift.md` with ADR-backed resolution plans. The most significant: agents can still emit facts directly via `AgentEffect::with_fact()`, bypassing the promotion gate. This is being fixed as a deliberate breaking change.

## Workflows

Run `just focus` at session start. See `kb/Workflow/Daily Journey.md` for the full cheat sheet.
Branch, worktree, and release discipline live in `kb/Workflow/Git Strategy.md`.

| Workflow | Purpose |
|---|---|
| `/focus` / `just focus` | Session opener — orient yourself, see team activity |
| `/sync` / `just sync` | Team sync — who did what, PRs waiting, unclaimed issues |
| `/next` | Pick the next task from the current milestone |
| `/dev` | Start local development environment |
| `/check` | Code quality — lint, check, test |
| `/fix` | Fix a GitHub issue by number |
| `/pr` | Create a pull request |
| `/ticket` | Create an issue any teammate can pick up |
| `/done` | End-of-session — what you moved, what's left for the team |
| `/review` | Review a pull request |
| `/wip` | Save work-in-progress and push |
| `/deploy` | Deploy to target environment |
| `/audit` | Security, dependency, compliance, and drift audit |
| `/help` | Show available skills and usage |

Agent-specific workflow details: `kb/Workflow/Working with Claude.md`, `kb/Workflow/Working with Codex.md`, `kb/Workflow/Working with Gemini.md`.

## Milestones

Read `kb/Planning/MILESTONES.md` at the start of every session. Scope all work to the current milestone. See `~/dev/work/EPIC.md` for the strategic context (Converge = E1) and `~/dev/work/MILESTONES.md` for the cross-project rollup.
