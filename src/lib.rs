//! Submod - Git submodule manager with sparse checkout support using gitoxide
//!
//! This library exists only for testing purposes. We're a CLI tool, not a library.
//! You're welcome to use it as a library, but we don't guarantee any API stability.

/// Configuration management for submodules
pub mod config;
/// Gitoxide-based submodule management implementation
pub mod gitoxide_manager;

pub use config::{Config, SubmoduleConfig, SubmoduleGitOptions, SubmoduleDefaults};
pub use gitoxide_manager::{GitoxideSubmoduleManager, SubmoduleError, SubmoduleStatus, SparseStatus};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Re-export commonly used types for convenience
pub mod prelude {
    pub use crate::{Config, GitoxideSubmoduleManager, SubmoduleError};
}
