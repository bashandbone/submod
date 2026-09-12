

<!--
SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>

SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
-->

# `submod`

[![Crates.io](https://img.shields.io/crates/v/submod.svg)](https://crates.io/crates/submod)
[![Documentation](https://docs.rs/submod/badge.svg)](https://docs.rs/submod)
[![Static Badge](https://img.shields.io/badge/Plain-MIT-15db95?style=flat-square&labelColor=0d19a3&cacheSeconds=86400&link=https%3A%2F%2Fplainlicense.org%2Flicenses%2Fpermissive%2Fmit%2Fmit%2F)](https://plainlicense.org/licenses/permissive/mit/)
[![Rust](https://img.shields.io/badge/rust-1.89%2B-blue.svg)](https://www.rust-lang.org)
[![codecov](https://codecov.io/gh/bashandbone/submod/branch/main/graph/badge.svg?token=MOW92KKK0G)](https://codecov.io/gh/bashandbone/submod)
![Crates.io Downloads (latest version)](https://img.shields.io/crates/dv/submod)

Git submodules solve a real problem. **Managing submodules is a pain.** You use them infrequently enough that you always forget which command does what — and when something breaks, the recovery steps are a small nightmare. New contributors hit this especially hard: onboarding onto a project that uses submodules is its own obstacle course.

`submod` manages Git submodules from a TOML configuration. Lifecycle mutations use native Git, with repository and path validation before changes. Read operations also use gitoxide and git2. Git must be installed and available on `PATH`.

## :rocket: Features

- **TOML config** — define submodules, sparse-checkout paths, and defaults in one file
- **Sparse checkout** — check out only the files and directories you need
- **Global defaults with per-submodule overrides** — set it once, customize where it matters
- **Native Git lifecycle** — preserve Git registration, parent pins, and recoverable module history
- **Clear status and errors** — you'll know what broke and why


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
git clone https://github.com/bashandbone/submod.git
cd submod
cargo install --path .
```

## 🚀 Quick Start

The `example` and `company` repository URLs below are placeholders. Replace them with repositories you can access; valid TOML does not guarantee that a remote exists. Git credential helpers, SSH agents, and transport restrictions still apply.

For an existing Git submodule setup, start with `submod generate-config --from-setup`, inspect the imported TOML, then run `submod check` and `submod sync`. Import reads the discovered repository; `--from-setup` is a boolean flag, not a path argument. Use `--output` to select the generated file and `--force` only to replace an existing output.

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

Create a `submod.toml` file in your repository root. Commands discover the enclosing worktree, so invoking them from a nested directory uses that same root and default config. A relative explicit `--config` path is resolved from the invocation directory; a missing explicit config is an error. Checkout paths are relative to the worktree root, must remain inside it, and cannot overlap Git administrative storage or other managed paths.

The TOML table name is the logical module name; `path` is its checkout location and defaults to that name. Submodules registered in Git but absent from TOML are unmanaged: submod reports and preserves them rather than adopting or deleting them automatically.

Example:

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
- `update`: Update strategy (`checkout`, `rebase`, `merge`, `none`); custom executable update commands are rejected
- `branch`: Default branch to track (`.` for current superproject branch)
- `fetchRecurse`: Fetch recursion (`always`, `on-demand`, `never`)
- `use_git_default_sparse_checkout`: Use Git's unprefixed sparse patterns (`false` by default)

An explicit per-module value overrides `[defaults]`; omitted fields inherit without being copied into the raw entry. Built-in defaults are `ignore = "none"`, `update = "checkout"`, `fetchRecurse = "on-demand"`, and no explicit branch. Use `submod change NAME --unset FIELD` to restore inheritance, or `submod change-global --unset FIELD` to remove a global default. `fetch` and `fetch_recurse` are accepted legacy aliases for canonical `fetchRecurse`; do not supply multiple spellings in one table.

#### Per-Submodule Settings

- `path`: Local path where submodule should be placed
- `url`: Required nonempty Git repository URL or local remote path
- `sparse_paths`: Ordered non-cone sparse patterns; absent or empty disables sparse checkout
- `active`: Whether automatic lifecycle work is enabled (default: `true`)
- `shallow`: Request shallow history (default: `false`)
- All global defaults can be overridden per submodule

Sparse checkout controls files in the working tree; it is not partial clone and does not by itself reduce downloaded objects or history. By default submod prepends `!/*` to the ordered patterns. Set `use_git_default_sparse_checkout = true` globally or per module to use the patterns without that prefix. `active` and sparse patterns stay in TOML/local configuration rather than portable `.gitmodules` fields.

The [sample configuration](sample_config/submod.toml) and [current JSON schema](schemas/current/submod_config.json) describe the current format. Historical versioned schemas remain available for their original contracts.

## 📖 Commands

### `submod add`

Add a new submodule to your configuration and repository. Existing declarations and occupied destinations are refused without replacing their contents; use init/sync for an existing managed module:

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
| `--shallow` | `-s` | Request shallow clone history |
| `--no-init` | | Add to config only; do not clone/initialize |

### `submod check`

Check the status of all configured submodules:

```bash
submod check
```

*alias*: `submod c`

### `submod init`

Initialize all missing submodules:

```bash
submod init
```

*alias*: `submod i`

### `submod update`

Materialize the commit recorded by the parent gitlink. Remote advancement is explicit:

```bash
# Parent-pin update (default)
submod update

# Fetch and apply the configured remote branch/default
submod update --remote
```

`--branch` selects the tracking branch; it does not make ordinary updates remote-tracking updates. Checkout, merge, and rebase follow the configured strategy; `update = "none"` skips automatic checkout. Review and stage changed parent gitlinks when adopting a remote update. Use `--recursive` on init/update/sync for nested submodules.
*alias*: `submod u`

### `submod reset`

Stash tracked and untracked changes, then reset to the parent gitlink commit. If stash preservation fails, reset refuses before discarding work. Ignored files and nested repositories are preserved; collisions with files required by the target commit cause refusal. A successful stash reports its identity and a recovery command; apply that stash in the child repository to recover the saved work:

```bash
# Reset all submodules
submod reset --all

# Reset specific submodules (comma-separated)
submod reset my-lib,vendor-utils
```
*alias*: `submod r`

### `submod sync`

Reconcile managed declarations, registration, missing checkouts, settings, and parent-pin checkout state. Disabled and update-none entries skip automatic materialization. Repeated sync with no drift avoids unnecessary cloning/fetching. Explicit sync makes the managed TOML URL authoritative: it can overwrite local parent/child URL overrides for that module. Keep machine-specific authentication in Git credential helpers instead of relying on an overridden managed URL:

```bash
submod sync
```
*alias*: `submod s`

### `submod change`

Change selected fields while preserving omitted settings. Metadata-only changes retain HEAD; a path change moves the verified checkout and preserves its repository identity/history. `--shallow false` clears shallow preference, and `--unset branch` restores branch inheritance:

```bash
submod change my-lib --branch main --sparse-paths "src/,include/" --fetch always
```

### `submod change-global`

Change global defaults for all submodules:

```bash
submod change-global --ignore dirty --update checkout --branch main
```
*aliases*: `submod cg`, `submod chgl`, `submod global`

### `submod list`

List all configured submodules:

```bash
submod list
submod list --recursive
```

*aliases*: `submod ls`, `submod l`

### `submod delete`

Remove the exact managed registration and checkout while retaining the module repository for recovery. Dirty, untracked, or ignored checkout contents require `--force` to discard; force is limited to that verified checkout and never authorizes deleting unrelated storage:

```bash
submod delete my-lib
```

*alias*: `submod del`

### `submod disable`

Disable automatic lifecycle work without deleting files or history (sets TOML and managed local activation to false):

```bash
submod disable my-lib
```

*alias*: `submod d`

### `submod nuke-it-from-orbit`

Rebuild selected verified checkouts, retaining recoverable repositories and local refs. `--kill` removes their declarations/checkouts without reinitializing; it does not purge retained history:

```bash
# Nuke all submodules (re-initializes by default)
submod nuke-it-from-orbit --all

# Remove specific checkouts without reinitializing
submod nuke-it-from-orbit --kill my-lib,old-dep
```
*aliases*: `submod nuke-em`, `submod nuke-it`, `submod nuke-them`

A failed rebuild leaves the intended declaration available for recovery. Inspect the reported failure before retrying; `--force` only authorizes discarding local content inside the verified checkout.

### `submod generate-config`

Generate a new configuration file:

```bash
# From current git submodule setup
submod generate-config --from-setup

# As a template with defaults
submod generate-config --template --output my-config.toml
```
*aliases*: `submod gc`, `submod genconf`

### `submod completeme`

Generate shell completion scripts:

#### bash

```bash "bash"
mkdir -p ~/.bash_completion.d
submod completeme bash > ~/.bash_completion.d/submod
```
*aliases*: `submod comp`, `submod complete`, `submod comp-me`, `submod complete-me`

<details>
<summary>Add completions to other shells</summary>

#### zsh

```zsh "zsh"
# zsh has an fpath array with possible function directories. You can
# put your completions in any of these; we use the first one here:
ZSH_DEFAULT="${XDG_DATA_HOME:-~/.local/share}/zsh/site-functions"
ZFUNCDIR="${fpath[1]:-$ZSH_DEFAULT}"
mkdir -p "$ZFUNCDIR"
submod completeme zsh > "${ZFUNCDIR}/_submod"
```

#### fish

```fish
mkdir -p ~/.config/fish/completions
submod completeme fish > ~/.config/fish/completions/submod.fish
```

#### powershell

```pwsh
mkdir -p $Home\Documents\PowerShell\completions
submod completeme powershell > $Home\Documents\PowerShell\completions\submod.completion.ps1
```

#### elvish

```elvish
mkdir -p ~/.config/elvish/completions
submod completeme elvish > ~/.config/elvish/completions/submod.elv
```

#### nushell

```nushell
submod completeme nu
```

Save the Nushell output as `submod.nu` and load it from your Nushell configuration. Completion scripts are generated from the installed binary's command model; regenerate them after upgrading.

</details>

## 💻 Usage Examples

### Basic Workflow

```bash
# Start with checking current state
submod check

# Initialize any missing submodules
submod init

# Materialize the recorded parent commits
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

If recovery is still needed, inspect the error and retained repository before choosing a [rebuild](#submod-nuke-it-from-orbit). Rebuilds retain Git history but can discard local checkout contents when explicitly forced.

## 🛠️ Development

### Prerequisites

- Rust 1.89 or later
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
- **cargo check** - Type checking
- **cargo nextest** - Test suite
- **typos** - Spell checking
- **cargo deny** - Security and license auditing
- **pkl eval** - Config validation

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

# Using cargo directly (nextest is the preferred runner)
cargo nextest run --all-features --no-fail-fast  # Run all tests
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
│   ├── long_abouts.rs       # Long help text for commands
│   ├── shells.rs            # Shell completion generation
│   ├── config.rs            # TOML configuration handling
│   ├── options.rs           # Git-config-compatible option newtypes
│   ├── utilities.rs         # Shared display and repository helpers
│   ├── lib.rs               # Library root (exposed for integration tests)
│   ├── git_manager.rs       # High-level submodule operations
│   └── git_ops/             # Git backend abstraction
│       ├── mod.rs           # GitOpsManager (native Git mutation boundary)
│       ├── gix_ops.rs       # gitoxide read backend
│       └── git2_ops.rs      # libgit2 read backend
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
- **Use Git semantics** - Delegate lifecycle mutations to native Git and preserve recoverable history

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
