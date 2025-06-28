// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
//
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
