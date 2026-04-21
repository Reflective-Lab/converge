---
name: deploy
model: sonnet
description: Deploy to production. Confirms before every destructive step.
user-invocable: true
argument-hint: [cloud-run]
allowed-tools: Bash, Read
---
# Deploy
## Steps
1. Run `/check` first. Stop if anything fails.
2. Use the documented Converge deploy path from `kb/Building/Deployment.md`.
3. Default runtime deploy: use the deploy scripts in `ops/scripts/`.
4. Verify health or deployment status after deploy.
5. Report status.
## Rules
- Confirm with user before each deploy step.
- If required env vars, auth, or cloud tools are missing, stop and report them.
- Do not invent deploy targets that are not present in the repo.
