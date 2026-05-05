# Security Policy

This policy covers vulnerability reporting for the repository.

## Supported Versions

We provide security updates for the following versions:

| Version | Supported          |
|---------|--------------------|
| 3.0.x   | :white_check_mark: |
| 2.x     | :x:                |
| < 2.0   | :x:                |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

Report vulnerabilities through [GitHub Security Advisories](https://github.com/Reflective-Lab/converge/security/advisories/new)
or by emailing **Kenneth Pernyer** at [kenneth@reflective.se](mailto:kenneth@reflective.se).

You should receive a response within 48 hours. If for some reason you do not, please follow up via email to ensure we received your original message.

Please include the following information in your report:

- The version of Converge you're using
- A description of the vulnerability
- Steps to reproduce the issue
- Any relevant logs or error messages
- Your assessment of the impact (CVSS score if possible)

## Security Update Process

1. **Acknowledgment**: We will acknowledge your report within 48 hours
2. **Assessment**: We will assess the vulnerability and determine its impact
3. **Patch Development**: We will develop a fix and test it thoroughly
4. **Release**: We will release the fix in a new version
5. **Disclosure**: We will publicly disclose the vulnerability after the fix is available

## Built-in Security Practices

- `unsafe_code = "forbid"` across all crates
- Dependency auditing via `cargo-deny` (RUSTSEC advisories + license compliance)
- Clippy pedantic lints enforced in CI
- Secrets handled via `zeroize` (opt-in `secure` feature on `converge-provider`)
- Ed25519 signed delegation tokens in `converge-policy`

## Current Security Baseline

The current runtime and policy control-surface baseline is recorded in
[kb/Architecture/Audits/2026-04-11 Security Review.md](kb/Architecture/Audits/2026-04-11%20Security%20Review.md).

That review is the source of truth for the latest closed findings and the
expected fail-closed posture on authority, authentication, logging, and
transport defaults.

## Security Regression Gate

Changes touching policy, runtime, auth, transport, or public control surfaces
must pass:

```bash
just security-gate
```

The gate currently runs:

```bash
cargo check --workspace
cargo test -p converge-policy
cargo test -p converge-runtime --lib
cargo test -p converge-pack --test compile_fail
cargo test -p converge-core --test compile_fail --test truth_pipeline --test negative --test properties
cargo test -p converge-client --test messages
```

Release candidates should also pass the dependency audit gate:

```bash
just security-audit
```

## Shared Responsibility

This repository provides a secure development baseline and reference runtime
patterns, but production compliance depends on deployment-specific controls.

Deployers are responsible for:

- infrastructure hardening and patching
- identity provider and access control configuration
- encryption key management and rotation
- retention, deletion, and privacy controls
- vendor review and subprocessor management
- legal/regulatory scoping for sensitive workloads

## Compliance Declarations

The project is designed to support enterprise security reviews, but this
repository does not itself claim certification or regulatory compliance unless
separately documented with evidence.

In particular, do not treat this repository alone as a declaration of:

- SOC 2 certification
- ISO 27001 certification
- HIPAA compliance
- PCI DSS compliance
- GDPR compliance

## Security Best Practices

When using Converge in production:

- Keep your dependencies updated
- Use the latest stable version
- Enable the `secure` feature on `converge-provider` for secret zeroization
- Follow the principle of least privilege
- Monitor your systems for unusual activity
- Use secure communication channels (TLS)

## Security Contact

For security-related questions or concerns:

**Kenneth Pernyer**
- Email: [kenneth@reflective.se](mailto:kenneth@reflective.se)
- PGP Key: Available upon request

## Responsible Disclosure

We ask security researchers to:

- Give us reasonable time to respond to your report before making it public
- Avoid exploiting the vulnerability in production systems
- Avoid violating privacy laws or disrupting services
- Provide sufficient detail to reproduce the issue

We commit to:

- Responding promptly to security reports
- Providing regular updates on our progress
- Crediting reporters in our security advisories (unless anonymous)
- Releasing fixes in a timely manner
