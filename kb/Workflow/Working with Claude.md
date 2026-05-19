---
tags: [workflow, claude]
source: mixed
---
# Working with Claude

This project has two layers of automation: **Claude Code skills** (slash commands) and **Justfile recipes** (shell commands). They do different things. Use the right one.

## When to Use Which

| I want to... | Use | Why |
|---|---|---|
| Build the project | `just build` | Deterministic shell command |
| Run tests | `just test` | Deterministic shell command |
| Run clippy | `just lint` | Deterministic shell command |
| Inspect branches/worktrees/releases | `just git-hygiene` | Deterministic repo-state report |
| Orient myself at session start | `/focus` | Reads kb, checks build, shows team activity |
| See team activity | `/sync` | PRs waiting, unclaimed issues, recent commits |
| Pick next task | `/next` | Reads milestone, picks highest-priority task |
| Fix a GitHub issue end-to-end | `/fix 42` | Multi-step: read issue, branch, code, test, PR |
| Run quality checks | `/check` | Lint, check, test in one pass |
| Create a well-defined ticket | `/ticket add risk agent` | Needs to explore code, write requirements |
| Create a PR | `/pr` | Creates PR from current branch |
| Review a PR | `/review 17` | Reads diff, reasons about security/correctness |
| Save and push WIP | `/wip` | Multi-step git workflow |
| Run a full audit | `/audit` | Security, dependency, compliance, drift |
| Deploy | `/deploy` | Deploy to target environment |
| Capture end-of-session state | `/done` | Reads git state, updates kb, writes summary |
| Get help | `/help` | Shows available skills and usage |

**Rule of thumb:** if it's a single deterministic command, use `just`. If it requires reading, thinking, or multi-step orchestration, use a skill.

## Git Discipline

- Do normal work on `next`
- Keep `main` for validated integration and release reference
- Do not create topic branches or worktrees without explicit human approval
- Treat annotated tags as releases; do not infer "latest release" from `main`

See [[Workflow/Git Strategy]].

## The Knowledgebase and Claude

Claude reads `kb/` pages when it needs context. The `/focus` skill starts by reading `kb/Home.md`. The `/ticket` skill reads relevant kb pages to write better issues.

When Claude learns something during a session that should be preserved:
- Code changes go in code
- Everything else goes in `kb/`

The kb is for humans AND agents.

See also: [[Workflow/Git Strategy]], [[Workflow/Daily Journey]], [[Workflow/Working with Codex]], [[Workflow/Working with Gemini]]
