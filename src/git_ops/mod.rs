// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
#![doc = r"
This module provides a unified interface for performing git operations using both `gix` and `git2` libraries. 
It implements a gix-first, git2-fallback strategy to ensure robust functionality across different environments and use cases. 

The `GitOpsManager` struct manages the operations and automatically falls back to `git2` if a `gix` operation fails, 
providing seamless integration for submodule management and configuration tasks.

We prefer Gix, but it's still unstable and several core features are missing, so we use git2 as a fallback for those features and for stability.
"]
/// git2-based git operations implementation
pub mod git2_ops;
/// gitoxide (gix)-based git operations implementation
pub mod gix_ops;
pub mod simple_gix;
pub use git2_ops::Git2Operations;
pub use gix_ops::GixOperations;

use anyhow::{Context, Result};
use bitflags::bitflags;
use std::collections::HashMap;
use std::path::Path;

use crate::config::{SubmoduleAddOptions, SubmoduleEntries, SubmoduleUpdateOptions};
use crate::options::{
    ConfigLevel, SerializableBranch, SerializableFetchRecurse, SerializableIgnore,
    SerializableUpdate,
};

/// Represents git configuration state
#[allow(dead_code)]
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
#[allow(dead_code)]
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
    #[allow(dead_code)]
    fn read_git_config(&self, level: ConfigLevel) -> Result<GitConfig>;
    /// Write git configuration at specified level
    #[allow(dead_code)]
    fn write_git_config(&self, config: &GitConfig, level: ConfigLevel) -> Result<()>;
    /// Set a single configuration value
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    fn get_submodule_status(&self, path: &str) -> Result<DetailedSubmoduleStatus>;
    /// List all submodules
    fn list_submodules(&self) -> Result<Vec<String>>;

    // Repository operations
    /// Fetch a submodule
    #[allow(dead_code)]
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
    verbose: bool,
}

/// Implement GitOperations for GitOpsManager, using gix first and falling back to git2 if gix fails
impl GitOpsManager {
    /// Create a new `GitOpsManager` with automatic fallback
    pub fn new(repo_path: Option<&Path>, verbose: bool) -> Result<Self> {
        let gix_ops = GixOperations::new(repo_path).ok();
        let git2_ops = Git2Operations::new(repo_path)
            .with_context(|| "Failed to initialize git2 operations")?;

        Ok(Self {
            gix_ops,
            git2_ops,
            verbose,
        })
    }

    /// Return the working directory of the underlying git repository, if any.
    pub fn workdir(&self) -> Option<&std::path::Path> {
        self.git2_ops.workdir()
    }

    /// Reopen the repository from the working directory to refresh any cached state.
    /// This is needed after destructive operations (e.g., submodule delete) so that the
    /// in-memory git2 repository object reflects the updated on-disk state.
    ///
    /// Returns an error if the git2 repository (the required backend) cannot be reopened.
    /// A gix reopen failure is non-fatal since gix is an optional optimistic backend.
    pub fn reopen(&mut self) -> Result<()> {
        let workdir = self
            .git2_ops
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Cannot reopen repository: no working directory"))?
            .to_path_buf();

        // git2 is the required backend — propagate its reopen error.
        self.git2_ops = Git2Operations::new(Some(&workdir)).with_context(|| {
            format!("Failed to reopen git2 repository at {}", workdir.display())
        })?;

        // gix is an optional optimistic backend — log failures but don't fail.
        match GixOperations::new(Some(&workdir)) {
            Ok(new_gix) => {
                self.gix_ops = Some(new_gix);
            }
            Err(e) => {
                if self.verbose {
                    eprintln!(
                        "Warning: failed to reopen gix repository at {}: {}",
                        workdir.display(),
                        e
                    );
                }
            }
        }

        Ok(())
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
                    if self.verbose {
                        eprintln!("gix operation failed, falling back to git2: {e}");
                    }
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
                    if self.verbose {
                        eprintln!("gix operation failed, falling back to git2: {e}");
                    }
                }
            }
        }
        git2_op(&mut self.git2_ops)
    }
}

