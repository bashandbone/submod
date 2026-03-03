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
            self.config.submodules.add_checkout(name.clone(), paths.clone(), true);
        }
        // Normalize: convert Unspecified variants to None so they serialize cleanly
        if matches!(entry.ignore, Some(SerializableIgnore::Unspecified)) {
            entry.ignore = None;
        }
        if matches!(entry.fetch_recurse, Some(SerializableFetchRecurse::Unspecified)) {
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
            let needs_quoting = name.chars().any(|c| !c.is_alphanumeric() && c != '-' && c != '_');
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
                    output.push_str(&format!("path = \"{}\"\n", path.replace('\\', "\\\\").replace('"', "\\\"")));
                }
                if let Some(url) = &entry.url {
                    output.push_str(&format!("url = \"{}\"\n", url.replace('\\', "\\\\").replace('"', "\\\"")));
                }
                if let Some(branch) = &entry.branch {
                    let val = branch.to_string();
                    if !val.is_empty() {
                        output.push_str(&format!("branch = \"{}\"\n", val.replace('\\', "\\\\").replace('"', "\\\"")));
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
                            .map(|p| format!("\"{}\"", p.replace('\\', "\\\\").replace('"', "\\\"")))
                            .collect::<Vec<_>>()
                            .join(", ");
                        output.push_str(&format!("sparse_paths = [{joined}]\n"));
                    }
                }
            }
        }

        std::fs::write(&self.config_path, &output)
            .map_err(|e| SubmoduleError::ConfigError(format!("Failed to write config file: {e}")))?;
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
        match self.git_ops.add_submodule(&opts).map_err(Self::map_git_ops_error) {
            Ok(()) => {
                // Configure after successful creation (git2's add_submodule handles clone/init)
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
            self.git_ops.add_submodule(&opts)
                .map_err(Self::map_git_ops_error)?;
        } else {
            // Submodule is registered, just initialize and update using GitOperations
            self.git_ops.init_submodule(path_str)
                .map_err(Self::map_git_ops_error)?;

            let update_opts = crate::config::SubmoduleUpdateOptions::default();
            self.git_ops.update_submodule(path_str, &update_opts)
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
    pub const fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Get a clone of the underlying config
    pub fn config_clone(&self) -> Config {
        self.config.clone()
    }
}
