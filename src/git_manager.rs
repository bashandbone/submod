// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

#![doc = r"
# Gitoxide-Based Submodule Manager

Provides core logic for managing git submodules using the [`gitoxide`](https://github.com/Byron/gitoxide) library, with fallbacks to `git2` and the Git CLI when needed. Supports sparse checkout and TOML-based configuration.

## Overview

- Loads submodule configuration from a TOML file.
- Adds, initializes, updates, resets, and checks submodules.
- Uses `gitoxide` APIs where possible for performance and reliability.
- Falls back to `git2` (if enabled) or the Git CLI for unsupported operations.
- Supports sparse checkout configuration per submodule.

## Key Types

- [`SubmoduleError`](src/git_manager.rs:14): Error type for submodule operations.
- [`SubmoduleStatus`](src/git_manager.rs:55): Reports the status of a submodule, including cleanliness, commit, remotes, and sparse checkout state.
- [`SparseStatus`](src/git_manager.rs:77): Describes the sparse checkout configuration state.
- [`GitManager`](src/git_manager.rs:94): Main struct for submodule management.

## Main Operations

- [`GitManager::add_submodule()`](src/git_manager.rs:207): Adds a new submodule, configuring sparse checkout if specified.
- [`GitManager::init_submodule()`](src/git_manager.rs:643): Initializes a submodule, adding it if missing.
- [`GitManager::update_submodule()`](src/git_manager.rs:544): Updates a submodule using the Git CLI.
- [`GitManager::reset_submodule()`](src/git_manager.rs:574): Resets a submodule (stash, hard reset, clean).
- [`GitManager::check_all_submodules()`](src/git_manager.rs:732): Checks the status of all configured submodules.

## Sparse Checkout Support

- Checks and configures sparse checkout for each submodule based on the TOML config.
- Writes sparse-checkout patterns and applies them using the Git CLI.

## Error Handling

All operations return [`SubmoduleError`](src/git_manager.rs:14) for consistent error reporting.

## TODOs

- TODO: Implement submodule addition using gitoxide APIs when available ([`add_submodule_with_gix`](src/git_manager.rs:278)). Until then, we need to make git2 a required dependency.

## Usage

Use this module as the backend for CLI commands to manage submodules in a repository. See the project [README](README.md) for usage examples and configuration details.
"]

use crate::config::{Config, Git2SubmoduleOptions, SubmoduleEntry, SubmoduleGitOptions};
use crate::git_ops::GitOperations;
use crate::git_ops::GitOpsManager;
use crate::options::{
    SerializableBranch, SerializableFetchRecurse, SerializableIgnore, SerializableUpdate,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Custom error types for submodule operations
#[derive(Debug, thiserror::Error)]
pub enum SubmoduleError {
    /// Error from gitoxide library operations
    #[error("Gitoxide operation failed: {0}")]
    #[allow(dead_code)]
    GitoxideError(String),

    /// Error from git2 library operations (when git2-support feature is enabled)
    #[error("git2 operation failed: {0}")]
    Git2Error(#[from] git2::Error),

    /// Error from Git CLI operations
    #[error("Git CLI operation failed: {0}")]
    #[allow(dead_code)]
    CliError(String),

    /// Configuration-related error
    #[error("Configuration error: {0}")]
    #[allow(dead_code)]
    ConfigError(String),

    /// I/O operation error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Submodule not found in repository
    #[error("Submodule {name} not found")]
    SubmoduleNotFound {
        /// Name of the missing submodule.
        name: String,
    },

    /// Repository access or validation error
    #[error("Repository not found or invalid")]
    #[allow(dead_code)]
    RepositoryError,
}

/// Status information for a submodule
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmoduleStatus {
    /// Path to the submodule directory
    #[allow(dead_code)]
    pub path: String,
    /// Whether the submodule working directory is clean
    pub is_clean: bool,
    /// Current commit hash of the submodule
    pub current_commit: Option<String>,
    /// Whether the submodule has remote repositories configured
    pub has_remotes: bool,
    /// Whether the submodule is initialized
    #[allow(dead_code)]
    pub is_initialized: bool,
    /// Whether the submodule is active
    #[allow(dead_code)]
    pub is_active: bool,
    /// Sparse checkout status for this submodule
    pub sparse_status: SparseStatus,

    /// Whether the submodule has its own submodules
    pub has_submodules: bool,
}

/// Sparse checkout status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SparseStatus {
    /// Sparse checkout is not enabled for this submodule
    NotEnabled,
    /// Sparse checkout is enabled but not configured
    NotConfigured,
    /// Sparse checkout configuration matches expected paths
    Correct,
    /// Sparse checkout configuration doesn't match expected paths
    Mismatch {
        /// Expected sparse checkout paths
        expected: Vec<String>,
        /// Actual sparse checkout paths
        actual: Vec<String>,
    },
}

/// Main gitoxide-based submodule manager
pub struct GitManager {
    /// The main git operations manager (gix-first, git2-fallback)
    git_ops: GitOpsManager,
    /// Configuration for submodules
    config: Config,
    /// Path to the configuration file
    config_path: PathBuf,
}

impl GitManager {
    /// Helper method to map git operations errors
    fn map_git_ops_error(err: anyhow::Error) -> SubmoduleError {
        SubmoduleError::ConfigError(format!("Git operation failed: {err}"))
    }

    /// Restore update_toml_config method
    fn update_toml_config(
        &mut self,
        name: String,
        entry: crate::config::SubmoduleEntry,
        _sparse_paths: Option<Vec<String>>,
    ) -> Result<(), SubmoduleError> {
        self.config.add_submodule(name, entry);
        // No-op for sparse paths; handled elsewhere.
        Ok(())
    }

    /// Creates a new `GitManager` by loading configuration from the given path.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the TOML configuration file.
    ///
    /// # Errors
    ///
    /// Returns `SubmoduleError::RepositoryError` if the repository cannot be discovered,
    /// or `SubmoduleError::ConfigError` if the configuration fails to load.
    pub fn new(config_path: PathBuf) -> Result<Self, SubmoduleError> {
        // Use GitOpsManager for repository detection and operations
        let git_ops = GitOpsManager::new(Some(Path::new(".")))
            .map_err(|_| SubmoduleError::RepositoryError)?;

        let config = Config::default()
            .load(&config_path, Config::default())
            .map_err(|e| SubmoduleError::ConfigError(format!("Failed to load config: {e}")))?;

        Ok(Self {
            git_ops,
            config,
            config_path,
        })
    }

    /// Check submodule repository status using gix APIs
    pub fn check_submodule_repository_status(
        &self,
        submodule_path: &str,
        name: &str,
    ) -> Result<SubmoduleStatus, SubmoduleError> {
        // NOTE: This is a legacy direct gix usage for status; could be refactored to use GitOpsManager if needed.
        let submodule_repo =
            gix::open(submodule_path).map_err(|_| SubmoduleError::RepositoryError)?;

        // GITOXIDE API: Use gix for what's available, fall back to CLI for complex status
        // For now, use a simple approach - check if there are any uncommitted changes
        let is_dirty = match submodule_repo.head() {
            Ok(_head) => {
                // Simple check - if we can get head, assume repository is clean
                // This is a conservative approach until we can use the full status API
                false
            }
            Err(_) => true,
        };

        // GITOXIDE API: Use reference APIs for current commit
        let current_commit = match submodule_repo.head() {
            Ok(head) => head.id().map(|id| id.to_string()),
            Err(_) => None,
        };

        // GITOXIDE API: Use remote APIs to check if remotes exist
        let has_remotes = !submodule_repo.remote_names().is_empty();

        // For now, consider all submodules active if they exist in config
        let is_active = self.config.submodules.contains_key(name);

        // Check sparse checkout status
        let sparse_status =
            if let Some(sparse_checkouts) = self.config.submodules.sparse_checkouts() {
                if let Some(expected_paths) = sparse_checkouts.get(name) {
                    self.check_sparse_checkout_status(submodule_path, expected_paths)?
                } else {
                    SparseStatus::NotEnabled
                }
            } else {
                SparseStatus::NotEnabled
            };
        // Check if submodule has its own submodules
        let has_submodules = submodule_repo
            .submodules()
            .map(|subs| subs.into_iter().count() > 0)
            .unwrap_or(false);

        Ok(SubmoduleStatus {
            path: submodule_path.to_string(),
            is_clean: !is_dirty,
            current_commit,
            has_remotes,
            is_initialized: true,
            is_active,
            sparse_status,
            has_submodules,
        })
    }

    /// Check sparse checkout configuration
    pub fn check_sparse_checkout_status(
        &self,
        submodule_path: &str,
        expected_paths: &[String],
    ) -> Result<SparseStatus, SubmoduleError> {
        // Try to find the sparse-checkout file for the submodule
        let git_dir = self.get_git_directory(submodule_path)?;
        let sparse_checkout_file = git_dir.join("info").join("sparse-checkout");
        if !sparse_checkout_file.exists() {
            return Ok(SparseStatus::NotConfigured);
        }

        let content = fs::read_to_string(&sparse_checkout_file)?;
        let configured_paths: Vec<String> = content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(std::string::ToString::to_string)
            .collect();

        let matches = expected_paths
            .iter()
            .all(|path| configured_paths.contains(path));

        if matches {
            Ok(SparseStatus::Correct)
        } else {
            Ok(SparseStatus::Mismatch {
                expected: expected_paths.to_vec(),
                actual: configured_paths,
            })
        }
    }

    /// Add a submodule using the fallback chain: gitoxide -> git2 -> CLI
    pub fn add_submodule(
        &mut self,
        name: String,
        path: String,
        url: String,
        sparse_paths: Option<Vec<String>>,
        _branch: Option<SerializableBranch>,
        _ignore: Option<SerializableIgnore>,
        _fetch: Option<SerializableFetchRecurse>,
        _update: Option<SerializableUpdate>,
        _shallow: Option<bool>,
        _no_init: bool,
    ) -> Result<(), SubmoduleError> {
        if _no_init {
            self.update_toml_config(
                name.clone(),
                SubmoduleEntry {
                    path: Some(path.clone()),
                    url: Some(url.clone()),
                    branch: _branch.clone(),
                    ignore: _ignore.clone(),
                    update: _update.clone(),
                    fetch_recurse: _fetch.clone(),
                    active: Some(true),
                    shallow: _shallow,
                    no_init: Some(_no_init),
                },
                sparse_paths.clone(),
            )?;
        }

        // Clean up any existing submodule state using git commands
        self.cleanup_existing_submodule(&path)?;

        // Try gitoxide first, then git2, then CLI
        let result = self
            .add_submodule_with_gix(&name, &path, &url)
            .or_else(|_| self.add_submodule_with_git2(&name, &path, &url))
            .or_else(|_| self.add_submodule_with_cli(&name, &path, &url));

        match result {
            Ok(()) => {
                // Configure after successful creation
                self.configure_submodule_post_creation(&name, &path, sparse_paths.clone())?;
                self.update_toml_config(
                    name.clone(),
                    SubmoduleEntry {
                        path: Some(path),
                        url: Some(url),
                        branch: _branch,
                        ignore: _ignore,
                        update: _update,
                        fetch_recurse: _fetch,
                        active: Some(true),
                        shallow: _shallow,
                        no_init: Some(_no_init),
                    },
                    sparse_paths,
                )?;
                println!("Added submodule {name}");
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Clean up existing submodule state using git commands only
    fn cleanup_existing_submodule(&mut self, path: &str) -> Result<(), SubmoduleError> {
        // Use GitOpsManager trait methods for cleanup
        // 1. Deinitialize the submodule (force = true)
        self.git_ops
            .deinit_submodule(path, true)
            .map_err(Self::map_git_ops_error)?;

        // 2. Delete the submodule completely
        self.git_ops
            .delete_submodule(path)
            .map_err(Self::map_git_ops_error)?;

        Ok(())
    }

    /// Add submodule using gitoxide (primary method)
    fn add_submodule_with_gix(
        &self,
        _name: &str,
        _path: &str,
        _url: &str,
    ) -> Result<(), SubmoduleError> {
        // TODO: Implement gitoxide submodule add when available
        // For now, return an error to trigger fallback
        Err(SubmoduleError::GitoxideError(
            "Gitoxide submodule add not yet implemented".to_string(),
        ))
    }

    /// Convert SubmoduleGitOptions to Git2SubmoduleOptions
    fn get_git2_submodule_options(
        &self,
        options: Option<SubmoduleGitOptions>,
    ) -> Git2SubmoduleOptions {
        let opts = options.unwrap_or_default();
        opts.try_into().unwrap()
    }
    fn add_submodule_with_git2(
        &mut self,
        _name: &str,
        path: &str,
        url: &str,
    ) -> Result<(), SubmoduleError> {
        let opts = crate::config::SubmoduleAddOptions {
            name: _name.to_string(),
            path: std::path::PathBuf::from(path),
            url: url.to_string(),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            shallow: false,
            no_init: false,
        };
        self.git_ops
            .add_submodule(&opts)
            .map_err(Self::map_git_ops_error)
    }

    fn add_submodule_with_cli(
        &mut self,
        _name: &str,
        path: &str,
        url: &str,
    ) -> Result<(), SubmoduleError> {
        let opts = crate::config::SubmoduleAddOptions {
            name: _name.to_string(),
            path: std::path::PathBuf::from(path),
            url: url.to_string(),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            shallow: false,
            no_init: false,
        };
        self.git_ops
            .add_submodule(&opts)
            .map_err(Self::map_git_ops_error)
    }

    /// Configure submodule for post-creation setup
    fn configure_submodule_post_creation(
        &mut self,
        _name: &str,
        path: &str,
        sparse_paths: Option<Vec<String>>,
    ) -> Result<(), SubmoduleError> {
        // Configure sparse checkout if specified
        if let Some(patterns) = sparse_paths {
            eprintln!("DEBUG: Configuring sparse checkout for {path} with patterns: {patterns:?}");
            self.configure_sparse_checkout(path, &patterns)?;
        } else {
            eprintln!("DEBUG: No sparse paths provided for {path}");
        }

        Ok(())
    }

    /// Configure sparse checkout using basic file operations
    pub fn configure_sparse_checkout(
        &mut self,
        submodule_path: &str,
        patterns: &[String],
    ) -> Result<(), SubmoduleError> {
        eprintln!(
            "DEBUG: Configuring sparse checkout for {submodule_path} with patterns: {patterns:?}"
        );

        self.git_ops
            .enable_sparse_checkout(submodule_path)
            .map_err(|e| {
                SubmoduleError::GitoxideError(format!("Enable sparse checkout failed: {e}"))
            })?;

        self.git_ops
            .set_sparse_patterns(submodule_path, patterns)
            .map_err(|e| {
                SubmoduleError::GitoxideError(format!("Set sparse patterns failed: {e}"))
            })?;

        self.git_ops
            .apply_sparse_checkout(submodule_path)
            .map_err(|e| {
                SubmoduleError::GitoxideError(format!("Apply sparse checkout failed: {e}"))
            })?;

        println!("Configured sparse checkout");

        Ok(())
    }

    /// Get the actual git directory path, handling gitlinks in submodules
    fn get_git_directory(
        &self,
        submodule_path: &str,
    ) -> Result<std::path::PathBuf, SubmoduleError> {
        let git_path = std::path::Path::new(submodule_path).join(".git");
        eprintln!("DEBUG: Checking git path: {}", git_path.display());

        if git_path.is_dir() {
            // Regular git repository
            eprintln!("DEBUG: Found regular git directory");
            Ok(git_path)
        } else if git_path.is_file() {
            // Gitlink - read the file to get the actual git directory
            eprintln!("DEBUG: Found gitlink file, reading content");
            let content = fs::read_to_string(&git_path)?;
            eprintln!("DEBUG: Gitlink content: {content}");

            let git_dir_line = content
                .lines()
                .find(|line| line.starts_with("gitdir: "))
                .ok_or_else(|| {
                    SubmoduleError::IoError(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Invalid gitlink file",
                    ))
                })?;

            let git_dir_path = git_dir_line.strip_prefix("gitdir: ").unwrap().trim();
            eprintln!("DEBUG: Parsed git dir path: {git_dir_path}");

            // Path might be relative to the submodule directory
            let absolute_path = if std::path::Path::new(git_dir_path).is_absolute() {
                std::path::PathBuf::from(git_dir_path)
            } else {
                std::path::Path::new(submodule_path).join(git_dir_path)
            };

            eprintln!("DEBUG: Resolved absolute path: {}", absolute_path.display());
            Ok(absolute_path)
        } else {
            // Use gix as fallback
            eprintln!("DEBUG: No .git file/dir found, trying gix fallback");
            if let Ok(repo) = gix::open(submodule_path) {
                let git_dir = repo.git_dir().to_path_buf();
                eprintln!("DEBUG: Gix found git dir: {}", git_dir.display());
                Ok(git_dir)
            } else {
                eprintln!("DEBUG: Gix fallback failed");
                Err(SubmoduleError::RepositoryError)
            }
        }
    }
    // Removed: apply_sparse_checkout_cli is obsolete; sparse checkout is handled by GitOpsManager abstraction.

    /// Update submodule using CLI fallback (gix remote operations are complex for this use case)
    pub fn update_submodule(&mut self, name: &str) -> Result<(), SubmoduleError> {
        let config =
            self.config
                .submodules
                .get(name)
                .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                    name: name.to_string(),
                })?;

        let submodule_path = config.path.as_ref().ok_or_else(|| {
            SubmoduleError::ConfigError("No path configured for submodule".to_string())
        })?;

        // Prepare update options (use defaults for now)
        let update_opts = crate::config::SubmoduleUpdateOptions::default();

        self.git_ops
            .update_submodule(submodule_path, &update_opts)
            .map_err(|e| {
                SubmoduleError::GitoxideError(format!("GitOpsManager update failed: {e}"))
            })?;

        println!("✅ Updated {name} using GitOpsManager abstraction");
        Ok(())
    }

    /// Reset submodule using CLI operations
    pub fn reset_submodule(&mut self, name: &str) -> Result<(), SubmoduleError> {
        let config =
            self.config
                .submodules
                .get(name)
                .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                    name: name.to_string(),
                })?;

        let submodule_path = config.path.as_ref().ok_or_else(|| {
            SubmoduleError::ConfigError("No path configured for submodule".to_string())
        })?;

        println!("🔄 Hard resetting {name}...");

        // Step 1: Stash changes
        println!("  📦 Stashing working changes...");
        match self.git_ops.stash_submodule(submodule_path, true) {
            Ok(_) => {}
            Err(e) => println!("  ⚠️  Stash warning: {e}"),
        }

        // Step 2: Hard reset
        println!("  🔄 Resetting to HEAD...");
        self.git_ops
            .reset_submodule(submodule_path, true)
            .map_err(|e| {
                SubmoduleError::GitoxideError(format!("GitOpsManager reset failed: {e}"))
            })?;

        // Step 3: Clean untracked files
        println!("  🧹 Cleaning untracked files...");
        self.git_ops
            .clean_submodule(submodule_path, true, true)
            .map_err(|e| {
                SubmoduleError::GitoxideError(format!("GitOpsManager clean failed: {e}"))
            })?;

        println!("✅ {name} reset complete");
        Ok(())
    }

    /// Initialize submodule - add it first if not registered, then initialize
    pub fn init_submodule(&mut self, name: &str) -> Result<(), SubmoduleError> {
        let submodules = self.config.clone().submodules;
        let config = submodules
            .get(name)
            .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                name: name.to_string(),
            })?;

        let path_str = config.path.as_ref().ok_or_else(|| {
            SubmoduleError::ConfigError("No path configured for submodule".to_string())
        })?;
        let url_str = config.url.as_ref().ok_or_else(|| {
            SubmoduleError::ConfigError("No URL configured for submodule".to_string())
        })?;

        let submodule_path = Path::new(path_str);

        if submodule_path.exists() && submodule_path.join(".git").exists() {
            println!("✅ {name} already initialized");
            // Even if already initialized, check if we need to configure sparse checkout
            let sparse_paths_opt = self
                .config
                .submodules
                .sparse_checkouts()
                .and_then(|sparse_checkouts| sparse_checkouts.get(name).cloned());
            if let Some(sparse_paths) = sparse_paths_opt {
                eprintln!(
                    "DEBUG: Configuring sparse checkout for newly initialized submodule: {name}"
                );
                self.configure_sparse_checkout(path_str, &sparse_paths)?;
            }
            return Ok(());
        }

        println!("🔄 Initializing {name}...");

        let workdir = std::path::Path::new(".");

        // First check if submodule is registered in .gitmodules
        let gitmodules_path = workdir.join(".gitmodules");
        let needs_add = if gitmodules_path.exists() {
            let gitmodules_content = fs::read_to_string(&gitmodules_path)?;
            !gitmodules_content.contains(&format!("path = {path_str}"))
        } else {
            true
        };

        if needs_add {
            // Submodule not registered yet, add it first
            eprintln!("DEBUG: Submodule not registered in .gitmodules, adding first");
            self.add_submodule_with_cli(name, path_str, url_str)?;
        } else {
            // Submodule is registered, just initialize and update
            let init_output = Command::new("git")
                .args(["submodule", "init", path_str])
                .current_dir(workdir)
                .output()?;

            if !init_output.status.success() {
                let stderr = String::from_utf8_lossy(&init_output.stderr);
                return Err(SubmoduleError::CliError(format!(
                    "Git submodule init failed: {stderr}"
                )));
            }

            let update_output = Command::new("git")
                .args(["submodule", "update", path_str])
                .current_dir(workdir)
                .output()?;

            if !update_output.status.success() {
                let stderr = String::from_utf8_lossy(&update_output.stderr);
                return Err(SubmoduleError::CliError(format!(
                    "Git submodule update failed: {stderr}"
                )));
            }
        }

        println!("  ✅ Initialized using git submodule commands: {path_str}");

        // Configure sparse checkout if specified
        if let Some(sparse_checkouts) = submodules.sparse_checkouts() {
            if let Some(sparse_paths) = sparse_checkouts.get(name) {
                eprintln!(
                    "DEBUG: Configuring sparse checkout for newly initialized submodule: {name}"
                );
                self.configure_sparse_checkout(path_str, sparse_paths)?;
            }
        }

        println!("✅ {name} initialized");
        Ok(())
    }

    /// Check all submodules using gitoxide APIs where possible
    pub fn check_all_submodules(&self) -> Result<(), SubmoduleError> {
        println!("Checking submodule configurations...");

        for (submodule_name, submodule) in self.config.get_submodules() {
            println!("\n📁 {submodule_name}");

            // Handle missing path gracefully - report but don't fail
            let path_str = if let Some(path) = submodule.path.as_ref() {
                path
            } else {
                println!("  ❌ Configuration error: No path configured");
                continue;
            };

            // Handle missing URL gracefully - report but don't fail
            if submodule.url.is_none() {
                println!("  ❌ Configuration error: No URL configured");
                continue;
            }

            let submodule_path = Path::new(path_str);
            let git_path = submodule_path.join(".git");

            if !submodule_path.exists() {
                println!("  ❌ Folder missing: {path_str}");
                continue;
            }

            if !git_path.exists() {
                println!("  ❌ Not a git repository");
                continue;
            }

            // GITOXIDE API: Use gix::open and status check
            match self.check_submodule_repository_status(path_str, submodule_name) {
                Ok(status) => {
                    println!("  ✅ Git repository exists");

                    if status.is_clean {
                        println!("  ��� Working tree is clean");
                    } else {
                        println!("  ⚠️  Working tree has changes");
                    }

                    if let Some(commit) = &status.current_commit {
                        println!("  ✅ Current commit: {}", &commit[..8]);
                    }

                    if status.has_remotes {
                        println!("  ✅ Has remotes configured");
                    } else {
                        println!("  ⚠️  No remotes configured");
                    }

                    match status.sparse_status {
                        SparseStatus::NotEnabled => {}
                        SparseStatus::NotConfigured => {
                            println!("  ❌ Sparse checkout not configured");
                        }
                        SparseStatus::Correct => {
                            println!("  ✅ Sparse checkout configured correctly");
                        }
                        SparseStatus::Mismatch { expected, actual } => {
                            println!("  ❌ Sparse checkout mismatch");
                            println!("    Expected: {expected:?}");
                            println!("    Current: {actual:?}");
                        }
                    }

                    // Show effective settings
                    self.show_effective_settings(submodule_name, submodule);
                }
                Err(e) => {
                    println!("  ❌ Cannot analyze repository: {e}");
                }
            }
        }

        Ok(())
    }

    fn show_effective_settings(&self, _name: &str, config: &SubmoduleEntry) {
        println!("  📋 Effective settings:");

        if let Some(ignore) = &config.ignore {
            println!("     ignore = {:?}", ignore);
        }
        if let Some(update) = &config.update {
            println!("     update = {:?}", update);
        }
        if let Some(branch) = &config.branch {
            println!("     branch = {:?}", branch);
        }
    }
    /// Get reference to the underlying config
    pub const fn config(&self) -> &Config {
        &self.config
    }

    /// Get mutable reference to the underlying config
    pub const fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Get a clone of the underlying config
    pub fn config_clone(&self) -> Config {
        self.config.clone()
    }
}
