---
source: mixed
---
# Phase Gates

> Concrete checklists that must pass before transitioning between phases.

---

## Gate 1: PR Merge (Development → Testing)

**Trigger**: Pull request ready for merge
**Approver**: System Architect (architecture), QA Engineer (quality)

### Required Evals

| Eval | Criterion | Must Be |
|------|-----------|---------|
| `system-architect-eval` | Layer boundaries | PASS |
| `system-architect-eval` | Axiom compliance | PASS |
| `qa-engineer-eval` | Tests exist | PASS |
| `qa-engineer-eval` | Tests pass | PASS |
| `qa-engineer-eval` | Coverage maintained | PASS |

### Conditional Evals

| If Changed | Run Eval | Must Be |
|------------|----------|---------|
| Auth/security files | `security-auditor-eval` | PASS |
| Dependencies | `build-vs-buy-eval` | PASS |
| Dependencies | `legal-counsel-eval` (license) | PASS |
| User-facing features | `ethics-safety-eval` | Not FAIL |

### Gate Checklist

```markdown
## PR Merge Gate: [PR #]

### Required
- [ ] Architecture eval: PASS
- [ ] QA eval: PASS
- [ ] CI pipeline: GREEN

### Conditional (if applicable)
- [ ] Security eval: ___
- [ ] Build-vs-buy eval: ___
- [ ] Legal eval: ___
- [ ] Ethics eval: ___

### Approval
- [ ] Code review approved
- [ ] All required evals PASS
- [ ] All conditional evals not FAIL

**Merge approved**: Yes / No
**By**: _______________
```

---

## Gate 2: Release Candidate (Testing → Review)

**Trigger**: All features for release merged to main
**Approver**: QA Engineer (quality), Founder (go-ahead)

### Required Evals (Must All PASS)

| Eval | Focus |
|------|-------|
| `system-architect-eval` | Full architecture check |
| `security-auditor-eval` | Full security check |
| `qa-engineer-eval` | Full quality check |
| `sre-operations-eval` | Operational readiness |

### Gate Checklist

```markdown
## Release Candidate Gate: [Version]

### Technical Sign-off
- [ ] System Architect eval: PASS
- [ ] Security Auditor eval: PASS
- [ ] QA Engineer eval: PASS
- [ ] SRE Operations eval: PASS

### Quality Metrics
- [ ] All tests pass
- [ ] Coverage >= threshold
- [ ] No critical vulnerabilities
- [ ] Build reproducible

### Approval
- [ ] Technical suite all PASS
- [ ] QA sign-off
- [ ] RC tagged

**RC approved**: Yes / No
**Tagged**: v_______________
**By**: _______________
```

---

## Gate 3: Release Approval (Review → Production/Marketing/Sales)

**Trigger**: Release candidate tagged
**Approver**: Founder (final), with sign-offs from all domains

### Required Sign-offs

| Domain | Persona | Eval | Status |
|--------|---------|------|--------|
| Architecture | System Architect | `system-architect-eval` | |
| Security | Security Auditor | `security-auditor-eval` | |
| Quality | QA Engineer | `qa-engineer-eval` | |
| Legal | Legal Counsel | `legal-counsel-eval` | |
| Ethics | Ethics & Safety | `ethics-safety-eval` | |
| Operations | SRE | `sre-operations-eval` | |
| Documentation | Developer Advocate | `developer-advocate-eval` | |
| Messaging | Marketing Lead | `marketing-lead-eval` | |
| Sales Readiness | Sales Engineer | `sales-engineer-eval` | |

### Advisory Reviews (Documented, Not Blocking)

| Perspective | Eval | Finding |
|-------------|------|---------|
| Sustainability | `sustainability-eval` | |
| Build vs Buy | `build-vs-buy-eval` | |
| Investor View | `investor-eval` | |
| Newcomer View | `curious-searcher-eval` | |
| Critic View | `skeptical-critic-eval` | |
| External Scrutiny | `external-perspective-eval` | |

### Gate Checklist

```markdown
## Release Approval Gate: [Version]

### Critical (Must PASS)
- [ ] System Architect: ___
- [ ] Security Auditor: ___
- [ ] QA Engineer: ___
- [ ] Legal Counsel: ___

### Important (Should PASS, PARTIAL accepted with reason)
- [ ] Ethics & Safety: ___
- [ ] SRE Operations: ___
- [ ] Developer Advocate: ___
- [ ] Marketing Lead: ___
- [ ] Sales Engineer: ___

### Advisory (Reviewed)
- [ ] Sustainability: reviewed
- [ ] Build vs Buy: reviewed
- [ ] Investor: reviewed
- [ ] Curious Searcher: reviewed
- [ ] Skeptical Critic: reviewed
- [ ] External Perspective: reviewed

### Known Issues Accepted
| Issue | Severity | Accepted By | Reason |
|-------|----------|-------------|--------|

### Final Approval
- [ ] All critical PASS
- [ ] All important not FAIL
- [ ] All advisory reviewed
- [ ] Known issues documented and accepted

**Release approved**: Yes / No
**By**: Founder
**Date**: _______________
```

