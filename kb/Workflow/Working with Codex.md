---
tags: [workflow, codex]
source: mixed
---
# Working with Codex

Start from the root `CODEX.md` entrypoint, then use this page for workflow guidance. Keep the same workflow names used in Claude docs. In Codex, name the workflow directly in plain text: `focus`, `run focus`, `check`, `done`, `audit`, `fix issue 42`, `review PR 5`.

## What to Read First

1. `AGENTS.md` — shared project rules, architecture, and public API
2. `kb/Home.md` — index, follow one relevant link at a time
3. The specific `kb/` page your task needs

Do not bulk-read the whole knowledgebase.

## Shared Automation

```bash
just focus     # Session opener — repo health + recent activity
just sync      # Team sync — PRs, issues, recent commits
just git-hygiene
```

Use those when you want deterministic output from the repo itself.

## Git Discipline

- Keep the root checkout on clean `main`
- Do non-trivial work on a topic branch
- Prefer a dedicated worktree for parallel work
- Treat the latest annotated tag, not `HEAD`, as the latest release

See [[Workflow/Git Strategy]].

## Repo-Local Codex Files

- `CODEX.md` is the root Codex entrypoint
- `.codex/settings.local.json` is reserved for repo-local command allowlists
- `.codex/skills/*/SKILL.md` mirrors the workflow set listed in the cheat sheet
- `.codex/skills/README.md` explains the role of the repo-local Codex skill set

## Canonical Workflows

| Workflow | Use with Codex |
|---|---|
| `/focus` | `focus`, `run focus`, or `just focus` |
| `/sync` | `sync`, `run sync`, or `just sync` |
| `/next` | `next` or `show remaining tasks for the current milestone` |
| `/check` | `check` or `run lint and check` |
| `/fix 42` | `fix 42` or `fix issue #42` |
| `/pr` | `pr` or `create a PR from the current branch` |
| `/ticket` | `ticket` or `create issue for <description>` |
| `/done` | `done` or `update kb/Planning/MILESTONES.md and kb/History/CHANGELOG.md` |
| `/review 5` | `review 5` or `review PR #5` |

`/done` may still be backed by the `checkpoint` workflow internally, and `/check` by `quality`. Keep `/done` and `/check` as the public names in docs and day-to-day use.

## Knowledgebase Discipline

When Codex learns something that should outlive the session:
- Code changes go in code
- Architecture and process knowledge go in `kb/`

See also: [[Workflow/Daily Journey]], [[Workflow/Working with Claude]], [[Workflow/Working with Gemini]]
