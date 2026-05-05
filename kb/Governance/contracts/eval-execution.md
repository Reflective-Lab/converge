---
source: mixed
---
# Eval Execution Contract

> How and when evals are executed throughout the development lifecycle.

---

## Execution Model

Evals are executed by **invoking a persona** against the codebase. This can happen:

1. **Manually**: Developer pastes eval prompt into Claude session
2. **CLI-triggered**: Script invokes Claude API with eval prompt + codebase context
3. **CI/CD-triggered**: Automated pipeline runs evals as gates
4. **Scheduled**: Cron-like execution for monitoring

---

## Phase Gates

### 1. Development Phase

| Trigger | Evals | Mode | Blocking? |
|---------|-------|------|-----------|
| Feature branch created | None | - | - |
| Before PR opened | `qa-engineer` (quick) | Manual | Advisory |
| PR opened | `system-architect`, `security-auditor` | Automated | Yes |
| PR touches auth/security | `security-auditor` (full) | Automated | Yes |
| PR adds dependency | `build-vs-buy`, `legal-counsel` | Automated | Yes |
| PR affects users | `ethics-safety`, `end-user-advocate` | Automated | Advisory |

**Gate Contract:**
```yaml
phase: development
gate: pr-merge
required_evals:
  - system-architect-eval (layers, axioms)
  - qa-engineer-eval (tests exist, pass)
conditional_evals:
  - security-auditor-eval: when files match "auth|security|crypto"
  - build-vs-buy-eval: when package.json or go.mod modified
  - legal-counsel-eval: when LICENSE or dependencies change
  - ethics-safety-eval: when user-facing features change
blocking: all required must PASS, conditional must not FAIL
```

### 2. Testing Phase

| Trigger | Evals | Mode | Blocking? |
|---------|-------|------|-----------|
| Test suite runs | `qa-engineer` | Automated | Yes |
| Coverage drops | `qa-engineer` (coverage) | Automated | Yes |
| Security scan | `security-auditor` | Automated | Yes |
| Before merge to main | Full technical suite | Automated | Yes |

**Gate Contract:**
```yaml
phase: testing
gate: merge-to-main
required_evals:
  - qa-engineer-eval (MUST PASS)
  - security-auditor-eval (MUST PASS)
  - system-architect-eval (MUST PASS)
  - sre-operations-eval (should PASS)
blocking: true
escalation: Founder if any FAIL
```

### 3. Review Phase (Pre-Release)

| Trigger | Evals | Mode | Blocking? |
|---------|-------|------|-----------|
| Release candidate tagged | Full suite | Automated | Yes |
| Manual review requested | Selected personas | Manual | Depends |
| Go/no-go meeting | Summary report | Automated | Yes |

**Gate Contract:**
```yaml
phase: review
gate: release-approval
required_evals:
  # Critical - MUST PASS
  - system-architect-eval
  - security-auditor-eval
  - qa-engineer-eval
  - legal-counsel-eval

  # Important - SHOULD PASS
  - ethics-safety-eval
  - sre-operations-eval
  - developer-advocate-eval

  # Advisory - REVIEW
  - build-vs-buy-eval
  - sustainability-eval
  - investor-eval
  - curious-searcher-eval
  - skeptical-critic-eval
  - external-perspective-eval

pass_criteria:
  critical: all PASS
  important: no FAIL (PARTIAL ok with acceptance)
  advisory: reviewed, issues documented

blocking: true
approver: Founder
```

### 4. Marketing Phase

| Trigger | Evals | Mode | Blocking? |
|---------|-------|------|-----------|
| Content created | `marketing-lead`, `legal-counsel` | Manual | Yes |
| Claims made | `skeptical-critic`, `legal-counsel` | Manual | Yes |
| Press release | `journalist-investigator`, `legal-counsel` | Manual | Yes |
| Before publish | Marketing suite | Manual | Yes |

**Gate Contract:**
```yaml
phase: marketing
gate: content-publish
required_evals:
  - marketing-lead-eval (messaging)
  - legal-counsel-eval (claims)
  - ethics-safety-eval (honest representation)

conditional_evals:
  - curious-searcher-eval: for public-facing content
  - skeptical-critic-eval: for strong claims
  - academic-researcher-eval: for technical claims

blocking: true
approver: Marketing Lead + Legal Counsel
```

### 5. Sales Phase

| Trigger | Evals | Mode | Blocking? |
|---------|-------|------|-----------|
| Demo prepared | `sales-engineer` | Manual | Advisory |
| Contract drafted | `legal-counsel` | Manual | Yes |
| Customer commitment | `founder`, `ethics-safety` | Manual | Yes |

**Gate Contract:**
```yaml
phase: sales
gate: customer-commitment
required_evals:
  - sales-engineer-eval (can we deliver?)
  - legal-counsel-eval (contract terms)
  - ethics-safety-eval (commitment ethics)

blocking: true
approver: Founder
```

### 6. Production Phase

| Trigger | Evals | Mode | Blocking? |
|---------|-------|------|-----------|
| Deploy to staging | `sre-operations` | Automated | Yes |
| Deploy to production | Full operational suite | Automated | Yes |
| Post-deploy (1h) | `sre-operations` (health) | Automated | Rollback |
| Weekly | Technical suite | Scheduled | Alert |
| Monthly | Full suite | Scheduled | Report |

**Gate Contract:**
```yaml
phase: production
gate: deploy
required_evals:
  - sre-operations-eval (operational readiness)
  - security-auditor-eval (no new vulns)
  - qa-engineer-eval (smoke tests)

post_deploy:
  - sre-operations-eval: 1h post-deploy health check

scheduled:
  weekly:
    - security-auditor-eval
    - sre-operations-eval
  monthly:
    - full-suite

blocking: true for deploy, alert for scheduled
```

