---
source: mixed
---
# Security Audit

> **Usage**: Run monthly to assess security posture of the codebase.

> **Note (2026-05-05):** The component table below references crates that
> have moved to extension repos: `converge-knowledge` → **mnemos**,
> `converge-analytics` → **prism**, `converge-policy` → **arbiter**,
> `converge-domain` → **atelier**. Audit those repos separately; the
> canonical crate map is [[Architecture/Crate Map]] and
> [[Architecture/Extension Topology]].

---

## Mission

You are a security auditor performing a code-level security review of Converge. Identify vulnerabilities, security anti-patterns, and areas needing hardening. Focus on issues that could compromise governance guarantees.

---

## 1) Audit Scope

### Components to Audit

| Component | Priority | Last Audited | Focus Areas |
|-----------|----------|--------------|-------------|
| converge-core | Critical | | Type safety, authority model |
| converge-domain | High | | Validation, invariants |
| converge-provider | High | | Input handling, isolation |
| converge-runtime | High | | Sandboxing, resource limits |
| converge-llm | Critical | | Injection, output handling |
| Applications | Medium | | Integration security |

### Out of Scope
- Third-party dependency vulnerabilities (see dependency-scan.md)
- Infrastructure security (separate audit)
- Operational security (separate audit)

---

## 2) Code Review Checklist

### Input Validation

| Check | Files/Functions | Status | Findings |
|-------|-----------------|--------|----------|
| All external input validated | | | |
| Type coercion safe | | | |
| Bounds checking present | | | |
| Encoding handled correctly | | | |

### Authority & Access Control

| Check | Files/Functions | Status | Findings |
|-------|-----------------|--------|----------|
| No implicit authority grants | | | |
| Authority checks not bypassable | | | |
| Capability boundaries enforced | | | |
| No privilege escalation paths | | | |

### Cryptographic Usage

| Check | Files/Functions | Status | Findings |
|-------|-----------------|--------|----------|
| Appropriate algorithms used | | | |
| No hardcoded secrets | | | |
| Proper key management | | | |
| Randomness sources secure | | | |

### Data Protection

| Check | Files/Functions | Status | Findings |
|-------|-----------------|--------|----------|
| Sensitive data identified | | | |
| Encryption at rest (if applicable) | | | |
| Secure transmission | | | |
| Proper data disposal | | | |

### Error Handling

| Check | Files/Functions | Status | Findings |
|-------|-----------------|--------|----------|
| Errors don't leak sensitive info | | | |
| Fail-secure behavior | | | |
| No panic in security paths | | | |
| Proper error propagation | | | |

### Concurrency & State

| Check | Files/Functions | Status | Findings |
|-------|-----------------|--------|----------|
| Race conditions checked | | | |
| TOCTOU vulnerabilities | | | |
| Atomic operations where needed | | | |
| State consistency maintained | | | |

---

## 3) AI/LLM-Specific Security

### Prompt Handling

| Check | Files/Functions | Status | Findings |
|-------|-----------------|--------|----------|
| User input sanitized before prompts | | | |
| System prompts protected | | | |
| Injection markers checked | | | |
| Context window limits enforced | | | |

### Output Processing

| Check | Files/Functions | Status | Findings |
|-------|-----------------|--------|----------|
| LLM output not trusted implicitly | | | |
| Content validation before use | | | |
| No direct execution of output | | | |
| Format validation present | | | |

### Proposal/Fact Boundaries

| Check | Files/Functions | Status | Findings |
|-------|-----------------|--------|----------|
| Cannot create Fact without promotion | | | |
| Promotion gates cannot be bypassed | | | |
| Validation reports cannot be faked | | | |
| Type system enforces boundaries | | | |

### Recall/Memory Security

| Check | Files/Functions | Status | Findings |
|-------|-----------------|--------|----------|
| Recall data validated on retrieval | | | |
| Provenance tracked | | | |
| Access controls on recall | | | |
| No unauthorized context access | | | |

---

## 4) Vulnerability Categories

### Critical (Governance Compromise)
Issues that could allow bypassing governance guarantees:
- Authority bypass
- Fact creation without promotion
- Audit trail manipulation
- Determinism violation

### High (Security Breach)
Issues that could lead to unauthorized access or data exposure:
- Information disclosure
- Authentication bypass
- Injection vulnerabilities
- Privilege escalation

### Medium (Defense Weakening)
Issues that weaken security posture but don't directly compromise:
- Missing input validation
- Weak error handling
- Insufficient logging
- Missing rate limiting

### Low (Best Practice)
Deviations from security best practices:
- Code quality issues
- Documentation gaps
- Minor hardening opportunities

---

## 5) Secure Coding Patterns

### Patterns to Verify

**Explicit Authority Pattern**
```rust
// GOOD: Authority is explicit parameter
fn perform_action(authority: Authority, action: Action) -> Result<()>

// BAD: Authority is ambient/implicit
fn perform_action(action: Action) -> Result<()>
```

**Proposal/Fact Type Wall**
```rust
// GOOD: Types prevent confusion
struct Proposal<T>(T);
struct Fact<T>(T);

impl<T> Fact<T> {
    // Only way to create a Fact is through promotion
    fn promote(proposal: Proposal<T>, validation: ValidationReport) -> Result<Fact<T>>
}

// BAD: No type-level distinction
type ProposalOrFact<T> = T;
```

**Append-Only Audit**
```rust
// GOOD: Only append operations
fn record_trace(trace: TraceLink) -> Result<TraceId>

// BAD: Mutation possible
fn update_trace(id: TraceId, trace: TraceLink) -> Result<()>
```

---

## 6) Required Output

### A. Executive Summary
- Overall security posture (Strong/Adequate/Concerning/Critical)
- Critical findings count
- Most severe issue
- Recommended immediate actions

### B. Vulnerability Report

| ID | Title | Severity | Location | Description | Remediation |
|----|-------|----------|----------|-------------|-------------|

### C. Security Anti-Patterns Found

| Pattern | Occurrences | Risk | Fix Approach |
|---------|-------------|------|--------------|

### D. Positive Findings
Security measures that are working well.

### E. Recommendations

**Immediate (P0)**
| Finding | Fix | Owner | Deadline |
|---------|-----|-------|----------|

**Short-term (P1)**
| Finding | Fix | Owner | Target |
|---------|-----|-------|--------|

**Medium-term (P2)**
| Finding | Fix | Owner | Target |
|---------|-----|-------|--------|

### F. Verification Tests
Suggested security tests to add:

| Test | Validates | Implementation Notes |
|------|-----------|---------------------|

---

## Audit Trail

| Date | Auditor | Scope | Key Findings |
|------|---------|-------|--------------|

---

## Constraints

- Focus on vulnerabilities, not style issues
- Provide proof-of-concept or clear reproduction steps for findings
- Prioritize findings that undermine governance guarantees
- Be specific about file paths and line numbers
- Distinguish between confirmed vulnerabilities and potential issues
