# Codex Entrypoint

Read and follow `AGENTS.md` — it is the canonical project documentation.

## Codex-Specific Notes

- Deployment context and verified facts live in `kb/Building/Deployment.md`.
- Repo-local Codex scaffolding lives in `.codex/`, including `.codex/skills/` for the workflow set listed in the cheat sheet. Treat `CODEX.md` and `kb/` as canonical.
- Wolfgang (`~/dev/reflective/marquee-apps/wolfgang-chat`) is the reference implementation for Firebase auth, Cloud Run, and Terraform patterns. When making deployment decisions, align with Wolfgang's conventions.
- Runtime feature defaults for deployment scripts: `gcp,auth,firebase`.
- See `~/dev/reflective/bedrock-platform/EPIC.md` for strategic context (Converge = E1).
- Run `just lint` before considering work done.
