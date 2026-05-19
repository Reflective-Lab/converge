---
tags: [workflow, git]
source: mixed
---
# Git Strategy

This repo needs a boring Git model. The goal is one visible line of truth:
`main` for validated integration and `next` for active integration.

Repo-native report:

```bash
just git-hygiene
```

Use it to see the current branch, worktrees, latest release tag, and cleanup
candidates before starting or after merging work.

## Core Rules

1. `main` is the validated branch.
2. `next` is the active integration branch.
3. Normal work happens on `next`, not on topic branches.
4. Keep one checkout per repo. Do not create worktrees by default.
5. Releases are defined by annotated tags, not by whatever commit `main` is on
   today.
6. Remote branches are not archival storage. The durable remote branches are
   `main` and `next`.

## Operating Model

- Keep the primary checkout on `next` while work is active.
- Use `main` only for validated integration, release tagging, and production
  reference.
- If the checkout is on `main`, switch to `next` before implementation.
- If `next` does not exist locally, create it from `main` once and keep it.

## Worktree Policy

- Do not create worktrees as normal operating practice.
- If a temporary worktree is unavoidable, get explicit human approval first.
- Remove temporary worktrees before handoff. They are not a durable planning
  surface.

## Daily Flow

1. Start in the root checkout.
2. Run `just focus` or `just sync`.
3. Check `git status --short --branch` and `git worktree list`.
4. If the checkout is not on `next`, switch to `next` before implementation.
5. Run `just git-hygiene` if branch or remote state looks suspicious.
6. Before pushing, run the repo's quality gate for the scope of the change.
7. Keep local and remote branch state close to `main` + `next`.

## Merge Policy

- Prefer a linear `main`.
- Advance `main` from validated `next`.
- After `main` advances, make `next` point at the same truth before starting the
  next tranche of work.
- Do not keep stale merge-commit archaeology or local branch names around.

## Release Policy

- A release is one specific commit with one specific annotated tag.
- Use tags like `v3.6.0`.
- The version bump, release notes, and release validation belong to `next` until
  they are promoted to `main`.
- `main` may move immediately after the tag. That is normal.
- Never assume "`HEAD` equals latest release." The tag is the source of truth.

## Automation Branches

### Dependabot

Dependabot cargo and GitHub Actions bumps are low-drama maintenance work.

- Default policy: auto-merge if the update is isolated, CI is green, and the
  change does not require code or contract edits.
- Manually review:
  - major version bumps
  - updates that break lockstep crate families
  - updates that require runtime, policy, protocol, or API changes

Do not let stale dependabot branches accumulate. If one falls far behind
`main`, close it and let automation recreate it.

### CI / Docs Maintenance Branches

Branches for badges, hooks, coverage wiring, or README cleanup do not become
permanent infrastructure. Put them through `next` unless automation requires a
temporary branch.

## Remote Hygiene

- Delete merged remote branches immediately.
- Delete stale unmerged branches once their PR is closed or superseded.
- If a branch is old, behind `main`, and unreviewed, it is clutter.
- Preserve only `main` and `next` as durable branch names.

## Agent Rules

- Do not begin substantive implementation directly on `main`; use `next`.
- Do not create feature/topic branches without explicit human approval.
- Do not create worktrees without explicit human approval.
- Do not leave stale branch names or worktrees as the default team state.
- Do not describe a branch as "the release" when the tag says otherwise.

See also: [[Workflow/Daily Journey]], [[Workflow/Working with Claude]], [[Workflow/Working with Codex]], [[Workflow/Working with Gemini]]
