# Gemini CLI Entrypoint

Read and follow `AGENTS.md` — it is the canonical project documentation.

## Gemini-Specific Notes

- Use `codebase_investigator` for deep architectural research or bug root-cause analysis.
- Use `generalist` for batch refactoring or high-volume file operations across the workspace.
- Prefer `grep_search` and `glob` over reading entire files. Lazy-load `kb/` pages as needed.
- Use `save_memory` for personal preferences only — project knowledge belongs in `kb/`.
- See `~/dev/reflective/bedrock-platform/EPIC.md` for strategic context (Converge = E1).

## Workflow Cadence

As a first-class collaborator, Gemini follows this rhythm:

- **Morning:** `/focus` → `/sync` → `/next`
- **Work:** `/fix`, `/check`, `/pr`
- **Evening:** `/done`
- **Monday:** `/audit`

## Skills & Commands

Gemini implements these personas through specific CLI workflows:

### ── Developer ──────────────────────────────────────
- **`/dev`**: Start local dev environment (run `just dev-up`).
- **`/check`**: Am I clean? (run `just lint`, then `just test`).
- **`/fix <issue>`**: Fix GitHub issue → branch → PR (reproduce → fix → test).
- **`/pr [title]`**: Create and push a pull request for current work.
- **`/wip`**: Save WIP, push, and switch devices.

### ── Product Owner ──────────────────────────────────
- **`/focus`**: Session opener. Run `just focus`, read `kb/Planning/MILESTONES.md`, and summarize.
- **`/next`**: Pick the next task from the current milestone.
- **`/ticket <desc>`**: File a new GitHub issue for the team.
- **`/done`**: End session. Summarize progress, blockers, and next steps.

### ── VP Engineering ─────────────────────────────────
- **`/audit`**: Weekly audit of security, compliance, and architectural drift (run `just deny` and `just compliance-check`).
- **`/review <pr>`**: Review a pull request for correctness and axiom alignment.

### ── DevOps ─────────────────────────────────────────
- **`/sync`**: Pull latest changes, check PRs/issues, and verify build health (run `just sync`).
