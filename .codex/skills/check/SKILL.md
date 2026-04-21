---
name: check
model: sonnet
description: Run the Converge quality gate.
user-invocable: true
---
# Check
Run the full quality gate for this project.

## Steps
1. Run `just check`.
2. Run `just test`.
3. Run `just lint`.
4. Report failures with file paths and line numbers when available.

## Rules
- Fix auto-fixable issues when appropriate.
- If everything passes, say `Clean.`
