# Claude Code Entrypoint

Read and follow `AGENTS.md` — it is the canonical project documentation.

## Session Scope

- **Milestones:** Read `kb/Planning/MILESTONES.md` at the start of every session. Scope work to the current milestone.
- **Changelog:** Update `kb/History/CHANGELOG.md` when shipping notable changes.
- **Strategic context:** `~/dev/work/EPIC.md`

## Root Directory Rule

**Keep the root clean.** Only GitHub-essential + config files at root level:
- **Keep at root:** `README.md`, `LICENSE`, `CODE_OF_CONDUCT.md`, `SECURITY.md`
- **Keep at root:** `AGENTS.md`, `CLAUDE.md`, `CODEX.md`, `GEMINI.md` (agent entrypoints)
- **Keep at root:** `Cargo.toml`, `Cargo.lock`, `Justfile`, `deny.toml`, `.env.example`, `.gitignore`
- **Everything else goes into:** `kb/` (docs), `schema/` (protocols), `crates/` (source), `examples/` (examples)
- **Moved to `runway/`:** `ops/` (deployment/infra), `dev/` (local tooling), `crates/application` (distribution binary), `crates/llm` (local inference)

No new markdown files at root. No new directories. Update existing docs in `kb/` instead. Treat root as a carefully curated interface, not a dumping ground.

## Claude-Specific Notes

- **Available skills:** `/experiment` — hypothesis-driven development with evidence logging.
- Use `kb/Architecture/System Overview.md` and `kb/Architecture/API Surfaces.md` as the authoritative API reference. When they conflict with other docs, the KB docs win.
- Prefer Edit over Write for existing files. Prefer Grep/Glob over Bash for search.
- Do not create documentation files unless explicitly asked. Knowledge belongs in `kb/`.
- When learning something about the project, update the relevant `kb/` page rather than saving it as memory.
- Run `just lint` before considering work done.
- Never push to main without confirmation.