---

## Execution Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                     DEVELOPMENT                                  │
│  PR Created → Auto-run: architect, security, qa                 │
│  Conditional: build-vs-buy, legal, ethics based on changes      │
│  Gate: All required PASS → Merge allowed                        │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                       TESTING                                    │
│  Merge to main → Auto-run: full technical suite                 │
│  Gate: All PASS → Can tag release candidate                     │
└─────────────────────────┬───────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                       REVIEW                                     │
│  RC tagged → Auto-run: full suite (all personas)                │
│  Manual: Go/no-go review meeting                                │
│  Gate: Critical PASS, Important no FAIL → Release approved      │
└─────────────────────────┬───────────────────────────────────────┘
                          │
          ┌───────────────┴───────────────┐
          ▼                               ▼
┌─────────────────────┐       ┌─────────────────────┐
│     MARKETING       │       │       SALES         │
│ Content → legal,    │       │ Commit → deliver,   │
│ marketing, ethics   │       │ legal, ethics       │
│ Gate: Approved →    │       │ Gate: Can deliver → │
│ Publish             │       │ Sign                │
└─────────────────────┘       └─────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│                     PRODUCTION                                   │
│  Deploy → sre, security, qa (smoke)                             │
│  Post-deploy → health check                                     │
│  Ongoing → weekly security, monthly full suite                  │
│  Gate: Healthy → Stay up, Unhealthy → Rollback/Alert            │
└─────────────────────────────────────────────────────────────────┘
```

---

## Eval Invocation Methods

### Method 1: Manual (Current)

```bash
# Copy eval to clipboard, paste into Claude with codebase access
cat evals/system-architect-eval.md | pbcopy
# Open Claude, paste, get report
```

### Method 2: Claude Code CLI

```bash
# Run eval in Claude Code session
claude "Run the system-architect eval against this codebase.
        Follow the eval at evals/system-architect-eval.md"
```

### Method 3: Batch Script

```bash
#!/bin/bash
# run-eval-suite.sh

EVALS=(
  "system-architect"
  "security-auditor"
  "qa-engineer"
)

for eval in "${EVALS[@]}"; do
  echo "Running $eval eval..."
  claude --print "$(cat evals/${eval}-eval.md)" > reports/${eval}-report.md
done

# Aggregate results with the eval runner configured for this repository.
```

### Method 4: CI/CD Integration (GitHub Actions example)

```yaml
# .github/workflows/eval-gate.yml
name: Eval Gate

on:
  pull_request:
    branches: [main]

jobs:
  run-evals:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run Required Evals
        run: |
          # System Architect Eval
          claude --api-key ${{ secrets.ANTHROPIC_KEY }} \
            --prompt "$(cat evals/system-architect-eval.md)" \
            --output reports/system-architect.json

          # Security Auditor Eval
          claude --api-key ${{ secrets.ANTHROPIC_KEY }} \
            --prompt "$(cat evals/security-auditor-eval.md)" \
            --output reports/security-auditor.json

      - name: Check Results
        run: |
          # Check reports with the eval runner configured for this repository.
          # Fails if any required eval is FAIL
```

### Method 5: Scheduled Monitoring

```yaml
# .github/workflows/scheduled-evals.yml
name: Weekly Eval Suite

on:
  schedule:
    - cron: '0 9 * * 1'  # Monday 9am

jobs:
  weekly-evals:
    runs-on: ubuntu-latest
    steps:
      - name: Run Weekly Suite
        run: echo "run the repository eval suite"

      - name: Post to Slack
        run: echo "post the eval summary"
```

---

## Result Handling

### Eval Result Schema

```json
{
  "eval": "system-architect",
  "version": "1.0",
  "timestamp": "2024-01-15T10:30:00Z",
  "status": "PASS | PARTIAL | FAIL",
  "scores": {
    "layers": "PASS",
    "axioms": "PASS",
    "types": "PARTIAL"
  },
  "issues": [
    {
      "severity": "warning",
      "criterion": "type-safety",
      "description": "Found 2 'any' types in provider layer",
      "files": ["src/providers/openai.ts:45", "src/providers/anthropic.ts:78"],
      "recommendation": "Replace with proper types"
    }
  ],
  "summary": "Architecture mostly sound, minor type issues"
}
```

### Aggregation Rules

| Condition | Aggregate Status |
|-----------|-----------------|
| All PASS | PASS |
| Any FAIL in critical | FAIL |
| PARTIAL in critical, no FAIL | PARTIAL |
| FAIL only in advisory | PARTIAL with warnings |

### Escalation

| Status | Action |
|--------|--------|
| PASS | Proceed |
| PARTIAL | Review required, may proceed with acceptance |
| FAIL | Block, escalate to responsible persona |

---

## Acceptance Workflow

When an eval returns PARTIAL or surfaces issues:

```markdown
## Issue Acceptance Record

**Eval**: [eval name]
**Issue**: [description]
**Severity**: Critical / High / Medium / Low
**Status**: PARTIAL

### Acceptance Decision

- [ ] Issue acknowledged
- [ ] Risk understood
- [ ] Mitigation plan (if any): _______________
- [ ] Accept for release: Yes / No

**Accepted by**: [Persona]
**Date**: [date]
**Review by**: [date or "next release"]
```

---

## Summary: The Contract

1. **Development**: Evals run on PR, block merge if FAIL
2. **Testing**: Full technical suite on main, block RC if FAIL
3. **Review**: All personas evaluate RC, Founder approves
4. **Marketing/Sales**: Domain evals before commitments
5. **Production**: Operational evals on deploy, scheduled monitoring

The contract is: **No phase transition without passing required evals.**
