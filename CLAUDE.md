<!--
SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>

SPDX-License-Identifier: MIT

Licensed under the [Plain MIT License](LICENSE.md)
-->

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`submod` is a Rust CLI tool for managing git submodules with advanced sparse checkout support. It uses `gitoxide` and `git2` libraries for high-performance git operations, with fallbacks to CLI git commands when needed.

## Development Commands

### Building and Testing

- `cargo build` - Build the project
- `cargo test` - Run unit tests
- `./scripts/run-tests.sh` - Run comprehensive integration test suite
- `./scripts/run-tests.sh --verbose` - Run tests with detailed output
- `./scripts/run-tests.sh --performance` - Include performance tests
- `./scripts/run-tests.sh --filter <pattern>` - Run specific test modules

### Linting and Formatting

- `cargo fmt` - Format code
- `cargo clippy` - Run linter
- `hk run check` - Run pre-commit checks via hk tool
- `hk run fix` - Run auto-fixable linters

### Mise Tasks (if using mise)

- `mise run build` or `mise run b` - Build the CLI
- `mise run test` - Run automated tests
- `mise run lint` - Lint with clippy
- `mise run ci` - Run CI tasks (build, lint, test)

### Testing Philosophy

The project follows integration-first testing. Focus on testing complete workflows and outputs rather than implementation details. Tests are run serially (`RUST_TEST_THREADS=1`) to avoid git submodule race conditions.

## Architecture

### Core Modules

- `src/main.rs` - CLI entry point, parses commands and dispatches to manager
- `src/commands.rs` - Command-line argument definitions using clap
- `src/config.rs` - TOML configuration parsing and submodule config management
- `src/gitoxide_manager.rs` - Core submodule operations using gitoxide/git2
- `src/lib.rs` - Library exports (not a stable API)

### Configuration System

- Uses TOML configuration files (default: `submod.toml`)
- Supports global defaults with per-submodule overrides
- Handles sparse checkout paths, git options, and submodule settings

### Git Operations Strategy

1. **Primary**: gitoxide library for performance
2. **Fallback**: git2 library (optional feature `git2-support`)
3. **Final fallback**: CLI git commands

### Key Design Patterns

- Error handling with `anyhow` for application errors, `thiserror` for library errors
- Comprehensive documentation for all public APIs
- Strict linting configuration with pedantic clippy settings
- Integration tests over unit tests

## Configuration Files

### Key Files to Know

- `Cargo.toml` - Project configuration with strict linting rules
- `hk.pkl` - Git hooks configuration (pre-commit, linting)
- `mise.toml` - Development environment and task definitions
- `sample_config/submod.toml` - Example configuration
- `scripts/run-tests.sh` - Comprehensive test runner

### Test Structure

- `tests/integration_tests.rs` - Core functionality tests
- `tests/config_tests.rs` - Configuration parsing tests
- `tests/sparse_checkout_tests.rs` - Sparse checkout functionality
- `tests/error_handling_tests.rs` - Error conditions and edge cases
- `tests/performance_tests.rs` - Performance and stress tests

## Working with the Codebase

### Before Making Changes

1. Run `./scripts/run-tests.sh` to ensure tests pass
2. Check code formatting with `cargo fmt --check`
3. Run linter with `cargo clippy`

### When Adding Features

- Add integration tests to appropriate test modules
- Update configuration parsing if new config options are added
- Follow existing error handling patterns
- Document public APIs thoroughly

### Code Quality Standards

- Unsafe code is forbidden (`unsafe_code = "forbid"`)
- All warnings treated seriously with comprehensive clippy configuration
- Focus on clear, descriptive naming and comprehensive error messages
