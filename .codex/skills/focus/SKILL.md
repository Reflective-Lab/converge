---
name: focus
model: sonnet
description: Session opener with current milestone and build state.
user-invocable: true
---
# Focus
Run `just focus`, then read `MILESTONES.md`.

## Output
- Current milestone and epic
- Deadline and days left
- Remaining unchecked deliverables
- Current build and recent-activity state from `just focus`

## Rules
- Warn if the deadline is under 7 days.
- Flag a blocker if the deadline has passed.
- Show state; do not pick work for the user.
