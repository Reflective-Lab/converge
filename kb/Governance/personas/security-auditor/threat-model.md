---
source: mixed
---
# Threat Model Review

> **Usage**: Run quarterly or before major releases to maintain threat model currency.

> **Note (2026-05-05):** Several crates referenced in the diagrams below have
> moved to extension repos: `converge-knowledge` → **mnemos**,
> `converge-analytics` → **prism**, `converge-policy` → **arbiter**,
> `converge-domain` (packs and examples) → **atelier**. The threat model
> categories still apply; the canonical crate map is
> [[Architecture/Crate Map]] and [[Architecture/Extension Topology]].

---

## Mission

You are a security professional reviewing the Converge threat model. Identify attack surfaces, enumerate threats, assess risks, and recommend mitigations. Focus on threats unique to AI governance systems.

---

## 1) System Overview

### Components to Model

```
┌─────────────────────────────────────────────────────────────────┐
│                        Application Layer                         │
├─────────────────────────────────────────────────────────────────┤
│  converge-runtime  │  converge-llm  │  converge-provider        │
├─────────────────────────────────────────────────────────────────┤
│                    converge-domain                               │
├─────────────────────────────────────────────────────────────────┤
│                    converge-core                                 │
└─────────────────────────────────────────────────────────────────┘
        ↕                    ↕                    ↕
   External APIs        LLM Providers        Data Stores
```

### Trust Boundaries

| Boundary | Between | Trust Assumptions |
|----------|---------|-------------------|
| User ↔ System | API surface | Untrusted input |
| System ↔ LLM | LLM provider | Semi-trusted output |
| Core ↔ Provider | Provider interface | Capability-bounded |
| System ↔ Storage | Persistence layer | Integrity required |

---

## 2) Attack Surface Analysis

### External Attack Surfaces

| Surface | Entry Points | Exposure |
|---------|--------------|----------|
| API endpoints | | |
| Configuration | | |
| CLI interface | | |
| Integration hooks | | |

### Internal Attack Surfaces

| Surface | Access Required | Risk |
|---------|-----------------|------|
| Provider implementations | | |
| Storage backends | | |
| Runtime environment | | |

### AI-Specific Attack Surfaces

| Surface | Threat Type | Unique Risks |
|---------|-------------|--------------|
| LLM input (prompts) | Injection, jailbreak | Governance bypass |
| LLM output (proposals) | Malicious content | Fact pollution |
| Recall system | Poisoning | Context manipulation |
| Adapter/LoRA | Supply chain | Model compromise |

---

## 3) STRIDE Threat Analysis

### Spoofing

| Threat | Target | Attack Scenario | Current Controls | Gaps |
|--------|--------|-----------------|------------------|------|
| Identity spoofing | Authority system | Attacker claims different authority | | |
| Provider spoofing | Provider interface | Malicious provider masquerading | | |

### Tampering

| Threat | Target | Attack Scenario | Current Controls | Gaps |
|--------|--------|-----------------|------------------|------|
| Audit log tampering | TraceLinks | Attacker modifies audit trail | | |
| Proposal modification | Promotion path | Modify proposal before promotion | | |
| Recall data tampering | Memory/context | Corrupt stored context | | |

### Repudiation

| Threat | Target | Attack Scenario | Current Controls | Gaps |
|--------|--------|-----------------|------------------|------|
| Action denial | Audit system | Actor denies taking action | | |
| Origin denial | Provenance | Deny source of content | | |

### Information Disclosure

| Threat | Target | Attack Scenario | Current Controls | Gaps |
|--------|--------|-----------------|------------------|------|
| Prompt leakage | System prompts | Extract governance rules | | |
| Recall extraction | Memory system | Access others' context | | |
| Audit data exposure | Trace logs | Access sensitive traces | | |

### Denial of Service

| Threat | Target | Attack Scenario | Current Controls | Gaps |
|--------|--------|-----------------|------------------|------|
| Resource exhaustion | Runtime | Overwhelming system | | |
| Budget bypass | Execution limits | Circumvent spending limits | | |
| Deadlock | Governance gates | Block all promotion | | |