/// Implement GitOperations for GitOpsManager, using gix first and falling back to git2 if gix fails
impl GitOperations for GitOpsManager {
    fn read_gitmodules(&self) -> Result<SubmoduleEntries> {
        self.try_with_fallback(|gix| gix.read_gitmodules(), |git2| git2.read_gitmodules())
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
        // Try gix first (not yet implemented → falls through), then git2 which now uses
        // the correct `submodule.clone() + add_finalize()` sequence.
        // CLI is kept as a last-resort safety net and sets current_dir to the superproject
        // workdir so it works regardless of the process's CWD.
        self.try_with_fallback_mut(
            |gix| gix.add_submodule(opts),
            |git2| git2.add_submodule(opts),
        )
        .or_else(|git2_err| {
            let workdir = self
                .git2_ops
                .workdir()
                .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;

            // Clean up potentially partially initialized submodule path before fallback
            let sub_path = workdir.join(&opts.path);
            if sub_path.exists() {
                let _ = std::fs::remove_dir_all(&sub_path);
            }

            // git2 also adds the submodule to .gitmodules, which will cause CLI to fail
            // if we don't clean it up.
            let gitmodules_path = workdir.join(".gitmodules");
            if gitmodules_path.exists() {
                // If it fails to read or write we just ignore it as it's a fallback cleanup
                if let Ok(content) = std::fs::read_to_string(&gitmodules_path) {
                    let mut new_content = String::new();
                    let mut in_target_section = false;
                    let target_name = format!("\"{}\"", opts.name);
                    for line in content.lines() {
                        if line.starts_with("[submodule \"") {
                            in_target_section = line.contains(&target_name);
                        }
                        if !in_target_section {
                            new_content.push_str(line);
                            new_content.push('\n');
                        }
                    }
                    let _ = std::fs::write(&gitmodules_path, new_content);
                }
            }

            // Also git2 might have added it to .git/config
            let gitconfig_path = workdir.join(".git").join("config");
            if gitconfig_path.exists() {
                // Remove by name (our submodule name)
                let _ = std::process::Command::new("git")
                    .args([
                        "config",
                        "--remove-section",
                        &format!("submodule.{}", opts.name),
                    ])
                    .current_dir(workdir)
                    .output();
                // Remove by path (git2 uses path as key when name != path)
                let path_key = opts.path.display().to_string();
                if path_key != opts.name {
                    let _ = std::process::Command::new("git")
                        .args([
                            "config",
                            "--remove-section",
                            &format!("submodule.{path_key}"),
                        ])
                        .current_dir(workdir)
                        .output();
                }
            }

            // Also git2 might have created the internal git directory
            let internal_git_dir = workdir.join(".git").join("modules").join(&opts.name);
            if internal_git_dir.exists() {
                let _ = std::fs::remove_dir_all(&internal_git_dir);
            }

            // git2's repo.submodule() uses the *path* (not the name) as the key for the
            // internal modules directory, so ".git/modules/lib/reinit" may exist even when
            // ".git/modules/<name>" has already been cleaned up.  Remove both.
            let path_internal_git_dir = workdir.join(".git").join("modules").join(&opts.path);
            if path_internal_git_dir.exists() {
                let _ = std::fs::remove_dir_all(&path_internal_git_dir);
            }

            // And removed from index
            let _ = std::process::Command::new("git")
                .args(["rm", "--cached", "-r", "--ignore-unmatch", "--"])
                .arg(&opts.path)
                .current_dir(workdir)
                .output();

            let mut cmd = std::process::Command::new("git");
            cmd.current_dir(workdir)
                .arg("submodule")
                .arg("add")
                .arg("--name")
                .arg(&opts.name);
            if let Some(branch) = &opts.branch {
                let branch_str = branch.to_string();
                // "." is the gitmodules/git-config token meaning "track the same branch as
                // the superproject" (SerializableBranch::CurrentInSuperproject).  It is only
                // meaningful as a stored config value; passing it as `--branch .` to
                // `git submodule add` is invalid and causes:
                //   fatal: 'HEAD' is not a valid branch name
                // Skip the flag so git resolves the remote's default branch automatically.
                if branch_str != "." {
                    cmd.arg("--branch").arg(&branch_str);
                }
            }
            if opts.shallow {
                cmd.arg("--depth").arg("1");
            }
            cmd.arg("--").arg(&opts.url).arg(&opts.path);
            let output = cmd.output().context("Failed to run git submodule add")?;
            if output.status.success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Failed to add submodule (git2 failed with: {}). CLI output: {}",
                    git2_err,
                    String::from_utf8_lossy(&output.stderr).trim()
                ))
            }
        })
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
        self.try_with_fallback(|gix| gix.list_submodules(), |git2| git2.list_submodules())
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
        .or_else(|_| {
            // CLI fallback: use git read-tree to apply sparse checkout
            let output = std::process::Command::new("git")
                .current_dir(path)
                .args(["read-tree", "-mu", "HEAD"])
                .output()
                .context("Failed to run git read-tree")?;
            if output.status.success() {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "git read-tree failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ))
            }
        })
    }
}
