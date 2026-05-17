---
source: mixed
---
# Escalation-to-Promotion Linkage Example

> How escalation packets flow through disposition review to influence gate decisions.

**Cross-references:**
- Escalation schemas: [escalation-packet](../../schemas/escalation-packet.md), [disposition-feedback](../../schemas/disposition-feedback.md)
- Escalation guide: [ESCALATION.md](../../docs/ESCALATION.md)
- Gate policy: [GATES.md](../../GATES.md) - Release Candidate section
- Example escalation: [approved-example.md](../../schemas/escalation-examples/approved-example.md)
- Fixtures: `fixtures/escalations/packet.approved.yaml`, `fixtures/escalations/disposition.approved.yaml`, `fixtures/gates/release_cut.escalation.yaml`

---

## Overview

This document demonstrates the complete flow from **escalation packet** → **disposition** → **gate decision**. It shows how Extended personas can influence promotion decisions through structured escalation, even when they don't have blocking authority.

**Key principle:** Escalation produces disposition that gate consumes under policy. Escalation does not decide promotion directly.

---

## Scenario Context

**Gate:** Release Candidate (v2.1.0-rc.1)
**Extended Persona:** Sustainability Lead
**Concern:** Binary size bloat from dependency mismanagement
**Outcome:** Approved escalation blocks RC until binary size reduced

---

## Step 1: Escalation Packet Filed

### What Triggered the Escalation

During release-candidate gate evaluation, Sustainability Lead reviewed the build artifacts and discovered binary size increased significantly from prior release:

- **v2.0.0 binary:** 12MB
- **v2.1.0 binary:** 45MB (273% increase)

Sustainability Lead investigated root cause and gathered evidence before filing escalation packet.

### Escalation Packet

**File:** `fixtures/escalations/packet.approved.yaml`

```yaml
---
escalation_id: ESC-2026-001
escalated_by: sustainability-lead
escalated_at: 2026-01-26T14:30:00Z
gate_id: release-candidate
eval_id: sustainability-eval
severity: P1

concern:
  risk_prevented: Shipping bloated binary wastes user bandwidth and cloud resources
  evidence:
    - "Binary size: 45MB (up from 12MB in v1.0)"
    - "Dependency tree audit: tokio[full] includes unused features (async-io, tracing, parking_lot)"
    - "Comparison: similar tools average 15MB (ripgrep 13MB, fd 11MB, bat 18MB)"
    - "Carbon footprint: 273TB bandwidth for 6M downloads = 82 metric tons CO2"
  stop_rule: Binary size reduced to <20MB OR justification documented in release notes with environmental impact disclosure
  confidence: high

context:
  investigation_summary: |
    Analyzed dependency tree with `cargo tree --edges features`. Found tokio[full]
    pulls in 8 features we don't use: async-io (we use sync file I/O), tracing
    (we use log crate), parking_lot (default is sufficient). Created test branch
    with minimal feature set (`tokio = { version = "1.36", features = ["rt", "fs"] }`).
    Ran full test suite - all 487 tests pass. Binary size drops from 45MB to 18MB.
    No functionality loss detected. Strip debug symbols reduces to 16.2MB.
  related_escalations: []
  recommended_disposition: approve
---
```

### Key Elements

**Evidence quality:** Specific measurements (45MB vs 12MB), root cause analysis (tokio unused features), tested solution (18MB result with passing tests), quantified impact (82 metric tons CO2).

**Stop rule:** Clear and measurable - either reduce binary to <20MB or document justification. This gives Core team concrete resolution criteria.

**Investigation summary:** Proves homework done - identified root cause, tested fix, validated no functionality loss. Core team doesn't need to redo investigation.

**Confidence level:** High - justified by thorough evidence and tested solution.

---

## Step 2: Disposition Review

### Core Team Review

**Reviewers:** Founder, System Architect
**Review duration:** 1h 45min (within P1 SLA: 24h)

