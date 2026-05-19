---
name: branch
description: Deprecated. Converge uses main + next; temporary branches/worktrees require explicit human approval.
model: haiku
user-invocable: true
argument-hint: <temporary-branch>
allowed-tools: Bash
---
# Branch
Do not create a topic branch or worktree by default.

## Steps

1. Explain that the normal workflow is `main` + `next`.
2. If the user explicitly approves a temporary branch/worktree, create the
   smallest temporary branch needed and record how it will be removed.
3. Otherwise, switch to `next` and continue there.

## Rules
- Durable branches are only `main` and `next`.
- Temporary branches/worktrees require explicit human approval.
- Remove temporary branches/worktrees before handoff.
