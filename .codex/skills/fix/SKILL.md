---
name: fix
model: opus
description: Fix a GitHub issue end to end.
user-invocable: true
argument-hint: [issue-number]
---
# Fix

## Steps
1. Read the issue with `gh issue view <issue-number>`.
2. Create a branch such as `fix/<issue-number>` if needed.
3. Explore the relevant code and docs.
4. Implement the smallest safe fix.
5. Verify with `just check`, `just test`, and `just lint` unless you have a documented reason not to.
6. Summarize the change and any remaining risk.
7. Commit, push, and open a PR only if the user asks.

## Rules
- Follow existing patterns.
- Keep the diff scoped to the issue.
