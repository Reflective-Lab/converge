---
name: wip
model: haiku
description: Save work in progress before switching context.
user-invocable: true
---
# WIP

## Steps
1. Show state with `git status`.
2. Save:
   ```bash
   git add -A
   git checkout -b wip/$(date +%Y%m%d-%H%M%S) 2>/dev/null || true
   git commit -m "WIP: $(date +%Y-%m-%d)"
   git push -u origin HEAD
   ```
3. Tell the user how to resume: `git checkout <branch> && git pull`.
