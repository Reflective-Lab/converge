---
name: dev
model: sonnet
description: Start local development environment.
user-invocable: true
argument-hint: [runtime|example <name>]
allowed-tools: Bash, Read
---
# Dev
Start local dev with `just dev-up` or run an example.
## Recipes
- `just dev-up` — start local runtime
- `just smoke-test` — verify the local runtime
- `just dev-down` — stop the local runtime
- `just example <name>` — run an example (e.g., `just example hello-convergence`)
- `just examples` — list all examples
## Rules
- Check required tools are installed (rust, just).
- Report missing dependencies clearly.
