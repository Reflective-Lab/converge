---
name: ticket
model: sonnet
description: Create an agent-ready GitHub issue.
user-invocable: true
argument-hint: [description]
---
# Ticket
Create a GitHub issue from the description with concrete requirements, key files, and a test plan.

## Steps
1. Explore the codebase to identify relevant files.
2. Determine area and size: small, medium, or large.
3. Create the issue with `gh issue create` including context, requirements, key files, test plan, and size.
4. Return the issue URL.

## Rules
- Every requirement must be testable.
- Key files must be real paths.
- If the issue is large, suggest splitting it.
