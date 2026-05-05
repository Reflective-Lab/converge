---
source: mixed
---
# Dependency Security Scan

> **Usage**: Run weekly or before releases to identify vulnerable dependencies.

> **Note (2026-05-05):** The crate inventory below references crates that
> have moved to extension repos: `converge-knowledge` → **mnemos**,
> `converge-analytics` → **prism**, `converge-policy` → **arbiter**,
> `converge-domain` → **atelier**. Each extension repo has its own
> dependency surface and should be scanned independently. See
> [[Architecture/Extension Topology]].

---

## Mission

You are a security analyst scanning Converge's dependencies for known vulnerabilities, security issues, and supply chain risks. Identify issues and recommend remediation.

---

## 1) Dependency Inventory

### Direct Dependencies by Crate

| Crate | Dependency | Version | Purpose | Criticality |
|-------|------------|---------|---------|-------------|
| converge-core | | | | |
| converge-domain | | | | |
| converge-provider | | | | |
| converge-runtime | | | | |
| converge-llm | | | | |

### Dependency Tree Depth
- Total direct dependencies: [N]
- Total transitive dependencies: [N]
- Maximum dependency depth: [N]

---

## 2) Vulnerability Scan

### Tools to Use

```bash
# Rust dependencies
cargo audit
cargo deny check advisories

# If using additional languages
npm audit  # for Node.js
pip-audit  # for Python
```

### Known Vulnerabilities Found

| Dependency | Version | CVE/RUSTSEC | Severity | CVSS | Exploitability | Fixed In |
|------------|---------|-------------|----------|------|----------------|----------|

### Vulnerability Details

For each HIGH or CRITICAL vulnerability:

**[CVE/RUSTSEC-XXXX]: [Title]**
- Affected: [dependency@version]
- Description: [What the vulnerability is]
- Impact on Converge: [How this affects us specifically]
- Exploitability: [Is this reachable from our code?]
- Fix: [Upgrade to X / Replace with Y / Accept risk because Z]

---

## 3) Supply Chain Risk Assessment

### Dependency Health

| Dependency | Maintainer | Last Update | Bus Factor | Risk Level |
|------------|------------|-------------|------------|------------|
| | | | | |

### Red Flags to Check

| Check | Status | Findings |
|-------|--------|----------|
| Unmaintained dependencies (>1yr no updates) | | |
| Single-maintainer critical dependencies | | |
| Dependencies with history of vulnerabilities | | |
| Unusual permission requirements | | |
| Dependencies from unknown sources | | |
| Typosquatting risk (similar names) | | |

### Source Verification

| Dependency | Source | Verified? | Concerns |
|------------|--------|-----------|----------|
| | crates.io / GitHub / etc. | | |

---

## 4) License Compliance (Security Angle)

### Concerning License Patterns

| Dependency | License | Concern |
|------------|---------|---------|
| | | Copyleft could force disclosure |
| | | License change risk |
| | | Unclear/missing license |

---

## 5) Dependency Update Assessment

### Available Updates

| Dependency | Current | Latest | Breaking? | Security Relevant? |
|------------|---------|--------|-----------|-------------------|
| | | | | |

### Update Recommendations

**Security-Critical Updates (Do Immediately)**
| Dependency | Current → Target | Reason |
|------------|------------------|--------|

**Recommended Updates (Do Soon)**
| Dependency | Current → Target | Reason |
|------------|------------------|--------|

**Optional Updates (When Convenient)**
| Dependency | Current → Target | Reason |
|------------|------------------|--------|

---

## 6) Mitigation Strategies

### For Unfixable Vulnerabilities

When a vulnerability can't be immediately fixed:

| Vulnerability | Why Can't Fix | Mitigation | Residual Risk |
|---------------|---------------|------------|---------------|
| | No patch available | | |
| | Breaking changes | | |
| | Feature dependency | | |

### Dependency Hardening

| Recommendation | Benefit | Effort |
|----------------|---------|--------|
| Pin exact versions | Prevent surprise updates | Low |
| Vendor critical deps | Control over source | Medium |
| Add cargo-vet | Verify trusted reviewers | Medium |
| Set up Dependabot/Renovate | Automated updates | Low |

---

## 7) Required Output

### A. Executive Summary
- Total vulnerabilities found: [N]
- Critical: [N], High: [N], Medium: [N], Low: [N]
- Immediate action required: [Yes/No]
- Overall supply chain health: [Good/Acceptable/Concerning/Critical]

### B. Vulnerability Summary

| Severity | Count | Immediately Exploitable | Action Required |
|----------|-------|------------------------|-----------------|
| Critical | | | |
| High | | | |
| Medium | | | |
| Low | | | |

### C. Action Items (Prioritized)

| # | Action | Vulnerability Addressed | Effort | Deadline |
|---|--------|------------------------|--------|----------|
| 1 | | | | Immediate |
| 2 | | | | This week |
| ... | | | | |

### D. Risk Acceptance Recommendations

Vulnerabilities we recommend accepting (with justification):

| Vulnerability | Justification | Review Date |
|---------------|---------------|-------------|
| | | |

### E. Process Recommendations

Improvements to dependency management:

| Recommendation | Current State | Target State |
|----------------|---------------|--------------|

### F. Scan Artifacts

- Full cargo-audit output
- Full cargo-deny output
- SBOM (Software Bill of Materials) if generated

---

## Automation Setup

### Recommended CI Checks

```yaml
# Example GitHub Actions
- name: Security audit
  run: |
    cargo install cargo-audit
    cargo audit

- name: Dependency check
  run: |
    cargo install cargo-deny
    cargo deny check
```

### Alerting

- Set up Dependabot/Renovate for automated PRs
- Configure security advisory notifications
- Weekly scheduled scans

---

## Constraints

- Focus on exploitability, not just existence of vulnerabilities
- Consider our specific usage—is the vulnerable code path reachable?
- Prioritize by actual risk, not just CVSS score
- Provide clear upgrade paths, not just "fix this"
- Document risk acceptance decisions
