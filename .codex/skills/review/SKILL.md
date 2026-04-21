---
name: review
model: opus
description: Review a pull request for security, correctness, style, and ops.
user-invocable: true
argument-hint: [pr-number]
---
# Review

## Steps
1. Read the PR with `gh pr view <pr-number>` and `gh pr diff <pr-number>`.
2. Review for security, correctness, style, and operational risk.
3. Report blockers first, then suggestions and questions.

## Rules
- Do not leave GitHub comments unless the user asks.
- Findings first, summary second.
