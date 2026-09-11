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
- Uses a **deny-all-by-default** (modified cone pattern) model: the `!/*` pattern is
  automatically prepended to the user-supplied patterns so that _only_ the explicitly
  listed paths are checked out.  Users simply list what they want to include; no
  knowledge of git's pattern ordering rules is required.

## Error Handling

All operations return [`SubmoduleError`](src/git_manager.rs:14) for consistent error reporting.

## Usage

Use this module as the backend for CLI commands to manage submodules in a repository. See the project [README](README.md) for usage examples and configuration details.
"]

use crate::config::{Config, SubmoduleEntry};
use crate::git_ops::GitOperations;
use crate::git_ops::GitOpsManager;
use crate::options::{
    SerializableBranch, SerializableFetchRecurse, SerializableIgnore, SerializableUpdate,
};
use crate::utilities::RepositoryContext;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::Write as _;
use std::path::{Path, PathBuf};

/// The deny-all pattern prepended to sparse-checkout files in deny-all-by-default mode.
///
/// Placing `!/*` as the first line ensures that all paths are excluded by
/// default and only the explicitly listed include patterns are checked out
/// (the "modified cone pattern" model).
///
/// This pattern is **not** written when there are no include patterns (i.e., when the
/// caller passes an empty list or a list consisting entirely of blank strings), and it
/// is **not** written when the submodule opts out via `use_git_default_sparse_checkout`.
const SPARSE_DENY_ALL: &str = "!/*";

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
    #[error("Repository error: {0}")]
    #[allow(dead_code)]
    RepositoryError(String),

    /// A read-only check found repository state that differs from the declaration.
    #[error("State drift: {0}")]
    Drift(String),

    /// A preflighted batch stopped after some per-module outcomes were known.
    #[error("{summary}Cause: {cause}")]
    IncompleteBatch {
        /// Structured completed, failed, and pending module results.
        summary: OperationSummary,
        /// Preserved native operation cause.
        cause: String,
    },

    /// Submodule path is invalid or escapes repository root
    #[error("Invalid path: {0}")]
    InvalidPath(String),
}

impl SubmoduleError {
    /// Return the documented process status for this error category.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        match self {
            Self::ConfigError(_) | Self::InvalidPath(_) | Self::SubmoduleNotFound { .. } => 2,
            Self::GitoxideError(_)
            | Self::Git2Error(_)
            | Self::CliError(_)
            | Self::IoError(_)
            | Self::RepositoryError(_)
            | Self::Drift(_)
            | Self::IncompleteBatch { .. } => 1,
        }
    }
}

/// Status information for a submodule
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
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

/// Desired-state planner and reconciler for managed submodules
pub struct GitManager {
    /// Native Git mutation boundary plus backend inspection reads
    git_ops: GitOpsManager,
    /// Native Git paths and the resolved config for this invocation.
    context: RepositoryContext,
    /// Configuration for submodules
    config: Config,
    /// Exact bytes used to parse `config`; compared again immediately before replacement.
    loaded_config_bytes: Option<Vec<u8>>,
    /// Raw declarations parsed from `loaded_config_bytes`.
    loaded_config: Config,
    /// Fields explicitly named by the command, including canonicalization-only edits.
    pending_edits: ConfigEditIntent,
    /// Path to the configuration file
    config_path: PathBuf,
    /// Whether to print verbose output
    verbose: bool,
}

#[derive(Default)]
struct ConfigEditIntent {
    defaults: std::collections::BTreeSet<&'static str>,
    modules: std::collections::BTreeMap<String, std::collections::BTreeSet<&'static str>>,
}

impl ConfigEditIntent {
    fn default_field(&mut self, field: &'static str) {
        self.defaults.insert(field);
    }

    fn module_field(&mut self, name: &str, field: &'static str) {
        self.modules
            .entry(name.to_string())
            .or_default()
            .insert(field);
    }

    fn module_contains(&self, name: &str, field: &str) -> bool {
        self.modules
            .get(name)
            .is_some_and(|fields| fields.contains(field))
    }
}

struct MutationLocks {
    paths: Vec<PathBuf>,
    _files: Vec<File>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReconcileScope {
    Init,
    Update,
    Sync,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModuleOutcomeKind {
    Changed,
    Unchanged,
    SkippedDisabled,
    SkippedPolicy,
    ChangedSkippedDisabled,
    ChangedSkippedPolicy,
    Failed,
    Pending,
}

#[derive(Clone, Debug)]
// Five bools, each gating a distinct, independently-tested reconciliation
// branch (registration, metadata, sparse, remote, recursive). Bundling them
// into flags would obscure the per-branch reads in the outcome builder.
#[allow(clippy::struct_excessive_bools)]
struct ReconcilePlan {
    name: String,
    path: String,
    kind: ModuleOutcomeKind,
    detail: String,
    target: Option<String>,
    verbose_detail: Option<String>,
    registration_changed: bool,
    metadata_changed: bool,
    sparse_changed: bool,
    remote_requested: bool,
    recursive_requested: bool,
    initial_head: Option<String>,
    initial_recursive_state: Option<Vec<u8>>,
}

/// Structured results for one CLI lifecycle operation.
#[derive(Debug)]
pub struct OperationSummary {
    operation: &'static str,
    preview: bool,
    modules: Vec<ReconcilePlan>,
}

impl std::fmt::Display for OperationSummary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.modules.is_empty() {
            return writeln!(formatter, "No submodules configured.");
        }

        let mut changed = 0_usize;
        let mut unchanged = 0_usize;
        let mut skipped = 0_usize;
        let mut failed = 0_usize;
        let mut pending = 0_usize;
        for module in &self.modules {
            let status = if self.preview {
                match module.kind {
                    ModuleOutcomeKind::Changed => "would-change",
                    ModuleOutcomeKind::Unchanged => "unchanged",
                    ModuleOutcomeKind::SkippedDisabled => "would-skip-disabled",
                    ModuleOutcomeKind::SkippedPolicy => "would-skip-policy",
                    ModuleOutcomeKind::ChangedSkippedDisabled => "would-change/would-skip-disabled",
                    ModuleOutcomeKind::ChangedSkippedPolicy => "would-change/would-skip-policy",
                    ModuleOutcomeKind::Failed => "would-fail",
                    ModuleOutcomeKind::Pending => "pending",
                }
            } else {
                match module.kind {
                    ModuleOutcomeKind::Changed => "changed",
                    ModuleOutcomeKind::Unchanged => "unchanged",
                    ModuleOutcomeKind::SkippedDisabled => "skipped-disabled",
                    ModuleOutcomeKind::SkippedPolicy => "skipped-policy",
                    ModuleOutcomeKind::ChangedSkippedDisabled => "changed/skipped-disabled",
                    ModuleOutcomeKind::ChangedSkippedPolicy => "changed/skipped-policy",
                    ModuleOutcomeKind::Failed => "failed",
                    ModuleOutcomeKind::Pending => "pending",
                }
            };
            match module.kind {
                ModuleOutcomeKind::Changed => changed += 1,
                ModuleOutcomeKind::Unchanged => unchanged += 1,
                ModuleOutcomeKind::SkippedDisabled | ModuleOutcomeKind::SkippedPolicy => {
                    skipped += 1;
                }
                ModuleOutcomeKind::ChangedSkippedDisabled
                | ModuleOutcomeKind::ChangedSkippedPolicy => {
                    changed += 1;
                    skipped += 1;
                }
                ModuleOutcomeKind::Failed => failed += 1,
                ModuleOutcomeKind::Pending => pending += 1,
            }
            write!(
                formatter,
                "{} at {}: {status}: {}",
                crate::utilities::safe_human_text(&module.name),
                crate::utilities::safe_human_text(&module.path),
                crate::utilities::safe_human_text(&module.detail)
            )?;
            if let Some(target) = &module.target {
                write!(
                    formatter,
                    " (target {})",
                    crate::utilities::safe_human_text(target)
                )?;
            }
            writeln!(formatter)?;
            if let Some(verbose) = &module.verbose_detail {
                writeln!(
                    formatter,
                    "  {}",
                    crate::utilities::safe_human_text(verbose)
                )?;
            }
        }
        writeln!(
            formatter,
            "{} summary: {changed} changed, {unchanged} unchanged, {skipped} skipped, {failed} failed, {pending} pending.",
            self.operation
        )
    }
}

struct AddPlan {
    name: String,
    path: String,
    raw_entry: SubmoduleEntry,
    managed: SubmoduleEntry,
    options: crate::config::SubmoduleAddOptions,
    sparse_paths: Option<Vec<String>>,
    no_init: bool,
}

struct NukePlan {
    name: String,
    effective: SubmoduleEntry,
    options: crate::config::SubmoduleAddOptions,
    registered: bool,
}

struct ChangePlan {
    move_paths: Option<(String, String)>,
    metadata_targets: Vec<(String, String, SubmoduleEntry)>,
}

impl Drop for MutationLocks {
    fn drop(&mut self) {
        for path in self.paths.iter().rev() {
            let _ = fs::remove_file(path);
        }
    }
}

impl GitManager {
    /// Helper method to map git operations errors
    #[allow(clippy::needless_pass_by_value)]
    fn map_git_ops_error(err: anyhow::Error) -> SubmoduleError {
        SubmoduleError::CliError(format!("Git operation failed: {err}"))
    }

    /// Preserve filesystem failures discovered while validating an otherwise
    /// lexical path. Unsafe syntax is an argument error; inability to inspect
    /// a valid path is an operational I/O failure.
    ///
    /// `error` is owned to match `map_err` call sites and the sibling
    /// `map_git_ops_error` helper.
    #[allow(clippy::needless_pass_by_value)]
    fn map_path_validation_error(error: anyhow::Error) -> SubmoduleError {
        error.downcast_ref::<std::io::Error>().map_or_else(
            || SubmoduleError::InvalidPath(error.to_string()),
            |io_error| {
                SubmoduleError::IoError(std::io::Error::new(io_error.kind(), format!("{error:#}")))
            },
        )
    }