**Review process:**
1. Founder reads escalation packet, notes environmental impact quantification
2. System Architect verifies dependency tree analysis is correct
3. Both review test branch results (18MB, all tests pass)
4. Discuss whether binary size reduction is release-blocking or can defer
5. Agree evidence is strong and impact is user-facing (download time, bandwidth cost)
6. Decide to approve escalation - block RC until binary size reduced

### Disposition Feedback

**File:** `fixtures/escalations/disposition.approved.yaml`

```yaml
---
disposition_id: DISP-2026-001
escalation_id: ESC-2026-001
reviewed_by: [founder, system-architect]
reviewed_at: 2026-01-26T16:15:00Z

outcome:
  disposition: approved
  rationale: |
    Evidence shows clear bloat with no functionality justification. Testing confirms minimal
    features sufficient for our use case. Binary size reduction improves user experience
    (faster downloads, lower bandwidth costs) and reduces cloud distribution costs. The
    environmental impact calculation (82 metric tons CO2) demonstrates sustainability concern
    is quantifiable, not theoretical. This aligns with responsible development principles.
  precedent_references:
    - "DISP-2025-187: Approved dependency reduction for similar bloat justification"
    - "DISP-2025-203: Approved minimal feature flag usage to reduce binary size"

learning:
  what_was_strong: |
    Excellent evidence quality: specific size numbers, dependency tree analysis, tested
    alternative with test results. Stop rule was clear and measurable (<20MB threshold).
    Investigation summary proved homework was done - you didn't just flag concern, you
    investigated root cause and validated fix. Environmental impact quantification (CO2
    calculation) demonstrated sustainability concern has real-world metrics. High confidence
    justified by thorough testing (full test suite pass).
  what_to_improve: |
    Could strengthen by estimating user impact: bandwidth savings per download (29MB saved ×
    average connection speed = X seconds faster), cost reduction for cloud distribution
    ($Y/GB × 29MB × 6M downloads). Adding user experience angle would make case even stronger
    beyond environmental concerns.
  coaching_notes: |
    This is model escalation quality. Use this as template for future sustainability concerns.
    Key success factors: (1) quantifiable evidence, (2) tested solution, (3) clear stop rule,
    (4) investigation summary showing due diligence. Your environmental impact calculation
    demonstrates that sustainability metrics can be as concrete as performance metrics.

action:
  next_steps: |
    1. System Architect will review minimal feature PR (#847) - target review by EOD 2026-01-27
    2. Update Cargo.toml to use tokio minimal features before RC tag
    3. Add release notes entry documenting binary size reduction and environmental rationale
    4. Update kb/History/CHANGELOG.md with sustainability improvement highlight
  compensating_controls: []
  follow_up_owner: system-architect
---
```

### Key Elements

**Disposition outcome:** Approved - escalation blocks RC until resolved.

**Rationale:** Explains why Core team agreed with escalation (strong evidence, user impact, responsible development alignment).

**Learning loop:** Positive feedback on what was strong (evidence, testing, stop rule, investigation) + constructive improvement (add user experience angle) + coaching for future escalations.

**Action plan:** Concrete next steps with ownership and timeline. System Architect owns PR review, timeline is EOD next day.

---

## Step 3: Gate Decision Impact

### How Disposition Affects Gate

The **approved disposition** flows into the release-candidate gate as a blocking concern:

**Gate status before escalation:**
- system-architect-eval: PASS
- qa-engineer-eval: PASS
- security-auditor-eval: PASS
- founder-eval: PASS
- sre-operations-eval: PASS (elevated)
- **Overall:** APPROVED (ready to tag RC)

**Gate status after escalation approved:**
- All evals still PASS
- **Escalation ESC-2026-001:** BLOCKING (approved by Core, requires resolution)
- **Overall:** BLOCKED until stop rule satisfied

### Gate Execution Record

**File:** `fixtures/gates/release_cut.escalation.yaml`

