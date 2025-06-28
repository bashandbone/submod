// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: MIT
// Licensed under the [Plain MIT License][../LICENSE.md]
//! Library entry point for submod, a Git submodule manager with sparse checkout support.
//!
//! This crate is primarily intended for CLI use. The library API is not stable and may change.
//!
//! # Modules
//! - [`utilities`]: Common utilities and helper functions.
//! - [`options`]: Git submodule configuration options and parsing.
//! - [`config`]: Submodule configuration management.
//! - [`git_ops`]: Git operations layer with gix-first, git2-fallback strategy.
//! - [`gitoxide_manager`]: Implementation of submodule operations using gitoxide.
//!
//! # Exports
//! - Common types and managers for use in tests or advanced integrations.
//!
//! # Version
//! - Exposes the current crate version as [`VERSION`].
//!
//! # Note
//! The API is not guaranteed to be stable. Use at your own risk.

pub mod utilities;
/// Configuration management for submodules
pub mod options;
pub mod config;

/// Git operations layer with gix-first, git2-fallback strategy
pub mod git_ops;
/// Gitoxide-based submodule management implementation
pub mod gitoxide_manager;

pub use config::{Config, SubmoduleEntry, SubmoduleDefaults, SubmoduleGitOptions, SubmoduleAddOptions, SubmoduleUpdateOptions};
pub use git_ops::{GixOperations, Git2Operations};
pub use gitoxide_manager::{
    GitoxideSubmoduleManager, SparseStatus, SubmoduleError, SubmoduleStatus,
};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Re-export commonly used types for convenience
pub mod prelude {
    pub use crate::{Config, GixOperations, Git2Operations, GitoxideSubmoduleManager, SubmoduleError};
}