### Elevation of Privilege

| Threat | Target | Attack Scenario | Current Controls | Gaps |
|--------|--------|-----------------|------------------|------|
| Authority escalation | Authority model | Gain unauthorized authority | | |
| Proposal→Fact bypass | Promotion gates | Skip validation | | |
| Provider privilege | Capability system | Provider exceeds bounds | | |

---

## 4) AI-Specific Threats

### Prompt Injection

| Vector | Attack | Impact | Mitigation |
|--------|--------|--------|------------|
| Direct injection | Malicious user input | Governance bypass | |
| Indirect injection | Malicious context data | Unintended actions | |
| System prompt extraction | Reveal governance rules | Attack planning | |

### Proposal Attacks

| Attack | Description | Impact | Mitigation |
|--------|-------------|--------|------------|
| Confidence manipulation | Fake high confidence | Bypass validation | |
| Semantic smuggling | Benign-looking malicious content | Policy bypass | |
| Batch corruption | Compromise multiple proposals | Large-scale impact | |

### Recall System Attacks

| Attack | Description | Impact | Mitigation |
|--------|-------------|--------|------------|
| Poisoning | Inject malicious memories | Future compromises | |
| Extraction | Retrieve sensitive context | Information leak | |
| Manipulation | Alter retrieval ranking | Bias decisions | |

### Adapter/Model Attacks

| Attack | Description | Impact | Mitigation |
|--------|-------------|--------|------------|
| Backdoor adapters | Trojan in LoRA weights | Triggered behavior | |
| Training data poisoning | Compromise fine-tuning | Biased outputs | |
| Model substitution | Replace with malicious model | Full compromise | |

---

## 5) Risk Assessment

### Risk Matrix

Rate each threat: Likelihood (1-5) × Impact (1-5) = Risk Score

| Threat | Likelihood | Impact | Risk Score | Priority |
|--------|------------|--------|------------|----------|
| | | | | |

### High-Risk Threats (Score ≥ 15)

Detailed analysis for each high-risk threat:

**Threat: [NAME]**
- Attack scenario (detailed)
- Required attacker capabilities
- Affected assets
- Business impact
- Recommended mitigations
- Residual risk after mitigation

---

## 6) Security Controls Mapping

### Converge Security Properties

| Property | Claim | Implementation | Verified? |
|----------|-------|----------------|-----------|
| Explicit authority | No implicit authority | | |
| Append-only audit | Immutable traces | | |
| Proposal/fact separation | Cannot bypass promotion | | |
| Determinism | Reproducible execution | | |
| Bounded execution | Enforced limits | | |

### Control Effectiveness

| Control | Threats Mitigated | Bypass Scenarios | Strength |
|---------|-------------------|------------------|----------|
| | | | |

---

## 7) Required Output

### A. Executive Summary
- Overall security posture
- Most critical threats
- Biggest gaps
- Recommended priorities

### B. Threat Register

| ID | Threat | Category | Risk | Status | Mitigation |
|----|--------|----------|------|--------|------------|

### C. Attack Trees
For top 3 threats, provide attack trees showing paths to compromise.

### D. Gap Analysis

| Gap | Threat Enabled | Severity | Remediation | Effort |
|-----|----------------|----------|-------------|--------|

### E. Recommendations (Prioritized)

| # | Recommendation | Threats Addressed | Effort | Impact |
|---|----------------|-------------------|--------|--------|

### F. Security Roadmap
Phased plan for addressing identified issues.

---

## Threat Modeling Principles

1. **Assume breach** - What if an attacker is already inside?
2. **Defense in depth** - No single point of failure
3. **Least privilege** - Minimum necessary access
4. **Fail secure** - Failures should not create vulnerabilities
5. **Trust but verify** - Even trusted components need validation

---

## Constraints

- Focus on threats unique to or amplified by AI governance context
- Prioritize threats that undermine core Converge claims
- Be specific about attack scenarios, not just categories
- Distinguish between theoretical and practical threats
- Provide actionable mitigations, not just identification