```yaml
---
gate_id: release-candidate
gate_execution_id: GX-2026-RC-2.1.0
executed_at: 2026-01-26T16:30:00Z
release_target: v2.1.0-rc.1
risk_class: high

evaluations:
  - eval_id: system-architect-eval
    result: PASS
    evidence: [architecture-review-complete.md]
  - eval_id: qa-engineer-eval
    result: PASS
    evidence: [test-coverage-84pct.md, regression-suite-pass.md]
  - eval_id: security-auditor-eval
    result: PASS
    evidence: [vulnerability-scan-clean.md, dependency-audit-complete.md]
  - eval_id: founder-eval
    result: PASS
    evidence: [business-risk-acceptable.md, roadmap-aligned.md]
  - eval_id: sre-operations-eval
    result: PASS
    elevated: true
    evidence: [deployment-runbook-updated.md, smoke-tests-pass.md]

escalations:
  - escalation_id: ESC-2026-001
    disposition_id: DISP-2026-001
    status: blocking
    escalated_by: sustainability-lead
    concern: Binary size bloat (45MB, should be <20MB)
    stop_rule: Binary size reduced to <20MB OR justification documented
    disposition: approved
    reviewed_by: [founder, system-architect]

gate_decision:
  status: BLOCKED
  blocking_reason: "Approved escalation ESC-2026-001 requires resolution (binary size reduction)"
  resolution_required: "Reduce binary to <20MB per stop rule, then re-evaluate gate"
  blocked_at: 2026-01-26T16:30:00Z
---
```

### Decision Flow

1. **All evals PASS** - Technical, operational, security, business all green
2. **Escalation approved** - Sustainability concern validated by Core team
3. **Gate blocked** - Despite all evals passing, escalation creates blocking condition
4. **Resolution path clear** - Stop rule defines what's needed (binary <20MB)

**Key insight:** Escalation doesn't override eval results. Instead, it adds an additional blocking condition that must be satisfied alongside eval passes.

---

## Step 4: Resolution and Re-Evaluation

### Developer Action

Developer-jane (original PR author) takes action to resolve escalation:

1. Updates `Cargo.toml` to use minimal tokio features
2. Re-runs test suite (all 487 tests pass)
3. Verifies binary size reduced to 18MB
4. Pushes commit `f7g8h9i` to release branch

### Gate Re-Evaluation

**New gate execution:** GX-2026-RC-2.1.0-v2

All evals re-run:
- system-architect-eval: PASS (dependency reduction approved)
- qa-engineer-eval: PASS (tests still pass with minimal features)
- security-auditor-eval: PASS (tokio 1.36 minimal is secure)
- founder-eval: PASS (release timing still good)
- sre-operations-eval: PASS (deployment runbook updated)

**Escalation status:**
- ESC-2026-001 stop rule satisfied (binary 18MB < 20MB threshold)
- **Status:** RESOLVED

**Gate decision:**
- **Status:** APPROVED
- **Promotion executed:** RC tag `v2.1.0-rc.1` created at 2026-01-27T10:00:00Z

---

## Linkage Diagram

```
┌─────────────────────────────────────────────────────────┐
│ Step 1: Extended Persona Files Escalation Packet       │
│                                                         │
│  sustainability-lead investigates binary bloat          │
│  → Gathers evidence (45MB, tokio unused features)       │
│  → Tests solution (18MB with minimal features)          │
│  → Files ESC-2026-001 at release-candidate gate         │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ Step 2: Core Team Reviews and Dispositions             │
│                                                         │
│  founder + system-architect review evidence             │
│  → Validate investigation (dependency tree audit)       │
│  → Assess impact (user experience + environmental)      │
│  → Decide: APPROVE (block RC until binary <20MB)        │
│  → File DISP-2026-001 with action plan and learning    │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ Step 3: Disposition Influences Gate Decision           │
│                                                         │
│  release-candidate gate consumes disposition            │
│  → All evals PASS, but escalation approved = BLOCKING   │
│  → Gate status: BLOCKED until stop rule satisfied       │
│  → Developer notified: reduce binary to <20MB           │
└─────────────────────────────────────────────────────────┘
                           ↓
┌─────────────────────────────────────────────────────────┐
│ Step 4: Resolution and Re-Evaluation                   │
│                                                         │
│  developer-jane updates tokio to minimal features       │
│  → Binary reduced to 18MB (satisfies stop rule)         │
│  → Gate re-evaluates, escalation marked RESOLVED        │
│  → Gate status: APPROVED, RC tag created                │
└─────────────────────────────────────────────────────────┘
```

