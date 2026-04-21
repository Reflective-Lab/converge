---
name: audit
model: opus
description: Full workspace health - security, compliance, drift, and milestone audit.
user-invocable: true
---
# Audit
Weekly Converge repo review.

## Steps
1. Run `just compliance-check`.
2. If `cargo-deny` is installed, run `just deny`; otherwise note that it was unavailable.
3. Run `just check`, `just test`, and `just lint`, or explain what you skipped.
4. Read `AGENTS.md`, `CODEX.md`, and the relevant `kb/` workflow pages for drift.
5. Read `MILESTONES.md` and `~/dev/work/EPIC.md` for milestone risk.
6. Update the relevant `kb/` page if you learn something that should persist.
7. Report findings, ordered by severity.

## Rules
- Prefer repo facts over generic process assumptions.
- If a command is unavailable locally, say so rather than guessing.
- Prioritize security, then compliance, then build health, then drift.
