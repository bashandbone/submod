// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
//! A Rust CLI tool for managing Git submodules with enhanced features and user-friendly configuration.
//! This module is exposed for integration testing; it is not intended for public use and may contain unstable APIs.

pub mod config;
/// Configuration management for submodules
pub mod options;
/// Shell completion generation support
pub mod shells;
pub mod utilities;

/// Gitoxide-based submodule management implementation
pub mod git_manager;
/// Git operations layer with gix-first, git2-fallback strategy
pub mod git_ops;

pub use config::{
    Config, SubmoduleAddOptions, SubmoduleDefaults, SubmoduleEntry, SubmoduleGitOptions,
    SubmoduleUpdateOptions,
};
pub use git_manager::{GitManager, SparseStatus, SubmoduleError, SubmoduleStatus};
pub use git_ops::{Git2Operations, GixOperations};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Re-export commonly used types for convenience
pub mod prelude {
    pub use crate::{Config, Git2Operations, GitManager, GixOperations, SubmoduleError};
}
