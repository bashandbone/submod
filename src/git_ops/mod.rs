// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
pub mod gix_ops;
pub mod git2_ops;
pub mod simple_gix;
pub use gix_ops::GixOperations;
pub use git2_ops::Git2Operations;

use anyhow::{Context, Result};
use bitflags::bitflags;
use std::collections::HashMap;
use std::path::Path;

use crate::options::{ ConfigLevel,
    SerializableBranch, SerializableFetchRecurse, SerializableIgnore, SerializableUpdate,
};
use crate::config::{
    SubmoduleEntries, SubmoduleAddOptions, SubmoduleUpdateOptions,
};

/// Represents git configuration state
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitConfig {
    /// Configuration entries as key-value pairs
    pub entries: HashMap<String, String>,
}


bitflags! {
    /// Submodule status flags (mirrors git2::SubmoduleStatus)
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SubmoduleStatusFlags: u32 {
        /// Superproject head contains submodule
        const IN_HEAD = 1 << 0;
        /// Superproject index contains submodule
        const IN_INDEX = 1 << 1;
        /// Superproject gitmodules has submodule
        const IN_CONFIG = 1 << 2;
        /// Superproject workdir has submodule
        const IN_WD = 1 << 3;
        /// In index, not in head
        const INDEX_ADDED = 1 << 4;
        /// In head, not in index
        const INDEX_DELETED = 1 << 5;
        /// Index and head don't match
        const INDEX_MODIFIED = 1 << 6;
        /// Workdir contains empty directory
        const WD_UNINITIALIZED = 1 << 7;
        /// In workdir, not index
        const WD_ADDED = 1 << 8;
        /// In index, not workdir
        const WD_DELETED = 1 << 9;
        /// Index and workdir head don't match
        const WD_MODIFIED = 1 << 10;
        /// Submodule workdir index is dirty
        const WD_INDEX_MODIFIED = 1 << 11;
        /// Submodule workdir has modified files
        const WD_WD_MODIFIED = 1 << 12;
        /// Workdir contains untracked files
        const WD_UNTRACKED = 1 << 13;
    }
}

/// Comprehensive submodule status information
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DetailedSubmoduleStatus {
    /// Path of the submodule
    pub path: String,
    /// Name of the submodule
    pub name: String,
    /// URL of the submodule (if available)
    pub url: Option<String>,
    /// HEAD OID of the submodule (if available)
    pub head_oid: Option<String>,
    /// Index OID of the submodule (if available)
    pub index_oid: Option<String>,
    /// Working directory OID of the submodule (if available)
    pub workdir_oid: Option<String>,
    /// Status flags
    pub status_flags: SubmoduleStatusFlags,
    /// Ignore rule for the submodule
    pub ignore_rule: SerializableIgnore,
    /// Update rule for the submodule
    pub update_rule: SerializableUpdate,
    /// Fetch recurse rule for the submodule
    pub fetch_recurse_rule: SerializableFetchRecurse,
    /// Branch being tracked (if any)
    pub branch: Option<SerializableBranch>,
    /// Whether the submodule is initialized
    pub is_initialized: bool,
    /// Whether the submodule is active
    pub is_active: bool,
    /// Whether the submodule has modifications
    pub has_modifications: bool,
    /// Whether sparse checkout is enabled
    pub sparse_checkout_enabled: bool,
    /// Sparse checkout patterns
    pub sparse_patterns: Vec<String>,
}

/// Main trait for git operations with gix-first, git2-fallback strategy
pub trait GitOperations {
    // Config operations
    /// Read .gitmodules configuration
    fn read_gitmodules(&self) -> Result<SubmoduleEntries>;
    /// Write .gitmodules configuration
    fn write_gitmodules(&mut self, config: &SubmoduleEntries) -> Result<()>;
    /// Read git configuration at specified level
    fn read_git_config(&self, level: ConfigLevel) -> Result<GitConfig>;
    /// Write git configuration at specified level
    fn write_git_config(&self, config: &GitConfig, level: ConfigLevel) -> Result<()>;
    /// Set a single configuration value
    fn set_config_value(&self, key: &str, value: &str, level: ConfigLevel) -> Result<()>;