---

## Key Principles Demonstrated

### 1. Escalation Does Not Decide Promotion Directly

Sustainability Lead (Extended Advisory persona) **cannot** block the gate unilaterally. Instead:
- Files escalation packet with evidence
- Core team reviews and decides whether to approve/deny
- Approved disposition creates blocking condition at gate
- Gate enforces Core team's decision

**Rule:** Escalation produces disposition that gate consumes under policy. Escalation does not decide promotion directly.

### 2. Stop Rules Make Escalations Actionable

The stop rule (`Binary size reduced to <20MB OR justification documented`) provides:
- Clear resolution criteria
- Measurable threshold
- Alternative path (justification if reduction infeasible)

Without stop rule, escalation would be open-ended and unresolvable.

### 3. Learning Loop Improves Future Escalations

Disposition feedback includes:
- **what_was_strong:** Reinforces effective evidence patterns
- **what_to_improve:** Suggests strengthening techniques
- **coaching_notes:** Teaches reusable escalation quality patterns

This helps Extended personas calibrate escalation thresholds over time.

### 4. Escalation IDs Link Packet → Disposition → Gate

Traceability chain:
- `ESC-2026-001` (escalation packet)
- `DISP-2026-001` (disposition feedback, references ESC-2026-001)
- `GX-2026-RC-2.1.0` (gate execution, references both ESC and DISP)

Audit trail enables post-hoc review: "Why was v2.1.0 RC delayed?" → "Binary size escalation ESC-2026-001 approved by Founder + System Architect."

---

## Alternative Outcome: Denied Escalation

**Hypothetical:** If Core team had **denied** the escalation:

**Disposition outcome:** denied
**Rationale:** "Binary size increase justified by new features (OAuth2, file upload). User bandwidth cost is marginal (29MB difference on modern connections). Environmental concern is valid but not release-blocking. Compensating control: Document binary size in release notes with justification."

**Gate impact:**
- Escalation ESC-2026-001 status: DENIED
- Gate decision: APPROVED (no blocking condition)
- RC tag created as originally scheduled

**Extended persona next steps:**
- Read learning feedback in disposition (why denial occurred)
- Check precedents (was binary size ever blocking before?)
- Consider filing new escalation with stronger evidence if concern persists
- Learn from coaching notes for future escalations

**Key difference:** Denied escalations don't block gates, but still provide learning loop for Extended persona.

---

## Summary

This example demonstrates the complete escalation-to-promotion linkage:

1. **Extended persona investigates** - Gathers evidence, tests solution, files packet
2. **Core team dispositions** - Reviews evidence, decides approve/deny, provides learning feedback
3. **Gate enforces disposition** - Approved escalation creates blocking condition, denied escalation doesn't
4. **Developer resolves** - Satisfies stop rule, gate re-evaluates, promotion proceeds

**For auditors:** Escalation mechanism provides formal path for Extended personas to influence blocking decisions without having direct blocking authority. The disposition review ensures Core team validates concerns before they block gates.

**For Extended personas:** Use this flow as template for filing effective escalations - strong evidence, clear stop rules, thorough investigation, high confidence justified by testing.

---

*Fixtures for this walkthrough will be created in Phase 5 Plan 03 (Machine-Verifiable Fixtures).*
