---
tags: [governance, moc]
source: mixed
---
# Governance — How This Works

This section is a **meta-governance system** — it governs how Converge itself is built, not how the Converge runtime governs agent decisions.

The runtime has [[Philosophy/Nine Axioms|axioms]], [[Concepts/Invariants|invariants]], and [[Concepts/HITL Gates|HITL gates]]. This section has **personas, contracts, evals, and strategic gates** that ensure the project is built responsibly.

## What's Here

### 24 Personas (`personas/`)

Each persona is a lens for validating decisions. Before something ships, the relevant personas weigh in.

| Category | Personas | Core Question |
|---|---|---|
| **Core Technical** | System Architect, Security Auditor, QA Engineer, SRE | Does it work correctly and safely? |
| **Business & Legal** | Legal Counsel, Marketing Lead, Sales Engineer, Founder | Is it viable and honest? |
| **Responsibility** | Ethics & Safety, Sustainability, Build vs Buy, Developer Advocate | Is it responsible and efficient? |
| **External** | Spiritual Advisor, Curious Searcher, Investor, End User Advocate, Skeptical Critic, Future Historian | How do outsiders see us? |
| **Scrutiny** | Regulator, Journalist, Academic, Insurance Underwriter | Would we withstand examination? |
| **Product & Privacy** | Product Manager, Data Protection Officer | Are we building the right thing safely? |

Each persona directory has a `profile.md` and task-specific playbooks (e.g. `security-auditor/threat-model.md`).

### Phase Contracts (`contracts/`)

Which personas participate during each phase of work:

| Phase | When |
|---|---|
| Development | Building features |
| Testing | Quality validation |
| Review | Pre-release approval |
| Marketing | Positioning and content |
| Sales | Customer engagement |
| Production | Live operation |
| Retrospective | Learning and improvement |

Plus `phase-gates.md` (concrete checklists) and `eval-execution.md` (how/when evals run).

### Evals (`evals/`)

Lightweight checks run before gates. Shell scripts that copy eval prompts to clipboard or run through Claude Code.

```bash
# Single eval
./evals/run-eval.sh system-architect

# Gate suite
./evals/run-suite.sh release-critical --claude --output-dir reports/
```

| Suite | When | Evals |
|---|---|---|
| `pr-merge` | Before PR merge | architect, qa |
| `release-candidate` | Before tagging RC | architect, security, qa, sre |
| `release-critical` | Release approval (blocking) | architect, security, qa, legal |
| `release-full` | Release approval (complete) | All 15+ evals |
| `deploy` | Before production deploy | sre, security, qa |
| `marketing` | Before content publish | marketing, legal, ethics |

### Strategic Layer (`strategic/`)

A heavier governance layer with:
- **Cedar policies** — Formal access control and authority rules
- **Gate fixtures** — YAML pass/warn/stop scenarios for testing gates
- **Rust validator** — A standalone crate that validates gate executions
- **Drift detection** — Tracks when the project drifts from its strategic intent
- **Delivery policy** — TOML configs for SLAs, thresholds, routing

### Authority Tiers

| Tier | Power | Example |
|---|---|---|
| **Blocking-by-Policy** | Can veto at designated gates | Security Auditor blocks release for critical vuln |
| **Escalating** | Files escalation packets for Core review | Sustainability Lead flags carbon data |
| **Advisory** | Comments and suggestions only | Curious Searcher notes confusing UX |

6 Core (blocking): System Architect, QA, Security, Founder, Legal, Ethics.
Everything else is Extended (escalating or advisory).

### Key Files

- [[Governance/README]] — Full persona catalog
- [[Governance/GATES]] — Gate framework and taxonomy
- [[Governance/TEAM]] — Authority tiers and roster

## How It Connects to the Runtime

The runtime's governance ([[Concepts/Invariants]], [[Concepts/HITL Gates]], [[Concepts/Proposals and Promotion]]) governs **agent decisions at execution time**.

For consequential business transitions, the runtime now uses a default gated path:

- flow code projects state into a neutral `FlowGateInput`
- the **arbiter** extension (formerly `converge-policy`) evaluates Cedar through `FlowGateAuthorizer`
- the result is `promote`, `reject`, or `escalate`
- HITL approval resumes the same flow instead of bypassing governance

This section governs **human decisions at development time** — what to build, what to ship, what to say publicly.

Both follow the same principle: **no promotion without review** ([[Philosophy/Nine Axioms#1. Explicit Authority|Axiom 1]]).
