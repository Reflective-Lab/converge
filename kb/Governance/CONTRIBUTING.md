# Contributing to Converge

Thank you for your interest in contributing to Converge! We welcome contributions from the community.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

## How to Contribute

### Reporting Issues

- Use the issue tracker to report bugs or suggest features
- Include clear reproduction steps for bugs
- Provide context about your environment (Rust version, OS, features enabled)

### Development Setup

```bash
# Fork the repository
# Clone your fork
git clone https://github.com/your-username/converge.git
cd converge

# Set up upstream remote
git remote add upstream https://github.com/Reflective-Lab/converge.git

# Install Rust 1.94+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup override set 1.94.0

# Install dependencies
# For CUDA support: install CUDA toolkit
# For Vulkan support: install Vulkan SDK
```

### Building and Testing

```bash
# Build all crates
cargo build --workspace

# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p converge-core

# Run with specific features
cargo test --features "llama3,ndarray"

# Validate security/compliance repository declarations
just compliance-check
```

### Pull Request Process

1. Fork the repository and create your branch from `main`
2. Make your changes with clear commit messages
3. Ensure all tests pass
4. If you touch policy, runtime, auth, transport, or other public control surfaces, run:

```bash
just security-gate
```

5. Update documentation if needed
6. If data handling, security posture, or policy boundaries changed, update `SECURITY.md` and the relevant KB audit or plan pages
7. Submit a pull request to the `main` branch
8. Wait for code review and address feedback

### Code Style

- Follow Rust API guidelines
- Use 4-space indentation
- Include documentation comments for public items
- Follow the existing code patterns in each crate
- Keep line length under 100 characters when possible

### Commit Messages

Use conventional commits format:

```
feat: add new feature
fix: fix bug
docs: update documentation
style: formatting changes
refactor: code refactoring
perf: performance improvements
test: add tests
chore: maintenance tasks
```

### Feature Development

For major features:
1. Open an issue first to discuss the design
2. Create a feature branch
3. Implement with tests
4. Update documentation
5. Submit PR with reference to the issue

## License

By contributing to Converge, you agree that your contributions will be licensed under the MIT License.

## Contact

For questions about contributing, contact:
- Kenneth Pernyer: [kenneth@reflective.se](mailto:kenneth@reflective.se)
- Open an issue on GitHub
