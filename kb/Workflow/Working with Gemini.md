---
tags: [workflow, gemini]
source: mixed
---
# Working with Gemini

This project uses Gemini CLI as a first-class collaborator. Gemini follows the same [[Workflow/Daily Journey|Daily Journey]] as other agents. It is instructed by `GEMINI.md` and `AGENTS.md`.

## Native Tools vs Shared Workflows

| I want to... | Tool | Why |
|---|---|---|
| Run a session /focus | `just focus` | Shared deterministic repo-state script |
| Sync with the team | `just sync` | Shared script for GitHub and git status |
| Check project health | `just status` | Shared build and test script |
| Inspect branches/worktrees/releases | `just git-hygiene` | Shared repo-state and cleanup report |
| Deep architecture research | `codebase_investigator` | Gemini's specialized tool for complex analysis |
| Batch refactoring | `generalist` | Efficient multi-file operations |
| Fix a bug or implement a feature | `replace`, `write_file`, `run_shell_command` | Surgical code modifications |
| Finalize a session | Discussion + `kb/` update | Capture what moved |

## Git Discipline

- Do normal work on `next`
- Keep `main` for validated integration and release reference
- Do not create topic branches or worktrees without explicit human approval
- Treat the latest annotated tag as the latest release

See [[Workflow/Git Strategy]].

## Workflow Patterns

```text
Run the /focus workflow. Read AGENTS.md, GEMINI.md, and kb/Home.md, then summarize build health and recent activity.
```

```text
Implement the /fix workflow for issue #42. Read the issue, find the relevant code, make the change, run just check/test/lint, and prepare the PR.
```

## Sub-Agent Delegation

- **`codebase_investigator`**: Deep dependency analysis, bug root-cause.
- **`generalist`**: High-volume tasks (license headers, import updates, lint fixes).

## Knowledgebase Discipline

Gemini is mandated to use `kb/` as the primary source of truth. While it can use `save_memory` for personal preferences, project-level knowledge MUST go in `kb/`.

See also: [[Workflow/Git Strategy]], [[Workflow/Daily Journey]], [[Workflow/Working with Claude]], [[Workflow/Working with Codex]]
