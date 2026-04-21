---
name: sync
model: sonnet
description: Morning briefing for team activity and repo health.
user-invocable: true
---
# Sync
Run `just sync`, then read `MILESTONES.md` if you need milestone context.

## Output
- PRs
- Issues
- Build and test state
- Any key blockers or recent changes

## Rules
- Prefer the repo's `just sync` script over ad hoc GitHub commands.
- Keep it brief.
