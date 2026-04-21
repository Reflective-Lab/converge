---
name: done
model: sonnet
description: End-of-session summary with milestone and KB updates.
user-invocable: true
---
# Done
End the session with accountability.

## Steps
1. Read `MILESTONES.md`.
2. Review session work with `git diff --stat HEAD` and `git log --oneline -5`.
3. Check off completed deliverables in `MILESTONES.md` with today's date.
4. Update `CHANGELOG.md` under `## [Unreleased]` when the session shipped notable changes.
5. If the session changed project knowledge, architecture, or process, update the relevant `kb/` page.
6. Summarize what moved, what remains, and any risks.

## Rules
- Be honest about partial work.
- Prefer updating existing `kb/` pages over creating ad hoc notes.
- Flag work outside the current milestone as scope drift.
