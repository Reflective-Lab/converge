---
tags: [workflow, cheat-sheet]
source: mixed
---
# Daily Journey

Your day, start to finish. Each phase has a skill or script.

## Morning

```
/focus              Orient yourself — kb, build health, team activity
/sync               What did the team do? PRs waiting? Unclaimed issues?
/next               Pick the next task from the current milestone
```

If it's your first session, `/focus` will point you to the key kb pages. Read them.

Before you start implementation:
1. Check `git status --short --branch`
2. Keep the root checkout on clean `main`
3. Create a topic branch, and a dedicated worktree if the task is more than trivial
4. Use `just git-hygiene` when branch, worktree, or release state is unclear

See [[Workflow/Git Strategy]].

## Working

```
/ticket <desc>      Create an issue any teammate can pick up
/fix <issue#>       Pick up an issue, branch, fix, PR
/check              Lint, check, test — must be clean before you stop
/pr [title]         Create a PR from current branch
```

Do not stack unrelated work on one branch. If the task splits, split the branch
or open another worktree.

## Reviewing

```
/review <pr#>       Security, correctness, style review
```

## Capturing Knowledge

When you learn something that isn't in the code:
1. Find the right page in `kb/`
2. Update it
3. If no page fits, create one and link it from `kb/Home.md`

The kb is shared. Keep it current. Your teammates and their agents read it too.

## End of Day

```
/done               What moved? What's open? KB updated?
/wip                Save and push everything
```

If the branch merged, remove the worktree and delete the remote branch. Do not
leave stale refs behind for archaeology.

## Weekly

```
Monday:  /audit     Security, dependency, compliance, and drift audit
```

## Quick Reference Card

| I want to... | Do this |
|---|---|
| Start my session | `/focus` |
| See what the team did | `/sync` |
| Pick the next task | `/next` |
| Fix a bug | `/fix 42` |
| Create a task anyone can grab | `/ticket add risk scoring agent` |
| Run quality checks | `/check` |
| Create a PR | `/pr` |
| Save and go | `/wip` |
| Review a teammate's PR | `/review 17` |
| Full audit | `/audit` |
| Deploy | `/deploy` |
| End the day | `/done` |
| Get help | `/help` |

See also: [[Workflow/Git Strategy]], [[Workflow/Working with Claude]], [[Workflow/Working with Codex]], [[Workflow/Working with Gemini]]
