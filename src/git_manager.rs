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

use crate::config::{Config, SubmoduleEntry};
use crate::git_ops::GitOperations;
use crate::git_ops::GitOpsManager;
use crate::options::{
    SerializableBranch, SerializableFetchRecurse, SerializableIgnore, SerializableUpdate,
};
use std::fs;
use std::path::{Path, PathBuf};

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
        mut entry: crate::config::SubmoduleEntry,
        sparse_paths: Option<Vec<String>>,
    ) -> Result<(), SubmoduleError> {
        if let Some(ref paths) = sparse_paths {
            entry.sparse_paths = Some(paths.clone());
            // Also populate sparse_checkouts so consumers using sparse_checkouts() see the paths
            self.config
                .submodules
                .add_checkout(name.clone(), paths.clone(), true);
        }
        // Normalize: convert Unspecified variants to None so they serialize cleanly
        if matches!(entry.ignore, Some(SerializableIgnore::Unspecified)) {
            entry.ignore = None;
        }
        if matches!(
            entry.fetch_recurse,
            Some(SerializableFetchRecurse::Unspecified)
        ) {
            entry.fetch_recurse = None;
        }
        if matches!(entry.update, Some(SerializableUpdate::Unspecified)) {
            entry.update = None;
        }
        self.config.add_submodule(name, entry);
        self.save_config()
    }

    /// Save the current in-memory configuration to the config file
    fn save_config(&self) -> Result<(), SubmoduleError> {
        // Read existing TOML to preserve content (defaults, comments, existing entries)
        let existing = if self.config_path.exists() {
            std::fs::read_to_string(&self.config_path)
                .map_err(|e| SubmoduleError::ConfigError(format!("Failed to read config: {e}")))?
        } else {
            String::new()
        };

        let mut output = existing.clone();

        // Append any new submodule sections not already in the file
        for (name, entry) in self.config.get_submodules() {
            // Determine whether this name needs quoting (contains TOML-special characters).
            // Simple names (alphanumeric, hyphens, underscores) can use the bare [name] form.
            let needs_quoting = name
                .chars()
                .any(|c| !c.is_alphanumeric() && c != '-' && c != '_');
            let escaped_name = name.replace('\\', "\\\\").replace('"', "\\\"");
            let section_header = if needs_quoting {
                format!("[\"{escaped_name}\"]")
            } else {
                format!("[{name}]")
            };
            // Check at line boundaries to avoid false positives from comments/values.
            // Accept either quoted or unquoted form so existing files written before this
            // change are recognised.
            let already_present = existing.lines().any(|line| {
                let trimmed = line.trim();
                trimmed == section_header
                    || trimmed == format!("[{name}]")
                    || trimmed == format!("[\"{escaped_name}\"]")
            });
            if !already_present {
                output.push('\n');
                output.push_str(&section_header);
                output.push('\n');
                if let Some(path) = &entry.path {
                    output.push_str(&format!(
                        "path = \"{}\"\n",
                        path.replace('\\', "\\\\").replace('"', "\\\"")
                    ));
                }
                if let Some(url) = &entry.url {
                    output.push_str(&format!(
                        "url = \"{}\"\n",
                        url.replace('\\', "\\\\").replace('"', "\\\"")
                    ));
                }
                if let Some(branch) = &entry.branch {
                    let val = branch.to_string();
                    if !val.is_empty() {
                        output.push_str(&format!(
                            "branch = \"{}\"\n",
                            val.replace('\\', "\\\\").replace('"', "\\\"")
                        ));
                    }
                }
                if let Some(ignore) = &entry.ignore {
                    let val = ignore.to_string();
                    if !val.is_empty() {
                        output.push_str(&format!("ignore = \"{val}\"\n"));
                    }
                }
                if let Some(fetch_recurse) = &entry.fetch_recurse {
                    let val = fetch_recurse.to_string();
                    if !val.is_empty() {
                        output.push_str(&format!("fetch = \"{val}\"\n"));
                    }
                }
                if let Some(update) = &entry.update {
                    let val = update.to_string();
                    if !val.is_empty() {
                        output.push_str(&format!("update = \"{val}\"\n"));
                    }
                }
                if let Some(active) = entry.active {
                    output.push_str(&format!("active = {active}\n"));
                }
                if let Some(shallow) = entry.shallow {
                    if shallow {
                        output.push_str("shallow = true\n");
                    }
                }
                if let Some(sparse_paths) = &entry.sparse_paths {
                    if !sparse_paths.is_empty() {
                        let joined = sparse_paths
                            .iter()
                            .map(|p| {
                                format!("\"{}\"", p.replace('\\', "\\\\").replace('"', "\\\""))
                            })
                            .collect::<Vec<_>>()
                            .join(", ");
                        output.push_str(&format!("sparse_paths = [{joined}]\n"));
                    }
                }
            }
        }

        std::fs::write(&self.config_path, &output).map_err(|e| {
            SubmoduleError::ConfigError(format!("Failed to write config file: {e}"))
        })?;
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
            .map(|subs| subs.map_or(false, |mut iter| iter.next().is_some()))
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
                    sparse_paths: None,
                },
                sparse_paths.clone(),
            )?;
            // When requested, only update configuration without touching repository state.
            return Ok(());
        }

        // Clean up any existing submodule state using git commands
        self.cleanup_existing_submodule(&path)?;

        let opts = crate::config::SubmoduleAddOptions {
            name: name.clone(),
            path: std::path::PathBuf::from(&path),
            url: url.clone(),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            shallow: false,
            no_init: false,
        };
        match self
            .git_ops
            .add_submodule(&opts)
            .map_err(Self::map_git_ops_error)
        {
            Ok(()) => {
                // Configure after successful submodule creation (clone/init handled by the underlying backend, currently the git CLI)
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
                        sparse_paths: None, // stored separately via configure_submodule_post_creation
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
        // Best-effort cleanup of any existing submodule state
        // These operations may fail if the submodule doesn't exist yet, which is fine,
        // but other errors (permissions, corruption, etc.) should at least be visible.
        if let Err(e) = self.git_ops.deinit_submodule(path, true) {
            eprintln!("Warning: failed to deinit submodule at '{}': {:?}", path, e);
        }
        if let Err(e) = self.git_ops.delete_submodule(path) {
            eprintln!("Warning: failed to delete submodule at '{}': {:?}", path, e);
        }
        Ok(())
    }

    /// Configure submodule for post-creation setup
    fn configure_submodule_post_creation(
        &mut self,
        _name: &str,
        path: &str,
        sparse_paths: Option<Vec<String>>,
    ) -> Result<(), SubmoduleError> {
        // Only configure git-level sparse checkout if the submodule directory exists
        // (it may not exist yet if --no-init was used)
        let submodule_exists = std::path::Path::new(path).exists();
        if submodule_exists {
            if let Some(patterns) = sparse_paths {
                self.configure_sparse_checkout(path, &patterns)?;
            }
        }
        Ok(())
    }

    /// Configure sparse checkout using basic file operations
    pub fn configure_sparse_checkout(
        &mut self,
        submodule_path: &str,
        patterns: &[String],
    ) -> Result<(), SubmoduleError> {
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

        if git_path.is_dir() {
            // Regular git repository
            Ok(git_path)
        } else if git_path.is_file() {
            // Gitlink - read the file to get the actual git directory
            let content = fs::read_to_string(&git_path)?;

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

            // Path might be relative to the submodule directory
            let absolute_path = if std::path::Path::new(git_dir_path).is_absolute() {
                std::path::PathBuf::from(git_dir_path)
            } else {
                std::path::Path::new(submodule_path).join(git_dir_path)
            };

            Ok(absolute_path)
        } else {
            // Use gix as fallback
            if let Ok(repo) = gix::open(submodule_path) {
                Ok(repo.git_dir().to_path_buf())
            } else {
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

        println!("✅ Updated {name} successfully");
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
            // Submodule not registered yet, add it first via GitOpsManager
            let opts = crate::config::SubmoduleAddOptions {
                name: name.to_string(),
                path: std::path::PathBuf::from(path_str),
                url: url_str.to_string(),
                branch: None,
                ignore: None,
                update: None,
                fetch_recurse: None,
                shallow: false,
                no_init: false,
            };
            self.git_ops
                .add_submodule(&opts)
                .map_err(Self::map_git_ops_error)?;
        } else {
            // Submodule is registered, just initialize and update using GitOperations
            self.git_ops
                .init_submodule(path_str)
                .map_err(Self::map_git_ops_error)?;

            let update_opts = crate::config::SubmoduleUpdateOptions::default();
            self.git_ops
                .update_submodule(path_str, &update_opts)
                .map_err(Self::map_git_ops_error)?;
        }

        println!("  ✅ Initialized using git submodule commands: {path_str}");

        // Configure sparse checkout if specified
        if let Some(sparse_checkouts) = submodules.sparse_checkouts() {
            if let Some(sparse_paths) = sparse_checkouts.get(name) {
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
    #[allow(dead_code)]
    pub const fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Get a clone of the underlying config
    #[allow(dead_code)]
    pub fn config_clone(&self) -> Config {
        self.config.clone()
    }

    /// Extract the submodule name from a TOML section header line, e.g. `[my-sub]` → `my-sub`.
    /// Returns `None` if the line does not look like a section header.
    fn section_name_from_header(header: &str) -> Option<String> {
        let inner = header.trim().strip_prefix('[')?.strip_suffix(']')?;
        // Reject table-array headers like `[[...]]`
        if inner.starts_with('[') {
            return None;
        }
        if inner.starts_with('"') {
            // Quoted: ["some name"]
            let unquoted = inner.strip_prefix('"')?.strip_suffix('"')?;
            // Un-escape backslash-escaped backslashes and quotes (order matters: \\ first)
            Some(unquoted.replace("\\\\", "\\").replace("\\\"", "\""))
        } else {
            Some(inner.to_string())
        }
    }

    /// Serialize the given `SubmoduleEntry` to a list of key = value lines (no section header).
    fn entry_to_kv_lines(entry: &SubmoduleEntry) -> Vec<(String, String)> {
        let mut kv: Vec<(String, String)> = Vec::new();
        if let Some(path) = &entry.path {
            kv.push((
                "path".into(),
                format!("\"{}\"", path.replace('\\', "\\\\").replace('"', "\\\"")),
            ));
        }
        if let Some(url) = &entry.url {
            kv.push((
                "url".into(),
                format!("\"{}\"", url.replace('\\', "\\\\").replace('"', "\\\"")),
            ));
        }
        if let Some(branch) = &entry.branch {
            let val = branch.to_string();
            if !val.is_empty() {
                kv.push((
                    "branch".into(),
                    format!("\"{}\"", val.replace('\\', "\\\\").replace('"', "\\\"")),
                ));
            }
        }
        if let Some(ignore) = &entry.ignore {
            let val = ignore.to_string();
            if !val.is_empty() {
                kv.push(("ignore".into(), format!("\"{val}\"")));
            }
        }
        if let Some(fetch_recurse) = &entry.fetch_recurse {
            let val = fetch_recurse.to_string();
            if !val.is_empty() {
                kv.push(("fetch".into(), format!("\"{val}\"")));
            }
        }
        if let Some(update) = &entry.update {
            let val = update.to_string();
            if !val.is_empty() {
                kv.push(("update".into(), format!("\"{val}\"")));
            }
        }
        if let Some(active) = entry.active {
            kv.push(("active".into(), active.to_string()));
        }
        if let Some(shallow) = entry.shallow {
            if shallow {
                kv.push(("shallow".into(), "true".into()));
            }
        }
        if let Some(sparse_paths) = &entry.sparse_paths {
            if !sparse_paths.is_empty() {
                let joined = sparse_paths
                    .iter()
                    .map(|p| format!("\"{}\"", p.replace('\\', "\\\\").replace('"', "\\\"")))
                    .collect::<Vec<_>>()
                    .join(", ");
                kv.push(("sparse_paths".into(), format!("[{joined}]")));
            }
        }
        kv
    }

    /// Known submodule key names (used to identify which lines to update vs. preserve).
    const KNOWN_SUBMODULE_KEYS: &'static [&'static str] = &[
        "path",
        "url",
        "branch",
        "ignore",
        "fetch",
        "update",
        "active",
        "shallow",
        "sparse_paths",
    ];

    /// Known [defaults] key names.
    const KNOWN_DEFAULTS_KEYS: &'static [&'static str] = &["ignore", "fetch", "update"];

    /// Return the key name if `line` is a key = value assignment for one of `known_keys`, else None.
    fn line_key<'a>(line: &str, known_keys: &[&'a str]) -> Option<&'a str> {
        let trimmed = line.trim();
        // Skip comments and blank lines quickly
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return None;
        }
        for key in known_keys {
            // Match "key =" or "key=" at start of trimmed line
            if trimmed.starts_with(key) {
                let rest = &trimmed[key.len()..];
                if rest.starts_with('=') || rest.starts_with(" =") {
                    return Some(key);
                }
            }
        }
        None
    }

    /// Rewrite the config file while preserving existing comments, unknown keys, and formatting.
    ///
    /// For each existing section in the file:
    /// - If the section name is still in the in-memory config: update key values in place,
    ///   preserving comments and the original order of known keys.
    /// - If the section name is no longer in config: the section is omitted (deleted).
    ///
    /// Sections in the in-memory config that were not in the original file are appended at the end.
    ///
    /// The `[defaults]` section is handled similarly (updated in place or added if absent).
    fn write_full_config(&self) -> Result<(), SubmoduleError> {
        let existing = if self.config_path.exists() {
            std::fs::read_to_string(&self.config_path)
                .map_err(|e| SubmoduleError::ConfigError(format!("Failed to read config: {e}")))?
        } else {
            String::new()
        };

        // Build the current submodule map sorted by name for deterministic append order
        let current_entries: std::collections::BTreeMap<String, &SubmoduleEntry> = self
            .config
            .get_submodules()
            .map(|(n, e)| (n.clone(), e))
            .collect();

        // Track which names appeared in the existing file (so we know what to append)
        let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut seen_defaults = false;

        // Parse the file into sections.
        // Each element: (header_line, body_lines)
        // Preamble (before any section header) stored as ("", preamble_lines).
        let mut sections: Vec<(String, Vec<String>)> = Vec::new();
        {
            let mut preamble: Vec<String> = Vec::new();
            let mut current_header: Option<String> = None;
            let mut current_body: Vec<String> = Vec::new();
            for raw_line in existing.lines() {
                let trimmed = raw_line.trim();
                // Detect a section header (but not a table-array `[[...]]`)
                let is_header = trimmed.starts_with('[')
                    && !trimmed.starts_with("[[")
                    && trimmed.ends_with(']');
                if is_header {
                    if let Some(hdr) = current_header.take() {
                        sections.push((hdr, std::mem::take(&mut current_body)));
                    } else {
                        // Flush preamble
                        sections.push((String::new(), std::mem::take(&mut preamble)));
                    }
                    current_header = Some(raw_line.to_string());
                } else if let Some(ref _hdr) = current_header {
                    current_body.push(raw_line.to_string());
                } else {
                    preamble.push(raw_line.to_string());
                }
            }
            // Flush last section or preamble
            if let Some(hdr) = current_header {
                sections.push((hdr, current_body));
            } else {
                sections.push((String::new(), preamble));
            }
        }

        let defaults = &self.config.defaults;
        let defaults_kv: Vec<(String, String)> = {
            let mut kv = Vec::new();
            if let Some(ignore) = &defaults.ignore {
                let val = ignore.to_string();
                if !val.is_empty() {
                    kv.push(("ignore".into(), format!("\"{val}\"")));
                }
            }
            if let Some(fetch_recurse) = &defaults.fetch_recurse {
                let val = fetch_recurse.to_string();
                if !val.is_empty() {
                    kv.push(("fetch".into(), format!("\"{val}\"")));
                }
            }
            if let Some(update) = &defaults.update {
                let val = update.to_string();
                if !val.is_empty() {
                    kv.push(("update".into(), format!("\"{val}\"")));
                }
            }
            kv
        };

        let mut output = String::new();

        for (header, body) in &sections {
            if header.is_empty() {
                // Preamble: write as-is
                for line in body {
                    output.push_str(line);
                    output.push('\n');
                }
                continue;
            }

            let sec_name = Self::section_name_from_header(header).unwrap_or_default();

            if sec_name == "defaults" {
                seen_defaults = true;
                // Rewrite [defaults] section preserving comments
                output.push_str(header);
                output.push('\n');
                let new_body =
                    Self::merge_section_body(body, &defaults_kv, Self::KNOWN_DEFAULTS_KEYS);
                for line in &new_body {
                    output.push_str(line);
                    output.push('\n');
                }
                continue;
            }

            // Submodule section
            seen_names.insert(sec_name.clone());
            if let Some(entry) = current_entries.get(sec_name.as_str()) {
                let kv = Self::entry_to_kv_lines(entry);
                output.push_str(header);
                output.push('\n');
                let new_body = Self::merge_section_body(body, &kv, Self::KNOWN_SUBMODULE_KEYS);
                for line in &new_body {
                    output.push_str(line);
                    output.push('\n');
                }
            }
            // else: section was deleted from config — omit it
        }

        // Append [defaults] if it wasn't in the existing file
        if !seen_defaults && !defaults_kv.is_empty() {
            output.push_str("[defaults]\n");
            for (key, val) in &defaults_kv {
                output.push_str(&format!("{key} = {val}\n"));
            }
            output.push('\n');
        }

        // Append submodule sections that weren't in the existing file (sorted for determinism)
        for (name, entry) in &current_entries {
            if !seen_names.contains(name.as_str()) {
                let needs_quoting = name
                    .chars()
                    .any(|c| !c.is_alphanumeric() && c != '-' && c != '_');
                let escaped_name = name.replace('\\', "\\\\").replace('"', "\\\"");
                let section_header = if needs_quoting {
                    format!("[\"{escaped_name}\"]")
                } else {
                    format!("[{name}]")
                };
                output.push_str(&section_header);
                output.push('\n');
                for (key, val) in Self::entry_to_kv_lines(entry) {
                    output.push_str(&format!("{key} = {val}\n"));
                }
                output.push('\n');
            }
        }

        std::fs::write(&self.config_path, &output).map_err(|e| {
            SubmoduleError::ConfigError(format!("Failed to write config file: {e}"))
        })?;
        Ok(())
    }

    /// Merge new key=value pairs into existing section body lines, preserving comments and
    /// unknown keys. Known keys that appear in `body` are updated to the new value; known keys
    /// absent from `body` but present in `new_kv` are appended at the end of the body.
    /// Known keys in `body` that are absent from `new_kv` are removed.
    fn merge_section_body(
        body: &[String],
        new_kv: &[(String, String)],
        known_keys: &[&str],
    ) -> Vec<String> {
        // Build a lookup of new values by key
        let kv_map: std::collections::HashMap<&str, &str> = new_kv
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();

        let mut emitted_keys: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut result: Vec<String> = Vec::new();

        for line in body {
            if let Some(key) = Self::line_key(line, known_keys) {
                if let Some(new_val) = kv_map.get(key) {
                    // Replace existing key line with new value, preserving inline comment if any
                    let comment_part = Self::extract_inline_comment(line);
                    if comment_part.is_empty() {
                        result.push(format!("{key} = {new_val}"));
                    } else {
                        result.push(format!("{key} = {new_val}  {comment_part}"));
                    }
                    emitted_keys.insert(key);
                }
                // else: key no longer present in new config → drop the line
            } else {
                // Not a known key line (comment, blank line, unknown key): preserve
                result.push(line.clone());
            }
        }

        // Append any new keys (from new_kv) that were not already in the body
        for (key, val) in new_kv {
            if !emitted_keys.contains(key.as_str()) {
                result.push(format!("{key} = {val}"));
            }
        }

        result
    }

    /// Extract an inline comment (e.g. `# ...`) from a TOML value line, if any.
    /// Returns the comment portion including `#`, or an empty string.
    fn extract_inline_comment(line: &str) -> &str {
        // Find `#` that is not inside a quoted string. We use a simple heuristic:
        // scan for ` #` (with space) OR `#` at the start of remaining content after
        // the first `=`. TOML allows `key = value# comment` without a space.
        // This heuristic won't handle `#` inside quoted values, but our generated TOML is safe.
        if let Some(eq_pos) = line.find('=') {
            let after_eq = &line[eq_pos + 1..];
            // Find the first unquoted `#` in the value portion
            let mut in_quote = false;
            for (i, ch) in after_eq.char_indices() {
                match ch {
                    '"' => in_quote = !in_quote,
                    '#' if !in_quote => return &after_eq[i..],
                    _ => {}
                }
            }
        }
        ""
    }

    /// List all submodules from the config. If `recursive` is true, also lists
    /// submodules found in the git repository (which may include nested ones).
    pub fn list_submodules(&self, recursive: bool) -> Result<(), SubmoduleError> {
        let submodules: Vec<_> = self.config.get_submodules().collect();

        if submodules.is_empty() && !recursive {
            println!("No submodules configured.");
            return Ok(());
        }

        if !submodules.is_empty() {
            println!("Submodules:");
            for (name, entry) in &submodules {
                let path = entry.path.as_deref().unwrap_or("<no path>");
                let url = entry.url.as_deref().unwrap_or("<no url>");
                let active = entry.active.unwrap_or(true);
                let active_str = if active { "active" } else { "disabled" };
                println!("  {name} [{active_str}]");
                println!("    path: {path}");
                println!("    url:  {url}");
            }
        } else {
            println!("No submodules configured.");
        }

        if recursive {
            // Also list submodules found in the git repository (may include nested ones)
            match self.git_ops.list_submodules() {
                Ok(git_submodules) => {
                    let config_paths: std::collections::HashSet<String> = submodules
                        .iter()
                        .filter_map(|(_, e)| e.path.clone())
                        .collect();
                    let extra: Vec<_> = git_submodules
                        .iter()
                        .filter(|p| !config_paths.contains(*p))
                        .collect();
                    if !extra.is_empty() {
                        println!("\nAdditional submodules found in git (not in config):");
                        for path in extra {
                            println!("  {path}");
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Warning: could not list git submodules: {e}");
                }
            }
        }

        Ok(())
    }

    /// Update global default settings and save the config.
    pub fn update_global_defaults(
        &mut self,
        ignore: Option<SerializableIgnore>,
        fetch_recurse: Option<SerializableFetchRecurse>,
        update: Option<SerializableUpdate>,
    ) -> Result<(), SubmoduleError> {
        if ignore.is_none() && fetch_recurse.is_none() && update.is_none() {
            return Err(SubmoduleError::ConfigError(
                "No settings provided to change.".to_string(),
            ));
        }
        if let Some(i) = ignore {
            self.config.defaults.ignore = Some(i);
        }
        if let Some(f) = fetch_recurse {
            self.config.defaults.fetch_recurse = Some(f);
        }
        if let Some(u) = update {
            self.config.defaults.update = Some(u);
        }
        self.write_full_config()
    }

    /// Disable a submodule by setting `active = false` in the config and deinitializing it.
    pub fn disable_submodule(&mut self, name: &str) -> Result<(), SubmoduleError> {
        let entry = self
            .config
            .get_submodule(name)
            .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                name: name.to_string(),
            })?
            .clone();

        let path = entry.path.as_deref().unwrap_or(name).to_string();

        // Deinit from git (best-effort; ignore errors if not initialized)
        let _ = self.git_ops.deinit_submodule(&path, false);

        // Update the entry in config
        let mut updated = entry.clone();
        updated.active = Some(false);
        self.config
            .submodules
            .update_entry(name.to_string(), updated);

        self.write_full_config()?;
        println!("Disabled submodule '{name}'.");
        Ok(())
    }

    /// Delete a submodule: deinit, remove from filesystem, and remove from config.
    pub fn delete_submodule_by_name(&mut self, name: &str) -> Result<(), SubmoduleError> {
        let entry = self
            .config
            .get_submodule(name)
            .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                name: name.to_string(),
            })?
            .clone();

        let path = entry.path.as_deref().unwrap_or(name).to_string();

        // Deinit (best-effort — submodule may not be registered in .gitmodules)
        let _ = self.git_ops.deinit_submodule(&path, true);
        // Git-layer delete (best-effort — submodule may only be in our config, not .gitmodules)
        if let Err(e) = self.git_ops.delete_submodule(&path) {
            eprintln!("Note: git cleanup for '{name}' skipped: {e}");
            // Still try to remove the directory from the filesystem directly
            let dir = std::path::Path::new(&path);
            if dir.exists() {
                let _ = fs::remove_dir_all(dir);
            }
        }

        // Remove from config
        self.config.submodules.remove_submodule(name);
        self.write_full_config()?;
        println!("Deleted submodule '{name}'.");
        Ok(())
    }

    /// Change settings of an existing submodule. If `path` changes, the submodule is
    /// deleted and re-cloned at the new location.
    #[allow(clippy::too_many_arguments)]
    pub fn change_submodule(
        &mut self,
        name: &str,
        path: Option<std::ffi::OsString>,
        branch: Option<String>,
        sparse_paths: Option<Vec<std::ffi::OsString>>,
        append_sparse: bool,
        ignore: Option<SerializableIgnore>,
        fetch: Option<SerializableFetchRecurse>,
        update: Option<SerializableUpdate>,
        shallow: Option<bool>,
        url: Option<String>,
        active: Option<bool>,
    ) -> Result<(), SubmoduleError> {
        let entry = self
            .config
            .get_submodule(name)
            .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                name: name.to_string(),
            })?
            .clone();

        let new_path = path.as_ref().map(|p| p.to_string_lossy().to_string());

        // If path is changing, delete and re-add
        if let Some(ref np) = new_path {
            let old_path = entry.path.as_deref().unwrap_or(name);
            if np != old_path {
                let sub_url = url
                    .as_deref()
                    .or(entry.url.as_deref())
                    .ok_or_else(|| {
                        SubmoduleError::ConfigError(
                            "Cannot re-clone submodule: no URL available.".to_string(),
                        )
                    })?
                    .to_string();

                // Delete old then re-add at new path
                self.delete_submodule_by_name(name)?;

                // Compute effective branch: caller's value if provided, else preserve existing
                let effective_branch = if branch.is_some() {
                    SerializableBranch::set_branch(branch.clone())
                        .map_err(|e| SubmoduleError::ConfigError(e.to_string()))?
                } else {
                    entry.branch.clone().unwrap_or_default()
                };

                // Compute effective sparse paths: caller's value if provided, else preserve existing
                let effective_sparse = if let Some(ref sp) = sparse_paths {
                    let paths: Vec<String> =
                        sp.iter().map(|p| p.to_string_lossy().to_string()).collect();
                    if paths.is_empty() { None } else { Some(paths) }
                } else {
                    entry.sparse_paths.clone().filter(|v| !v.is_empty())
                };

                let effective_ignore = ignore.or(entry.ignore);
                let effective_fetch = fetch.or(entry.fetch_recurse);
                let effective_update = update.or(entry.update);
                // Preserve shallow/active from entry unless caller explicitly set them
                let effective_shallow = shallow.or(entry.shallow);

                self.add_submodule(
                    name.to_string(),
                    np.clone().into(),
                    sub_url,
                    effective_sparse,
                    Some(effective_branch),
                    effective_ignore,
                    effective_fetch,
                    effective_update,
                    effective_shallow,
                    false,
                )?;
                return Ok(());
            }
        }

        // Otherwise update fields in place
        {
            let entry = self
                .config
                .get_submodule(name)
                .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                    name: name.to_string(),
                })?
                .clone();
            let mut updated = entry;
            if let Some(np) = new_path {
                updated.path = Some(np);
            }
            if let Some(b) = branch {
                updated.branch = SerializableBranch::set_branch(Some(b))
                    .map(Some)
                    .map_err(|err| SubmoduleError::ConfigError(err.to_string()))?;
            }
            if let Some(i) = ignore {
                updated.ignore = Some(i);
            }
            if let Some(f) = fetch {
                updated.fetch_recurse = Some(f);
            }
            if let Some(u) = update {
                updated.update = Some(u);
            }
            if let Some(new_url) = url {
                updated.url = Some(new_url);
            }
            if let Some(a) = active {
                updated.active = Some(a);
            }
            if let Some(s) = shallow {
                updated.shallow = Some(s);
            }

            // Update sparse paths
            if let Some(new_sparse) = sparse_paths {
                let new_paths: Vec<String> = new_sparse
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();
                if append_sparse {
                    let existing = updated.sparse_paths.get_or_insert_with(Vec::new);
                    existing.extend(new_paths.clone());
                } else {
                    updated.sparse_paths = Some(new_paths.clone());
                }

                // Keep SubmoduleEntries.sparse_checkouts in sync with sparse_paths
                let replace = !append_sparse;
                self.config
                    .submodules
                    .add_checkout(name.to_string(), new_paths, replace);
            }
            self.config
                .submodules
                .update_entry(name.to_string(), updated);
        }

        self.write_full_config()?;
        println!("Updated submodule '{name}'.");
        Ok(())
    }

    /// Nuke (deinit + delete + remove from config) all or specific submodules.
    /// If `kill` is false, reinitializes them after deletion.
    pub fn nuke_submodules(
        &mut self,
        all: bool,
        names: Option<Vec<String>>,
        kill: bool,
    ) -> Result<(), SubmoduleError> {
        let targets: Vec<String> = if all {
            self.config
                .get_submodules()
                .map(|(n, _)| n.clone())
                .collect()
        } else {
            names.unwrap_or_default()
        };

        if targets.is_empty() {
            return Err(SubmoduleError::ConfigError(
                "No submodules specified. Use --all or provide names.".to_string(),
            ));
        }

        // Snapshot entries before deleting (needed for reinit)
        let snapshots: Vec<(String, SubmoduleEntry)> = targets
            .iter()
            .filter_map(|n| self.config.get_submodule(n).map(|e| (n.clone(), e.clone())))
            .collect();

        // Validate all targets exist before starting
        for name in &targets {
            if self.config.get_submodule(name).is_none() {
                return Err(SubmoduleError::SubmoduleNotFound { name: name.clone() });
            }
        }

        for name in &targets {
            println!("💥 Nuking submodule '{name}'...");
            self.delete_submodule_by_name(name)?;
        }

        if !kill {
            // Reinitialize each deleted submodule
            for (name, entry) in snapshots {
                let url = match entry.url.clone() {
                    Some(u) if !u.is_empty() => u,
                    _ => {
                        eprintln!("Skipping reinit of '{name}': no URL in config entry.");
                        continue;
                    }
                };
                println!("🔄 Reinitializing submodule '{name}'...");
                let path = entry.path.as_deref().unwrap_or(&name).to_string();
                let sparse = entry.sparse_paths.clone().filter(|paths| !paths.is_empty());
                self.add_submodule(
                    name.clone(),
                    path.into(),
                    url,
                    sparse,
                    entry.branch.clone(),
                    entry.ignore,
                    entry.fetch_recurse,
                    entry.update,
                    entry.shallow,
                    false,
                )?;
            }
        }

        Ok(())
    }

    /// Generate a config file. If `from_setup` is true, reads `.gitmodules` from the repo.
    /// If `template` is true, writes an annotated sample config.
    /// If the output file exists and `force` is false, returns an error.
    pub fn generate_config(
        output: &std::path::Path,
        from_setup: bool,
        template: bool,
        force: bool,
    ) -> Result<(), SubmoduleError> {
        if output.exists() && !force {
            return Err(SubmoduleError::ConfigError(format!(
                "Output file '{}' already exists. Use --force to overwrite.",
                output.display()
            )));
        }

        if template {
            // Write an annotated sample config
            let sample = include_str!("../sample_config/submod.toml");
            std::fs::write(output, sample).map_err(SubmoduleError::IoError)?;
            println!("Generated template config at '{}'.", output.display());
            return Ok(());
        }

        if from_setup {
            // Read .gitmodules from the repo and convert to our config format
            let git_ops = crate::git_ops::GitOpsManager::new(Some(std::path::Path::new(".")))
                .map_err(|_| SubmoduleError::RepositoryError)?;
            let entries = git_ops.read_gitmodules().map_err(|e| {
                SubmoduleError::ConfigError(format!("Failed to read .gitmodules: {e}"))
            })?;

            // Build a Config from the SubmoduleEntries
            let config = Config::new(crate::config::SubmoduleDefaults::default(), entries);

            // Serialize using write_full_config logic but to the output path
            let tmp_manager = GitManager {
                git_ops,
                config,
                config_path: output.to_path_buf(),
            };
            tmp_manager.write_full_config()?;
            println!(
                "Generated config from .gitmodules at '{}'.",
                output.display()
            );
            return Ok(());
        }

        // Neither template nor from-setup: write an empty config
        let empty = "[defaults]\n";
        std::fs::write(output, empty).map_err(SubmoduleError::IoError)?;
        println!("Generated empty config at '{}'.", output.display());
        Ok(())
    }
}
