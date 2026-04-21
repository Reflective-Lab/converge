---
name: pr
model: sonnet
description: Create a pull request from the current branch.
user-invocable: true
argument-hint: [title]
---
# PR

## Steps
1. Check state with `git status` and `git log --oneline main..HEAD`.
2. If on `main`, create a feature branch first.
3. Push with `git push -u origin HEAD`.
4. Create the PR with `gh pr create`. Use the provided title if one was given; otherwise draft from commits.
5. Return the PR URL.
