# Contributing to PrintBridge

Thank you for your interest in contributing to PrintBridge! We welcome contributions from developers of all skill levels. This document outlines how to get started and best practices for contributing.

## Code of Conduct

We are committed to providing a welcoming and inclusive environment for all contributors. Please be respectful and constructive in all interactions.

## Getting Started

### Prerequisites

- Windows 10/11 for testing (Linux for cross-compilation)
- Rust 1.70 or later
- Node.js 16 or later
- Git

### Setting Up Your Development Environment

1. **Fork and clone the repository**

   ```bash
   git clone https://github.com/YOUR_USERNAME/printbridge.git
   cd printbridge
   ```

2. **Build the Rust project**

   ```bash
   cargo build
   cargo run  # Run development version
   ```

3. **Build the TypeScript SDK**
   ```bash
   cd sdk
   npm install
   npm run build  # Or npm run dev for watch mode
   ```

### Project Structure

- `src/` - Rust desktop client source code
- `sdk/` - TypeScript browser SDK
- `scripts/` - Installation and setup scripts
- `.github/workflows/` - CI/CD workflows
- `Cargo.toml` - Rust manifest
- `sdk/package.json` - SDK manifest

## Making Changes

### Branching

- Create a new branch for each feature or fix
- Use descriptive branch names: `feature/add-usb-support` or `fix/memory-leak`
- Always branch from `main`

### Code Style

#### Rust

- Follow the official Rust style guidelines
- Run `cargo fmt` before committing
- Run `cargo clippy` to check for common mistakes
- Write descriptive commit messages

#### TypeScript

- Follow Google's TypeScript style guide
- Use `npm run build` to verify your changes
- Add type annotations for all public APIs
- Write JSDoc comments for exported functions/classes

### Commits

- Make atomic commits (one logical change per commit)
- Write clear, descriptive commit messages
- Use the imperative mood: "add feature" not "added feature"
- Reference issues when applicable: `Fixes #123`

Example:

```
feat: Add USB printer support

Implement direct USB printer communication for devices
that don't use ESC/POS. Reduces latency and improves
compatibility with non-thermal printers.

Fixes #145
```

## Testing

### Running Tests

```bash
# Rust tests
cargo test

# TypeScript tests
cd sdk && npm test

# Build for Windows (Linux with cross-compile)
cargo build --release --target x86_64-pc-windows-gnu
```

### Manual Testing

1. Build and run locally
2. Test with your printer setup
3. Verify both service and browser connectivity
4. Check Windows Event Viewer for any errors

## Pull Requests

### Before Submitting

- Ensure all tests pass: `cargo test && cargo clippy`
- Format your code: `cargo fmt`
- Sync with latest main: `git pull origin main`
- Add tests for new functionality
- Update documentation if needed

### PR Guidelines

- Use a clear title: `feat: Add X` or `fix: Resolve Y`
- Provide a detailed description of changes
- Include screenshots/GIFs for UI changes
- Link related issues: `Closes #123`
- Keep commits clean and organized

Example PR description:

```markdown
## Description

Implements thermal printer detection for automatic printer selection.

## Changes

- Added device capability detection
- Implement automatic ESC/POS detection
- Add configuration option for default printer

## Testing

- Tested with thermal and inkjet printers
- Verified fallback behavior
- No regression in existing functionality

## Screenshots

[Include relevant screenshots]

Closes #456
```

## Areas Where We Need Help

- **Platform Support**: macOS and Linux ports
- **Documentation**: Improving guides and examples
- **Testing**: Regression and edge-case testing
- **Performance**: Optimization opportunities
- **Features**: New printer models and commands
- **Examples**: Demo applications and integrations

## Reporting Bugs

1. Check [existing issues](https://github.com/printbridge/printbridge/issues) first
2. Use the bug report template
3. Provide:
   - Clear description of the issue
   - Steps to reproduce
   - Expected vs actual behavior
   - Windows version and PrintBridge version
   - Relevant logs or screenshots

## Requesting Features

1. Check [existing discussions](https://github.com/printbridge/printbridge/discussions)
2. Describe the use case and expected behavior
3. Suggest a potential implementation if possible
4. Be open to feedback and alternative approaches

## Development Workflow

```
main (stable)
  ↓
develop (staging)
  ↓
feature/xyz (your branch)
```

### Typical Flow

1. Create feature branch from `develop`
2. Make changes and commit regularly
3. Push to your fork
4. Open PR against `develop`
5. After review, PR merged to `develop`
6. Periodic releases merge `develop` → `main`

## Release Process

We use semantic versioning (MAJOR.MINOR.PATCH):

- Increment MAJOR for breaking changes
- Increment MINOR for new features
- Increment PATCH for bug fixes

Releases are tagged with `v*` and automatically published to GitHub Releases.

## Community

- **Issues**: Report bugs and request features
- **Discussions**: Ask questions and share ideas
- **Projects**: Track work in progress

## Questions?

Feel free to:

- Open an issue with the `question` label
- Start a discussion in the community section
- Check existing documentation and examples

## License

By contributing, you agree that your contributions will be licensed under the MIT License, consistent with the project's license.

---

**Thank you for making PrintBridge better! 🎉**
