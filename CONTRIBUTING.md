# Contributing to `submod`

Thank you for your interest in contributing to `submod`! This document provides guidelines and information for contributors to help make the process smooth and effective.

## 📋 Table of Contents

- [Code of Conduct](#-code-of-conduct)
- [Getting Started](#-getting-started)
- [Development Setup](#-development-setup)
- [Making Changes](#-making-changes)
- [Testing](#-testing)
- [Pull Request Process](#-pull-request-process)
- [Coding Standards](#-coding-standards)
- [Documentation](#-documentation)
- [Issue Guidelines](#-issue-guidelines)
- [Release Process](#-release-process)

## 🤝 Code of Conduct

This project adheres to a Code of Conduct that we expect all contributors to follow. Please be respectful, inclusive, and professional in all interactions.

### Our Standards

- **Be respectful** - Treat everyone with respect and kindness
- **Be inclusive** - Welcome newcomers and help them succeed
- **Be collaborative** - Work together constructively
- **Be constructive** - Provide helpful feedback and suggestions
- **Be patient** - Remember that everyone has different experience levels

## 🚀 Getting Started

### Prerequisites

Before contributing, ensure you have:

- **Rust 1.87+** - Latest stable version recommended
- **Git** - For version control
- **Basic Git knowledge** - Understanding of branches, commits, and pull requests
- **GitHub account** - For submitting contributions

### Areas for Contribution

We welcome contributions in several areas:

- **🐛 Bug fixes** - Help us squash bugs and improve reliability
- **✨ New features** - Add functionality that benefits users
- **📚 Documentation** - Improve clarity and completeness
- **🧪 Tests** - Expand test coverage and add edge cases
- **🔧 Performance** - Optimize operations and reduce resource usage
- **🎨 UX improvements** - Better error messages and user experience

## 🛠️ Development Setup

### 1. Fork and Clone

```bash
# Fork the repository on GitHub, then clone your fork
git clone https://github.com/yourusername/submod.git
cd submod

# Add the upstream remote
git remote add upstream https://github.com/originaluser/submod.git
```

### 2. Install Mise and Dependencies

```bash
# Install mise (if you haven't already)
curl https://mise.run | sh

# Install all development tools and dependencies
mise install

# This automatically installs:
# - Rust 1.87+
# - hk (git hooks)
# - cargo tools (nextest, audit, deny, watch)
# - prettier, typos, and other linters
```

### 3. Verify Setup

```bash
# Build the project
mise run build

# Run tests to ensure everything works
mise run test

# Run the full CI suite
mise run ci

# Verify git hooks are installed
hk --version
```

### 4. Development Workflow

With mise and hk installed, you have access to streamlined development commands:

```bash
# Development tasks
mise run build         # Build the project
mise run test          # Run tests
mise run lint          # Run clippy
mise run ci            # Full CI pipeline

# Git hooks (run automatically on commit)
hk pre-commit          # Run pre-commit checks manually
hk fix                 # Auto-fix issues where possible
hk check               # Run all linters
```

### 5. Manual Setup (Alternative)

If you prefer not to use mise:

```bash
# Install Rust manually
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools manually
cargo install cargo-watch      # Watch for changes
cargo install cargo-audit      # Security auditing
cargo install cargo-deny       # Dependency checking
cargo install cargo-nextest    # Better test runner

# Build and test
cargo build
cargo test
./scripts/run-tests.sh
```

## 🔧 Development Tools Overview

This project uses modern development tools to streamline the contribution process:

### Mise - Task Runner & Tool Manager

[Mise](https://mise.jdx.dev/) manages our development environment and provides consistent task execution:

```bash
# Available tasks
mise run build         # Build the project (alias: mise run b)
mise run test          # Run the test suite
mise run lint          # Run clippy linting
mise run ci            # Full CI pipeline (build + lint + test)
mise run clean         # Clean build artifacts
mise run release       # Cut a new release (maintainers only)
```

### hk - Git Hooks Manager

[hk](https://github.com/jdx/hk) provides automated git hooks for code quality:

```bash
# Hook commands
hk run pre-commit          # Run pre-commit checks
hk run pre-push            # Run pre-push checks
hk check                   # Run all linters
hk fix                     # Auto-fix issues where possible
hk run test                # Run tests only
hk run ci                  # Run CI checks
```

### Automated Quality Checks

The pre-commit hooks automatically run these tools on every commit:

- **cargo fmt** - Formats Rust code
- **cargo clippy** - Lints Rust code for common issues
- **cargo test** - Runs the test suite (with nextest for parallel execution)
- **typos** - Checks for spelling errors in code and documentation
- **prettier** - Formats TOML, YAML, and other configuration files
- **cargo deny** - Audits dependencies for security vulnerabilities and license compliance
- **pkl** - Validates pkl configuration files

### Tool Integration

Both tools work together seamlessly:

- **mise** handles tool installation and version management
- **hk** uses the tools installed by mise for git hooks
- Both can run the same underlying commands (e.g., `mise run test` and `hk run test`)
- CI uses the same tools for consistency between local and remote environments

## 🔄 Making Changes

### 1. Create a Branch

```bash
# Sync with upstream
git fetch upstream
git checkout main
git merge upstream/main

# Create a feature branch
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-number-description
```

### 2. Branch Naming Conventions

- **Features**: `feature/description` (e.g., `feature/sparse-checkout-improvements`)
- **Bug fixes**: `fix/issue-description` (e.g., `fix/config-parsing-error`)
- **Documentation**: `docs/description` (e.g., `docs/api-documentation`)
- **Refactoring**: `refactor/description` (e.g., `refactor/error-handling`)
- **Tests**: `test/description` (e.g., `test/integration-coverage`)

### 3. Making Quality Commits

#### Commit Message Format

Briefly describe your changes in the commit message. Keep commits focused and atomic.

````plaintext

feat: Add support for super-unicorn submodules :unicorn:

## 🧪 Testing

My philosophy on testing is "test what matters." Tests focus on integration and output -- if the tool performs as expected in realistic tests, then it's good. I'm not a fan of a flurry of unit tests that test implementation details and create a maintenance burden.

### Test Categories

1. **Unit Tests** - We currently don't have unit tests, but they can be added in the future for critical functionality.

   ```bash
   cargo test --test unit_tests
   ```

2. **Integration Tests** - Test complete workflows

    ```bash
    cargo test --test integration_tests
    ```

3. **Configuration Tests** - Test TOML parsing and validation

    ```bash
    cargo test --test config_tests
    ```

4. **Sparse Checkout Tests** - Test sparse checkout functionality

    ```bash
    cargo test --test sparse_checkout_tests
    ```

5. **Error Handling Tests** - Test error conditions and edge cases

    ```bash
    cargo test --test error_handling_tests
    ```

### Running All Tests

```bash
# Using mise (recommended)
mise run test           # Quick test run
mise run ci             # Full CI suite (build + lint + test)

# Using hk
hk run test                 # Run tests only
hk run ci                   # Run CI checks
hk check                    # Run all linters and checks

# Using cargo directly
cargo test              # Quick test run

# Using the test script -- more granular control
./scripts/run-tests.sh --verbose     # Comprehensive test suite with reporting
./scripts/run-tests.sh --performance # Include performance tests
./scripts/run-tests.sh --filter sparse_checkout  # Filter specific tests
```

### Writing Tests

#### Unit Test Example

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing() {
        let config_str = r#"
            [defaults]
            ignore = "dirty"

            [test-submodule]
            path = "test/path"
            url = "https://example.com/repo.git"
        "#;

        let config: Config = toml::from_str(config_str).unwrap();
        assert_eq!(config.submodules.len(), 1);
    }
}
```

#### Integration Test Example

```rust
use tempfile::TempDir;
use std::process::Command;

#[test]
fn test_submod_init_command() {
    let temp_dir = TempDir::new().unwrap();
    // Set up test repository and config
    // Run submod commands
    // Assert expected outcomes
}
```

### Test Requirements for PRs

- **All existing tests must pass** (unless you have a good reason...)
- **New features must include tests** (at least integration tests)
- **Bug fixes should include regression tests**

## 📋 Pull Request Process

### 1. Pre-submission Checklist

Before submitting your PR, ensure:

- [ ] **Code compiles** without warnings
- [ ] **All tests pass** (will run in pre-commit and pre-push)
- [ ] **Pre-commit hooks pass** (automatically run on commit, or manually with `hk run pre-commit`)
- [ ] **Documentation is updated** if needed
- [ ] **CHANGELOG is updated** for user-facing changes
- [ ] **Commit messages follow conventions**

**Note**: If you're using the recommended mise/hk setup, many checks are automated:

- **Code formatting** (`cargo fmt`) - Auto-fixed by pre-commit hooks
- **Linting** (`cargo clippy`) - Checked by pre-commit hooks
- **Spell checking** (`typos`) - Checked and auto-fixed by pre-commit hooks
- **TOML/YAML formatting** (`prettier`) - Auto-fixed by pre-commit hooks
- **Security auditing** (`cargo deny`) - Checked by pre-commit hooks

### 2. Submitting the PR

```bash
# Push your branch
git push origin feature/your-feature-name

```

### 3. PR Description Template

```markdown
## Description

Brief description of the changes and motivation.

## Type of Change

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that causes existing functionality to change)
- [ ] Documentation update

## Testing

- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes
- [ ] I have run the full test suite (`./scripts/run-tests.sh`)

## Checklist

- [ ] My code follows the project's style guidelines
- [ ] I have performed a self-review of my own code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings

## Related Issues

Fixes #(issue number)
```

### 4. Review Process

- **Automated checks** must pass (CI/CD)
- **At least one maintainer review** required
- **Address feedback** promptly and thoroughly
- **Squash commits** if requested before merge

## 📖 Coding Standards

### Rust Style Guidelines

We follow standard Rust conventions with some project-specific guidelines:

#### Code Formatting

Just use `mise` or `hk`
```bash
# Format all code
mise run fix
# or
hk fix

# if you're really anti-mise/hk, then you can use cargo directly
cargo fmt
```

# Check formatting without changing files
```bash
mise run check
# or
hk check

# or if you're really anti-mise/hk, then you can use cargo directly
cargo fmt --check
```

#### Linting

```bash
# the above mise and hk commands also run clippy
# Again, if you're a purist:
cargo clippy --all-targets --all-features -- -D warnings
```

#### Specific Guidelines

1. **Error Handling**

    - Use `anyhow::Result` for application errors
    - Use `thiserror` for library errors
    - Provide context with `.with_context()`

2. **Documentation**

    - All public APIs must have doc comments
    - Include examples in doc comments when helpful
    - Use `#[doc = "..."]` for complex documentation

3. **Naming Conventions**

    - Use descriptive names for variables and functions
    - Prefer full words over abbreviations
    - Use `snake_case` for functions and variables
    - Use `PascalCase` for types and enums

4. **Code Organization**
    - Group related functionality into modules
    - Keep functions focused and single-purpose
    - Use appropriate visibility (`pub`, `pub(crate)`, private)

#### Example Code Style

````rust
use anyhow::{Context, Result};
use std::path::Path;

/// Represents the configuration for a git submodule.
///
/// # Examples
///
/// ```
/// use submod::SubmoduleConfig;
///
/// let config = SubmoduleConfig {
///     path: Some("vendor/lib".to_string()),
///     url: Some("https://github.com/example/lib.git".to_string()),
///     // ...
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmoduleConfig {
    /// Local filesystem path for the submodule
    pub path: Option<String>,
    /// Git repository URL
    pub url: Option<String>,
    // ... other fields
}

impl SubmoduleConfig {
    /// Creates a new submodule configuration with default values.
    pub fn new() -> Self {
        Self {
            path: None,
            url: None,
            // ... initialize other fields
        }
    }

    /// Validates the submodule configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if required fields are missing or invalid.
    pub fn validate(&self) -> Result<()> {
        self.path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Path is required"))
            .with_context(|| "Validating submodule configuration")?;

        // ... additional validation
        Ok(())
    }
}
````

## 📚 Documentation

### Documentation Requirements

- **Public APIs** - All public functions, structs, and modules. We aren't a library, so there really _isn't_ a public API, but we still document everything.
- **Complex algorithms** - Explain the approach and reasoning (if you need them for a submodule handling tool... I'll have questions)
- **Configuration options** - Document all settings and their effects
- **Error conditions** - When and why functions might fail

### Documentation Style

Common sense documentation style applies. If a function's purpose is obvious and it's well-typed, a sentence is probably enough. If it has complex logic or side effects, provide a detailed explanation.

````rust,ignore
/// Short one-line description.   // <-- stop here for obvious functions
///
/// Longer description explaining the purpose, behavior, and any important
/// details about the function or type.
///
/// # Arguments
///
/// * `param1` - Description of the first parameter
/// * `param2` - Description of the second parameter
///
/// # Returns
///
/// Description of what is returned.
///
/// # Errors
///
/// Description of when this function will return an error.
///
/// # Examples
///
/// ```
/// use submod::example_function;
///
/// let result = example_function("input").unwrap();
/// assert_eq!(result, "expected");
/// ```
pub fn example_function(input: &str) -> Result<String> {
    // Implementation...
}
````

### Updating Documentation

- **README.md** - For user-facing changes
- **API docs** - For code changes
- **CHANGELOG.md** - For all notable changes
- **Configuration docs** - For new config options

## 🐛 Issue Guidelines

### Reporting Bugs

When reporting bugs, please include:

1. **Clear title** describing the issue
2. **Environment details** (OS, Rust version, submod version)
3. **Steps to reproduce** the issue
4. **Expected behavior** vs actual behavior
5. **Error messages** or logs if available
6. **Configuration file** (sanitized if needed)

### Bug Report Template

````markdown
**Describe the bug**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:

1. Run command '...'
2. See error

**Expected behavior**
A clear description of what you expected to happen.

**Environment:**

- OS: [e.g. Ubuntu 22.04, macOS 13.0, Windows 11]
- Rust version: [e.g. 1.75.0]
- submod version: [e.g. 0.1.0]

**Configuration:**

```toml

# Your configuration file content here
# If you have super secret private repos on it
# feel free to censor/change them
```

**Additional context**
Add any other context about the problem here.

### Feature Requests

For feature requests, please include:

1. **Problem description** - What problem does this solve?
2. **Proposed solution** - How should it work?
3. **Alternatives considered** - Other approaches you've thought about
4. **Use cases** - Who would benefit and how?

## 🚀 Release Process

### Versioning

We use [Semantic Versioning](https://semver.org/):

- **MAJOR** version for incompatible API changes
- **MINOR** version for new functionality (backwards compatible)
- **PATCH** version for bug fixes (backwards compatible)

### Release Checklist (Maintainers)

1. **Update version** in `Cargo.toml`
2. **Update CHANGELOG.md** with release notes
3. **Run full test suite** to ensure stability
4. **Create release commit** and tag
5. **Publish to crates.io**
6. **Create GitHub release** with notes

## 🎉 Recognition

Contributors are recognized in several ways:

- **Listed in releases** - Notable contributions mentioned in release notes
- **GitHub contributors page** - Automatic recognition for all contributors

## 💬 Getting Help

If you need help with contributing:

- **Open a discussion or issue** on GitHub for questions
- **Check existing issues** for similar questions
- **Read the documentation** thoroughly first

## 🙏 Thank You

Every contribution, no matter how small, makes `submod` better for everyone. We appreciate your time and effort in helping improve this tool!

---

_This contributing guide is a living document. If you find areas for improvement, please suggest changes!_