    /// Restore `update_toml_config` method
    fn update_toml_config(
        &mut self,
        name: String,
        mut entry: crate::config::SubmoduleEntry,
        sparse_paths: Option<Vec<String>>,
    ) -> Result<(), SubmoduleError> {
        if let Some(paths) = sparse_paths {
            // Move the Vec into entry.sparse_paths to avoid cloning,
            // then borrow it for add_checkout.
            entry.sparse_paths = Some(paths);
            if let Some(ref stored_paths) = entry.sparse_paths {
                // Also populate sparse_checkouts so consumers using sparse_checkouts() see the paths
                self.config
                    .submodules
                    .add_checkout(&name, stored_paths, true);
            }
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

    /// Save the current in-memory configuration to the config file.
    ///
    /// Delegates to [`write_full_config`], which rewrites the file
    /// section-by-section: it preserves the preamble, comments, `[defaults]`,
    /// and any unknown keys, *updates* the bodies of sections that already
    /// exist, and appends sections that are new. The previous implementation
    /// was append-only and silently dropped edits to existing sections (#62 P1).
    fn save_config(&mut self) -> Result<(), SubmoduleError> {
        self.write_full_config()
    }

    fn validate_identity(name: &str) -> Result<(), SubmoduleError> {
        let normalized = crate::utilities::normalize_submodule_path(Path::new(name));
        if matches!(name, "defaults" | "schema_version")
            || name.contains(['\0', '\n', '\r', '\\'])
            || normalized
                .as_ref()
                .map_or(true, |normalized| normalized != Path::new(name))
        {
            return Err(SubmoduleError::InvalidPath(format!(
                "invalid submodule administrative name {name:?}"
            )));
        }
        Ok(())
    }

    fn paths_collide(left: &Path, right: &Path) -> bool {
        let folded = |path: &Path| {
            path.components()
                .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
                .collect::<Vec<_>>()
        };
        let left = folded(left);
        let right = folded(right);
        left.starts_with(&right) || right.starts_with(&left)
    }

    fn validate_all_paths(&self) -> Result<(), SubmoduleError> {
        let mut paths: Vec<(String, PathBuf)> = Vec::new();
        for (name, entry) in self.config.get_submodules() {
            Self::validate_identity(name)?;
            let raw = entry.path.as_deref().unwrap_or(name);
            crate::utilities::validate_submodule_path(&self.context.worktree_root, Path::new(raw))
                .map_err(Self::map_path_validation_error)?;
            let normalized = crate::utilities::normalize_submodule_path(Path::new(raw))
                .map_err(|error| SubmoduleError::InvalidPath(error.to_string()))?;
            paths.push((name.clone(), normalized));
        }
        for left in 0..paths.len() {
            for right in left + 1..paths.len() {
                let (left_name, left_path) = &paths[left];
                let (right_name, right_path) = &paths[right];
                if Self::paths_collide(left_path, right_path) {
                    return Err(SubmoduleError::InvalidPath(format!(
                        "managed paths for {left_name:?} and {right_name:?} collide or overlap"
                    )));
                }
            }
        }
        Ok(())
    }

    fn validate_requested_path(
        &self,
        name: &str,
        path: &Path,
        replacing: Option<&str>,
    ) -> Result<PathBuf, SubmoduleError> {
        Self::validate_identity(name)?;
        crate::utilities::validate_submodule_path(&self.context.worktree_root, path)
            .map_err(Self::map_path_validation_error)?;
        let requested = crate::utilities::normalize_submodule_path(path)
            .map_err(|error| SubmoduleError::InvalidPath(error.to_string()))?;
        for (other_name, entry) in self.config.get_submodules() {
            if replacing == Some(other_name.as_str()) {
                continue;
            }
            let other = crate::utilities::normalize_submodule_path(Path::new(
                entry.path.as_deref().unwrap_or(other_name),
            ))
            .map_err(|error| SubmoduleError::InvalidPath(error.to_string()))?;
            if Self::paths_collide(&requested, &other) {
                return Err(SubmoduleError::InvalidPath(format!(
                    "requested path collides or overlaps managed module {other_name:?}"
                )));
            }
        }
        Ok(requested)
    }

    fn registration_for_path(&self, path: &str) -> Result<Option<String>, SubmoduleError> {
        self.git_ops
            .registration_name(path)
            .map_err(Self::map_git_ops_error)
    }

    fn managed_settings(config: &Config, name: &str) -> Result<SubmoduleEntry, SubmoduleError> {
        let mut declared = config.submodules.get(name).cloned().ok_or_else(|| {
            SubmoduleError::SubmoduleNotFound {
                name: name.to_string(),
            }
        })?;
        declared.branch = declared.branch.or_else(|| config.defaults.branch.clone());
        declared.ignore = declared.ignore.or(config.defaults.ignore);
        declared.update = declared.update.or_else(|| config.defaults.update.clone());
        declared.fetch_recurse = declared.fetch_recurse.or(config.defaults.fetch_recurse);
        declared.active = Some(declared.active.unwrap_or(true));
        Ok(declared)
    }

    fn sync_effective_settings(&self, name: &str, path: &str) -> Result<(), SubmoduleError> {
        let declared = Self::managed_settings(&self.config, name)?;
        self.git_ops
            .sync_submodule_settings(path, &declared)
            .map_err(Self::map_git_ops_error)
    }

    fn metadata_targets(
        &self,
        names: &[String],
    ) -> Result<Vec<(String, String, SubmoduleEntry)>, SubmoduleError> {
        let mut targets = Vec::new();
        for name in names {
            let settings = Self::managed_settings(&self.config, name)?;
            let path = settings.path.clone().unwrap_or_else(|| name.clone());
            if self.registration_for_path(&path)?.is_some() {
                targets.push((name.clone(), path, settings));
            }
        }
        targets.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(targets)
    }

    fn preflight_metadata_targets(
        &self,
        targets: &[(String, String, SubmoduleEntry)],
    ) -> Result<(), SubmoduleError> {
        for (_, path, settings) in targets {
            self.git_ops
                .preflight_submodule_settings(path, settings)
                .map_err(Self::map_git_ops_error)?;
        }
        Ok(())
    }

    fn apply_metadata_targets(
        &self,
        targets: &[(String, String, SubmoduleEntry)],
    ) -> Result<(), SubmoduleError> {
        for (_, path, settings) in targets {
            self.git_ops
                .sync_submodule_settings(path, settings)
                .map_err(Self::map_git_ops_error)?;
        }
        Ok(())
    }

    fn config_lock_path(config_path: &Path) -> Result<PathBuf, SubmoduleError> {
        let config_path = if config_path.is_absolute() {
            config_path.to_path_buf()
        } else {
            std::env::current_dir()?.join(config_path)
        };
        let name = config_path.file_name().ok_or_else(|| {
            SubmoduleError::InvalidPath("config path has no file name".to_string())
        })?;
        let parent = config_path.parent().ok_or_else(|| {
            SubmoduleError::InvalidPath("config path has no parent directory".to_string())
        })?;
        let parent = parent.canonicalize().map_err(|error| {
            SubmoduleError::ConfigError(format!(
                "Could not resolve config directory {}: {error}",
                parent.display()
            ))
        })?;
        let mut lock_name = OsString::from(name);
        lock_name.push(".submod.lock");
        Ok(parent.join(lock_name))
    }

    fn validate_config_destination(config_path: &Path) -> Result<(), SubmoduleError> {
        match fs::symlink_metadata(config_path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(SubmoduleError::ConfigError(format!(
                    "Refusing to write configuration through symlink {}",
                    config_path.display()
                )));
            }
            Ok(metadata) if !metadata.is_file() => {
                return Err(SubmoduleError::ConfigError(format!(
                    "Configuration destination is not a file: {}",
                    config_path.display()
                )));
            }
            Ok(_) => {}
            Err(error) if crate::utilities::is_absent_path(&error) => {}
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    fn acquire_lock_paths(mut paths: Vec<PathBuf>) -> Result<MutationLocks, SubmoduleError> {
        paths.sort();
        paths.dedup();
        let mut files = Vec::with_capacity(paths.len());
        for path in &paths {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
            loop {
                match OpenOptions::new().write(true).create_new(true).open(path) {
                    Ok(file) => {
                        files.push(file);
                        break;
                    }
                    Err(error)
                        if error.kind() == std::io::ErrorKind::AlreadyExists
                            && std::time::Instant::now() < deadline =>
                    {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                    }
                    Err(error) => {
                        for acquired in paths.iter().take(files.len()).rev() {
                            let _ = fs::remove_file(acquired);
                        }
                        return Err(SubmoduleError::ConfigError(format!(
                            "Could not acquire lock {}: {error}. Finish the other submod operation or remove a verified stale lock",
                            path.display()
                        )));
                    }
                }
            }
        }
        Ok(MutationLocks {
            paths,
            _files: files,
        })
    }

    fn acquire_mutation_locks(&self) -> Result<MutationLocks, SubmoduleError> {
        Self::validate_config_destination(&self.config_path)?;
        Self::acquire_lock_paths(vec![
            self.context.common_dir.canonicalize()?.join("submod.lock"),
            Self::config_lock_path(&self.config_path)?,
        ])
    }

    fn acquire_output_lock(output: &Path) -> Result<MutationLocks, SubmoduleError> {
        let output = if output.is_absolute() {
            output.to_path_buf()
        } else {
            std::env::current_dir()?.join(output)
        };
        Self::validate_config_destination(&output)?;
        Self::acquire_lock_paths(vec![Self::config_lock_path(&output)?])
    }

    fn reload_locked_config(&mut self) -> Result<(), SubmoduleError> {
        let (config, bytes) = Self::read_config_snapshot(&self.config_path).map_err(|error| {
            SubmoduleError::ConfigError(format!("Failed to reload locked config: {error}"))
        })?;
        self.loaded_config = config.clone();
        self.config = config;
        self.loaded_config_bytes = bytes;
        self.pending_edits = ConfigEditIntent::default();
        self.validate_all_paths()
    }

    fn read_config_snapshot(path: &Path) -> Result<(Config, Option<Vec<u8>>), SubmoduleError> {
        let bytes = match fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => {
                return Err(SubmoduleError::ConfigError(format!(
                    "Failed to read configuration {}: {error}",
                    path.display()
                )));
            }
        };
        let Some(bytes) = bytes else {
            return Ok((Config::default(), None));
        };
        let source = std::str::from_utf8(&bytes).map_err(|error| {
            SubmoduleError::ConfigError(format!(
                "Configuration {} is not valid UTF-8: {error}",
                path.display()
            ))
        })?;
        let config = Config::parse(source).map_err(|error| {
            SubmoduleError::ConfigError(format!(
                "Invalid configuration {}: {error}",
                path.display()
            ))
        })?;
        Ok((config, Some(bytes)))
    }

    /// Creates a new `GitManager` by loading configuration from the given path
    /// with default (non-verbose) output.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the TOML configuration file.
    ///
    /// # Errors
    ///
    /// Returns `SubmoduleError::RepositoryError` if the repository cannot be discovered,
    /// or `SubmoduleError::ConfigError` if the configuration fails to load.
    #[allow(dead_code)]
    pub fn new(config_path: PathBuf) -> Result<Self, SubmoduleError> {
        Self::with_verbose(config_path, false)
    }

    /// Creates a new `GitManager` with the specified verbosity level.
    pub fn with_verbose(config_path: PathBuf, verbose: bool) -> Result<Self, SubmoduleError> {
        let explicit_config = config_path != Path::new("submod.toml");
        Self::with_verbose_config(config_path, verbose, explicit_config)
    }

    /// Create a manager while retaining whether `--config` was explicitly supplied.
    ///
    /// `config_path` is owned for constructor ergonomics: every call site
    /// already holds an owned path.
    #[allow(clippy::needless_pass_by_value)]
    pub fn with_verbose_config(
        config_path: PathBuf,
        verbose: bool,
        explicit_config: bool,
    ) -> Result<Self, SubmoduleError> {
        let invocation_dir = std::env::current_dir()?;
        let explicit_config = explicit_config.then_some(config_path.as_path());
        if let Some(path) = explicit_config {
            let candidate = invocation_dir.join(path);
            match fs::symlink_metadata(&candidate) {
                Ok(metadata) if !metadata.is_file() && !metadata.file_type().is_symlink() => {
                    return Err(SubmoduleError::ConfigError(format!(
                        "Explicit config path is not a file: {}",
                        candidate.display()
                    )));
                }
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    return Err(SubmoduleError::ConfigError(format!(
                        "Explicit config file not found: {}",
                        candidate.display()
                    )));
                }
                Err(error) => return Err(error.into()),
            }
        }
        let context =
            RepositoryContext::discover(&invocation_dir, explicit_config).map_err(|error| {
                SubmoduleError::RepositoryError(format!(
                    "Failed to discover the repository: {error}"
                ))
            })?;
        let git_ops =
            GitOpsManager::new(Some(&context.worktree_root), verbose).map_err(|error| {
                SubmoduleError::RepositoryError(format!(
                    "Failed to open Git repository at {}: {error}",
                    context.worktree_root.display()
                ))
            })?;

        let (config, loaded_config_bytes) = Self::read_config_snapshot(&context.config_path)?;
        let manager = Self {
            git_ops,
            config_path: context.config_path.clone(),
            context,
            loaded_config: config.clone(),
            config,
            loaded_config_bytes,
            pending_edits: ConfigEditIntent::default(),
            verbose,
        };
        manager.validate_all_paths()?;
        Ok(manager)
    }

    /// Require an existing declaration file for commands that cannot create one.
    pub fn require_config(&self) -> Result<(), SubmoduleError> {
        if self.loaded_config_bytes.is_some() {
            return Ok(());
        }
        Err(SubmoduleError::ConfigError(format!(
            "Configuration '{}' was not found. Run `submod generate-config --from-setup` to import existing Git submodules, or `submod add URL` to create it.",
            self.config_path.display()
        )))
    }

    /// Creates a `GitManager` pointed at an explicit repository path.
    ///
    /// Used in tests to avoid depending on the caller's working directory
    /// being a git repository.
    ///
    /// `config_path` is owned so test call sites can pass temporaries
    /// directly.
    #[cfg(test)]
    #[allow(clippy::needless_pass_by_value)]
    fn with_repo_path(config_path: PathBuf, repo_path: &Path) -> Result<Self, SubmoduleError> {
        let context =
            RepositoryContext::discover(repo_path, Some(&config_path)).map_err(|error| {
                SubmoduleError::ConfigError(format!(
                    "Failed to resolve repository and config path: {error}"
                ))
            })?;
        let git_ops = GitOpsManager::new(Some(&context.worktree_root), false).map_err(|error| {
            SubmoduleError::ConfigError(format!("Failed to open Git repository: {error}"))
        })?;

        let (config, loaded_config_bytes) = Self::read_config_snapshot(&context.config_path)?;
        let manager = Self {
            git_ops,
            config_path: context.config_path.clone(),
            context,
            loaded_config: config.clone(),
            config,
            loaded_config_bytes,
            pending_edits: ConfigEditIntent::default(),
            verbose: false,
        };
        manager.validate_all_paths()?;
        Ok(manager)
    }

    /// Check submodule repository status using gix APIs
    pub fn check_submodule_repository_status(
        &self,
        submodule_path: &str,
        name: &str,
    ) -> Result<SubmoduleStatus, SubmoduleError> {
        // NOTE: This is a legacy direct gix usage for status; could be refactored to use GitOpsManager if needed.
        let rooted_path = self.context.worktree_root.join(submodule_path);
        self.git_ops
            .verify_submodule_checkout(submodule_path)
            .map_err(Self::map_git_ops_error)?;
        let submodule_repo = gix::open(&rooted_path).map_err(|error| {
            SubmoduleError::RepositoryError(format!(
                "Failed to open submodule {name:?} at {}: {error}",
                rooted_path.display()
            ))
        })?;

        // GITOXIDE API: Determine whether the worktree has uncommitted changes.
        // `is_dirty()` runs the real status computation (modified tracked files
        // and untracked files alike). If it cannot be determined, conservatively
        // report dirty so a potential problem is surfaced rather than hidden.
        let is_dirty = submodule_repo.is_dirty().unwrap_or(true);

        // GITOXIDE API: Use reference APIs for current commit
        let current_commit = submodule_repo
            .head()
            .ok()
            .and_then(|head| head.id().map(|id| id.to_string()));

        // GITOXIDE API: Use remote APIs to check if remotes exist
        let has_remotes = !submodule_repo.remote_names().is_empty();

        // For now, consider all submodules active if they exist in config
        let is_active = self.config.submodules.contains_key(name);

        let effective =
            self.config
                .effective_entry(name)
                .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                    name: name.to_string(),
                })?;
        let expected_sparse = if effective.use_git_default_sparse_checkout.unwrap_or(false) {
            effective.sparse_paths.unwrap_or_default()
        } else {
            Self::build_deny_all_sparse_patterns(effective.sparse_paths.as_deref().unwrap_or(&[]))
        };
        let sparse_status = self.check_sparse_checkout_status(submodule_path, &expected_sparse)?;
        // Check if submodule has its own submodules
        let has_submodules = submodule_repo
            .submodules()
            .is_ok_and(|subs| subs.is_some_and(|mut iter| iter.next().is_some()));

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

    /// Check whether the sparse-checkout configuration for a submodule matches
    /// the expected paths.
    ///
    /// Returns [`SparseStatus::Correct`] only when the ordered user patterns match.
    /// Extra, missing, or reordered patterns are a mismatch because each can change
    /// which files Git materializes.
    pub fn check_sparse_checkout_status(
        &self,
        submodule_path: &str,
        expected_paths: &[String],
    ) -> Result<SparseStatus, SubmoduleError> {
        let (enabled, cone, configured_paths) = self
            .git_ops
            .sparse_checkout_state(submodule_path)
            .map_err(Self::map_git_ops_error)?;
        if expected_paths.is_empty() && !enabled {
            Ok(SparseStatus::NotEnabled)
        } else if !enabled {
            Ok(SparseStatus::NotConfigured)
        } else if !cone && expected_paths == configured_paths {
            Ok(SparseStatus::Correct)
        } else {
            Ok(SparseStatus::Mismatch {
                expected: expected_paths.to_vec(),
                actual: configured_paths,
            })
        }
    }

    #[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
    fn prepare_add(
        &self,
        name: String,
        path: String,
        url: String,
        sparse_paths: Option<Vec<String>>,
        branch: Option<SerializableBranch>,
        ignore: Option<SerializableIgnore>,
        fetch_recurse: Option<SerializableFetchRecurse>,
        update: Option<SerializableUpdate>,
        shallow: Option<bool>,
        no_init: bool,
        use_git_default_sparse_checkout: Option<bool>,
    ) -> Result<AddPlan, SubmoduleError> {
        if self.config.get_submodule(&name).is_some() {
            return Err(SubmoduleError::ConfigError(format!(
                "submodule {name:?} is already declared"
            )));
        }
        let path = crate::config::stored_submodule_path(&self.validate_requested_path(
            &name,
            Path::new(&path),
            None,
        )?);
        let raw_entry = SubmoduleEntry {
            path: Some(path.clone()),
            url: Some(url.clone()),
            branch,
            ignore,
            update,
            fetch_recurse,
            active: None,
            shallow,
            no_init: None,
            sparse_paths: sparse_paths.clone(),
            use_git_default_sparse_checkout,
        };
        let mut prospective = self.config.clone();
        prospective.add_submodule(name.clone(), raw_entry.clone());
        let managed = Self::managed_settings(&prospective, &name)?;
        let options = crate::config::SubmoduleAddOptions {
            name: name.clone(),
            path: PathBuf::from(&path),
            url,
            branch: managed.branch.clone(),
            ignore: managed.ignore,
            update: managed.update.clone(),
            fetch_recurse: managed.fetch_recurse,
            shallow: managed.shallow.unwrap_or(false),
            no_init,
        };
        if !no_init {
            self.git_ops
                .preflight_add_submodule(&options)
                .map_err(Self::map_git_ops_error)?;
        }
        Ok(AddPlan {
            name,
            path,
            raw_entry,
            managed,
            options,
            sparse_paths,
            no_init,
        })
    }

    /// Validate and describe an add without locks, writes, staging, or remote access.
    #[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
    pub fn preview_add_submodule(
        &self,
        name: String,
        path: String,
        url: String,
        sparse_paths: Option<Vec<String>>,
        branch: Option<SerializableBranch>,
        ignore: Option<SerializableIgnore>,
        fetch_recurse: Option<SerializableFetchRecurse>,
        update: Option<SerializableUpdate>,
        shallow: Option<bool>,
        no_init: bool,
        use_git_default_sparse_checkout: Option<bool>,
    ) -> Result<(), SubmoduleError> {
        let plan = self.prepare_add(
            name,
            path,
            url,
            sparse_paths,
            branch,
            ignore,
            fetch_recurse,
            update,
            shallow,
            no_init,
            use_git_default_sparse_checkout,
        )?;
        if plan.no_init {
            println!(
                "{} at {}: would-change: write the TOML declaration only; no Git metadata, index, checkout, or remote access.",
                crate::utilities::safe_human_text(&plan.name),
                crate::utilities::safe_human_text(&plan.path)
            );
        } else {
            let branch = plan.managed.branch.as_ref().map_or_else(
                || "the remote default branch".to_string(),
                |value| format!("branch {}", value.as_config_value()),
            );
            let policy = if plan.managed.update == Some(SerializableUpdate::None) {
                "; the explicit add still creates this initial checkout despite update=none"
            } else {
                ""
            };
            println!(
                "{} at {}: would-change: clone from {} using {}, stage its .gitmodules registration and resulting gitlink, reconcile managed Git settings, and write the TOML declaration{}.",
                crate::utilities::safe_human_text(&plan.name),
                crate::utilities::safe_human_text(&plan.path),
                crate::utilities::safe_human_text(&plan.options.url),
                crate::utilities::safe_human_text(&branch),
                policy,
            );
        }
        Ok(())
    }