---

## Gate 4: Content Publish (Marketing)

**Trigger**: Marketing content ready for publication
**Approver**: Marketing Lead + Legal Counsel

### Required Evals

| Eval | Focus | Must Be |
|------|-------|---------|
| `marketing-lead-eval` | Messaging accuracy | PASS |
| `legal-counsel-eval` | Claims validity | PASS |
| `ethics-safety-eval` | Honest representation | Not FAIL |

### Conditional

| If Content Contains | Also Run |
|--------------------|----------|
| Technical claims | `academic-researcher-eval` |
| Competitive claims | `skeptical-critic-eval` |
| User testimonials | `end-user-advocate-eval` |

### Gate Checklist

```markdown
## Content Publish Gate: [Content Name]

### Content Type
- [ ] Blog post
- [ ] Press release
- [ ] Marketing page
- [ ] Social media
- [ ] Other: _______________

### Required Reviews
- [ ] Marketing Lead eval: ___
- [ ] Legal Counsel eval: ___
- [ ] Ethics & Safety eval: ___

### Claims Verified
| Claim | Evidence | Legal Approved |
|-------|----------|----------------|

### Approval
**Publish approved**: Yes / No
**By**: Marketing Lead + Legal Counsel
**Date**: _______________
```

---

## Gate 5: Customer Commitment (Sales)

**Trigger**: About to sign contract or make delivery commitment
**Approver**: Founder + Legal Counsel

### Required Evals

| Eval | Focus | Must Be |
|------|-------|---------|
| `sales-engineer-eval` | Can we deliver? | PASS |
| `legal-counsel-eval` | Contract terms | PASS |
| `ethics-safety-eval` | Commitment ethics | Not FAIL |

### Gate Checklist

```markdown
## Customer Commitment Gate: [Customer Name]

### Commitment Details
- Customer: _______________
- Commitment: _______________
- Timeline: _______________
- Value: _______________

### Validation
- [ ] Sales Engineer: We can deliver this
- [ ] Legal Counsel: Terms are acceptable
- [ ] Ethics & Safety: Commitment is honest

### Risk Assessment
| Risk | Mitigation |
|------|------------|

### Approval
**Commitment approved**: Yes / No
**By**: Founder
**Date**: _______________
```

---

## Gate 6: Production Deploy

**Trigger**: Ready to deploy to production
**Approver**: SRE / Operations

### Required Evals

| Eval | Focus | Must Be |
|------|-------|---------|
| `sre-operations-eval` | Operational readiness | PASS |
| `security-auditor-eval` | No new vulnerabilities | PASS |
| `qa-engineer-eval` | Smoke tests | PASS |

### Gate Checklist

```markdown
## Production Deploy Gate: [Version]

### Pre-Deploy
- [ ] SRE Operations eval: PASS
- [ ] Security eval: PASS
- [ ] Smoke tests: PASS
- [ ] Rollback plan documented
- [ ] Monitoring configured

### Deploy
- [ ] Deploy to staging: SUCCESS
- [ ] Staging health check: PASS
- [ ] Deploy to production: SUCCESS

### Post-Deploy (within 1 hour)
- [ ] Health check: PASS
- [ ] Error rates: Normal
- [ ] Performance: Normal
- [ ] User-reported issues: None

**Deploy status**: Success / Rolled Back
**By**: SRE
**Date**: _______________
```

---

## Gate Summary

| Gate | Trigger | Approver | Critical Evals |
|------|---------|----------|----------------|
| PR Merge | PR ready | Architect + QA | architect, qa |
| Release Candidate | Features done | QA + Founder | architect, security, qa, sre |
| Release Approval | RC tagged | Founder | all domains |
| Content Publish | Content ready | Marketing + Legal | marketing, legal, ethics |
| Customer Commitment | Contract ready | Founder + Legal | sales, legal, ethics |
| Production Deploy | Deploy ready | SRE | sre, security, qa |

---

## Automation Hooks

### Git Hooks

```bash
# .git/hooks/pre-push
#!/bin/bash
# Run quick evals before push
just test || exit 1
```

### CI/CD Triggers

```yaml
# Trigger on PR
on: pull_request → run PR merge gate evals

# Trigger on tag
on: push tags v* → run release approval gate evals

# Trigger on deploy
on: deployment → run production deploy gate evals
```

### Scheduled

```yaml
# Weekly security check
schedule: 0 9 * * 1 → security-auditor-eval

# Monthly full suite
schedule: 0 9 1 * * → all evals
```
