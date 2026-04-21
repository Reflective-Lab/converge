---
name: audit
model: opus
description: Full workspace health — security, compliance, drift, observations. Weekly.
user-invocable: true
allowed-tools: Bash, Read, Edit, Grep, Glob, Agent
---
# Audit
Weekly Converge repo review.
## Steps
1. **Security**:
   - Run `just compliance-check`
   - If `cargo-deny` is installed, run `just deny`; otherwise note that it was unavailable
   - Scan for obvious secrets or unsafe compliance claims
2. **Build health**:
   - Run `just check`
   - Run `just test`
   - Run `just lint`
   - Report any command you intentionally skipped
3. **Drift**:
   - Read `AGENTS.md`, `CLAUDE.md`, and the relevant `kb/` workflow pages
   - Verify canonical docs live in `kb/`
   - Verify Rust edition/rust-version, `unsafe` policy, and public-crate rules still match the repo docs
4. **Milestones**:
   - Read `MILESTONES.md` and `~/dev/work/EPIC.md`
   - Flag overdue work or deadline risk
5. If you learn something that should persist, update the relevant `kb/` page.
6. Output:
```
── Audit ──────────────────────────────────────────
Date: <today>
Security:     <PASS|ISSUES>
Compliance:   <PASS|ISSUES>
Drift:        <PASS|ISSUES>
Milestones:   <at risk or on track>
KB updates:    <none or list>
Action items:
1. ...
────────────────────────────────────────────────────
```
## Rules
- Be direct about problems.
- Stalled = no progress in 7+ days.
- Priority: security > compliance > drift > milestones.
- Prefer repo facts over generic process assumptions.
- If a command is unavailable locally, say so rather than guessing.