    // Submodule operations
    /// Add a new submodule
    fn add_submodule(&mut self, opts: &SubmoduleAddOptions) -> Result<()>;
    /// Initialize a submodule
    fn init_submodule(&mut self, path: &str) -> Result<()>;
    /// Update a submodule
    fn update_submodule(&mut self, path: &str, opts: &SubmoduleUpdateOptions) -> Result<()>;
    /// Delete a submodule completely
    fn delete_submodule(&mut self, path: &str) -> Result<()>;
    /// Deinitialize a submodule
    fn deinit_submodule(&mut self, path: &str, force: bool) -> Result<()>;
    /// Get detailed status of a submodule
    fn get_submodule_status(&self, path: &str) -> Result<DetailedSubmoduleStatus>;
    /// List all submodules
    fn list_submodules(&self) -> Result<Vec<String>>;

    // Repository operations
    /// Fetch a submodule
    fn fetch_submodule(&self, path: &str) -> Result<()>;
    /// Reset a submodule
    fn reset_submodule(&self, path: &str, hard: bool) -> Result<()>;
    /// Clean a submodule
    fn clean_submodule(&self, path: &str, force: bool, remove_directories: bool) -> Result<()>;
    /// Stash changes in a submodule
    fn stash_submodule(&self, path: &str, include_untracked: bool) -> Result<()>;

    // Sparse checkout operations
    /// Enable sparse checkout for a submodule
    fn enable_sparse_checkout(&self, path: &str) -> Result<()>;
    /// Set sparse checkout patterns for a submodule
    fn set_sparse_patterns(&self, path: &str, patterns: &[String]) -> Result<()>;
    /// Get current sparse checkout patterns for a submodule
    fn get_sparse_patterns(&self, path: &str) -> Result<Vec<String>>;
    /// Apply sparse checkout configuration
    fn apply_sparse_checkout(&self, path: &str) -> Result<()>;
}

/// Unified git operations manager with automatic fallback
pub struct GitOpsManager {
    gix_ops: Option<GixOperations>,
    git2_ops: Git2Operations,
}

impl GitOpsManager {
    /// Create a new GitOpsManager with automatic fallback
    pub fn new(repo_path: Option<&Path>) -> Result<Self> {
        let gix_ops = GixOperations::new(repo_path).ok();
        let git2_ops = Git2Operations::new(repo_path)
            .with_context(|| "Failed to initialize git2 operations")?;

        Ok(Self { gix_ops, git2_ops })
    }

    /// Try gix first, fall back to git2
    fn try_with_fallback<T, F1, F2>(&self, gix_op: F1, git2_op: F2) -> Result<T>
    where
        F1: FnOnce(&GixOperations) -> Result<T>,
        F2: FnOnce(&Git2Operations) -> Result<T>,
    {
        if let Some(ref gix) = self.gix_ops {
            match gix_op(gix) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    eprintln!("gix operation failed, falling back to git2: {}", e);
                }
            }
        }

        git2_op(&self.git2_ops)
    }

    /// Try gix first, fall back to git2 (mutable version)
    fn try_with_fallback_mut<T, F1, F2>(&mut self, gix_op: F1, git2_op: F2) -> Result<T>
    where
        F1: FnOnce(&mut GixOperations) -> Result<T>,
        F2: FnOnce(&mut Git2Operations) -> Result<T>,
    {
        if let Some(ref mut gix) = self.gix_ops {
            match gix_op(gix) {
                Ok(result) => return Ok(result),
                Err(e) => {
                    eprintln!("gix operation failed, falling back to git2: {}", e);
                }
            }
        }
        git2_op(&mut self.git2_ops)
    }
}

impl GitOperations for GitOpsManager {
    fn read_gitmodules(&self) -> Result<SubmoduleEntries> {
        self.try_with_fallback(
            |gix| gix.read_gitmodules(),
            |git2| git2.read_gitmodules(),
        )
    }

    fn write_gitmodules(&mut self, config: &SubmoduleEntries) -> Result<()> {
        self.try_with_fallback_mut(
            |gix| gix.write_gitmodules(config),
            |git2| git2.write_gitmodules(config),
        )
    }

