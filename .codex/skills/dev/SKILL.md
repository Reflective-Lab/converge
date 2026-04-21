---
name: dev
model: sonnet
description: Start local development or run examples.
user-invocable: true
argument-hint: [runtime|example <name>]
---
# Dev
Start local dev with `just dev-up` or run an example.

## Recipes
- `just dev-up` - start local runtime
- `just smoke-test` - verify the local runtime
- `just dev-down` - stop the local runtime
- `just example <name>` - run an example such as `just example hello-convergence`
- `just examples` - list all examples

## Rules
- Check required tools are installed: Rust and `just`.
- Report missing dependencies clearly.
