<!--
SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>

SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
-->

# `submod`

[![Crates.io](https://img.shields.io/crates/v/submod.svg)](https://crates.io/crates/submod)
[![Documentation](https://docs.rs/submod/badge.svg)](https://docs.rs/submod)
[![Static Badge](https://img.shields.io/badge/Plain-MIT-15db95?style=flat-square&labelColor=0d19a3&cacheSeconds=86400&link=https%3A%2F%2Fplainlicense.org%2Flicenses%2Fpermissive%2Fmit%2Fmit%2F)](https://plainlicense.org/licenses/permissive/mit/)
[![Rust](https://img.shields.io/badge/rust-1.87%2B-blue.svg)](https://www.rust-lang.org)
[![codecov](https://codecov.io/gh/bashandbone/submod/branch/main/graph/badge.svg?token=MOW92KKK0G)](https://codecov.io/gh/bashandbone/submod)
![Crates.io Downloads (latest version)](https://img.shields.io/crates/dv/submod)

A lightweight, fast CLI tool for managing git submodules with advanced sparse checkout support. Built on top of `gitoxide` and `git2` libraries for maximum performance and reliability.

## 🚀 Features

- **TOML-based configuration** - Define submodules, sparse-checkout paths, and settings in a simple config file
- **Global defaults with overrides** - Set project-wide submodule settings with per-submodule customization
- **Sparse checkout support** - Efficiently checkout only the parts of submodules you need
- **Fast operations** - Leverages `gitoxide` for high-performance git operations
- **Robust fallbacks** - Automatic fallback to `git2` and CLI when needed
- **Comprehensive commands** - Add, check, init, update, reset, and sync submodules with ease
- **Developer-friendly** - Clear status reporting and error messages

## 📋 Table of Contents

- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Configuration](#-configuration)
- [Commands](#-commands)
- [Usage Examples](#-usage-examples)
- [Development](#-development)
- [Contributing](#-contributing)
- [License](#-license)

## 🔧 Installation

### Using Cargo

```bash
cargo install submod
```

### Using Mise

[Mise](https://mise.jdx.dev/) is a project management tool and package manager that can manage your development environment.

```bash
# Global installation
mise use -g cargo:submod@latest

# Project-specific installation
mise use cargo:submod@latest
```

### From Source

```bash
git clone https://github.com/yourusername/submod.git
cd submod
cargo install --path .
```

## 🚀 Quick Start

1. **Initialize a config file** in your git repository:

    ```bash
    # Create a basic submod.toml configuration
    cat > submod.toml << EOF
    [defaults]
    ignore = "dirty"

    [my-submodule]
    path = "vendor/my-lib"
    url = "https://github.com/example/my-lib.git"
    sparse_paths = ["src/", "include/", "*.md"]
    EOF
    ```

2. **Initialize your submodules**:

    ```bash
    submod init
    ```

3. **Check status**:

    ```bash
    submod check
    ```

## ⚙️ Configuration

Create a `submod.toml` file in your repository root:

```toml
# Global defaults applied to all submodules
[defaults]
ignore = "dirty"          # ignore dirty state in status
update = "checkout"       # update method
branch = "main"          # default branch to track

# Individual submodule configuration
[vendor-utils]
path = "vendor/utils"
url = "https://github.com/example/utils.git"
sparse_paths = ["src/", "include/", "*.md"]
ignore = "all"           # override default ignore setting
active = true            # whether submodule is active

[my-submodule]
path = "libs/my-submodule"
url = "https://github.com/example/my-submodule.git"
sparse_paths = ["src/core/", "docs/"]
branch = "develop"       # track specific branch
```

### Configuration Options

#### Global Defaults

- `ignore`: How to handle dirty submodules (`all`, `dirty`, `untracked`, `none`)
- `update`: Update strategy (`checkout`, `rebase`, `merge`, `none`, `!command`)
- `branch`: Default branch to track (`.` for current superproject branch)
- `fetchRecurse`: Fetch recursion (`always`, `on-demand`, `never`)

#### Per-Submodule Settings

- `path`: Local path where submodule should be placed
- `url`: Git repository URL
- `sparse_paths`: Array of paths to include in sparse checkout
- `active`: Whether the submodule is active (default: `true`)
- All global defaults can be overridden per submodule

## 📖 Commands

### `submod add`

Add a new submodule to your configuration and repository:

```bash
# Basic add
submod add https://github.com/example/my-lib.git --name my-lib --path libs/my-lib

# With sparse checkout paths and extra options
submod add https://github.com/example/my-lib.git \
  --name my-lib \
  --path libs/my-lib \
  --sparse-paths "src/,include/" \
  --branch main \
  --ignore all \
  --fetch on-demand
```

**Options:**

| Flag | Short | Description |
|------|-------|-------------|
| `<URL>` | | *(required)* URL or local path of the submodule repository |
| `--name` | `-n` | Nickname for the submodule used in your config and commands |
| `--path` | `-p` | Local directory path where the submodule should be placed |
| `--branch` | `-b` | Branch to track |
| `--ignore` | `-i` | Dirty-state ignore level (`all`, `dirty`, `untracked`, `none`) |
| `--sparse-paths` | `-x` | Comma-separated sparse checkout paths or globs |
| `--fetch` | `-f` | Recursive fetch behavior (`always`, `on-demand`, `never`) |
| `--update` | `-u` | Update strategy (`checkout`, `rebase`, `merge`, `none`) |
| `--shallow` | `-s` | Shallow clone (last commit only) |
| `--no-init` | | Add to config only; do not clone/initialize |

### `submod check`

Check the status of all configured submodules:

```bash
submod check
```

### `submod init`

Initialize all missing submodules:

```bash
submod init
```

### `submod update`

Update all submodules to their latest commits:

```bash
submod update
```

### `submod reset`

Hard reset submodules (stash changes, reset --hard, clean):

```bash
# Reset all submodules
submod reset --all

# Reset specific submodules (comma-separated)
submod reset my-lib,vendor-utils
```

### `submod sync`

Run a complete sync (check + init + update):

```bash
submod sync
```

### `submod change`

Change the configuration of an existing submodule:

```bash
submod change my-lib --branch main --sparse-paths "src/,include/" --fetch always
```

### `submod change-global`

Change global defaults for all submodules:

```bash
submod change-global --ignore dirty --update checkout
```

### `submod list`

List all configured submodules:

```bash
submod list
submod list --recursive
```

### `submod delete`

Delete a submodule from configuration and filesystem:

```bash
submod delete
```

### `submod disable`

Disable a submodule without deleting files (sets `active = false`):

```bash
submod disable
```

### `submod nuke-it-from-orbit`

Delete all or specific submodules from config and filesystem, with optional reinit:

```bash
# Nuke all submodules (re-initializes by default)
submod nuke-it-from-orbit --all

# Nuke specific submodules permanently
submod nuke-it-from-orbit --kill my-lib,old-dep
```

### `submod generate-config`

Generate a new configuration file:

```bash
# From current git submodule setup
submod generate-config --from-setup .

# As a template with defaults
submod generate-config --template --output my-config.toml
```

### `submod completeme`

Generate shell completion scripts:

```bash
submod completeme bash   # or: zsh, fish, powershell, elvish, nushell
```

## 💻 Usage Examples

### Basic Workflow

```bash
# Start with checking current state
submod check

# Initialize any missing submodules
submod init

# Update everything to latest
submod update

# Or do it all at once
submod sync
```

### Adding Submodules with Sparse Checkout

```bash
# Add a submodule that only checks out specific directories
submod add https://github.com/company/react-components.git \
  --name react-components \
  --path src/components \
  --sparse-paths "src/Button/,src/Input/,README.md"
```

### Working with Different Configurations

```bash
# Use a custom config file
submod --config my-custom.toml check

# Check status with custom config
submod --config production.toml sync
```

### Handling Problematic Submodules

```bash
# Reset a problematic submodule
submod reset my-problematic-submodule

# Check what's wrong
submod check

# Re-sync everything
submod sync
```

## 🛠️ Development

### Prerequisites

- Rust 1.87 or later
- Git
- [Mise](https://mise.jdx.dev/) (recommended) - for tool management and task running

### Quick Setup with Mise (Recommended)

```bash
# Clone the repository
git clone https://github.com/bashandbone/submod.git
cd submod

# Install mise if you haven't already
curl https://mise.run | sh

# Install all development tools and dependencies
mise install

# Build the project
mise run build
# or: mise run b (alias)

# Run tests
mise run test

# Run the full CI suite (build + lint + test)
mise run ci
```

### Available Mise Tasks

```bash
# Build the project
mise run build          # or: mise run b

# Run tests
mise run test

# Lint with clippy
mise run lint

# Run full CI pipeline
mise run ci

# Clean build artifacts
mise run clean

# Cut a new release (maintainers only)
mise run release
```

### Git Hooks with hk

This project uses [hk](https://github.com/jdx/hk) for automated git hooks that ensure code quality:

```bash
# Install git hooks (done automatically with mise install)
hk install

# Run pre-commit checks manually
hk run pre-commit

# Run all linters and checks
hk check

# Auto-fix issues where possible
hk fix

# Run CI checks locally
hk run ci
```

The pre-commit hooks automatically run:
- **cargo fmt** - Code formatting
- **cargo clippy** - Linting
- **cargo test** - Test suite
- **typos** - Spell checking
- **prettier** - TOML/YAML formatting
- **cargo deny** - Security and license auditing

### Manual Setup (Alternative)

If you prefer not to use mise:

```bash
# Clone the repository
git clone https://github.com/bashandbone/submod.git
cd submod

# Install Rust if needed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build the project
cargo build

# Run tests
cargo test
# or hk run test

# Or use the comprehensive test runner
./scripts/run-tests.sh --verbose
```

### Running Tests

```bash
# Using mise (recommended)
mise run test           # Run all tests
mise run ci             # Run full CI suite

# Using hk
hk run test                 # Run tests only
hk run ci                   # Run CI checks

# Using cargo directly
cargo test              # Run all tests
cargo test --test integration_tests  # Integration tests only

# Using the test script
./scripts/run-tests.sh --verbose     # Comprehensive reporting
./scripts/run-tests.sh --performance # Include performance tests
./scripts/run-tests.sh --filter sparse_checkout  # Filter tests
```

### Project Structure

```plaintext
submod/
├── src/
│   ├── main.rs              # CLI entry point
│   ├── commands.rs          # Command definitions (clap)
│   ├── config.rs            # TOML configuration handling
│   ├── git_manager.rs       # High-level submodule operations
│   └── git_ops/             # Git backend abstraction
│       ├── mod.rs           # GitOpsManager (gix→git2→CLI fallback)
│       ├── gix_ops.rs       # gitoxide backend
│       └── git2_ops.rs      # libgit2 backend
├── tests/                   # Integration tests
├── sample_config/           # Example configurations
├── scripts/                 # Development scripts
└── docs/                    # Documentation
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Quick Contributing Steps

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/amazing-feature`
3. **Set up development environment**: `mise install` (installs all tools and git hooks)
4. **Make your changes** and add tests if applicable
5. **Commit your changes**: `git commit -m 'Add amazing feature'` (hooks run automatically)
6. **Push to your branch**: `git push origin feature/amazing-feature` (they'll actually run again in check mode, so they need to pass)
7. **Open a Pull Request**

### Development Guidelines

- Follow Rust best practices and idioms
- Add tests for new functionality. I'm not big on unit tests, but integration tests are essential.
- Update documentation for user-facing changes
- Use conventional commit messages
- Run `mise run ci` or `hk run ci` before submitting PR
- Pre-commit hooks will automatically format code and run basic checks
- All automated checks must pass before PR can be merged

## 🔍 Troubleshooting

### Common Issues

**Submodule not initializing:**

```bash
# Check if the URL is accessible
git ls-remote <submodule-url>

# Verify your configuration
submod check
```

**Sparse checkout not working:**

- Ensure paths in `sparse_paths` are relative to the submodule root
- Check that the submodule repository contains the specified paths
- Verify sparse checkout is enabled: `git config core.sparseCheckout` in the submodule

**Permission issues:**

- Ensure you have proper SSH keys set up for private repositories
- Check if your Git credentials are configured correctly

## 📋 Motivation

Managing git submodules, especially with sparse checkouts, can be complex and error-prone. Traditional git submodule commands require multiple steps and careful attention to configuration details.

This tool was created to:

- **Reduce barriers to contribution** - Make it easier for new developers to work with projects using submodules
- **Simplify complex workflows** - Handle initialization, updates, and sparse checkout configuration automatically
- **Provide better tooling** - Clear status reporting and error messages
- **Leverage modern Git libraries** - Use `gitoxide` for better performance and reliability

The tool is actively used in multiple projects at [@knitli](https://github.com/knitli) and [@plainlicense](https://github.com/plainlicense), where submodules are essential for sharing core functionality across repositories.

## 📄 License

This project is licensed under the [Plain MIT License](https://plainlicense.org/licenses/permissive/mit/).

## 🙏 Acknowledgments

- [gitoxide](https://github.com/Byron/gitoxide) - Fast and safe pure Rust implementation of Git
- [git2-rs](https://github.com/rust-lang/git2-rs) - Rust bindings to libgit2
- [clap](https://github.com/clap-rs/clap) - Command line argument parser

---

<div align="center">

**[Homepage](https://github.com/bashandbone/submod)** • **[Documentation](https://docs.rs/submod)** • **[Crate](https://crates.io/crates/submod)**

Made with ❤️ for the Rust and Git communities

</div>