    /// Add a submodule through the shared native Git lifecycle.
    #[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
    pub fn add_submodule(
        &mut self,
        name: String,
        path: String,
        url: String,
        sparse_paths: Option<Vec<String>>,
        branch: Option<SerializableBranch>,
        ignore: Option<SerializableIgnore>,
        fetch_recurse: Option<SerializableFetchRecurse>,
        update: Option<SerializableUpdate>,
        shallow: Option<bool>,
        no_init: bool,
        use_git_default_sparse_checkout: Option<bool>,
    ) -> Result<(), SubmoduleError> {
        let _locks = self.acquire_mutation_locks()?;
        self.reload_locked_config()?;
        let plan = self.prepare_add(
            name,
            path,
            url,
            sparse_paths,
            branch,
            ignore,
            fetch_recurse,
            update,
            shallow,
            no_init,
            use_git_default_sparse_checkout,
        )?;
        let AddPlan {
            name,
            path,
            raw_entry,
            managed,
            options,
            sparse_paths,
            no_init,
        } = plan;

        if no_init {
            self.update_toml_config(name, raw_entry, sparse_paths)?;
            // When requested, only update configuration without touching repository state.
            return Ok(());
        }

        match self
            .git_ops
            .add_submodule(&options)
            .map_err(Self::map_git_ops_error)
        {
            Ok(()) => {
                self.git_ops
                    .sync_added_submodule_settings(&path, &managed)
                    .map_err(Self::map_git_ops_error)?;
                // Store the opt-out flag in config before configuring sparse checkout
                // so that the helper can resolve it.
                self.config.add_submodule(name.clone(), raw_entry.clone());
                // Configure after successful submodule creation
                self.configure_submodule_post_creation(&name, &path, sparse_paths.clone())?;
                self.update_toml_config(name.clone(), raw_entry, sparse_paths)?;
                println!(
                    "Added submodule {}",
                    crate::utilities::safe_human_text(&name)
                );
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Configure submodule for post-creation setup
    fn configure_submodule_post_creation(
        &self,
        name: &str,
        path: &str,
        sparse_paths: Option<Vec<String>>,
    ) -> Result<(), SubmoduleError> {
        // Only configure git-level sparse checkout if the submodule directory exists
        // (it may not exist yet if --no-init was used)
        let submodule_exists = self.context.worktree_root.join(path).exists();
        if submodule_exists && let Some(patterns) = sparse_paths {
            let use_git_default = self.effective_use_git_default_sparse_checkout(name);
            self.configure_sparse_checkout(path, &patterns, use_git_default)?;
        }
        Ok(())
    }

    /// Configure sparse checkout using basic file operations.
    ///
    /// By default (`use_git_default = false`) the deny-all-by-default model is applied:
    /// `!/*` is prepended so only the explicitly listed `patterns` are checked out, and a
    /// one-time informational message is printed to help users understand the behavior and
    /// opt out if needed.
    ///
    /// When `use_git_default = true` the patterns are written as-is, matching git's own
    /// sparse-checkout semantics.
    pub fn configure_sparse_checkout(
        &self,
        submodule_path: &str,
        patterns: &[String],
        use_git_default: bool,
    ) -> Result<(), SubmoduleError> {
        let effective_patterns = if use_git_default {
            // Pass through unchanged — caller opts out of the deny-all model.
            patterns.to_vec()
        } else {
            // Normalize to the deny-all-by-default model.
            let normalized = Self::build_deny_all_sparse_patterns(patterns);
            if !normalized.is_empty() {
                eprintln!(
                    "submod uses a deny-all-by-default sparse-checkout model: `!/*` is \
                     automatically prepended so only the paths you list are checked out.\n\
                     To use git's default behavior instead, set \
                     `use_git_default_sparse_checkout = true` in your submod.toml (globally \
                     under `[defaults]` or per submodule) or pass \
                     `--use-git-default-sparse-checkout`."
                );
            }
            normalized
        };

        let changed = self
            .git_ops
            .reconcile_sparse_checkout(submodule_path, &effective_patterns)
            .map_err(Self::map_git_ops_error)?;
        if changed {
            if effective_patterns.is_empty() {
                println!("Disabled sparse checkout and restored the full checkout");
            } else {
                println!("Configured sparse checkout");
            }
        }

        Ok(())
    }

    /// Normalizes the input by stripping blank entries and removing any existing `!/*`
    /// entries, then prepends a single `!/*` when at least one include pattern remains.
    ///
    /// This implements the "modified cone pattern" approach: all paths are denied by
    /// default and only the explicitly listed patterns are checked out. This makes the
    /// intent clear and avoids surprises from git's default include-everything behavior.
    ///
    /// Blank entries (empty or whitespace-only strings) are stripped before processing.
    /// If no non-blank include patterns remain after normalization, an empty list is
    /// returned (no sparse-checkout file is written for an empty pattern list).
    fn build_deny_all_sparse_patterns(patterns: &[String]) -> Vec<String> {
        // Strip blank entries that can arrive from empty CLI values (e.g., --sparse-paths "").
        // Also remove any existing deny-all markers so we can prepend a single canonical one.
        let includes: Vec<String> = patterns
            .iter()
            .filter_map(|p| {
                let trimmed = p.trim();
                if trimmed.is_empty() || trimmed == SPARSE_DENY_ALL {
                    None
                } else {
                    Some(p.clone())
                }
            })
            .collect();

        if includes.is_empty() {
            Vec::new()
        } else {
            let mut result = Vec::with_capacity(includes.len() + 1);
            result.push(SPARSE_DENY_ALL.to_string());
            result.extend(includes);
            result
        }
    }

    /// Resolve the effective `use_git_default_sparse_checkout` setting for a submodule.
    ///
    /// The per-submodule entry takes precedence over the global `[defaults]` setting.
    /// When neither is set, `false` is returned (submod's deny-all-by-default model).
    fn effective_use_git_default_sparse_checkout(&self, submodule_name: &str) -> bool {
        self.config
            .effective_entry(submodule_name)
            .and_then(|entry| entry.use_git_default_sparse_checkout)
            .unwrap_or(false)
    }

    // Removed: get_git_directory was unused.

    // Removed: apply_sparse_checkout_cli is obsolete; sparse checkout is handled by GitOpsManager abstraction.

    /// Update a submodule to its parent-recorded commit using the effective strategy.
    pub fn update_submodule(&mut self, name: &str) -> Result<(), SubmoduleError> {
        let _locks = self.acquire_mutation_locks()?;
        self.reload_locked_config()?;
        self.update_submodule_locked(name, false, false)
    }

    fn update_submodule_locked(
        &mut self,
        name: &str,
        remote: bool,
        recursive: bool,
    ) -> Result<(), SubmoduleError> {
        let config =
            self.config
                .effective_entry(name)
                .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                    name: name.to_string(),
                })?;
        let submodule_path = config.path.clone().expect("effective path is populated");
        let registered = self.registration_for_path(&submodule_path)?.is_some();
        if registered {
            self.sync_effective_settings(name, &submodule_path)?;
        }
        if !config.active.unwrap_or(true) || config.update == Some(SerializableUpdate::None) {
            return Ok(());
        }
        if !registered {
            return Err(SubmoduleError::ConfigError(format!(
                "submodule {name:?} is not registered; run init or sync"
            )));
        }
        let mut update_opts =
            crate::config::SubmoduleUpdateOptions::from_options(config.git_options());
        update_opts.remote = remote;
        update_opts.recursive = recursive;

        self.git_ops
            .update_submodule(&submodule_path, &update_opts)
            .map_err(Self::map_git_ops_error)?;
        self.git_ops
            .verify_submodule_checkout(&submodule_path)
            .map_err(Self::map_git_ops_error)?;

        let sparse_paths = config.sparse_paths.as_deref().unwrap_or(&[]);
        let use_git_default = config.use_git_default_sparse_checkout.unwrap_or(false);
        self.configure_sparse_checkout(&submodule_path, sparse_paths, use_git_default)?;

        Ok(())
    }

    /// Reset submodule using CLI operations
    pub fn reset_submodule(&mut self, name: &str) -> Result<(), SubmoduleError> {
        self.reset_submodules(false, vec![name.to_string()])
    }

    fn prepare_reset(
        &self,
        all: bool,
        names: Vec<String>,
    ) -> Result<Vec<(String, String, String)>, SubmoduleError> {
        let mut names = if all {
            self.config
                .get_submodules()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>()
        } else {
            names
        };
        names.sort();
        if names.is_empty() {
            return Err(SubmoduleError::ConfigError(
                "No submodules specified for reset".to_string(),
            ));
        }
        if names.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(SubmoduleError::ConfigError(
                "Duplicate submodule target in reset selection".to_string(),
            ));
        }
        names
            .into_iter()
            .map(|name| {
                let effective = self
                    .config
                    .effective_entry(&name)
                    .ok_or_else(|| SubmoduleError::SubmoduleNotFound { name: name.clone() })?;
                let path = effective
                    .path
                    .expect("effective submodule path is populated");
                let target = self
                    .git_ops
                    .preflight_reset_submodule(&path)
                    .map_err(Self::map_git_ops_error)?;
                Ok((name, path, target))
            })
            .collect()
    }

    /// Preview preservation and reset targets without creating a stash or changing files.
    pub fn preview_reset_submodules(
        &self,
        all: bool,
        names: Vec<String>,
    ) -> Result<(), SubmoduleError> {
        self.require_config()?;
        for (name, path, target) in self.prepare_reset(all, names)? {
            println!(
                "Would preserve local work and reset submodule '{name}' at '{path}' to parent pin {target}.",
                name = crate::utilities::safe_human_text(&name),
                path = crate::utilities::safe_human_text(&path),
                target = crate::utilities::safe_human_text(&target)
            );
        }
        Ok(())
    }

    /// Reset a validated selection under one command lock and whole-batch preflight.
    pub fn reset_submodules(
        &mut self,
        all: bool,
        names: Vec<String>,
    ) -> Result<(), SubmoduleError> {
        let _locks = self.acquire_mutation_locks()?;
        self.reload_locked_config()?;
        self.require_config()?;
        let plans = self.prepare_reset(all, names)?;

        let mut completed = Vec::new();
        for (index, (name, path, target)) in plans.iter().enumerate() {
            eprintln!(
                "Resetting {} to parent pin {}...",
                crate::utilities::safe_human_text(name),
                crate::utilities::safe_human_text(target)
            );
            let stash = match self.git_ops.stash_submodule(path, true) {
                Ok(stash) => stash,
                Err(error) => {
                    let pending: Vec<&str> = plans[index + 1..]
                        .iter()
                        .map(|(name, _, _)| name.as_str())
                        .collect();
                    eprintln!("Reset stopped after preservation failed.");
                    eprintln!(
                        "  completed: {}",
                        crate::utilities::safe_human_text(&completed.join(", "))
                    );
                    eprintln!(
                        "  failed: {}: {}",
                        crate::utilities::safe_human_text(name),
                        crate::utilities::safe_human_text(&error.to_string())
                    );
                    eprintln!(
                        "  pending: {}",
                        crate::utilities::safe_human_text(&pending.join(", "))
                    );
                    return Err(SubmoduleError::CliError(format!(
                        "Could not preserve work for {name}; reset was not attempted: {error}"
                    )));
                }
            };
            let recovery = if let Some(oid) = &stash {
                println!(
                    "  Preserved local work in stash {}",
                    crate::utilities::safe_human_text(oid)
                );
                let command = self
                    .git_ops
                    .stash_recovery_command(path, oid)
                    .map_err(Self::map_git_ops_error)?;
                println!(
                    "  Recovery (run inside {}): {command}",
                    crate::utilities::safe_human_text(
                        &self.context.worktree_root.join(path).to_string_lossy()
                    ),
                    command = crate::utilities::safe_human_text(&command),
                );
                Some(command)
            } else {
                println!("  No local work needed a stash");
                None
            };
            if let Err(error) = self.git_ops.reset_submodule(path, true) {
                let pending: Vec<&str> = plans[index + 1..]
                    .iter()
                    .map(|(name, _, _)| name.as_str())
                    .collect();
                eprintln!("Reset stopped after a runtime failure.");
                eprintln!(
                    "  completed: {}",
                    crate::utilities::safe_human_text(&completed.join(", "))
                );
                eprintln!(
                    "  failed: {}: {}",
                    crate::utilities::safe_human_text(name),
                    crate::utilities::safe_human_text(&error.to_string())
                );
                eprintln!(
                    "  pending: {}",
                    crate::utilities::safe_human_text(&pending.join(", "))
                );
                if let Some(command) = recovery {
                    eprintln!(
                        "  preserved work (run inside {}): {command}",
                        crate::utilities::safe_human_text(
                            &self.context.worktree_root.join(path).to_string_lossy()
                        ),
                        command = crate::utilities::safe_human_text(&command),
                    );
                }
                return Err(SubmoduleError::CliError(format!(
                    "Reset failed for {name}: {error}"
                )));
            }
            completed.push(name.clone());
            println!(
                "{} reset to {}",
                crate::utilities::safe_human_text(name),
                crate::utilities::safe_human_text(target)
            );
        }
        println!(
            "Reset summary: {} changed, 0 unchanged, 0 skipped, 0 failed.",
            plans.len()
        );
        Ok(())
    }

    /// Initialize submodule - add it first if not registered, then initialize
    pub fn init_submodule(&mut self, name: &str) -> Result<(), SubmoduleError> {
        let _locks = self.acquire_mutation_locks()?;
        self.reload_locked_config()?;
        self.init_submodule_locked(name, false)
    }

    fn init_submodule_locked(&mut self, name: &str, recursive: bool) -> Result<(), SubmoduleError> {
        let (path_str, url_str, branch, ignore, update, fetch_recurse, sparse_paths_opt, active) = {
            let config = self.config.effective_entry(name).ok_or_else(|| {
                SubmoduleError::SubmoduleNotFound {
                    name: name.to_string(),
                }
            })?;

            let path_str = config.path.expect("effective path is populated");

            let url_str = config
                .url
                .as_ref()
                .ok_or_else(|| {
                    SubmoduleError::ConfigError("No URL configured for submodule".to_string())
                })?
                .clone();

            (
                path_str,
                url_str,
                config.branch,
                config.ignore,
                config.update,
                config.fetch_recurse,
                config.sparse_paths,
                config.active.unwrap_or(true),
            )
        };

        self.validate_requested_path(name, Path::new(&path_str), Some(name))?;
        let submodule_path = self.context.worktree_root.join(&path_str);
        let mut registered = self.registration_for_path(&path_str)?.is_some();
        let mut gitlink = self
            .git_ops
            .index_gitlink_oid(&path_str)
            .map_err(Self::map_git_ops_error)?;
        if !registered && gitlink.is_some() {
            let managed = Self::managed_settings(&self.config, name)?;
            self.git_ops
                .restore_registration(name, &path_str, &managed)
                .map_err(Self::map_git_ops_error)?;
            registered = true;
        }

        if !active || update == Some(SerializableUpdate::None) {
            if registered {
                self.sync_effective_settings(name, &path_str)?;
            }
            return Ok(());
        }

        if submodule_path.exists() && submodule_path.join(".git").exists() {
            self.git_ops
                .verify_submodule_checkout(&path_str)
                .map_err(Self::map_git_ops_error)?;
            self.sync_effective_settings(name, &path_str)?;
            let mut update_opts = crate::config::SubmoduleUpdateOptions::from_options(
                crate::config::SubmoduleGitOptions::new(ignore, fetch_recurse, branch, update),
            );
            update_opts.recursive = recursive;
            self.git_ops
                .update_submodule(&path_str, &update_opts)
                .map_err(Self::map_git_ops_error)?;
            let use_git_default = self.effective_use_git_default_sparse_checkout(name);
            self.configure_sparse_checkout(
                &path_str,
                sparse_paths_opt.as_deref().unwrap_or(&[]),
                use_git_default,
            )?;
            return Ok(());
        }

        if self.verbose {
            eprintln!(
                "Initializing {}...",
                crate::utilities::safe_human_text(name)
            );
        }

        if registered && gitlink.is_none() {
            let managed = Self::managed_settings(&self.config, name)?;
            self.git_ops
                .complete_registration(&path_str, &managed)
                .map_err(Self::map_git_ops_error)?;
            gitlink = self
                .git_ops
                .index_gitlink_oid(&path_str)
                .map_err(Self::map_git_ops_error)?;
            debug_assert!(gitlink.is_some());
        }

        let needs_add = !registered;

        if needs_add {
            let managed = Self::managed_settings(&self.config, name)?;
            // Submodule not registered yet, add it first via GitOpsManager
            let opts = crate::config::SubmoduleAddOptions {
                name: name.to_string(),
                path: std::path::PathBuf::from(&path_str),
                url: url_str,
                branch: managed.branch,
                ignore: managed.ignore,
                update: managed.update,
                fetch_recurse: managed.fetch_recurse,
                shallow: managed.shallow.unwrap_or(false),
                no_init: false,
            };
            self.git_ops
                .add_submodule(&opts)
                .map_err(Self::map_git_ops_error)?;
            if recursive {
                let mut update_opts = crate::config::SubmoduleUpdateOptions::from_options(
                    crate::config::SubmoduleGitOptions::new(ignore, fetch_recurse, branch, update),
                );
                update_opts.recursive = true;
                self.git_ops
                    .update_submodule(&path_str, &update_opts)
                    .map_err(Self::map_git_ops_error)?;
            }
        } else {
            self.sync_effective_settings(name, &path_str)?;
            // Submodule is registered, just initialize and update using GitOperations
            self.git_ops
                .init_submodule(&path_str)
                .map_err(Self::map_git_ops_error)?;

            let mut update_opts = crate::config::SubmoduleUpdateOptions::from_options(
                crate::config::SubmoduleGitOptions::new(ignore, fetch_recurse, branch, update),
            );
            update_opts.recursive = recursive;
            self.git_ops
                .update_submodule(&path_str, &update_opts)
                .map_err(Self::map_git_ops_error)?;
        }

        self.sync_effective_settings(name, &path_str)?;

        if self.verbose {
            println!(
                "  Initialized using Git submodule commands: {}",
                crate::utilities::safe_human_text(&path_str)
            );
        }

        let use_git_default = self.effective_use_git_default_sparse_checkout(name);
        self.configure_sparse_checkout(
            &path_str,
            sparse_paths_opt.as_deref().unwrap_or(&[]),
            use_git_default,
        )?;

        if self.verbose {
            println!("{} initialized", crate::utilities::safe_human_text(name));
        }
        Ok(())
    }