    fn read_git_config(&self, level: ConfigLevel) -> Result<GitConfig> {
        self.try_with_fallback(
            |gix| gix.read_git_config(level),
            |git2| git2.read_git_config(level),
        )
    }

    fn write_git_config(&self, config: &GitConfig, level: ConfigLevel) -> Result<()> {
        self.try_with_fallback(
            |gix| gix.write_git_config(config, level),
            |git2| git2.write_git_config(config, level),
        )
    }

    fn set_config_value(&self, key: &str, value: &str, level: ConfigLevel) -> Result<()> {
        self.try_with_fallback(
            |gix| gix.set_config_value(key, value, level),
            |git2| git2.set_config_value(key, value, level),
        )
    }

    fn add_submodule(&mut self, opts: &SubmoduleAddOptions) -> Result<()> {
        self.try_with_fallback_mut(
            |gix| gix.add_submodule(opts),
            |git2| git2.add_submodule(opts),
        )
    }

    fn init_submodule(&mut self, path: &str) -> Result<()> {
        self.try_with_fallback_mut(
            |gix| gix.init_submodule(path),
            |git2| git2.init_submodule(path),
        )
    }

    fn update_submodule(&mut self, path: &str, opts: &SubmoduleUpdateOptions) -> Result<()> {
        self.try_with_fallback_mut(
            |gix| gix.update_submodule(path, opts),
            |git2| git2.update_submodule(path, opts),
        )
    }

    fn delete_submodule(&mut self, path: &str) -> Result<()> {
            self.try_with_fallback_mut(
                |gix| gix.delete_submodule(path),
                |git2| git2.delete_submodule(path),
            )
        }

    fn deinit_submodule(&mut self, path: &str, force: bool) -> Result<()> {
        self.try_with_fallback_mut(
            |gix| gix.deinit_submodule(path, force),
            |git2| git2.deinit_submodule(path, force),
        )
    }

    fn get_submodule_status(&self, path: &str) -> Result<DetailedSubmoduleStatus> {
        self.try_with_fallback(
            |gix| gix.get_submodule_status(path),
            |git2| git2.get_submodule_status(path),
        )
    }

    fn list_submodules(&self) -> Result<Vec<String>> {
        self.try_with_fallback(
            |gix| gix.list_submodules(),
            |git2| git2.list_submodules(),
        )
    }

    fn fetch_submodule(&self, path: &str) -> Result<()> {
        self.try_with_fallback(
            |gix| gix.fetch_submodule(path),
            |git2| git2.fetch_submodule(path),
        )
    }

    fn reset_submodule(&self, path: &str, hard: bool) -> Result<()> {
        self.try_with_fallback(
            |gix| gix.reset_submodule(path, hard),
            |git2| git2.reset_submodule(path, hard),
        )
    }

    fn clean_submodule(&self, path: &str, force: bool, remove_directories: bool) -> Result<()> {
        self.try_with_fallback(
            |gix| gix.clean_submodule(path, force, remove_directories),
            |git2| git2.clean_submodule(path, force, remove_directories),
        )
    }

    fn stash_submodule(&self, path: &str, include_untracked: bool) -> Result<()> {
        self.try_with_fallback(
            |gix| gix.stash_submodule(path, include_untracked),
            |git2| git2.stash_submodule(path, include_untracked),
        )
    }

    fn enable_sparse_checkout(&self, path: &str) -> Result<()> {
        self.try_with_fallback(
            |gix| gix.enable_sparse_checkout(path),
            |git2| git2.enable_sparse_checkout(path),
        )
    }

    fn set_sparse_patterns(&self, path: &str, patterns: &[String]) -> Result<()> {
        self.try_with_fallback(
            |gix| gix.set_sparse_patterns(path, patterns),
            |git2| git2.set_sparse_patterns(path, patterns),
        )
    }

    fn get_sparse_patterns(&self, path: &str) -> Result<Vec<String>> {
        self.try_with_fallback(
            |gix| gix.get_sparse_patterns(path),
            |git2| git2.get_sparse_patterns(path),
        )
    }

    fn apply_sparse_checkout(&self, path: &str) -> Result<()> {
        self.try_with_fallback(
            |gix| gix.apply_sparse_checkout(path),
            |git2| git2.apply_sparse_checkout(path),
        )
    }
}
