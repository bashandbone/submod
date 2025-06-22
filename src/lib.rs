//! Library entry point for submod, a Git submodule manager with sparse checkout support.
//!
//! This crate is primarily intended for CLI use. The library API is not stable and may change.
//!
//! # Modules
//! - [`config`]: Submodule configuration management.
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

/// Configuration management for submodules
pub mod config;
/// Gitoxide-based submodule management implementation
pub mod gitoxide_manager;

pub use config::{Config, SubmoduleConfig, SubmoduleDefaults, SubmoduleGitOptions};
pub use gitoxide_manager::{
    GitoxideSubmoduleManager, SparseStatus, SubmoduleError, SubmoduleStatus,
};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Re-export commonly used types for convenience
pub mod prelude {
    pub use crate::{Config, GitoxideSubmoduleManager, SubmoduleError};
}