    fn add_options_for(
        &self,
        name: &str,
    ) -> Result<crate::config::SubmoduleAddOptions, SubmoduleError> {
        let effective =
            self.config
                .effective_entry(name)
                .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                    name: name.to_string(),
                })?;
        let managed = Self::managed_settings(&self.config, name)?;
        Ok(crate::config::SubmoduleAddOptions {
            name: name.to_string(),
            path: PathBuf::from(
                effective
                    .path
                    .expect("effective submodule path is populated"),
            ),
            url: effective
                .url
                .expect("validated effective submodule URL is populated"),
            branch: managed.branch,
            ignore: managed.ignore,
            update: managed.update,
            fetch_recurse: managed.fetch_recurse,
            shallow: managed.shallow.unwrap_or(false),
            no_init: false,
        })
    }

    fn preflight_reconcile(
        &self,
        names: &[String],
        scope: ReconcileScope,
        remote: bool,
    ) -> Result<(), SubmoduleError> {
        self.validate_all_paths()?;
        for name in names {
            let effective = self
                .config
                .effective_entry(name)
                .ok_or_else(|| SubmoduleError::SubmoduleNotFound { name: name.clone() })?;
            let path = effective
                .path
                .as_deref()
                .expect("effective submodule path is populated");
            let registered = self.registration_for_path(path)?.is_some();
            if !registered
                && let Some(existing) = self
                    .git_ops
                    .registration_path(name)
                    .map_err(Self::map_git_ops_error)?
            {
                return Err(SubmoduleError::ConfigError(format!(
                    "managed entry {name:?} changed path from {existing:?} to {path:?} without an explicit Git-aware move; use `submod change {name} --path {path}`"
                )));
            }
            let gitlink = self
                .git_ops
                .index_gitlink_oid(path)
                .map_err(Self::map_git_ops_error)?;
            let skipped = !effective.active.unwrap_or(true)
                || effective.update == Some(SerializableUpdate::None);
            let managed = Self::managed_settings(&self.config, name)?;
            match (registered, gitlink.is_some(), skipped) {
                (true, true, _) | (true, false, true) => self
                    .git_ops
                    .preflight_submodule_settings(path, &managed)
                    .map_err(Self::map_git_ops_error)?,
                (true, false, false) => self
                    .git_ops
                    .preflight_complete_registration(path, &managed)
                    .map_err(Self::map_git_ops_error)?,
                (false, true, _) => self
                    .git_ops
                    .preflight_restore_registration(name, path, &managed)
                    .map_err(Self::map_git_ops_error)?,
                (false, false, true) => {}
                (false, false, false) => {
                    if scope == ReconcileScope::Update {
                        return Err(SubmoduleError::ConfigError(format!(
                            "submodule {name:?} is not registered; run init or sync"
                        )));
                    }
                    let options = self.add_options_for(name)?;
                    self.git_ops
                        .preflight_add_submodule(&options)
                        .map_err(Self::map_git_ops_error)?;
                }
            }
            if registered && !skipped && self.context.worktree_root.join(path).join(".git").exists()
            {
                let mut update_opts =
                    crate::config::SubmoduleUpdateOptions::from_options(effective.git_options());
                update_opts.remote = scope == ReconcileScope::Update && remote;
                self.git_ops
                    .preflight_update_submodule(path, &update_opts)
                    .map_err(Self::map_git_ops_error)?;
                let patterns = if effective.use_git_default_sparse_checkout.unwrap_or(false) {
                    effective.sparse_paths.clone().unwrap_or_default()
                } else {
                    Self::build_deny_all_sparse_patterns(
                        effective.sparse_paths.as_deref().unwrap_or(&[]),
                    )
                };
                self.git_ops
                    .preflight_sparse_checkout(path, &patterns)
                    .map_err(Self::map_git_ops_error)?;
            }
        }
        Ok(())
    }

    fn report_unmanaged_submodules(&self) -> Result<(), SubmoduleError> {
        let managed: std::collections::HashSet<String> = self
            .config
            .get_submodules()
            .map(|(name, entry)| entry.path.clone().unwrap_or_else(|| name.clone()))
            .collect();
        let mut unmanaged: Vec<String> = self
            .git_ops
            .list_submodules()
            .map_err(Self::map_git_ops_error)?
            .into_iter()
            .filter(|path| !managed.contains(path))
            .collect();
        unmanaged.sort();
        for path in unmanaged {
            println!(
                "Unmanaged Git submodule preserved: {}",
                crate::utilities::safe_human_text(&path)
            );
        }
        Ok(())
    }

    /// Put registration-changing actions before metadata-only reconciliation.
    ///
    /// Native registration commands stage `.gitmodules`. Existing managed
    /// metadata changes are deliberately left unstaged, so applying those
    /// first would make a later add reject the command's own edit. The full
    /// batch has already been preflighted before this ordering is computed.
    fn registration_first_order(&self, names: &[String]) -> Result<Vec<String>, SubmoduleError> {
        let mut registration = Vec::new();
        let mut existing = Vec::new();
        for name in names {
            let effective = self
                .config
                .effective_entry(name)
                .ok_or_else(|| SubmoduleError::SubmoduleNotFound { name: name.clone() })?;
            let path = effective
                .path
                .as_deref()
                .expect("effective submodule path is populated");
            let registered = self.registration_for_path(path)?.is_some();
            let gitlink = self
                .git_ops
                .index_gitlink_oid(path)
                .map_err(Self::map_git_ops_error)?
                .is_some();
            if !registered || !gitlink {
                registration.push(name.clone());
            } else {
                existing.push(name.clone());
            }
        }
        registration.extend(existing);
        Ok(registration)
    }

    fn prepare_reconcile(
        &self,
        scope: ReconcileScope,
        remote: bool,
        recursive: bool,
    ) -> Result<Vec<ReconcilePlan>, SubmoduleError> {
        let mut names: Vec<String> = self
            .config
            .get_submodules()
            .map(|(name, _)| name.clone())
            .collect();
        names.sort();
        if !names.is_empty() {
            self.preflight_reconcile(&names, scope, remote)?;
            names = self.registration_first_order(&names)?;
        }
        names
            .into_iter()
            .map(|name| self.reconcile_plan_for(name, scope, remote, recursive))
            .collect()
    }

    fn reconcile_plan_for(
        &self,
        name: String,
        scope: ReconcileScope,
        remote: bool,
        recursive: bool,
    ) -> Result<ReconcilePlan, SubmoduleError> {
        let effective = self
            .config
            .effective_entry(&name)
            .ok_or_else(|| SubmoduleError::SubmoduleNotFound { name: name.clone() })?;
        let path = effective
            .path
            .clone()
            .expect("effective submodule path is populated");
        let registered = self.registration_for_path(&path)?.is_some();
        let parent_pin = self
            .git_ops
            .index_gitlink_oid(&path)
            .map_err(Self::map_git_ops_error)?;
        let initialized = self.context.worktree_root.join(&path).join(".git").exists();
        let managed = Self::managed_settings(&self.config, &name)?;
        let verbose_detail = self.verbose.then(|| {
            format!(
                "effective: branch={}, update={:?}, ignore={:?}, active={}, shallow={}, sparse_paths={:?}",
                effective
                    .branch
                    .as_ref()
                    .map_or_else(|| "<remote-default>".to_string(), super::options::SerializableBranch::as_config_value),
                effective.update,
                effective.ignore,
                effective.active.unwrap_or(true),
                effective.shallow.unwrap_or(false),
                effective.sparse_paths.as_deref().unwrap_or(&[]),
            )
        });
        let metadata_changed = registered
            && !self
                .git_ops
                .submodule_settings_match(&path, &managed)
                .map_err(Self::map_git_ops_error)?;
        let initial_head = initialized
            .then(|| {
                self.git_ops
                    .submodule_head(&path)
                    .map_err(Self::map_git_ops_error)
            })
            .transpose()?;
        let eligible =
            effective.active.unwrap_or(true) && effective.update != Some(SerializableUpdate::None);
        let initial_recursive_state = (recursive && initialized && eligible)
            .then(|| {
                self.git_ops
                    .recursive_submodule_state(&path)
                    .map_err(Self::map_git_ops_error)
            })
            .transpose()?;
        let remote_requested = scope == ReconcileScope::Update && remote;

        if !effective.active.unwrap_or(true) {
            let detail = if metadata_changed {
                "checkout disabled; reconcile managed metadata without materializing or moving HEAD"
            } else {
                "checkout disabled; managed metadata already matches"
            };
            return Ok(ReconcilePlan {
                name,
                path,
                kind: if metadata_changed {
                    ModuleOutcomeKind::ChangedSkippedDisabled
                } else {
                    ModuleOutcomeKind::SkippedDisabled
                },
                detail: detail.to_string(),
                target: parent_pin,
                verbose_detail,
                registration_changed: false,
                metadata_changed,
                sparse_changed: false,
                remote_requested: false,
                recursive_requested: false,
                initial_head,
                initial_recursive_state,
            });
        }
        if effective.update == Some(SerializableUpdate::None) {
            let detail = if metadata_changed {
                "update=none; reconcile managed metadata without fetching, materializing, or moving HEAD"
            } else {
                "update=none; managed metadata already matches"
            };
            return Ok(ReconcilePlan {
                name,
                path,
                kind: if metadata_changed {
                    ModuleOutcomeKind::ChangedSkippedPolicy
                } else {
                    ModuleOutcomeKind::SkippedPolicy
                },
                detail: detail.to_string(),
                target: parent_pin,
                verbose_detail,
                registration_changed: false,
                metadata_changed,
                sparse_changed: false,
                remote_requested: false,
                recursive_requested: false,
                initial_head,
                initial_recursive_state,
            });
        }

        if !registered || parent_pin.is_none() {
            let branch = effective.branch.as_ref().map_or_else(
                || "the remote default branch".to_string(),
                |value| format!("branch {}", value.as_config_value()),
            );
            let initial_action = if registered {
                format!(
                    "complete registration, stage .gitmodules and its gitlink, then initialize from {branch}"
                )
            } else if parent_pin.is_some() {
                "restore the missing registration, stage .gitmodules, and materialize the recorded parent pin"
                    .to_string()
            } else {
                format!(
                    "register and initialize from {branch}; stage .gitmodules and the resulting gitlink"
                )
            };
            let (detail, target) = if remote_requested {
                (
                    format!(
                        "{initial_action}; then fetch and update from the selected {branch}; final remote target unresolved until execution"
                    ),
                    None,
                )
            } else {
                (initial_action, parent_pin)
            };
            return Ok(ReconcilePlan {
                name,
                path,
                kind: ModuleOutcomeKind::Changed,
                detail,
                target,
                verbose_detail,
                registration_changed: true,
                metadata_changed,
                sparse_changed: false,
                remote_requested,
                recursive_requested: recursive,
                initial_head,
                initial_recursive_state,
            });
        }

        if !initialized {
            let branch = effective.branch.as_ref().map_or_else(
                || "remote default branch".to_string(),
                |value| format!("remote branch {}", value.as_config_value()),
            );
            let (detail, target) = if remote_requested {
                (
                    format!(
                        "materialize the recorded parent pin, then fetch and update from the selected {branch}; final remote target unresolved until execution"
                    ),
                    None,
                )
            } else {
                (
                    "materialize the existing registration at its recorded parent pin".to_string(),
                    parent_pin,
                )
            };
            return Ok(ReconcilePlan {
                name,
                path,
                kind: ModuleOutcomeKind::Changed,
                detail,
                target,
                verbose_detail,
                registration_changed: true,
                metadata_changed,
                sparse_changed: false,
                remote_requested,
                recursive_requested: recursive,
                initial_head,
                initial_recursive_state,
            });
        }

        let mut work = Vec::new();
        if metadata_changed {
            let url = effective.url.as_deref().unwrap_or("<unset>");
            work.push(format!(
                "reconcile managed metadata and native-resolved local URL from {url} (portable edits remain unstaged)"
            ));
        }
        let mut update =
            crate::config::SubmoduleUpdateOptions::from_options(effective.git_options());
        update.remote = remote_requested;
        update.recursive = recursive;
        if update.remote {
            let branch = effective.branch.as_ref().map_or_else(
                || "remote default branch".to_string(),
                |value| format!("remote branch {}", value.as_config_value()),
            );
            work.push(format!(
                "fetch and update from the selected {branch}; resolve its target during execution"
            ));
        } else if !self
            .git_ops
            .update_postcondition_matches(&path, &update)
            .map_err(Self::map_git_ops_error)?
        {
            work.push("apply the configured strategy to the recorded parent pin".to_string());
        }

        let expected_sparse = if effective.use_git_default_sparse_checkout.unwrap_or(false) {
            effective.sparse_paths.unwrap_or_default()
        } else {
            Self::build_deny_all_sparse_patterns(effective.sparse_paths.as_deref().unwrap_or(&[]))
        };
        let sparse_changed = !matches!(
            self.check_sparse_checkout_status(&path, &expected_sparse)?,
            SparseStatus::NotEnabled | SparseStatus::Correct
        );
        if sparse_changed {
            work.push("reconcile the ordered sparse-checkout policy".to_string());
        }
        if recursive {
            work.push("reconcile selected nested submodules recursively".to_string());
        }

        let (kind, detail) = if work.is_empty() {
            (
                ModuleOutcomeKind::Unchanged,
                "registration, metadata, checkout target, and sparse policy already match"
                    .to_string(),
            )
        } else {
            (ModuleOutcomeKind::Changed, work.join("; "))
        };
        Ok(ReconcilePlan {
            name,
            path,
            kind,
            detail,
            target: if remote_requested { None } else { parent_pin },
            verbose_detail,
            registration_changed: false,
            metadata_changed,
            sparse_changed,
            remote_requested,
            recursive_requested: recursive,
            initial_head,
            initial_recursive_state,
        })
    }

    fn finalize_reconcile_plan(
        &self,
        mut plan: ReconcilePlan,
    ) -> Result<ReconcilePlan, SubmoduleError> {
        if matches!(
            plan.kind,
            ModuleOutcomeKind::SkippedDisabled
                | ModuleOutcomeKind::SkippedPolicy
                | ModuleOutcomeKind::ChangedSkippedDisabled
                | ModuleOutcomeKind::ChangedSkippedPolicy
        ) {
            if plan.metadata_changed {
                let managed = Self::managed_settings(&self.config, &plan.name)?;
                if !self
                    .git_ops
                    .submodule_settings_match(&plan.path, &managed)
                    .map_err(Self::map_git_ops_error)?
                {
                    return Err(SubmoduleError::CliError(format!(
                        "managed metadata postcondition failed for {:?} at {:?}",
                        plan.name, plan.path
                    )));
                }
                plan.detail = match plan.kind {
                    ModuleOutcomeKind::ChangedSkippedDisabled => {
                        "managed metadata reconciled; checkout remained skipped-disabled without materializing or moving HEAD"
                    }
                    ModuleOutcomeKind::ChangedSkippedPolicy => {
                        "managed metadata reconciled; checkout remained skipped-policy (update=none) without fetching, materializing, or moving HEAD"
                    }
                    _ => unreachable!("metadata change uses a combined skipped outcome"),
                }
                .to_string();
            } else {
                plan.detail = match plan.kind {
                    ModuleOutcomeKind::SkippedDisabled => {
                        "checkout skipped-disabled; managed metadata already matched"
                    }
                    ModuleOutcomeKind::SkippedPolicy => {
                        "checkout skipped-policy (update=none); managed metadata already matched"
                    }
                    _ => unreachable!("unchanged skip uses a plain skipped outcome"),
                }
                .to_string();
            }
            return Ok(plan);
        }

        let effective = self.config.effective_entry(&plan.name).ok_or_else(|| {
            SubmoduleError::SubmoduleNotFound {
                name: plan.name.clone(),
            }
        })?;
        let mut update =
            crate::config::SubmoduleUpdateOptions::from_options(effective.git_options());
        update.remote = plan.remote_requested;
        update.recursive = plan.recursive_requested;
        let final_head = self
            .git_ops
            .submodule_head(&plan.path)
            .map_err(Self::map_git_ops_error)?;
        let target = self
            .git_ops
            .submodule_update_target(&plan.path, &update)
            .map_err(Self::map_git_ops_error)?;
        if !self
            .git_ops
            .update_postcondition_matches(&plan.path, &update)
            .map_err(Self::map_git_ops_error)?
        {
            return Err(SubmoduleError::CliError(format!(
                "checkout postcondition failed for {:?} at {:?}: expected target {}",
                plan.name, plan.path, target
            )));
        }
        if plan.metadata_changed {
            let managed = Self::managed_settings(&self.config, &plan.name)?;
            if !self
                .git_ops
                .submodule_settings_match(&plan.path, &managed)
                .map_err(Self::map_git_ops_error)?
            {
                return Err(SubmoduleError::CliError(format!(
                    "managed metadata postcondition failed for {:?} at {:?}",
                    plan.name, plan.path
                )));
            }
        }
        let final_recursive_state = plan
            .recursive_requested
            .then(|| {
                self.git_ops
                    .recursive_submodule_state(&plan.path)
                    .map_err(Self::map_git_ops_error)
            })
            .transpose()?;
        let head_changed = plan.initial_head.as_deref() != Some(final_head.as_str());
        let recursive_changed = plan.recursive_requested
            && plan.initial_recursive_state.as_deref() != final_recursive_state.as_deref();
        let changed = plan.registration_changed
            || plan.metadata_changed
            || plan.sparse_changed
            || head_changed
            || recursive_changed;

        let mut details = Vec::new();
        if plan.registration_changed {
            details.push("registration/materialization reconciled".to_string());
        }
        if plan.metadata_changed {
            details.push("managed metadata reconciled".to_string());
        }
        if head_changed {
            details.push(plan.initial_head.as_deref().map_or_else(
                || format!("checkout materialized at {final_head}"),
                |initial| format!("checkout moved from {initial} to {final_head}"),
            ));
        }
        if plan.sparse_changed {
            details.push("ordered sparse-checkout policy reconciled".to_string());
        }
        if recursive_changed {
            details.push("nested submodule state reconciled".to_string());
        } else if plan.recursive_requested {
            details.push("nested submodules already matched".to_string());
        }
        if plan.remote_requested && !head_changed {
            details.push("selected remote target already satisfied".to_string());
        }
        if details.is_empty() {
            details.push(
                "registration, managed metadata, checkout target, and sparse policy already matched"
                    .to_string(),
            );
        }
        plan.kind = if changed {
            ModuleOutcomeKind::Changed
        } else {
            ModuleOutcomeKind::Unchanged
        };
        plan.detail = details.join("; ");
        plan.target = Some(target.clone());
        if let Some(verbose) = &mut plan.verbose_detail {
            use std::fmt::Write as _;
            let _ = write!(
                verbose,
                ", result_head={final_head}, selected_target={target}"
            );
        }
        Ok(plan)
    }

    fn preview_reconcile(
        &self,
        scope: ReconcileScope,
        remote: bool,
        recursive: bool,
    ) -> Result<OperationSummary, SubmoduleError> {
        self.require_config()?;
        Ok(OperationSummary {
            operation: match scope {
                ReconcileScope::Init => "Initialization preview",
                ReconcileScope::Update => "Update preview",
                ReconcileScope::Sync => "Sync preview",
            },
            preview: true,
            modules: self.prepare_reconcile(scope, remote, recursive)?,
        })
    }

    /// Preview initialization using the same batch selection and preflights as execution.
    pub fn preview_init_all_submodules(
        &self,
        recursive: bool,
    ) -> Result<OperationSummary, SubmoduleError> {
        self.preview_reconcile(ReconcileScope::Init, false, recursive)
    }

    /// Preview an update using the same batch selection and preflights as execution.
    pub fn preview_update_all_submodules(
        &self,
        remote: bool,
        recursive: bool,
    ) -> Result<OperationSummary, SubmoduleError> {
        self.preview_reconcile(ReconcileScope::Update, remote, recursive)
    }

    /// Preview synchronization using the same batch selection and preflights as execution.
    pub fn preview_sync_all_submodules(
        &self,
        recursive: bool,
    ) -> Result<OperationSummary, SubmoduleError> {
        self.preview_reconcile(ReconcileScope::Sync, false, recursive)
    }

    fn reconcile_all(
        &mut self,
        scope: ReconcileScope,
        remote: bool,
        recursive: bool,
    ) -> Result<OperationSummary, SubmoduleError> {
        let _locks = self.acquire_mutation_locks()?;
        self.reload_locked_config()?;
        self.require_config()?;
        let plans = self.prepare_reconcile(scope, remote, recursive)?;
        if plans.is_empty() {
            self.report_unmanaged_submodules()?;
            return Ok(OperationSummary {
                operation: match scope {
                    ReconcileScope::Init => "Initialization",
                    ReconcileScope::Update => "Update",
                    ReconcileScope::Sync => "Sync",
                },
                preview: false,
                modules: plans,
            });
        }

        let mut outcomes = plans.clone();
        let mut unresolved = Vec::new();
        for (index, plan) in plans.iter().enumerate() {
            let name = &plan.name;
            let result = if plan.kind == ModuleOutcomeKind::Unchanged
                && !plan.remote_requested
                && !plan.recursive_requested
            {
                // The shared plan has already inspected registration, managed
                // metadata, parent-pin strategy, and sparse state. Avoid a
                // mutation-shaped native update when no action is required;
                // finalization below still rechecks the checkout postcondition.
                Ok(())
            } else {
                match scope {
                    ReconcileScope::Init => self.init_submodule_locked(name, recursive),
                    ReconcileScope::Update if plan.registration_changed => {
                        self.init_submodule_locked(name, recursive).and_then(|()| {
                            if remote {
                                self.update_submodule_locked(name, true, recursive)
                            } else {
                                Ok(())
                            }
                        })
                    }
                    ReconcileScope::Update => self.update_submodule_locked(name, remote, recursive),
                    ReconcileScope::Sync => self
                        .init_submodule_locked(name, recursive)
                        .and_then(|()| self.update_submodule_locked(name, false, recursive)),
                }
            };
            outcomes[index] = match result.and_then(|()| self.finalize_reconcile_plan(plan.clone()))
            {
                Ok(outcome) => outcome,
                Err(error) => {
                    let mut completed = outcomes[..index].to_vec();
                    let mut failed = plan.clone();
                    failed.kind = ModuleOutcomeKind::Failed;
                    failed.detail = format!(
                        "{scope:?} failed while attempting {}: {error}; repair the reported cause and retry",
                        plan.detail
                    );
                    completed.push(failed);
                    completed.extend(plans[index + 1..].iter().cloned().map(|mut pending| {
                        pending.kind = ModuleOutcomeKind::Pending;
                        pending.detail =
                            format!("not attempted; planned action: {}", pending.detail);
                        pending
                    }));
                    return Err(SubmoduleError::IncompleteBatch {
                        summary: OperationSummary {
                            operation: match scope {
                                ReconcileScope::Init => "Initialization incomplete",
                                ReconcileScope::Update => "Update incomplete",
                                ReconcileScope::Sync => "Sync incomplete",
                            },
                            preview: false,
                            modules: completed,
                        },
                        cause: format!(
                            "module {name:?} failed during {scope:?} at {:?}: {error}",
                            plan.path
                        ),
                    });
                }
            };
            if matches!(
                outcomes[index].kind,
                ModuleOutcomeKind::SkippedDisabled
                    | ModuleOutcomeKind::SkippedPolicy
                    | ModuleOutcomeKind::ChangedSkippedDisabled
                    | ModuleOutcomeKind::ChangedSkippedPolicy
            ) && self
                .context
                .worktree_root
                .join(&plan.path)
                .join(".git")
                .exists()
                && !self
                    .git_ops
                    .submodule_worktree_is_clean(&plan.path)
                    .map_err(Self::map_git_ops_error)?
            {
                outcomes[index].detail.push_str(
                    "; checkout changes were preserved and remain unresolved by this policy",
                );
                unresolved.push(plan.name.clone());
            }
        }
        self.report_unmanaged_submodules()?;
        if !unresolved.is_empty() {
            return Err(SubmoduleError::IncompleteBatch {
                summary: OperationSummary {
                    operation: match scope {
                        ReconcileScope::Init => "Initialization incomplete",
                        ReconcileScope::Update => "Update incomplete",
                        ReconcileScope::Sync => "Sync incomplete",
                    },
                    preview: false,
                    modules: outcomes,
                },
                cause: format!(
                    "local changes remain in policy-skipped module(s) {}; preserve or resolve them explicitly",
                    unresolved.join(", ")
                ),
            });
        }
        Ok(OperationSummary {
            operation: match scope {
                ReconcileScope::Init => "Initialization",
                ReconcileScope::Update => "Update",
                ReconcileScope::Sync => "Sync",
            },
            preview: false,
            modules: outcomes,
        })
    }

    /// Initialize all eligible declarations under one preflighted command lock.
    pub fn init_all_submodules(
        &mut self,
        recursive: bool,
    ) -> Result<OperationSummary, SubmoduleError> {
        self.reconcile_all(ReconcileScope::Init, false, recursive)
    }

    /// Update all eligible registrations under one preflighted command lock.
    pub fn update_all_submodules(
        &mut self,
        remote: bool,
        recursive: bool,
    ) -> Result<OperationSummary, SubmoduleError> {
        self.reconcile_all(ReconcileScope::Update, remote, recursive)
    }

    /// Converge every eligible declaration and report unmanaged registrations.
    pub fn sync_all_submodules(
        &mut self,
        recursive: bool,
    ) -> Result<OperationSummary, SubmoduleError> {
        self.reconcile_all(ReconcileScope::Sync, false, recursive)
    }

    /// Inspect declarations, registrations, managed metadata, checkout targets, and sparse state.
    pub fn check_all_submodules(&self) -> Result<(), SubmoduleError> {
        self.require_config()?;
        let mut drifted = Vec::new();
        for (submodule_name, _) in self.config.get_submodules() {
            let submodule = self
                .config
                .effective_entry(submodule_name)
                .expect("name came from config");
            let path_str = submodule
                .path
                .as_deref()
                .expect("effective path is populated");
            let submodule_path = self.context.worktree_root.join(path_str);
            let git_path = submodule_path.join(".git");
            let safe_name = crate::utilities::safe_human_text(submodule_name);
            let safe_path = crate::utilities::safe_human_text(path_str);

            let skip_reason = if !submodule.active.unwrap_or(true) {
                Some("disabled")
            } else if submodule.update == Some(SerializableUpdate::None) {
                Some("update-none")
            } else {
                None
            };
            let mut issues = Vec::new();
            if !submodule_path.exists() {
                if let Some(reason) = skip_reason {
                    println!("{safe_name}: skipped-{reason} at {safe_path} (not materialized)");
                    continue;
                }
                issues.push("checkout is missing".to_string());
            } else if !git_path.exists() {
                let empty_checkout = submodule_path.read_dir()?.next().is_none();
                if let Some(reason) = skip_reason.filter(|_| empty_checkout) {
                    println!("{safe_name}: skipped-{reason} at {safe_path} (not materialized)");
                    continue;
                }
                issues.push("checkout is not a Git repository".to_string());
            } else {
                self.git_ops
                    .verify_submodule_checkout(path_str)
                    .map_err(|error| {
                        SubmoduleError::CliError(format!(
                            "Could not inspect module {submodule_name:?} at {path_str:?}: {error}"
                        ))
                    })?;
                if self.registration_for_path(path_str)?.is_none() {
                    issues.push("portable Git registration is missing".to_string());
                }
                let parent_pin = self
                    .git_ops
                    .index_gitlink_oid(path_str)
                    .map_err(Self::map_git_ops_error)?;
                if parent_pin.is_none() {
                    issues.push("parent index gitlink is missing".to_string());
                }
                let managed = Self::managed_settings(&self.config, submodule_name)?;
                if !self
                    .git_ops
                    .submodule_settings_match(path_str, &managed)
                    .map_err(Self::map_git_ops_error)?
                {
                    issues.push("managed Git metadata differs".to_string());
                }
                if parent_pin.is_some() && skip_reason.is_none() {
                    let update = crate::config::SubmoduleUpdateOptions::from_options(
                        submodule.git_options(),
                    );
                    if !self
                        .git_ops
                        .update_postcondition_matches(path_str, &update)
                        .map_err(Self::map_git_ops_error)?
                    {
                        issues
                            .push("checkout does not satisfy its parent-pin strategy".to_string());
                    }
                }
                if !self
                    .git_ops
                    .submodule_worktree_is_clean(path_str)
                    .map_err(Self::map_git_ops_error)?
                {
                    issues.push("working tree has changes".to_string());
                }
                let expected_sparse = if submodule.use_git_default_sparse_checkout.unwrap_or(false)
                {
                    submodule.sparse_paths.clone().unwrap_or_default()
                } else {
                    Self::build_deny_all_sparse_patterns(
                        submodule.sparse_paths.as_deref().unwrap_or(&[]),
                    )
                };
                match self.check_sparse_checkout_status(path_str, &expected_sparse)? {
                    SparseStatus::NotEnabled | SparseStatus::Correct => {}
                    SparseStatus::NotConfigured => {
                        issues.push("sparse checkout is not configured".to_string());
                    }
                    SparseStatus::Mismatch { expected, actual } => {
                        issues.push(format!(
                            "sparse patterns differ (expected {expected:?}, current {actual:?})"
                        ));
                    }
                }
                if self.verbose {
                    let head = self
                        .git_ops
                        .submodule_head(path_str)
                        .map_err(Self::map_git_ops_error)?;
                    println!("  context: path={safe_path}, head={head}");
                    self.show_effective_settings(submodule_name, &submodule);
                }
            }
            if issues.is_empty() {
                if let Some(reason) = skip_reason {
                    println!("{safe_name}: skipped-{reason} (inspectable metadata matches)");
                } else {
                    println!("{safe_name}: unchanged (matches configured state)");
                }
            } else {
                println!(
                    "{safe_name}: drift: {}",
                    crate::utilities::safe_human_text(&issues.join("; "))
                );
                drifted.push(submodule_name.clone());
            }
        }
        self.report_unmanaged_submodules()?;
        if drifted.is_empty() {
            println!("Check complete: all configured submodules match.");
            Ok(())
        } else {
            Err(SubmoduleError::Drift(format!(
                "{} managed submodule(s) have drift: {}",
                drifted.len(),
                drifted.join(", ")
            )))
        }
    }

    #[allow(clippy::unused_self)]
    fn show_effective_settings(&self, _name: &str, config: &SubmoduleEntry) {
        println!("  effective settings:");

        if let Some(ignore) = &config.ignore {
            println!("     ignore = {ignore:?}");
        }
        if let Some(update) = &config.update {
            println!("     update = {update:?}");
        }
        if let Some(branch) = &config.branch {
            println!("     branch = {branch:?}");
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

    fn values_match(existing: &toml_edit::Value, desired: &toml_edit::Value) -> bool {
        match (existing, desired) {
            (toml_edit::Value::String(left), toml_edit::Value::String(right)) => {
                left.value() == right.value()
            }
            (toml_edit::Value::Boolean(left), toml_edit::Value::Boolean(right)) => {
                left.value() == right.value()
            }
            (toml_edit::Value::Array(left), toml_edit::Value::Array(right)) => {
                left.len() == right.len()
                    && left
                        .iter()
                        .zip(right.iter())
                        .all(|(left, right)| Self::values_match(left, right))
            }
            _ => false,
        }
    }

    fn set_document_value(
        table: &mut dyn toml_edit::TableLike,
        key: &str,
        desired: Option<toml_edit::Value>,
    ) {
        let Some(mut desired) = desired else {
            table.remove(key);
            return;
        };
        if let Some(existing) = table.get_mut(key) {
            if let Some(value) = existing.as_value() {
                if Self::values_match(value, &desired) {
                    return;
                }
                *desired.decor_mut() = value.decor().clone();
            }
            *existing = toml_edit::Item::Value(desired);
        } else {
            table.insert(key, toml_edit::Item::Value(desired));
        }
    }

    fn string_value(value: Option<String>) -> Option<toml_edit::Value> {
        value.map(toml_edit::Value::from)
    }

    fn default_field_changed(
        &self,
        field: &'static str,
        old: &crate::config::SubmoduleDefaults,
        new: &crate::config::SubmoduleDefaults,
    ) -> bool {
        self.pending_edits.defaults.contains(field)
            || match field {
                "branch" => old.branch != new.branch,
                "ignore" => old.ignore != new.ignore,
                "fetchRecurse" => old.fetch_recurse != new.fetch_recurse,
                "update" => old.update != new.update,
                "use_git_default_sparse_checkout" => {
                    old.use_git_default_sparse_checkout != new.use_git_default_sparse_checkout
                }
                _ => false,
            }
    }

    fn module_field_changed(
        &self,
        name: &str,
        field: &'static str,
        old: &SubmoduleEntry,
        new: &SubmoduleEntry,
    ) -> bool {
        self.pending_edits.module_contains(name, field)
            || match field {
                "path" => old.path != new.path,
                "url" => old.url != new.url,
                "branch" => old.branch != new.branch,
                "ignore" => old.ignore != new.ignore,
                "fetchRecurse" => old.fetch_recurse != new.fetch_recurse,
                "update" => old.update != new.update,
                "active" => old.active != new.active,
                "shallow" => old.shallow != new.shallow,
                "sparse_paths" => old.sparse_paths != new.sparse_paths,
                "use_git_default_sparse_checkout" => {
                    old.use_git_default_sparse_checkout != new.use_git_default_sparse_checkout
                }
                _ => false,
            }
    }

    fn patch_defaults(&self, document: &mut toml_edit::DocumentMut) -> Result<(), SubmoduleError> {
        const FIELDS: &[&str] = &[
            "branch",
            "ignore",
            "fetchRecurse",
            "update",
            "use_git_default_sparse_checkout",
        ];
        let old = &self.loaded_config.defaults;
        let new = &self.config.defaults;
        if !FIELDS
            .iter()
            .any(|field| self.default_field_changed(field, old, new))
        {
            return Ok(());
        }
        if document.get("defaults").is_none() {
            document
                .as_table_mut()
                .insert("defaults", toml_edit::Item::Table(toml_edit::Table::new()));
        }
        let table = document
            .get_mut("defaults")
            .and_then(toml_edit::Item::as_table_like_mut)
            .ok_or_else(|| {
                SubmoduleError::ConfigError("[defaults] must be a TOML table".to_string())
            })?;
        for field in FIELDS {
            if !self.default_field_changed(field, old, new) {
                continue;
            }
            let value = match *field {
                "branch" => {
                    Self::string_value(new.branch.as_ref().map(SerializableBranch::as_config_value))
                }
                "ignore" => Self::string_value(new.ignore.map(|value| value.to_string())),
                "fetchRecurse" => Self::string_value(
                    new.fetch_recurse
                        .map(|value| value.as_config_value().to_string()),
                ),
                "update" => Self::string_value(new.update.as_ref().map(ToString::to_string)),
                "use_git_default_sparse_checkout" => new
                    .use_git_default_sparse_checkout
                    .map(toml_edit::Value::from),
                _ => unreachable!(),
            };
            if *field == "fetchRecurse" {
                table.remove("fetch");
                table.remove("fetch_recurse");
            }
            Self::set_document_value(table, field, value);
        }
        Ok(())
    }

    fn patch_module_fields(
        &self,
        table: &mut dyn toml_edit::TableLike,
        name: &str,
        old: Option<&SubmoduleEntry>,
        new: &SubmoduleEntry,
    ) {
        const FIELDS: &[&str] = &[
            "path",
            "url",
            "branch",
            "ignore",
            "fetchRecurse",
            "update",
            "active",
            "shallow",
            "sparse_paths",
            "use_git_default_sparse_checkout",
        ];
        for field in FIELDS {
            let changed = old.is_none_or(|old| self.module_field_changed(name, field, old, new));
            if !changed {
                continue;
            }
            let value = match *field {
                "path" => Self::string_value(new.path.clone()),
                "url" => Self::string_value(new.url.clone()),
                "branch" => {
                    Self::string_value(new.branch.as_ref().map(SerializableBranch::as_config_value))
                }
                "ignore" => Self::string_value(new.ignore.map(|value| value.to_string())),
                "fetchRecurse" => Self::string_value(
                    new.fetch_recurse
                        .map(|value| value.as_config_value().to_string()),
                ),
                "update" => Self::string_value(new.update.as_ref().map(ToString::to_string)),
                "active" => new.active.map(toml_edit::Value::from),
                "shallow" => new.shallow.map(toml_edit::Value::from),
                "sparse_paths" => new.sparse_paths.as_ref().map(|paths| {
                    let mut array = toml_edit::Array::new();
                    for path in paths {
                        array.push(path.as_str());
                    }
                    toml_edit::Value::Array(array)
                }),
                "use_git_default_sparse_checkout" => new
                    .use_git_default_sparse_checkout
                    .map(toml_edit::Value::from),
                _ => unreachable!(),
            };
            if *field == "fetchRecurse" {
                table.remove("fetch");
                table.remove("fetch_recurse");
            }
            Self::set_document_value(table, field, value);
        }
    }

    fn render_config_document(&self) -> Result<Vec<u8>, SubmoduleError> {
        let source = std::str::from_utf8(self.loaded_config_bytes.as_deref().unwrap_or_default())
            .map_err(|error| {
            SubmoduleError::ConfigError(format!("Loaded configuration is not UTF-8: {error}"))
        })?;
        let parsed = source
            .parse::<toml_edit::Document<String>>()
            .map_err(|error| {
                SubmoduleError::ConfigError(format!("Failed to edit configuration: {error}"))
            })?;
        let mut document = parsed.into_mut();
        self.patch_defaults(&mut document)?;

        let old_names: std::collections::BTreeSet<_> = self
            .loaded_config
            .get_submodules()
            .map(|(name, _)| name.clone())
            .collect();
        let new_names: std::collections::BTreeSet<_> = self
            .config
            .get_submodules()
            .map(|(name, _)| name.clone())
            .collect();
        for removed in old_names.difference(&new_names) {
            document.as_table_mut().remove(removed);
        }
        for name in &new_names {
            let old = self.loaded_config.get_submodule(name);
            let new = self
                .config
                .get_submodule(name)
                .expect("name came from config");
            let changed = old.is_none()
                || [
                    "path",
                    "url",
                    "branch",
                    "ignore",
                    "fetchRecurse",
                    "update",
                    "active",
                    "shallow",
                    "sparse_paths",
                    "use_git_default_sparse_checkout",
                ]
                .iter()
                .any(|field| {
                    old.is_some_and(|old| self.module_field_changed(name, field, old, new))
                });
            if !changed {
                continue;
            }
            if document.get(name).is_none() {
                document
                    .as_table_mut()
                    .insert(name, toml_edit::Item::Table(toml_edit::Table::new()));
            }
            let table = document
                .get_mut(name)
                .and_then(toml_edit::Item::as_table_like_mut)
                .ok_or_else(|| {
                    SubmoduleError::ConfigError(format!("[{name}] must be a TOML table"))
                })?;
            self.patch_module_fields(table, name, old, new);
        }
        let rendered = document.to_string().into_bytes();
        let rendered_source = std::str::from_utf8(&rendered).expect("DocumentMut emits UTF-8");
        Config::parse(rendered_source).map_err(|error| {
            SubmoduleError::ConfigError(format!(
                "Refusing to replace configuration with invalid rendered TOML: {error}"
            ))
        })?;
        Ok(rendered)
    }

    fn atomic_replace_config(
        path: &Path,
        expected: Option<&[u8]>,
        rendered: &[u8],
    ) -> Result<(), SubmoduleError> {
        Self::atomic_replace_config_with(path, expected, rendered, |temporary, path| {
            temporary.persist(path).map_err(|error| {
                SubmoduleError::ConfigError(format!(
                    "Failed to atomically replace configuration {}: {}",
                    path.display(),
                    error.error
                ))
            })?;
            Ok(())
        })
    }

    fn atomic_replace_config_with<F>(
        path: &Path,
        expected: Option<&[u8]>,
        rendered: &[u8],
        persist: F,
    ) -> Result<(), SubmoduleError>
    where
        F: FnOnce(tempfile::NamedTempFile, &Path) -> Result<(), SubmoduleError>,
    {
        if expected.is_some_and(|expected| rendered == expected) {
            return Ok(());
        }
        Self::validate_config_destination(path)?;
        let parent = path.parent().ok_or_else(|| {
            SubmoduleError::InvalidPath("config path has no parent directory".to_string())
        })?;
        let permissions = match fs::metadata(path) {
            Ok(metadata) => Some(metadata.permissions()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|error| {
            SubmoduleError::ConfigError(format!(
                "Failed to create temporary configuration beside {}: {error}",
                path.display()
            ))
        })?;
        if let Some(permissions) = permissions {
            temporary.as_file().set_permissions(permissions)?;
        }
        temporary.write_all(rendered)?;
        temporary.flush()?;
        temporary.as_file().sync_all()?;

        // This is intentionally the last check before replacement. A process that ignores
        // submod's sibling lock can still race after this comparison and before rename.
        Self::validate_config_destination(path)?;
        let current = match fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if current.as_deref() != expected {
            return Err(SubmoduleError::ConfigError(format!(
                "Configuration {} changed after it was loaded; no changes were written",
                path.display()
            )));
        }
        persist(temporary, path)
    }

    /// Patch raw declarations through `toml_edit`, validate, and atomically replace once.
    fn write_full_config(&mut self) -> Result<(), SubmoduleError> {
        let rendered = self.render_config_document()?;
        Self::atomic_replace_config(
            &self.config_path,
            self.loaded_config_bytes.as_deref(),
            &rendered,
        )?;
        self.loaded_config_bytes = Some(rendered);
        self.loaded_config = self.config.clone();
        self.pending_edits = ConfigEditIntent::default();
        Ok(())
    }

    fn collect_registered_paths(
        repository: &Path,
        prefix: &Path,
        paths: &mut Vec<(String, bool)>,
    ) -> Result<(), SubmoduleError> {
        let git_ops = GitOpsManager::new(Some(repository), false).map_err(|error| {
            SubmoduleError::RepositoryError(format!(
                "Could not inspect nested repository at {}: {error}",
                repository.display()
            ))
        })?;
        let entries = git_ops.read_gitmodules().map_err(|error| {
            SubmoduleError::CliError(format!(
                "Could not inspect .gitmodules for '{}' at {}: {error}",
                prefix.display(),
                repository.join(".gitmodules").display()
            ))
        })?;
        let mut entries = entries
            .submodule_iter()
            .map(|(name, entry)| {
                (
                    name.clone(),
                    entry.path.clone().unwrap_or_else(|| name.clone()),
                )
            })
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| left.1.cmp(&right.1));
        for (_name, relative) in entries {
            crate::utilities::validate_submodule_path(repository, Path::new(&relative))
                .map_err(Self::map_path_validation_error)?;
            let display = prefix.join(&relative);
            let checkout = repository.join(&relative);
            let initialized = checkout.join(".git").exists();
            paths.push((display.to_string_lossy().into_owned(), initialized));
            if initialized {
                git_ops
                    .verify_submodule_checkout(&relative)
                    .map_err(|error| {
                        SubmoduleError::RepositoryError(format!(
                            "Nested submodule at {} does not belong to its declared checkout: {error}",
                            display.display()
                        ))
                    })?;
                Self::collect_registered_paths(&checkout, &display, paths)?;
            }
        }
        Ok(())
    }

    /// List configured submodules and, when requested, descend initialized Git registrations.
    pub fn list_submodules(&self, recursive: bool) -> Result<(), SubmoduleError> {
        let mut submodules: Vec<_> = self.config.get_submodules().collect();
        submodules.sort_by(|left, right| left.0.cmp(right.0));

        if submodules.is_empty() && !recursive {
            println!("No submodules configured.");
            return Ok(());
        }

        if submodules.is_empty() {
            println!("No submodules configured.");
        } else {
            println!("Submodules:");
            for (name, entry) in &submodules {
                let effective = self
                    .config
                    .effective_entry(name)
                    .expect("name came from config");
                let path = effective.path.as_deref().unwrap_or(name);
                let url = effective.url.as_deref().unwrap_or("<no url>");
                let active = entry.active.unwrap_or(true);
                let active_str = if active { "active" } else { "disabled" };
                println!(
                    "  {} [{active_str}]",
                    crate::utilities::safe_human_text(name)
                );
                println!("    path: {}", crate::utilities::safe_human_text(path));
                println!("    url:  {}", crate::utilities::safe_human_text(url));
            }
        }

        if recursive {
            let config_paths: std::collections::HashSet<String> = submodules
                .iter()
                .map(|(name, entry)| entry.path.clone().unwrap_or_else(|| (*name).clone()))
                .collect();
            let mut git_paths = Vec::new();
            Self::collect_registered_paths(
                &self.context.worktree_root,
                Path::new(""),
                &mut git_paths,
            )?;
            git_paths.sort_by(|left, right| left.0.cmp(&right.0));
            git_paths.dedup_by(|left, right| left.0 == right.0);
            let not_inspected = git_paths
                .iter()
                .filter(|(_, initialized)| !initialized)
                .map(|(path, _)| path.as_str())
                .collect::<Vec<_>>();
            let extra = git_paths
                .iter()
                .filter(|(path, _)| !config_paths.contains(path))
                .collect::<Vec<_>>();
            if !extra.is_empty() {
                println!("Additional submodules found in Git:");
                for (path, initialized) in extra {
                    let status = if *initialized {
                        "initialized"
                    } else {
                        "not inspected: checkout is uninitialized"
                    };
                    println!("  {} [{status}]", crate::utilities::safe_human_text(path));
                }
            }
            if !not_inspected.is_empty() {
                println!("Recursive inspection skipped for uninitialized checkouts:");
                for path in not_inspected {
                    println!("  {}", crate::utilities::safe_human_text(path));
                }
            }
        }

        Ok(())
    }

    fn prepare_global_defaults(
        &mut self,
        branch: Option<SerializableBranch>,
        ignore: Option<SerializableIgnore>,
        fetch_recurse: Option<SerializableFetchRecurse>,
        update: Option<SerializableUpdate>,
        use_git_default_sparse_checkout: Option<bool>,
        unset: &[&str],
    ) -> Result<Vec<(String, String, SubmoduleEntry)>, SubmoduleError> {
        if branch.is_none()
            && ignore.is_none()
            && fetch_recurse.is_none()
            && update.is_none()
            && use_git_default_sparse_checkout.is_none()
            && unset.is_empty()
        {
            return Err(SubmoduleError::ConfigError(
                "No settings provided to change.".to_string(),
            ));
        }
        for field in unset {
            match *field {
                "branch" => self.config.defaults.branch = None,
                "ignore" => self.config.defaults.ignore = None,
                "fetch" => self.config.defaults.fetch_recurse = None,
                "update" => self.config.defaults.update = None,
                "use-git-default-sparse-checkout" => {
                    self.config.defaults.use_git_default_sparse_checkout = None;
                }
                _ => {
                    return Err(SubmoduleError::ConfigError(format!(
                        "Unsupported global setting to unset: {field}"
                    )));
                }
            }
            self.pending_edits.default_field(match *field {
                "branch" => "branch",
                "ignore" => "ignore",
                "fetch" => "fetchRecurse",
                "update" => "update",
                "use-git-default-sparse-checkout" => "use_git_default_sparse_checkout",
                _ => unreachable!("validated above"),
            });
        }
        if let Some(value) = branch {
            self.config.defaults.branch = Some(value);
            self.pending_edits.default_field("branch");
        }
        if let Some(i) = ignore {
            self.config.defaults.ignore = Some(i);
            self.pending_edits.default_field("ignore");
        }
        if let Some(f) = fetch_recurse {
            self.config.defaults.fetch_recurse = Some(f);
            self.pending_edits.default_field("fetchRecurse");
        }
        if let Some(u) = update {
            self.config.defaults.update = Some(u);
            self.pending_edits.default_field("update");
        }
        if let Some(v) = use_git_default_sparse_checkout {
            self.config.defaults.use_git_default_sparse_checkout = Some(v);
            self.pending_edits
                .default_field("use_git_default_sparse_checkout");
        }
        let names: Vec<String> = self
            .config
            .get_submodules()
            .map(|(name, _)| name.clone())
            .collect();
        let targets = self.metadata_targets(&names)?;
        self.preflight_metadata_targets(&targets)?;
        Ok(targets)
    }

    /// Preview a global-default edit and its registered metadata targets.
    pub fn preview_global_defaults(
        &mut self,
        branch: Option<SerializableBranch>,
        ignore: Option<SerializableIgnore>,
        fetch_recurse: Option<SerializableFetchRecurse>,
        update: Option<SerializableUpdate>,
        use_git_default_sparse_checkout: Option<bool>,
        unset: &[&str],
    ) -> Result<(), SubmoduleError> {
        self.require_config()?;
        let branch_text = branch.as_ref().map(ToString::to_string);
        self.prepare_global_defaults(
            branch,
            ignore,
            fetch_recurse,
            update,
            use_git_default_sparse_checkout,
            unset,
        )?;
        let mut changes = unset
            .iter()
            .map(|field| format!("unset {field}"))
            .collect::<Vec<_>>();
        if let Some(branch) = branch_text {
            changes.push(format!("branch={branch}"));
        }
        println!(
            "Would change global defaults: {}.",
            crate::utilities::safe_human_text(&changes.join(", "))
        );
        Ok(())
    }

    /// Update global default settings and save the config.
    pub fn update_global_defaults(
        &mut self,
        branch: Option<SerializableBranch>,
        ignore: Option<SerializableIgnore>,
        fetch_recurse: Option<SerializableFetchRecurse>,
        update: Option<SerializableUpdate>,
        use_git_default_sparse_checkout: Option<bool>,
        unset: &[&str],
    ) -> Result<(), SubmoduleError> {
        let _locks = self.acquire_mutation_locks()?;
        self.reload_locked_config()?;
        self.require_config()?;
        let targets = self.prepare_global_defaults(
            branch,
            ignore,
            fetch_recurse,
            update,
            use_git_default_sparse_checkout,
            unset,
        )?;
        self.write_full_config()?;
        self.apply_metadata_targets(&targets)
    }

    fn prepare_disable(
        &mut self,
        name: &str,
    ) -> Result<Vec<(String, String, SubmoduleEntry)>, SubmoduleError> {
        let entry = self
            .config
            .get_submodule(name)
            .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                name: name.to_string(),
            })?
            .clone();

        // Update the entry in config
        let mut updated = entry;
        updated.active = Some(false);
        self.config
            .submodules
            .update_entry(name.to_string(), updated);

        let targets = self.metadata_targets(&[name.to_string()])?;
        self.preflight_metadata_targets(&targets)?;
        Ok(targets)
    }

    /// Preview disabling a declaration while preserving its checkout and repository.
    pub fn preview_disable_submodule(&mut self, name: &str) -> Result<(), SubmoduleError> {
        self.require_config()?;
        self.prepare_disable(name)?;
        println!(
            "Would disable submodule '{}' and preserve its checkout and history.",
            crate::utilities::safe_human_text(name)
        );
        Ok(())
    }

    /// Disable a submodule while preserving its checkout and stored repository.
    pub fn disable_submodule(&mut self, name: &str) -> Result<(), SubmoduleError> {
        let _locks = self.acquire_mutation_locks()?;
        self.reload_locked_config()?;
        self.require_config()?;
        let targets = self.prepare_disable(name)?;
        self.write_full_config()?;
        self.apply_metadata_targets(&targets)?;
        println!(
            "Disabled submodule '{}'.",
            crate::utilities::safe_human_text(name)
        );
        Ok(())
    }

    fn prepare_delete(&self, name: &str, force: bool) -> Result<(String, bool), SubmoduleError> {
        let entry =
            self.config
                .get_submodule(name)
                .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                    name: name.to_string(),
                })?;
        let path = entry.path.as_deref().unwrap_or(name).to_string();
        let registered = self.registration_for_path(&path)?.is_some();
        if registered {
            self.git_ops
                .preflight_delete_submodule(&path, force)
                .map_err(Self::map_git_ops_error)?;
        }
        Ok((path, registered))
    }

    /// Preview exact Git-aware deletion without changing the declaration or checkout.
    pub fn preview_delete_submodule_by_name(
        &self,
        name: &str,
        force: bool,
    ) -> Result<(), SubmoduleError> {
        self.require_config()?;
        let (path, registered) = self.prepare_delete(name, force)?;
        let scope = if registered {
            if force {
                "discard selected checkout content, retain its repository, and remove its registration and declaration"
            } else {
                "remove its checkout, registration, and declaration while retaining its repository"
            }
        } else {
            "remove only its config declaration"
        };
        println!(
            "Would delete submodule '{}' at '{}': {scope}.",
            crate::utilities::safe_human_text(name),
            crate::utilities::safe_human_text(&path)
        );
        Ok(())
    }

    /// Delete a submodule: deinit, remove from filesystem, and remove from config.
    pub fn delete_submodule_by_name(
        &mut self,
        name: &str,
        force: bool,
    ) -> Result<(), SubmoduleError> {
        let _locks = self.acquire_mutation_locks()?;
        self.reload_locked_config()?;
        self.require_config()?;
        let (path, registered) = self.prepare_delete(name, force)?;
        if registered {
            self.git_ops
                .delete_submodule(&path, force)
                .map_err(Self::map_git_ops_error)?;
        }

        // Remove from config
        let _ = self.config.submodules.remove_submodule(name);
        self.write_full_config()?;

        println!(
            "Deleted submodule '{}'.",
            crate::utilities::safe_human_text(name)
        );
        Ok(())
    }

    /// Prepare a change completely before its first write or Git mutation.
    #[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
    fn prepare_change_submodule(
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
        use_git_default_sparse_checkout: Option<bool>,
        unset: &[&str],
        clear_sparse_paths: bool,
    ) -> Result<ChangePlan, SubmoduleError> {
        let entry = self
            .config
            .get_submodule(name)
            .ok_or_else(|| SubmoduleError::SubmoduleNotFound {
                name: name.to_string(),
            })?
            .clone();

        let new_path = path
            .map(|path| {
                path.into_string().map_err(|_| {
                    SubmoduleError::InvalidPath(
                        "submodule path is not valid Unicode for TOML storage".to_string(),
                    )
                })
            })
            .transpose()?;
        let mut move_paths = None;

        // Move initialized registrations through Git; config-only declarations remain metadata-only.
        if let Some(ref np) = new_path {
            let old_path = entry.path.as_deref().unwrap_or(name);
            if np != old_path {
                let normalized = self.validate_requested_path(name, Path::new(np), Some(name))?;
                let destination = self.context.worktree_root.join(&normalized);
                if fs::symlink_metadata(&destination).is_ok() {
                    return Err(SubmoduleError::InvalidPath(format!(
                        "destination {} is occupied",
                        destination.display()
                    )));
                }
                if self.registration_for_path(old_path)?.is_some() {
                    self.git_ops
                        .preflight_move_submodule(old_path, np)
                        .map_err(Self::map_git_ops_error)?;
                    move_paths = Some((old_path.to_string(), np.clone()));
                }
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
            for field in unset {
                match *field {
                    "branch" => updated.branch = None,
                    "ignore" => updated.ignore = None,
                    "fetch" => updated.fetch_recurse = None,
                    "update" => updated.update = None,
                    "shallow" => updated.shallow = None,
                    "active" => updated.active = None,
                    "use-git-default-sparse-checkout" => {
                        updated.use_git_default_sparse_checkout = None;
                    }
                    _ => {
                        return Err(SubmoduleError::ConfigError(format!(
                            "Unsupported submodule setting to unset: {field}"
                        )));
                    }
                }
                self.pending_edits.module_field(
                    name,
                    match *field {
                        "branch" => "branch",
                        "ignore" => "ignore",
                        "fetch" => "fetchRecurse",
                        "update" => "update",
                        "shallow" => "shallow",
                        "active" => "active",
                        "use-git-default-sparse-checkout" => "use_git_default_sparse_checkout",
                        _ => unreachable!("validated above"),
                    },
                );
            }
            if let Some(np) = new_path {
                updated.path = Some(np);
                self.pending_edits.module_field(name, "path");
            }
            if let Some(b) = branch {
                updated.branch = SerializableBranch::set_branch(Some(b))
                    .map(Some)
                    .map_err(|err| SubmoduleError::ConfigError(err.to_string()))?;
                self.pending_edits.module_field(name, "branch");
            }
            if let Some(i) = ignore {
                updated.ignore = Some(i);
                self.pending_edits.module_field(name, "ignore");
            }
            if let Some(f) = fetch {
                updated.fetch_recurse = Some(f);
                self.pending_edits.module_field(name, "fetchRecurse");
            }
            if let Some(u) = update {
                updated.update = Some(u);
                self.pending_edits.module_field(name, "update");
            }
            if let Some(new_url) = url {
                updated.url = Some(new_url);
                self.pending_edits.module_field(name, "url");
            }
            if let Some(a) = active {
                updated.active = Some(a);
                self.pending_edits.module_field(name, "active");
            }
            if let Some(s) = shallow {
                updated.shallow = Some(s);
                self.pending_edits.module_field(name, "shallow");
            }
            if let Some(v) = use_git_default_sparse_checkout {
                updated.use_git_default_sparse_checkout = Some(v);
                self.pending_edits
                    .module_field(name, "use_git_default_sparse_checkout");
            }

            if clear_sparse_paths {
                updated.sparse_paths = Some(Vec::new());
                self.pending_edits.module_field(name, "sparse_paths");
            }

            // Update sparse paths
            if let Some(new_sparse) = sparse_paths {
                let new_paths: Vec<String> = new_sparse
                    .iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();
                // Keep SubmoduleEntries.sparse_checkouts in sync with sparse_paths
                let replace = !append_sparse;
                self.config
                    .submodules
                    .add_checkout(name, &new_paths, replace);

                if append_sparse {
                    let existing = updated.sparse_paths.get_or_insert_with(Vec::new);
                    existing.extend(new_paths);
                } else {
                    updated.sparse_paths = Some(new_paths);
                }
                self.pending_edits.module_field(name, "sparse_paths");
            }
            self.config
                .submodules
                .update_entry(name.to_string(), updated);
        }

        let metadata_targets = if let Some((old_path, new_path)) = &move_paths {
            let settings = Self::managed_settings(&self.config, name)?;
            self.git_ops
                .preflight_submodule_settings(old_path, &settings)
                .map_err(Self::map_git_ops_error)?;
            vec![(name.to_string(), new_path.clone(), settings)]
        } else {
            let targets = self.metadata_targets(&[name.to_string()])?;
            self.preflight_metadata_targets(&targets)?;
            targets
        };
        Ok(ChangePlan {
            move_paths,
            metadata_targets,
        })
    }

    /// Preview a configuration change after the same validation as execution.
    #[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
    pub fn preview_change_submodule(
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
        use_git_default_sparse_checkout: Option<bool>,
        unset: &[&str],
        clear_sparse_paths: bool,
    ) -> Result<(), SubmoduleError> {
        self.require_config()?;
        let plan = self.prepare_change_submodule(
            name,
            path,
            branch,
            sparse_paths,
            append_sparse,
            ignore,
            fetch,
            update,
            shallow,
            url,
            active,
            use_git_default_sparse_checkout,
            unset,
            clear_sparse_paths,
        )?;
        if let Some((old, new)) = plan.move_paths {
            println!(
                "Would change submodule '{}' and move it from '{}' to '{}'.",
                crate::utilities::safe_human_text(name),
                crate::utilities::safe_human_text(&old),
                crate::utilities::safe_human_text(&new)
            );
        } else {
            println!(
                "Would change submodule '{}' and reconcile its managed Git settings.",
                crate::utilities::safe_human_text(name)
            );
        }
        Ok(())
    }

    /// Change settings of an existing submodule, moving its checkout through Git when requested.
    #[allow(clippy::too_many_arguments, clippy::needless_pass_by_value)]
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
        use_git_default_sparse_checkout: Option<bool>,
        unset: &[&str],
        clear_sparse_paths: bool,
    ) -> Result<(), SubmoduleError> {
        let _locks = self.acquire_mutation_locks()?;
        self.reload_locked_config()?;
        self.require_config()?;
        let plan = self.prepare_change_submodule(
            name,
            path,
            branch,
            sparse_paths,
            append_sparse,
            ignore,
            fetch,
            update,
            shallow,
            url,
            active,
            use_git_default_sparse_checkout,
            unset,
            clear_sparse_paths,
        )?;
        if let Some((old, new)) = &plan.move_paths {
            self.git_ops
                .move_submodule(old, new)
                .map_err(Self::map_git_ops_error)?;
        }
        self.write_full_config()?;
        self.apply_metadata_targets(&plan.metadata_targets)?;
        println!(
            "Updated submodule '{}'.",
            crate::utilities::safe_human_text(name)
        );
        Ok(())
    }

    fn prepare_nuke(
        &self,
        all: bool,
        names: Option<Vec<String>>,
        kill: bool,
        force: bool,
    ) -> Result<Vec<NukePlan>, SubmoduleError> {
        let mut targets: Vec<String> = if all {
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
        targets.sort();
        if targets.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(SubmoduleError::ConfigError(
                "Duplicate submodule target in nuke selection".to_string(),
            ));
        }

        let mut snapshots = Vec::with_capacity(targets.len());
        for name in &targets {
            self.config
                .get_submodule(name)
                .ok_or_else(|| SubmoduleError::SubmoduleNotFound { name: name.clone() })?;
            let effective = self.config.effective_entry(name).expect("raw entry exists");
            let path = effective
                .path
                .as_deref()
                .expect("effective path is populated");
            let registration_name = self.registration_for_path(path)?;
            let registered = registration_name.is_some();
            let managed = Self::managed_settings(&self.config, name)?;
            let options = crate::config::SubmoduleAddOptions {
                name: registration_name.unwrap_or_else(|| name.clone()),
                path: PathBuf::from(path),
                url: effective.url.clone().expect("validated URL is populated"),
                branch: managed.branch.clone(),
                ignore: managed.ignore,
                fetch_recurse: managed.fetch_recurse,
                update: managed.update.clone(),
                shallow: managed.shallow.unwrap_or(false),
                no_init: false,
            };
            if registered {
                if kill {
                    self.git_ops
                        .preflight_delete_submodule(path, force)
                        .map_err(Self::map_git_ops_error)?;
                } else {
                    self.git_ops
                        .preflight_submodule_settings(path, &managed)
                        .map_err(Self::map_git_ops_error)?;
                    self.git_ops
                        .preflight_rebuild_submodule(path, force)
                        .map_err(Self::map_git_ops_error)?;
                }
            } else if !kill {
                self.git_ops
                    .preflight_add_submodule(&options)
                    .map_err(Self::map_git_ops_error)?;
            }
            snapshots.push(NukePlan {
                name: name.clone(),
                effective,
                options,
                registered,
            });
        }
        if !kill {
            // Native registration may stage `.gitmodules`; complete every
            // config-only structural add before registered rebuilds create
            // reviewable unstaged metadata deltas.
            snapshots.sort_by(|left, right| {
                left.registered
                    .cmp(&right.registered)
                    .then_with(|| left.name.cmp(&right.name))
            });
        }
        Ok(snapshots)
    }

    /// Preview a rebuild or permanent removal after the same whole-selection preflight.
    pub fn preview_nuke_submodules(
        &self,
        all: bool,
        names: Option<Vec<String>>,
        kill: bool,
        force: bool,
    ) -> Result<(), SubmoduleError> {
        self.require_config()?;
        for plan in self.prepare_nuke(all, names, kill, force)? {
            let path = plan.effective.path.as_deref().unwrap_or(&plan.name);
            if kill {
                println!(
                    "Would permanently remove submodule '{}' at '{}'{} and retain only its recoverable Git repository.",
                    crate::utilities::safe_human_text(&plan.name),
                    crate::utilities::safe_human_text(path),
                    if force {
                        ", discarding selected checkout content"
                    } else {
                        ""
                    }
                );
            } else {
                println!(
                    "Would rebuild submodule '{}' at '{}'{} while retaining its declaration, parent pin, and repository history.",
                    crate::utilities::safe_human_text(&plan.name),
                    crate::utilities::safe_human_text(path),
                    if force {
                        ", discarding selected checkout content"
                    } else {
                        ""
                    }
                );
            }
        }
        Ok(())
    }

    /// Nuke (deinit + delete + remove from config) all or specific submodules.
    /// If `kill` is false, reinitializes them after deletion.
    pub fn nuke_submodules(
        &mut self,
        all: bool,
        names: Option<Vec<String>>,
        kill: bool,
        force: bool,
    ) -> Result<(), SubmoduleError> {
        let _locks = self.acquire_mutation_locks()?;
        self.reload_locked_config()?;
        self.require_config()?;
        let snapshots = self.prepare_nuke(all, names, kill, force)?;

        let mut completed = Vec::new();
        for (index, plan) in snapshots.iter().enumerate() {
            let name = &plan.name;
            let effective = &plan.effective;
            let options = &plan.options;
            let registered = plan.registered;
            eprintln!(
                "{} submodule '{}'...",
                if kill { "Removing" } else { "Rebuilding" },
                crate::utilities::safe_human_text(name)
            );
            let path = effective
                .path
                .as_deref()
                .expect("effective path is populated");
            let result = (|| -> Result<(), SubmoduleError> {
                let managed = Self::managed_settings(&self.config, name)?;
                if registered {
                    if kill {
                        self.git_ops
                            .delete_submodule(path, force)
                            .map_err(Self::map_git_ops_error)?;
                    } else {
                        let mut update = crate::config::SubmoduleUpdateOptions::from_options(
                            effective.git_options(),
                        );
                        if update.strategy == SerializableUpdate::None {
                            update.strategy = SerializableUpdate::Checkout;
                        }
                        self.git_ops
                            .rebuild_submodule(path, update, &managed, force)
                            .map_err(Self::map_git_ops_error)?;
                    }
                } else if !kill {
                    self.git_ops
                        .add_submodule(options)
                        .map_err(Self::map_git_ops_error)?;
                }
                if !kill {
                    if registered {
                        self.git_ops
                            .sync_submodule_settings(path, &managed)
                            .map_err(Self::map_git_ops_error)?;
                    } else {
                        self.git_ops
                            .sync_added_submodule_settings(path, &managed)
                            .map_err(Self::map_git_ops_error)?;
                    }
                    println!(
                        "Reinitialized submodule '{}'.",
                        crate::utilities::safe_human_text(name)
                    );
                    if let Some(patterns) = &effective.sparse_paths {
                        self.configure_sparse_checkout(
                            path,
                            patterns,
                            effective.use_git_default_sparse_checkout.unwrap_or(false),
                        )?;
                    }
                }
                Ok(())
            })();
            if let Err(error) = result {
                let pending: Vec<&str> = snapshots[index + 1..]
                    .iter()
                    .map(|plan| plan.name.as_str())
                    .collect();
                eprintln!("Nuke stopped with a partial outcome after a runtime failure.");
                eprintln!(
                    "  completed: {}",
                    crate::utilities::safe_human_text(&completed.join(", "))
                );
                eprintln!(
                    "  failed: {}: {}",
                    crate::utilities::safe_human_text(name),
                    crate::utilities::safe_human_text(&error.to_string())
                );
                eprintln!(
                    "  pending: {}",
                    crate::utilities::safe_human_text(&pending.join(", "))
                );
                if kill {
                    eprintln!(
                        "  completed removals already removed their checkout and registration; TOML declarations remain until every selected removal succeeds, and retained repositories remain recoverable"
                    );
                } else {
                    eprintln!(
                        "  intended declarations, gitlinks, and retained repositories remain; repair the reported cause and retry"
                    );
                    eprintln!(
                        "  if native deinit left a deletion-only checkout, inspect it and rerun this nuke command with --force only when no local work is needed"
                    );
                }
                return Err(error);
            }
            completed.push(name.clone());
        }

        if kill {
            for plan in &snapshots {
                let _ = self.config.submodules.remove_submodule(&plan.name);
            }
            self.write_full_config()?;
            for plan in &snapshots {
                println!(
                    "{}: changed: removed its checkout, Git registration, and TOML declaration; retained repository history remains recoverable",
                    crate::utilities::safe_human_text(&plan.name)
                );
            }
        }

        println!(
            "Nuke summary: {} changed, 0 unchanged, 0 skipped, 0 failed.",
            snapshots.len()
        );

        Ok(())
    }

    /// Preview config generation using the same source inspection and rendering as execution.
    pub fn preview_generate_config(
        output: &std::path::Path,
        from_setup: bool,
        template: bool,
        force: bool,
    ) -> Result<(), SubmoduleError> {
        Self::generate_config_inner(output, from_setup, template, force, true)
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
        Self::generate_config_inner(output, from_setup, template, force, false)
    }

    // The bools mirror the `generate-config` CLI flags 1:1; both call sites
    // pass identically-named variables in the same order, and the dry-run
    // contract tests catch any transposition.
    #[allow(clippy::fn_params_excessive_bools)]
    fn generate_config_inner(
        output: &std::path::Path,
        from_setup: bool,
        template: bool,
        force: bool,
        dry_run: bool,
    ) -> Result<(), SubmoduleError> {
        Self::validate_config_destination(output)?;
        let output_lock = Self::config_lock_path(output)?;
        let setup_context = if from_setup {
            Some(
                RepositoryContext::discover(&std::env::current_dir()?, None).map_err(|error| {
                    SubmoduleError::RepositoryError(format!(
                        "Cannot import .gitmodules from the current directory: {error}"
                    ))
                })?,
            )
        } else {
            None
        };
        let _lock = if dry_run {
            None
        } else if let Some(context) = &setup_context {
            Some(Self::acquire_lock_paths(vec![
                context.common_dir.canonicalize()?.join("submod.lock"),
                output_lock,
            ])?)
        } else {
            Some(Self::acquire_output_lock(output)?)
        };
        let existing = match fs::read(output) {
            Ok(bytes) => Some(bytes),
            Err(error) if crate::utilities::is_absent_path(&error) => None,
            Err(error) => return Err(error.into()),
        };
        if existing.is_some() && !force {
            return Err(SubmoduleError::ConfigError(format!(
                "Output file '{}' already exists. Use --force to overwrite.",
                output.display()
            )));
        }

        if template {
            let sample = include_str!("../sample_config/submod.toml");
            Config::parse(sample).map_err(|error| {
                SubmoduleError::ConfigError(format!("Bundled template is invalid: {error}"))
            })?;
            if dry_run {
                println!(
                    "Would generate template config at '{}'.",
                    crate::utilities::safe_human_text(&output.to_string_lossy())
                );
                return Ok(());
            }
            Self::atomic_replace_config(output, existing.as_deref(), sample.as_bytes())?;
            println!(
                "Generated template config at '{}'.",
                crate::utilities::safe_human_text(&output.to_string_lossy())
            );
            return Ok(());
        }

        if from_setup {
            // Read .gitmodules from the repo and convert to our config format
            let context = setup_context.expect("from_setup context was resolved before locking");
            let git_ops = crate::git_ops::GitOpsManager::new(Some(&context.worktree_root), false)
                .map_err(|error| {
                SubmoduleError::RepositoryError(format!(
                    "Cannot inspect repository at {}: {error}",
                    context.worktree_root.display()
                ))
            })?;
            let mut entries = git_ops.read_gitmodules().map_err(|e| {
                SubmoduleError::CliError(format!("Failed to inspect .gitmodules: {e}"))
            })?;

            // Populate sparse_paths from the actual sparse-checkout config for each submodule.
            // Sparse checkout patterns are not stored in .gitmodules; they live in each
            // submodule's .git/info/sparse-checkout file.
            let names_and_paths: Vec<(String, String)> = entries
                .submodule_iter()
                .filter_map(|(name, entry)| {
                    entry.path.as_ref().map(|path| (name.clone(), path.clone()))
                })
                .collect();
            for (name, path) in names_and_paths {
                let checkout = context.worktree_root.join(&path);
                match fs::symlink_metadata(&checkout) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Ok(_) => {}
                    Err(error) => {
                        return Err(SubmoduleError::ConfigError(format!(
                            "Failed to inspect checkout for {name:?} at {path:?}: {error}"
                        )));
                    }
                }
                if !checkout.join(".git").exists() {
                    continue;
                }
                match git_ops.get_sparse_patterns(&path) {
                    Ok(patterns) if !patterns.is_empty() => {
                        entries.set_sparse_paths_for(&name, patterns);
                    }
                    Ok(_) => {}
                    Err(error) => {
                        return Err(SubmoduleError::ConfigError(format!(
                            "Failed to inspect sparse checkout for {name:?} at {path:?}: {error}"
                        )));
                    }
                }
            }

            // Build a Config from the SubmoduleEntries
            let config = Config::new(crate::config::SubmoduleDefaults::default(), entries);

            // Serialize using write_full_config logic but to the output path
            let mut tmp_manager = Self {
                git_ops,
                loaded_config: Config::default(),
                config,
                context,
                config_path: output.to_path_buf(),
                loaded_config_bytes: None,
                pending_edits: ConfigEditIntent::default(),
                verbose: false,
            };
            let rendered = tmp_manager.render_config_document()?;
            if dry_run {
                println!(
                    "Would generate config from .gitmodules at '{}'.",
                    crate::utilities::safe_human_text(&output.to_string_lossy())
                );
                return Ok(());
            }
            Self::atomic_replace_config(output, existing.as_deref(), &rendered)?;
            tmp_manager.loaded_config_bytes = Some(rendered);
            println!(
                "Generated config from .gitmodules at '{}'.",
                crate::utilities::safe_human_text(&output.to_string_lossy())
            );
            return Ok(());
        }

        // Neither template nor from-setup: write an empty config
        let empty = "[defaults]\n";
        Config::parse(empty).map_err(|error| {
            SubmoduleError::ConfigError(format!("Generated empty config is invalid: {error}"))
        })?;
        if dry_run {
            println!(
                "Would generate empty config at '{}'.",
                crate::utilities::safe_human_text(&output.to_string_lossy())
            );
            return Ok(());
        }
        Self::atomic_replace_config(output, existing.as_deref(), empty.as_bytes())?;
        println!(
            "Generated empty config at '{}'.",
            crate::utilities::safe_human_text(&output.to_string_lossy())
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    // Helper to create a `GitManager` for tests that need to call methods such as
    // `check_sparse_checkout_status`. It initializes a real git repository in `repo_dir`
    // via `git2` so that `GitOpsManager` can open it without depending on the caller's
    // working directory being inside a git repo.
    fn create_test_manager(repo_dir: &Path, config_path: PathBuf) -> GitManager {
        git2::Repository::init(repo_dir).expect("Failed to init git repo");
        fs::write(&config_path, "[defaults]\n").unwrap();
        GitManager::with_repo_path(config_path, repo_dir).expect("Failed to create GitManager")
    }

    fn create_test_manager_with_submodule(repo_dir: &Path, config_path: PathBuf) -> GitManager {
        git2::Repository::init(repo_dir).expect("Failed to init parent repo");
        let child = repo_dir.join("submodule");
        git2::Repository::init(&child).expect("Failed to init child repo");
        fs::write(
            repo_dir.join(".gitmodules"),
            "[submodule \"submodule\"]\n\tpath = submodule\n\turl = https://example.invalid/submodule.git\n",
        )
        .unwrap();
        fs::write(&config_path, "[defaults]\n").unwrap();
        GitManager::with_repo_path(config_path, repo_dir).expect("Failed to create GitManager")
    }

    #[test]
    fn test_sparse_checkout_not_configured() {
        let temp_dir = tempdir().unwrap();
        let manager = create_test_manager_with_submodule(
            temp_dir.path(),
            temp_dir.path().join("submod.toml"),
        );

        let expected_paths: Vec<String> = vec!["path/a".to_string()];

        let status = manager
            .check_sparse_checkout_status("submodule", &expected_paths)
            .unwrap();

        assert_eq!(status, SparseStatus::NotConfigured);
    }

    #[test]
    fn test_sparse_checkout_correct() {
        let temp_dir = tempdir().unwrap();
        let manager = create_test_manager_with_submodule(
            temp_dir.path(),
            temp_dir.path().join("submod.toml"),
        );
        let submodule_path = temp_dir.path().join("submodule");

        // Create .git/info/sparse-checkout with the expected paths
        let info_dir = submodule_path.join(".git").join("info");
        fs::create_dir_all(&info_dir).unwrap();
        let content = format!("{SPARSE_DENY_ALL}\npath/a\npath/b\n");
        fs::write(info_dir.join("sparse-checkout"), content).unwrap();
        let repo = git2::Repository::open(&submodule_path).unwrap();
        let mut child_config = repo.config().unwrap();
        child_config.set_bool("core.sparseCheckout", true).unwrap();
        child_config
            .set_bool("core.sparseCheckoutCone", false)
            .unwrap();

        let expected_paths = vec![
            SPARSE_DENY_ALL.to_string(),
            "path/a".to_string(),
            "path/b".to_string(),
        ];

        let status = manager
            .check_sparse_checkout_status("submodule", &expected_paths)
            .unwrap();

        assert_eq!(status, SparseStatus::Correct);
    }

    #[test]
    fn test_sparse_checkout_correct_with_extras() {
        // Extra patterns alter the materialized tree and must be reported.
        let temp_dir = tempdir().unwrap();
        let manager = create_test_manager_with_submodule(
            temp_dir.path(),
            temp_dir.path().join("submod.toml"),
        );
        let submodule_path = temp_dir.path().join("submodule");

        let info_dir = submodule_path.join(".git").join("info");
        fs::create_dir_all(&info_dir).unwrap();
        // File has path/a, path/b AND an extra path/c not in expected_paths
        let content = format!("{SPARSE_DENY_ALL}\npath/a\npath/b\npath/c\n");
        fs::write(info_dir.join("sparse-checkout"), content).unwrap();
        let repo = git2::Repository::open(&submodule_path).unwrap();
        let mut child_config = repo.config().unwrap();
        child_config.set_bool("core.sparseCheckout", true).unwrap();
        child_config
            .set_bool("core.sparseCheckoutCone", false)
            .unwrap();

        let expected_paths = vec![
            SPARSE_DENY_ALL.to_string(),
            "path/a".to_string(),
            "path/b".to_string(),
        ];

        let status = manager
            .check_sparse_checkout_status("submodule", &expected_paths)
            .unwrap();

        assert_eq!(
            status,
            SparseStatus::Mismatch {
                expected: expected_paths,
                actual: vec![
                    SPARSE_DENY_ALL.into(),
                    "path/a".into(),
                    "path/b".into(),
                    "path/c".into(),
                ],
            }
        );
    }

    #[test]
    fn test_write_full_config_fetch_recurse_round_trips() {
        // Regression for the fetch-recurse round-trip bug: submod must be able to
        // read back the fetch-recurse setting it writes. The writer previously emitted
        // `fetch = "true"/"false"` — both the wrong TOML key (neither the documented
        // `fetchRecurse` nor the field `fetch_recurse`) and the git-config value form
        // (`true`/`false` instead of `always`/`never`) — so reloading silently dropped it.
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("submod.toml");
        let mut manager = create_test_manager(temp_dir.path(), config_path.clone());

        manager.config.defaults.fetch_recurse = Some(SerializableFetchRecurse::Always);
        manager.config.add_submodule(
            "mymod".to_string(),
            SubmoduleEntry::new(
                Some("https://example.com/repo.git".to_string()),
                Some("libs/mymod".to_string()),
                None,
                None,
                None,
                Some(SerializableFetchRecurse::Never),
                Some(true),
                None,
                None,
            ),
        );

        manager.write_full_config().expect("write_full_config");

        let written = fs::read_to_string(&config_path).unwrap();
        let reloaded: Config = toml::from_str(&written)
            .expect("config submod writes must be valid TOML it can parse back");
        assert_eq!(
            reloaded.defaults.fetch_recurse,
            Some(SerializableFetchRecurse::Always),
            "defaults fetch-recurse must survive a write/read round-trip; written file:\n{written}"
        );
        assert_eq!(
            reloaded.submodules.get("mymod").unwrap().fetch_recurse,
            Some(SerializableFetchRecurse::Never),
            "per-submodule fetch-recurse must survive a write/read round-trip; written file:\n{written}"
        );
    }

    #[test]
    fn test_save_config_persists_edits_to_existing_section() {
        // `save_config` (the writer used by `add`) was append-only: once a
        // submodule's section existed in submod.toml, later edits to that entry
        // were silently not written back, because the section header was
        // "already present". A load→modify→save→reload must reflect the edit
        // (#62 P1).
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("submod.toml");
        let mut manager = create_test_manager(temp_dir.path(), config_path.clone());

        // Seed an existing [mymod] section tracking branch = "main".
        manager.config.add_submodule(
            "mymod".to_string(),
            SubmoduleEntry::new(
                Some("https://example.com/repo.git".to_string()),
                Some("libs/mymod".to_string()),
                Some(SerializableBranch::Name("main".to_string())),
                None,
                None,
                None,
                Some(true),
                None,
                None,
            ),
        );
        manager.save_config().expect("initial save");

        // Precondition (guards against a vacuous pass): the section was written
        // with branch = "main".
        let first = fs::read_to_string(&config_path).unwrap();
        let parsed_first: Config = toml::from_str(&first).expect("initial save must be valid TOML");
        assert_eq!(
            parsed_first.submodules.get("mymod").unwrap().branch,
            Some(SerializableBranch::Name("main".to_string())),
            "precondition: initial save must record branch=main; file:\n{first}"
        );

        // Modify the existing entry: branch main → develop.
        manager.config.add_submodule(
            "mymod".to_string(),
            SubmoduleEntry::new(
                Some("https://example.com/repo.git".to_string()),
                Some("libs/mymod".to_string()),
                Some(SerializableBranch::Name("develop".to_string())),
                None,
                None,
                None,
                Some(true),
                None,
                None,
            ),
        );
        manager.save_config().expect("second save");

        // The edit must be persisted, not dropped because the section pre-existed.
        let second = fs::read_to_string(&config_path).unwrap();
        let reloaded: Config = toml::from_str(&second).expect("second save must be valid TOML");
        assert_eq!(
            reloaded.submodules.get("mymod").unwrap().branch,
            Some(SerializableBranch::Name("develop".to_string())),
            "save_config must persist edits to an existing section; file:\n{second}"
        );
    }

    #[test]
    fn r10_atomic_compare_preserves_external_edits_and_presence() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("submod.toml");
        let loaded = b"[defaults]\nignore = \"dirty\"\n";
        let external = b"[defaults]\nignore = \"all\" # external\n";
        let rendered = b"[defaults]\nignore = \"none\"\n";
        fs::write(&config_path, external).unwrap();

        let error = GitManager::atomic_replace_config(&config_path, Some(loaded), rendered)
            .expect_err("an external edit after the locked snapshot must be rejected");
        assert!(error.to_string().contains("changed after it was loaded"));
        assert_eq!(fs::read(&config_path).unwrap(), external);
        Config::parse(std::str::from_utf8(external).unwrap()).unwrap();

        let appeared = temp_dir.path().join("appeared.toml");
        fs::write(&appeared, []).unwrap();
        GitManager::atomic_replace_config(&appeared, None, b"[defaults]\n")
            .expect_err("an absent destination that appears must be rejected");
        assert_eq!(fs::read(&appeared).unwrap(), b"");

        let deleted = temp_dir.path().join("deleted.toml");
        GitManager::atomic_replace_config(&deleted, Some(b""), b"[defaults]\n")
            .expect_err("an originally empty destination that disappears must be rejected");
        assert!(!deleted.exists());

        let names: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 2, "temporary siblings must be cleaned up");
    }

    #[test]
    fn r10_atomic_commit_failure_preserves_original_and_cleans_temporary() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("submod.toml");
        let original = b"[defaults]\nignore = \"dirty\"\n";
        let rendered = b"[defaults]\nignore = \"all\"\n";
        fs::write(&config_path, original).unwrap();

        let error = GitManager::atomic_replace_config_with(
            &config_path,
            Some(original),
            rendered,
            |_temporary, _path| {
                Err(SubmoduleError::ConfigError(
                    "injected failure at atomic commit boundary".to_string(),
                ))
            },
        )
        .expect_err("commit failure must be returned");
        assert!(error.to_string().contains("injected failure"));
        assert_eq!(fs::read(&config_path).unwrap(), original);
        Config::parse(std::str::from_utf8(original).unwrap()).unwrap();
        let names: Vec<_> = fs::read_dir(temp_dir.path())
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        assert_eq!(names, [config_path.file_name().unwrap()]);
    }

    #[test]
    fn r10_stale_manager_reloads_after_acquiring_locks() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("submod.toml");
        git2::Repository::init(temp_dir.path()).unwrap();
        fs::write(&config_path, "# retained\n[defaults]\n").unwrap();
        let mut first = GitManager::with_repo_path(config_path.clone(), temp_dir.path()).unwrap();
        let mut stale = GitManager::with_repo_path(config_path.clone(), temp_dir.path()).unwrap();

        first
            .update_global_defaults(None, Some(SerializableIgnore::All), None, None, None, &[])
            .unwrap();
        stale
            .update_global_defaults(
                None,
                None,
                Some(SerializableFetchRecurse::Never),
                None,
                None,
                &[],
            )
            .unwrap();

        let source = fs::read_to_string(config_path).unwrap();
        let current = Config::parse(&source).unwrap();
        assert_eq!(current.defaults.ignore, Some(SerializableIgnore::All));
        assert_eq!(
            current.defaults.fetch_recurse,
            Some(SerializableFetchRecurse::Never)
        );
        assert!(source.contains("# retained"));
    }

    #[test]
    fn test_sparse_checkout_mismatch() {
        let temp_dir = tempdir().unwrap();
        let manager = create_test_manager_with_submodule(
            temp_dir.path(),
            temp_dir.path().join("submod.toml"),
        );
        let submodule_path = temp_dir.path().join("submodule");

        // sparse-checkout has only path/a; path/b is expected but absent
        let info_dir = submodule_path.join(".git").join("info");
        fs::create_dir_all(&info_dir).unwrap();
        let content = format!("{SPARSE_DENY_ALL}\npath/a\n");
        fs::write(info_dir.join("sparse-checkout"), content).unwrap();
        let repo = git2::Repository::open(&submodule_path).unwrap();
        let mut child_config = repo.config().unwrap();
        child_config.set_bool("core.sparseCheckout", true).unwrap();
        child_config
            .set_bool("core.sparseCheckoutCone", false)
            .unwrap();

        let expected_paths = vec![
            SPARSE_DENY_ALL.to_string(),
            "path/a".to_string(),
            "path/b".to_string(), // expected but not configured
        ];

        let status = manager
            .check_sparse_checkout_status("submodule", &expected_paths)
            .unwrap();

        match status {
            SparseStatus::Mismatch { expected, actual } => {
                assert_eq!(
                    expected,
                    vec![
                        SPARSE_DENY_ALL.to_string(),
                        "path/a".to_string(),
                        "path/b".to_string()
                    ]
                );
                assert_eq!(
                    actual,
                    vec![SPARSE_DENY_ALL.to_string(), "path/a".to_string()]
                );
            }
            _ => panic!("Expected Mismatch, got {status:?}"),
        }
    }
}
