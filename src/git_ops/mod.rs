// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
#![doc = r"
This module is the native Git mutation boundary.

Every lifecycle mutation (add/init/update/move/deinit/delete/reset/stash/clean/sparse)
runs through one checked `git` invocation path on `GitOpsManager`. There are
no competing backend mutation implementations and no cross-backend retry
after a failure.

`gix` and `git2` survive only as read backends behind `try_with_fallback`
(gix first, git2 fallback) for inspection reads: gitmodules, config, status,
list, and sparse patterns.
"]
/// git2-based git operations implementation
pub mod git2_ops;
/// gitoxide (gix)-based git operations implementation
pub mod gix_ops;
pub use git2_ops::Git2Operations;
pub use gix_ops::GixOperations;

use anyhow::{Context, Result};
use bitflags::bitflags;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use crate::config::{
    SubmoduleAddOptions, SubmoduleEntries, SubmoduleEntry, SubmoduleUpdateOptions,
};
use crate::options::{
    ConfigLevel, GitmodulesConvert, SerializableBranch, SerializableFetchRecurse,
    SerializableIgnore, SerializableUpdate,
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
#[allow(dead_code, clippy::struct_excessive_bools)]
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

/// Git operations served by `GitOpsManager`: native Git mutations plus
/// gix-first, git2-fallback inspection reads.
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
    fn delete_submodule(&mut self, path: &str, force: bool) -> Result<()>;
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
    fn stash_submodule(&self, path: &str, include_untracked: bool) -> Result<Option<String>>;

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
    worktree: PathBuf,
    git_dir: PathBuf,
    verbose: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ModuleConfigSnapshot {
    local: Vec<(String, String)>,
    worktree: Option<Vec<(String, String)>>,
}

/// Native Git mutations with supported repository inspection backends.
impl GitOpsManager {
    fn managed_portable_fields(entry: &SubmoduleEntry) -> Vec<(&'static str, Option<String>)> {
        vec![
            ("url", entry.url.clone()),
            (
                "branch",
                entry.branch.as_ref().map(GitmodulesConvert::to_gitmodules),
            ),
            ("ignore", entry.ignore.map(|value| value.to_string())),
            ("update", entry.update.as_ref().map(ToString::to_string)),
            (
                "fetchRecurseSubmodules",
                entry.fetch_recurse.map(|value| value.to_gitmodules()),
            ),
            ("shallow", entry.shallow.map(|value| value.to_string())),
        ]
    }

    fn config_value(&self, scope: &str, key: &str) -> Result<Option<String>> {
        let output = match scope {
            "portable" => self.git_output(["config", "--file", ".gitmodules", "--get", key])?,
            "staged" => self.git_output(["config", "--blob", ":0:.gitmodules", "--get", key])?,
            "local" => self.git_output(["config", "--local", "--get", key])?,
            "worktree" => self.git_output(["config", "--worktree", "--get", key])?,
            _ => unreachable!("known Git config scope"),
        };
        match output.status.code() {
            Some(0) => Ok(Some(
                String::from_utf8(output.stdout)?.trim_end().to_string(),
            )),
            Some(1) => Ok(None),
            _ => anyhow::bail!(
                "Could not inspect {scope} Git key {key}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        }
    }

    fn worktree_config_enabled(&self) -> Result<bool> {
        let output = self.git_output(["config", "--bool", "extensions.worktreeConfig"])?;
        match output.status.code() {
            Some(0) => Ok(String::from_utf8(output.stdout)?.trim() == "true"),
            Some(1) => Ok(false),
            _ => anyhow::bail!(
                "Could not inspect extensions.worktreeConfig: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        }
    }

    fn set_config_value_exact(&self, scope: &str, key: &str, desired: Option<&str>) -> Result<()> {
        let mut args = vec![OsString::from("config"), OsString::from("--no-includes")];
        match scope {
            "portable" => args.extend([OsString::from("--file"), OsString::from(".gitmodules")]),
            "local" => args.push(OsString::from("--local")),
            "worktree" => args.push(OsString::from("--worktree")),
            _ => unreachable!("writable Git config scope"),
        }
        if let Some(value) = desired {
            args.extend([OsString::from(key), OsString::from(value)]);
            self.git(args)?;
        } else {
            args.extend([OsString::from("--unset-all"), OsString::from(key)]);
            let output = self.git_output(args)?;
            if !output.status.success() && !matches!(output.status.code(), Some(1 | 5)) {
                anyhow::bail!(
                    "Could not unset {scope} Git key {key}: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                );
            }
        }
        Ok(())
    }

    fn module_config_values(&self, scope: &str, name: &str) -> Result<Vec<(String, String)>> {
        let mut args = vec![OsString::from("config"), OsString::from("--no-includes")];
        match scope {
            "local" => args.push(OsString::from("--local")),
            "worktree" => args.push(OsString::from("--worktree")),
            _ => unreachable!("known module config scope"),
        }
        args.extend([
            OsString::from("-z"),
            OsString::from("--get-regexp"),
            OsString::from(format!(
                r"^submodule\.{}\.",
                Self::config_key_regex_literal(name)
            )),
        ]);
        let output = self.git_output(args)?;
        match output.status.code() {
            Some(0) => output
                .stdout
                .split(|byte| *byte == 0)
                .filter(|record| !record.is_empty())
                .map(|record| {
                    let newline = record
                        .iter()
                        .position(|byte| *byte == b'\n')
                        .context("Git returned a malformed module configuration entry")?;
                    Ok((
                        String::from_utf8(record[..newline].to_vec())?,
                        String::from_utf8(record[newline + 1..].to_vec())?,
                    ))
                })
                .collect(),
            Some(1) => Ok(Vec::new()),
            _ => anyhow::bail!(
                "Could not inspect {scope} Git settings for submodule {name:?}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        }
    }

    fn snapshot_module_config(&self, name: &str) -> Result<ModuleConfigSnapshot> {
        Ok(ModuleConfigSnapshot {
            local: self.module_config_values("local", name)?,
            worktree: self
                .worktree_config_enabled()?
                .then(|| self.module_config_values("worktree", name))
                .transpose()?,
        })
    }

    fn restore_module_config_values(
        &self,
        scope: &str,
        name: &str,
        values: &[(String, String)],
    ) -> Result<()> {
        if self.module_config_values(scope, name)? == values {
            return Ok(());
        }

        let mut remove = vec![OsString::from("config"), OsString::from("--no-includes")];
        match scope {
            "local" => remove.push(OsString::from("--local")),
            "worktree" => remove.push(OsString::from("--worktree")),
            _ => unreachable!("known module config scope"),
        }
        remove.extend([
            OsString::from("--remove-section"),
            OsString::from(format!("submodule.{name}")),
        ]);
        let output = self.git_output(remove)?;
        if !output.status.success() && output.status.code() != Some(1) {
            anyhow::bail!(
                "Could not clear {scope} Git settings for submodule {name:?}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        for (key, value) in values {
            let mut add = vec![OsString::from("config"), OsString::from("--no-includes")];
            match scope {
                "local" => add.push(OsString::from("--local")),
                "worktree" => add.push(OsString::from("--worktree")),
                _ => unreachable!("known module config scope"),
            }
            add.extend([
                OsString::from("--add"),
                OsString::from(key),
                OsString::from(value),
            ]);
            self.git(add)?;
        }
        anyhow::ensure!(
            self.module_config_values(scope, name)? == values,
            "Git did not restore the exact {scope} settings for submodule {name:?}"
        );
        Ok(())
    }

    fn restore_module_config(&self, name: &str, snapshot: &ModuleConfigSnapshot) -> Result<()> {
        self.restore_module_config_values("local", name, &snapshot.local)?;
        if let Some(values) = &snapshot.worktree {
            self.restore_module_config_values("worktree", name, values)?;
        }
        Ok(())
    }

    fn module_config_key_is_managed(name: &str, key: &str) -> bool {
        let prefix = format!("submodule.{name}.");
        key.strip_prefix(&prefix).is_some_and(|field| {
            [
                "url",
                "branch",
                "ignore",
                "update",
                "fetchRecurseSubmodules",
                "shallow",
                "active",
            ]
            .iter()
            .any(|managed| field.eq_ignore_ascii_case(managed))
        })
    }

    fn preserve_unmanaged_module_config(
        name: &str,
        desired: &ModuleConfigSnapshot,
        original: &ModuleConfigSnapshot,
    ) -> ModuleConfigSnapshot {
        let merge = |desired: &[(String, String)], original: &[(String, String)]| {
            desired
                .iter()
                .filter(|(key, _)| Self::module_config_key_is_managed(name, key))
                .chain(
                    original
                        .iter()
                        .filter(|(key, _)| !Self::module_config_key_is_managed(name, key)),
                )
                .cloned()
                .collect()
        };
        ModuleConfigSnapshot {
            local: merge(&desired.local, &original.local),
            worktree: desired
                .worktree
                .as_ref()
                .map(|desired| merge(desired, original.worktree.as_deref().unwrap_or_default())),
        }
    }

    fn config_value_in(repository: &Path, key: &str) -> Result<Option<String>> {
        let output = Command::new("git")
            .args(["config", "--get", key])
            .current_dir(repository)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .with_context(|| format!("Failed to inspect Git config in {}", repository.display()))?;
        match output.status.code() {
            Some(0) => Ok(Some(
                String::from_utf8(output.stdout)?.trim_end().to_string(),
            )),
            Some(1) => Ok(None),
            _ => anyhow::bail!(
                "Could not inspect Git key {key} in {}: {}",
                repository.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        }
    }

    fn set_config_value_in(repository: &Path, key: &str, value: &str) -> Result<()> {
        if Self::config_value_in(repository, key)?.as_deref() == Some(value) {
            return Ok(());
        }
        let output = Command::new("git")
            .args(["config", "--no-includes", "--local", key, value])
            .current_dir(repository)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .with_context(|| format!("Failed to update Git config in {}", repository.display()))?;
        anyhow::ensure!(
            output.status.success(),
            "Could not set Git key {key} in {}: {}",
            repository.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
        Ok(())
    }

    fn default_remote_in(repository: &Path) -> Result<String> {
        let branch = Command::new("git")
            .args(["symbolic-ref", "--quiet", "--short", "HEAD"])
            .current_dir(repository)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .with_context(|| format!("Failed to inspect HEAD in {}", repository.display()))?;
        match branch.status.code() {
            Some(0) => {
                let branch = String::from_utf8(branch.stdout)?.trim_end().to_string();
                Ok(
                    Self::config_value_in(repository, &format!("branch.{branch}.remote"))?
                        .unwrap_or_else(|| "origin".to_string()),
                )
            }
            Some(1) => Ok("origin".to_string()),
            _ => anyhow::bail!(
                "Could not inspect HEAD in {}: {}",
                repository.display(),
                String::from_utf8_lossy(&branch.stderr).trim()
            ),
        }
    }

    /// Mirror native `git submodule sync` remote selection.
    ///
    /// Prefer the child remote whose fetch URL already matches the resolved
    /// URL (native sync looks the remote up by URL first since Git 2.51),
    /// falling back to the checked-out branch's remote or `origin`, which is
    /// what every Git version uses when no remote URL matches.
    fn synced_child_remote(child: &Path, expected_url: &str) -> Result<String> {
        if let Some(remote) = Self::child_remote_with_url(child, expected_url)? {
            return Ok(remote);
        }
        Self::default_remote_in(child)
    }

    /// First child remote (in `git remote` order) with a fetch URL exactly
    /// equal to `expected_url`, mirroring `remote_has_url` in native sync.
    fn child_remote_with_url(child: &Path, expected_url: &str) -> Result<Option<String>> {
        let output = Command::new("git")
            .args(["remote"])
            .current_dir(child)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .with_context(|| format!("Failed to list remotes in {}", child.display()))?;
        anyhow::ensure!(
            output.status.success(),
            "Could not list remotes in {}: {}",
            child.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
        let remotes = String::from_utf8(output.stdout)?;
        for name in remotes
            .lines()
            .map(str::trim)
            .filter(|name| !name.is_empty())
        {
            let values = Command::new("git")
                .args(["config", "--get-all", &format!("remote.{name}.url")])
                .current_dir(child)
                .env("GIT_OPTIONAL_LOCKS", "0")
                .output()
                .with_context(|| {
                    format!("Failed to inspect remote {name:?} in {}", child.display())
                })?;
            match values.status.code() {
                Some(0) => {
                    let urls = String::from_utf8(values.stdout)?;
                    if urls.lines().any(|url| url == expected_url) {
                        return Ok(Some(name.to_string()));
                    }
                }
                Some(1) => {}
                _ => anyhow::bail!(
                    "Could not inspect remote {name:?} in {}: {}",
                    child.display(),
                    String::from_utf8_lossy(&values.stderr).trim()
                ),
            }
        }
        Ok(None)
    }

    /// Compare a `.gitmodules` path value against a requested path. The value
    /// arrives as raw bytes while the request is a normalized OS string, and
    /// on Windows the two spell separators differently (forward slashes in
    /// storage, backslashes in normalized paths) for the same file, so fold
    /// separators there; elsewhere a byte comparison is exact.
    fn gitmodules_path_matches(value: &[u8], requested: &OsStr) -> bool {
        #[cfg(windows)]
        let matches = {
            let fold = |byte: u8| {
                if byte == b'\\' { b'/' } else { byte }
            };
            value
                .iter()
                .map(|byte| fold(*byte))
                .eq(requested.as_encoded_bytes().iter().map(|byte| fold(*byte)))
        };
        #[cfg(not(windows))]
        let matches = value == requested.as_encoded_bytes();
        matches
    }

    fn starts_dot_component(value: &str, parent: bool) -> bool {
        let prefix = if parent { ".." } else { "." };
        value
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/') || rest.starts_with('\\'))
    }

    fn remote_url_is_relative(value: &str) -> bool {
        if Path::new(value).is_absolute()
            || value.starts_with('/')
            || value.starts_with('\\')
            || value.contains("://")
        {
            return false;
        }
        value
            .find(':')
            .is_none_or(|colon| value[..colon].contains(['/', '\\']))
    }

    /// Match Git's `relative_url()` for the two values written by `submodule sync`.
    fn resolve_relative_submodule_url(
        remote_url: &str,
        relative_url: &str,
        up_path: Option<&str>,
    ) -> Result<String> {
        if !Self::starts_dot_component(relative_url, false)
            && !Self::starts_dot_component(relative_url, true)
        {
            return Ok(relative_url.to_string());
        }

        let base_is_relative = Self::remote_url_is_relative(remote_url);
        let mut base = remote_url.trim_end_matches(['/', '\\']).to_string();
        anyhow::ensure!(!base.is_empty(), "Git remote URL is empty");
        if base_is_relative
            && !Self::starts_dot_component(&base, false)
            && !Self::starts_dot_component(&base, true)
        {
            base.insert_str(0, "./");
        }

        let mut remainder = relative_url;
        let mut colon_separator = false;
        loop {
            if Self::starts_dot_component(remainder, true) {
                remainder = &remainder[3..];
                if let Some(separator) = base.rfind(['/', '\\']) {
                    base.truncate(separator);
                } else if let Some(colon) = base.rfind(':') {
                    base.truncate(colon);
                    colon_separator = true;
                } else if base_is_relative || base == "." {
                    anyhow::bail!("cannot strip one component from Git remote URL {remote_url:?}");
                } else {
                    base = ".".to_string();
                }
            } else if Self::starts_dot_component(remainder, false) {
                remainder = &remainder[2..];
            } else {
                break;
            }
        }

        let separator = if colon_separator { ':' } else { '/' };
        let mut resolved = format!("{base}{separator}{remainder}");
        if relative_url.ends_with(['/', '\\']) {
            resolved.pop();
        }
        if let Some(stripped) = resolved.strip_prefix("./") {
            resolved = stripped.to_string();
        }
        if base_is_relative && let Some(up_path) = up_path {
            resolved.insert_str(0, up_path);
        }
        // Absolute URLs always use forward slashes: resolving against a
        // Windows-spelled base (`file://C:\...`) must not propagate
        // backslashes into the recorded URL, which would defeat exact-match
        // remote lookups. Relative results keep their spelling.
        if resolved.contains("://") {
            resolved = resolved.replace('\\', "/");
        }
        Ok(resolved)
    }

    fn expected_synced_urls(
        &self,
        path: &Path,
        url: &str,
    ) -> Result<(String, Option<(String, String)>)> {
        if !Self::starts_dot_component(url, false) && !Self::starts_dot_component(url, true) {
            let child = self.worktree.join(path);
            let selected = if child.join(".git").exists() {
                Some((Self::synced_child_remote(&child, url)?, url.to_string()))
            } else {
                None
            };
            return Ok((url.to_string(), selected));
        }

        let parent_remote = Self::default_remote_in(&self.worktree)?;
        let parent_base =
            Self::config_value_in(&self.worktree, &format!("remote.{parent_remote}.url"))?
                .unwrap_or_else(|| self.worktree.to_string_lossy().into_owned());
        let parent_url = Self::resolve_relative_submodule_url(&parent_base, url, None)?;
        let child = self.worktree.join(path);
        let child_expected = if child.join(".git").exists() {
            let up_path = "../".repeat(path.components().count());
            let expected = Self::resolve_relative_submodule_url(&parent_base, url, Some(&up_path))?;
            Some((Self::synced_child_remote(&child, &expected)?, expected))
        } else {
            None
        };
        Ok((parent_url, child_expected))
    }

    fn reconcile_rebuild_child_url(&self, path: &Path, entry: &SubmoduleEntry) -> Result<()> {
        let Some(url) = &entry.url else {
            return Ok(());
        };
        let (_, child) = self.expected_synced_urls(path, url)?;
        let Some((remote, expected)) = child else {
            return Ok(());
        };
        let child = self.worktree.join(path);
        Self::set_config_value_in(&child, &format!("remote.{remote}.url"), &expected)
    }

    fn preflight_portable_key(&self, key: &str, desired: Option<&str>) -> Result<Option<String>> {
        let portable = self.config_value("portable", key)?;
        if portable.as_deref() != desired {
            let staged = self.config_value("staged", key)?;
            anyhow::ensure!(
                staged == portable,
                ".gitmodules has an unstaged edit to managed key {key}; preserve or resolve it before reconciliation"
            );
        }
        Ok(portable)
    }

    /// Validate one existing registration and every metadata key before a batch writes.
    pub fn preflight_submodule_settings(&self, path: &str, entry: &SubmoduleEntry) -> Result<()> {
        let path = self.validated_path(Path::new(path))?;
        let name = self.registered_name_for_path(&path)?.with_context(|| {
            format!(
                "No exact submodule registration exists for {}",
                path.display()
            )
        })?;
        Self::validate_name(&name)?;
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        self.read_native_gitmodules()?;
        let destination = self.worktree.join(&path);
        if destination.join(".git").exists() {
            self.validated_child(&path)?;
            let context = crate::utilities::RepositoryContext::discover(&destination, None)?;
            for lock in [
                context.common_dir.join("config.lock"),
                context.git_dir.join("config.worktree.lock"),
            ] {
                match std::fs::symlink_metadata(&lock) {
                    Ok(_) => anyhow::bail!(
                        "Git lock exists at {}; finish the other Git operation or remove a verified stale lock",
                        lock.display()
                    ),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            for config in [
                context.common_dir.join("config"),
                context.git_dir.join("config.worktree"),
            ] {
                match std::fs::symlink_metadata(&config) {
                    Ok(metadata) if metadata.file_type().is_symlink() => anyhow::bail!(
                        "Refusing to write child Git metadata through symlink {}",
                        config.display()
                    ),
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
        } else if let Ok(mut children) = destination.read_dir() {
            anyhow::ensure!(
                children.next().is_none(),
                "Submodule destination {} contains unrelated content",
                destination.display()
            );
        }
        let prefix = format!("submodule.{name}");
        for (field, desired) in Self::managed_portable_fields(entry) {
            self.preflight_portable_key(&format!("{prefix}.{field}"), desired.as_deref())?;
        }
        Ok(())
    }

    fn add_preconditions(&self, opts: &SubmoduleAddOptions) -> Result<bool> {
        Self::validate_name(&opts.name)?;
        let path = self.validated_path(&opts.path)?;
        if let Some(existing) = self.registration_path(&opts.name)? {
            anyhow::ensure!(
                self.validated_path(Path::new(&existing))? == path,
                "Git registration {:?} already exists at {:?}; use `submod change {} --path ...` for an explicit Git-aware move",
                opts.name,
                existing,
                opts.name
            );
        }
        if opts.branch == Some(SerializableBranch::CurrentInSuperproject) {
            self.current_superproject_branch()?;
        }
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        self.preflight_gitmodules_layers()?;
        let destination = self.worktree.join(&path);
        match std::fs::symlink_metadata(&destination) {
            Ok(_) => anyhow::bail!(
                "Destination {} is occupied; add never replaces existing content",
                destination.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let retained_git_dir = self.validated_module_storage(&opts.name)?;
        match std::fs::symlink_metadata(&retained_git_dir) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                if !self.retained_repo_matches_url(&retained_git_dir, &opts.url)? {
                    anyhow::bail!(
                        "A retained submodule repository exists at {} but its origin does not match the requested URL; refusing to attach it",
                        retained_git_dir.display()
                    );
                }
                Ok(true)
            }
            Ok(_) => anyhow::bail!(
                "Refusing unsafe retained submodule repository at {}",
                retained_git_dir.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn current_superproject_branch(&self) -> Result<String> {
        let output = self.git_output(["symbolic-ref", "--quiet", "--short", "HEAD"])?;
        anyhow::ensure!(
            output.status.success(),
            "The superproject HEAD is detached; branch='.' requires a symbolic branch before any submodule mutation"
        );
        Ok(String::from_utf8(output.stdout)?.trim_end().to_string())
    }

    /// Validate an add completely without contacting the configured remote.
    pub fn preflight_add_submodule(&self, opts: &SubmoduleAddOptions) -> Result<()> {
        self.add_preconditions(opts).map(|_| ())
    }

    /// Validate the knowable removal and re-add requirements before a rebuild starts.
    /// Remote availability is deliberately not probed during preflight.
    pub fn preflight_rebuild_submodule(&self, path: &str, force: bool) -> Result<()> {
        let path = self.validated_path(Path::new(path))?;
        let name = self.registered_name_for_path(&path)?.with_context(|| {
            format!(
                "No exact registration exists for rebuild at {}",
                path.display()
            )
        })?;
        self.index_gitlink_oid(path.to_str().context("Submodule path is not valid UTF-8")?)?
            .context("Registered submodule has no stage-0 gitlink to rebuild")?;
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        self.preflight_gitmodules_layers()?;
        let destination = self.worktree.join(&path);
        if destination.join(".git").exists() {
            self.preflight_child_locks(&path, true)?;
            if !force {
                self.ensure_no_discardable_checkout_data(
                    path.to_str().context("Submodule path is not valid UTF-8")?,
                )
                .with_context(|| {
                    "Rebuild would discard checkout content. Preserve it, or rerun the same nuke command with --force only if no local work is needed"
                })?;
            }
        } else {
            let storage = self.validated_module_storage(&name)?;
            Self::preflight_gitdir_locks(&storage, true)?;
        }
        Ok(())
    }

    /// Rebuild a registered checkout without changing its logical name, gitlink, or storage.
    pub(crate) fn rebuild_submodule(
        &mut self,
        path: &str,
        mut opts: SubmoduleUpdateOptions,
        entry: &SubmoduleEntry,
        force: bool,
    ) -> Result<()> {
        // The manager preflights every selected module before the first
        // mutation. Repeating the layer check here would reject an intended
        // `.gitmodules` delta produced by an earlier completed rebuild.
        let path_buf = self.validated_materialization_target(Path::new(path))?;
        let name = self
            .registered_name_for_path(&path_buf)?
            .context("Registered submodule disappeared before rebuild")?;
        let original_config = self.snapshot_module_config(&name)?;
        // Update the selected retained child remote before deinit removes the
        // checkout. This lets the later native update fetch a newly selected
        // pin without writing `.gitmodules` early enough to block deinit.
        self.reconcile_rebuild_child_url(&path_buf, entry)?;
        let pathspec = Self::literal_pathspec(&path_buf);
        self.git([
            OsStr::new("submodule"),
            OsStr::new("absorbgitdirs"),
            OsStr::new("--"),
            pathspec.as_os_str(),
        ])?;
        // Native deinit refuses the command's own unstaged metadata delta.
        // Revalidate all tracked, untracked, ignored, and nested content at
        // the mutation boundary before allowing Git to bypass that check.
        if !force {
            self.ensure_no_discardable_checkout_data(path)?;
        }
        if let Err(operation) = self.deinit_submodule(path, true) {
            return match self.restore_module_config(&name, &original_config) {
                Ok(()) => Err(operation),
                Err(restoration) => Err(operation.context(format!(
                    "The failed deinit also failed to restore exact Git settings: {restoration:#}"
                ))),
            };
        }
        if let Err(operation) = self.sync_submodule_settings(path, entry) {
            return match self.restore_module_config(&name, &original_config) {
                Ok(()) => Err(operation),
                Err(restoration) => Err(operation.context(format!(
                    "Metadata reconciliation also failed to restore exact Git settings: {restoration:#}"
                ))),
            };
        }
        let desired_config = self.snapshot_module_config(&name)?;
        let restored_config =
            Self::preserve_unmanaged_module_config(&name, &desired_config, &original_config);
        opts.force = force;
        let operation = self
            .update_submodule(path, &opts)
            .and_then(|()| self.sync_submodule_settings(path, entry));
        let restoration = self.restore_module_config(&name, &restored_config);
        match (operation, restoration) {
            (Ok(()), Ok(())) => Ok(()),
            (Err(operation), Ok(())) => Err(operation),
            (Ok(()), Err(restoration)) => Err(restoration.context(
                "Submodule checkout was rebuilt, but its exact Git settings were not restored",
            )),
            (Err(operation), Err(restoration)) => Err(operation.context(format!(
                "The rebuild also failed to restore exact Git settings: {restoration:#}"
            ))),
        }
    }

    /// Validate reconstruction of a uniquely managed registration around an existing gitlink.
    pub fn preflight_restore_registration(
        &self,
        name: &str,
        path: &str,
        entry: &SubmoduleEntry,
    ) -> Result<()> {
        Self::validate_name(name)?;
        let path = self.validated_path(Path::new(path))?;
        anyhow::ensure!(
            self.registered_name_for_path(&path)?.is_none(),
            "An exact registration already exists for {}",
            path.display()
        );
        anyhow::ensure!(
            self.index_gitlink_oid(path.to_str().context("Submodule path is not valid UTF-8")?)?
                .is_some(),
            "No gitlink exists for {}",
            path.display()
        );
        anyhow::ensure!(entry.url.is_some(), "Managed registration has no URL");
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        self.preflight_gitmodules_layers()?;
        let existing_name = self.config_value("portable", &format!("submodule.{name}.path"))?;
        anyhow::ensure!(
            existing_name.is_none(),
            "Portable registration name {name:?} already refers to another path"
        );
        let destination = self.worktree.join(&path);
        match std::fs::symlink_metadata(&destination) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(metadata) if metadata.is_dir() && destination.read_dir()?.next().is_none() => {}
            Ok(_) => anyhow::bail!(
                "Cannot reconstruct registration while {} contains content",
                destination.display()
            ),
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    /// Recreate one portable registration without changing its existing gitlink object.
    pub fn restore_registration(
        &self,
        name: &str,
        path: &str,
        entry: &SubmoduleEntry,
    ) -> Result<()> {
        self.preflight_restore_registration(name, path, entry)?;
        let before = self
            .index_gitlink_oid(path)?
            .expect("preflight required a gitlink");
        let prefix = format!("submodule.{name}");
        self.set_config_value_exact("portable", &format!("{prefix}.path"), Some(path))?;
        for (field, desired) in Self::managed_portable_fields(entry) {
            self.set_config_value_exact(
                "portable",
                &format!("{prefix}.{field}"),
                desired.as_deref(),
            )?;
        }
        self.git(["add", "--", ".gitmodules"])?;
        self.sync_submodule_settings(path, entry)?;
        anyhow::ensure!(
            self.index_gitlink_oid(path)?.as_deref() == Some(before.as_str()),
            "Registration reconstruction changed the existing gitlink"
        );
        anyhow::ensure!(
            self.registered_name_for_path(Path::new(path))?.as_deref() == Some(name),
            "Registration reconstruction did not create the intended exact identity"
        );
        Ok(())
    }

    /// Validate controlled completion of a portable registration with no index gitlink.
    pub fn preflight_complete_registration(
        &self,
        path: &str,
        entry: &SubmoduleEntry,
    ) -> Result<()> {
        let path = self.validated_path(Path::new(path))?;
        let name = self
            .registered_name_for_path(&path)?
            .with_context(|| format!("No exact registration exists for {}", path.display()))?;
        anyhow::ensure!(
            self.index_gitlink_oid(path.to_str().context("Submodule path is not valid UTF-8")?)?
                .is_none(),
            "A gitlink already exists for {}",
            path.display()
        );
        self.preflight_submodule_settings(
            path.to_str().context("Submodule path is not valid UTF-8")?,
            entry,
        )?;
        // Completion eventually delegates to `git submodule add`, which stages
        // `.gitmodules`.  Refuse a pre-existing unstaged layer before removing
        // an otherwise reusable empty destination.
        self.preflight_gitmodules_layers()?;
        let destination = self.worktree.join(&path);
        match std::fs::symlink_metadata(&destination) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(metadata) if metadata.is_dir() && destination.read_dir()?.next().is_none() => {}
            Ok(_) => anyhow::bail!(
                "Cannot complete registration while {} contains unrelated content",
                destination.display()
            ),
            Err(error) => return Err(error.into()),
        }
        let retained = self.validated_module_storage(&name)?;
        match std::fs::symlink_metadata(&retained) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                let url = entry
                    .url
                    .as_deref()
                    .context("Managed registration has no URL")?;
                anyhow::ensure!(
                    self.retained_repo_matches_url(&retained, url)?,
                    "Retained repository for {name:?} does not match the managed URL"
                );
            }
            Ok(_) => anyhow::bail!(
                "Refusing unsafe retained submodule repository at {}",
                retained.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    /// Complete an exact existing registration using its Git section identity.
    pub fn complete_registration(&mut self, path: &str, entry: &SubmoduleEntry) -> Result<()> {
        self.preflight_complete_registration(path, entry)?;
        let path_buf = self.validated_path(Path::new(path))?;
        let name = self
            .registered_name_for_path(&path_buf)?
            .expect("completion preflight required an exact registration");
        let destination = self.worktree.join(&path_buf);
        if destination.is_dir() && destination.read_dir()?.next().is_none() {
            std::fs::remove_dir(&destination)?;
        }
        let opts = SubmoduleAddOptions {
            name,
            path: path_buf,
            url: entry
                .url
                .clone()
                .context("Managed registration has no URL")?,
            branch: entry.branch.clone(),
            ignore: entry.ignore,
            update: entry.update.clone(),
            fetch_recurse: entry.fetch_recurse,
            shallow: entry.shallow.unwrap_or(false),
            no_init: false,
        };
        self.add_submodule(&opts)?;
        self.sync_added_submodule_settings(path, entry)
    }

    /// Create a new `GitOpsManager` with automatic fallback
    pub fn new(repo_path: Option<&Path>, verbose: bool) -> Result<Self> {
        let gix_ops = GixOperations::new(repo_path).ok();
        let git2_ops = Git2Operations::new(repo_path)
            .with_context(|| "Failed to initialize git2 operations")?;
        let worktree = git2_ops
            .workdir()
            .context("Repository has no working tree")?
            .to_path_buf();
        let git_dir = crate::utilities::git_path(&worktree, &["--absolute-git-dir"])?
            .canonicalize()
            .context("Failed to resolve repository Git directory")?;

        Ok(Self {
            gix_ops,
            git2_ops,
            worktree,
            git_dir,
            verbose,
        })
    }

    /// Create a manager with git2-only fallback reads. Mutations use native Git.
    #[allow(dead_code)]
    pub fn without_gix(repo_path: Option<&Path>, verbose: bool) -> Result<Self> {
        let git2_ops = Git2Operations::new(repo_path)
            .with_context(|| "Failed to initialize git2 operations")?;
        let worktree = git2_ops
            .workdir()
            .context("Repository has no working tree")?
            .to_path_buf();
        let git_dir = crate::utilities::git_path(&worktree, &["--absolute-git-dir"])?
            .canonicalize()
            .context("Failed to resolve repository Git directory")?;

        Ok(Self {
            gix_ops: None,
            git2_ops,
            worktree,
            git_dir,
            verbose,
        })
    }

    /// Whether the optimistic gix backend is currently active. When `false`,
    /// fallback reads are served by git2. Mutations always use native Git.
    #[allow(dead_code)]
    pub const fn gix_enabled(&self) -> bool {
        self.gix_ops.is_some()
    }

    /// Return the working directory of the underlying git repository, if any.
    pub fn workdir(&self) -> Option<&std::path::Path> {
        Some(&self.worktree)
    }

    /// Require an exact registered checkout whose Git worktree is the requested path.
    pub fn verify_submodule_checkout(&self, path: &str) -> Result<()> {
        self.validated_child(Path::new(path)).map(|_| ())
    }

    /// Return the one exact portable registration for a checkout path.
    pub fn registration_name(&self, path: &str) -> Result<Option<String>> {
        let path = self.validated_path(Path::new(path))?;
        self.registered_name_for_path(&path)
    }

    /// Return the path held by one exact portable Git section name.
    pub fn registration_path(&self, name: &str) -> Result<Option<String>> {
        Self::validate_name(name)?;
        self.config_value("portable", &format!("submodule.{name}.path"))
    }

    /// Return the exact mode-160000 index object for a path, if present.
    pub fn index_gitlink_oid(&self, path: &str) -> Result<Option<String>> {
        let path = self.validated_path(Path::new(path))?;
        let output = self.git_output([
            OsStr::new("ls-files"),
            OsStr::new("--stage"),
            OsStr::new("--"),
            Self::literal_pathspec(&path).as_os_str(),
        ])?;
        anyhow::ensure!(
            output.status.success(),
            "Could not inspect index gitlink for {}",
            path.display()
        );
        if output.stdout.is_empty() {
            return Ok(None);
        }
        let text = String::from_utf8(output.stdout)?;
        let mut records = text.lines();
        let record = records
            .next()
            .context("Git returned an empty index record")?;
        anyhow::ensure!(
            records.next().is_none(),
            "Managed submodule path {} has unmerged or duplicate index stages",
            path.display()
        );
        let mut fields = record.split_whitespace();
        anyhow::ensure!(
            fields.next() == Some("160000"),
            "Index path is not a gitlink"
        );
        let oid = fields
            .next()
            .context("Git returned a gitlink without an object ID")?;
        anyhow::ensure!(
            fields.next() == Some("0"),
            "Managed submodule path {} has an unmerged index entry instead of stage 0",
            path.display()
        );
        Ok(Some(oid.to_string()))
    }

    fn remote_update_target(&self, path: &Path) -> Result<String> {
        let name = self
            .registered_name_for_path(path)?
            .context("Remote update requires an exact submodule registration")?;
        let child = self.worktree.join(path);
        let remote = Self::default_remote_in(&child)?;
        let branch = match self.config_value("portable", &format!("submodule.{name}.branch"))? {
            Some(branch) if branch == "." => self.current_superproject_branch()?,
            Some(branch) => branch,
            None => {
                let remote_head = format!("refs/remotes/{remote}/HEAD");
                let output = Command::new("git")
                    .args(["symbolic-ref", "--quiet", &remote_head])
                    .current_dir(&child)
                    .env("GIT_OPTIONAL_LOCKS", "0")
                    .output()?;
                anyhow::ensure!(
                    output.status.success(),
                    "Could not resolve the default tracking branch for remote {remote:?}"
                );
                let reference = String::from_utf8(output.stdout)?.trim_end().to_string();
                return Ok(String::from_utf8(
                    self.child_git(
                        path.to_str().context("Submodule path is not valid UTF-8")?,
                        ["rev-parse", &reference],
                    )?
                    .stdout,
                )?
                .trim_end()
                .to_string());
            }
        };
        let reference = format!("refs/remotes/{remote}/{branch}");
        Ok(String::from_utf8(
            self.child_git(
                path.to_str().context("Submodule path is not valid UTF-8")?,
                ["rev-parse", &reference],
            )?
            .stdout,
        )?
        .trim_end()
        .to_string())
    }

    fn update_postcondition_holds(
        &self,
        path: &Path,
        opts: &SubmoduleUpdateOptions,
    ) -> Result<bool> {
        if opts.strategy == SerializableUpdate::None {
            return Ok(true);
        }
        let parent_target = self
            .index_gitlink_oid(path.to_str().context("Submodule path is not valid UTF-8")?)?
            .context("Parent index has no mode-160000 target for submodule update")?;
        let destination = self.worktree.join(path);
        if !destination.join(".git").exists() {
            return Ok(false);
        }
        let target = if opts.remote {
            self.remote_update_target(path)?
        } else {
            parent_target
        };
        let path_text = path.to_str().context("Submodule path is not valid UTF-8")?;
        let head = self.child_git(path_text, ["rev-parse", "HEAD"])?;
        let head = String::from_utf8(head.stdout)?.trim_end().to_string();
        match opts.strategy {
            SerializableUpdate::Checkout | SerializableUpdate::Unspecified => Ok(head == target),
            SerializableUpdate::Merge | SerializableUpdate::Rebase => {
                let child = self.worktree.join(path);
                let output = Command::new("git")
                    .args(["merge-base", "--is-ancestor", &target, "HEAD"])
                    .current_dir(&child)
                    .env("GIT_OPTIONAL_LOCKS", "0")
                    .output()
                    .with_context(|| {
                        format!("Failed to verify update ancestry in {}", child.display())
                    })?;
                match output.status.code() {
                    Some(0) => Ok(true),
                    Some(1) => Ok(false),
                    _ => anyhow::bail!(
                        "Could not verify update ancestry in {}: {}",
                        child.display(),
                        String::from_utf8_lossy(&output.stderr).trim()
                    ),
                }
            }
            SerializableUpdate::None => Ok(true),
        }
    }

    /// Report whether an initialized checkout satisfies its configured parent-pin strategy.
    pub fn update_postcondition_matches(
        &self,
        path: &str,
        opts: &SubmoduleUpdateOptions,
    ) -> Result<bool> {
        let path = self.validated_path(Path::new(path))?;
        self.update_postcondition_holds(&path, opts)
    }

    /// Inspect tracked and untracked checkout changes without refreshing the index.
    pub fn submodule_worktree_is_clean(&self, path: &str) -> Result<bool> {
        let output = self.child_git_output(
            path,
            [
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--ignore-submodules=none",
            ],
        )?;
        anyhow::ensure!(
            output.status.success(),
            "Could not inspect working tree at {}: {}",
            self.worktree.join(path).display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
        Ok(output.stdout.is_empty())
    }

    /// Read the exact checkout HEAD after validating its repository identity.
    pub fn submodule_head(&self, path: &str) -> Result<String> {
        let output = self.child_git(path, ["rev-parse", "HEAD"])?;
        Ok(String::from_utf8(output.stdout)?.trim_end().to_string())
    }

    /// Read the recursive native submodule state without refreshing an index.
    ///
    /// The raw bytes are used only as an before/after observation for lifecycle
    /// summaries; they are never rendered or interpreted as trusted text.
    pub fn recursive_submodule_state(&self, path: &str) -> Result<Vec<u8>> {
        let path = self.validated_child(Path::new(path))?;
        let path = path.to_str().context("Submodule path is not valid UTF-8")?;
        let output = self.child_git_output(path, ["submodule", "status", "--recursive"])?;
        anyhow::ensure!(
            output.status.success(),
            "Could not inspect recursive submodule state at {}: {}",
            path,
            String::from_utf8_lossy(&output.stderr).trim()
        );
        Ok(output.stdout)
    }

    /// Resolve the target commit selected by an update after any requested
    /// remote fetch has completed.
    pub fn submodule_update_target(
        &self,
        path: &str,
        opts: &SubmoduleUpdateOptions,
    ) -> Result<String> {
        let path = self.validated_child(Path::new(path))?;
        if opts.remote {
            self.remote_update_target(&path)
        } else {
            self.index_gitlink_oid(path.to_str().context("Submodule path is not valid UTF-8")?)?
                .context("Parent index has no mode-160000 target for submodule update")
        }
    }

    /// Compare every managed portable/local/worktree setting without writing metadata.
    pub fn submodule_settings_match(&self, path: &str, entry: &SubmoduleEntry) -> Result<bool> {
        let path = self.validated_path(Path::new(path))?;
        let Some(name) = self.registered_name_for_path(&path)? else {
            return Ok(false);
        };
        let prefix = format!("submodule.{name}");
        let fields = Self::managed_portable_fields(entry);
        let worktree_config = self.worktree_config_enabled()?;
        for (field, desired) in &fields {
            let key = format!("{prefix}.{field}");
            if self.config_value("portable", &key)? != *desired {
                return Ok(false);
            }
            if *field == "url" && desired.is_some() {
                let (expected_local, expected_child) =
                    self.expected_synced_urls(&path, desired.as_deref().expect("URL is present"))?;
                if self.config_value("local", &key)?.as_deref() != Some(expected_local.as_str()) {
                    return Ok(false);
                }
                if let Some((remote, expected)) = expected_child {
                    let child = self.worktree.join(&path);
                    if Self::config_value_in(&child, &format!("remote.{remote}.url"))?.as_deref()
                        != Some(expected.as_str())
                    {
                        return Ok(false);
                    }
                }
            } else if self.config_value("local", &key)? != *desired {
                return Ok(false);
            }
            if worktree_config && self.config_value("worktree", &key)?.is_some() {
                return Ok(false);
            }
        }
        let active_key = format!("{prefix}.active");
        let active = entry.active.map(|value| value.to_string());
        if self.config_value("local", &active_key)? != active {
            return Ok(false);
        }
        if worktree_config && self.config_value("worktree", &active_key)?.is_some() {
            return Ok(false);
        }
        Ok(true)
    }

    /// Validate a known parent-pin transition before any module in a batch mutates.
    pub fn preflight_update_submodule(
        &self,
        path: &str,
        opts: &SubmoduleUpdateOptions,
    ) -> Result<()> {
        if opts.strategy == SerializableUpdate::None {
            return Ok(());
        }
        let path = self.validated_materialization_target(Path::new(path))?;
        self.read_native_gitmodules()?;
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        if !opts.remote && !opts.recursive && self.update_postcondition_holds(&path, opts)? {
            return Ok(());
        }
        let destination = self.worktree.join(&path);
        if destination.join(".git").exists()
            && !opts.force
            && !self.is_incomplete_materialization(&path)?
        {
            self.ensure_clean_checkout(
                path.to_str().context("Submodule path is not valid UTF-8")?,
            )?;
        }
        Ok(())
    }

    /// Inspect the exact native sparse-checkout mode and ordered pattern sequence.
    pub fn sparse_checkout_state(&self, path: &str) -> Result<(bool, bool, Vec<String>)> {
        let path = self.validated_child(Path::new(path))?;
        let child = self.worktree.join(&path);
        let enabled = Self::config_value_in(&child, "core.sparseCheckout")?
            .is_some_and(|value| value.eq_ignore_ascii_case("true"));
        let cone = Self::config_value_in(&child, "core.sparseCheckoutCone")?
            .is_some_and(|value| value.eq_ignore_ascii_case("true"));
        let output = self.child_git_output_at_validated(
            &path,
            [
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "info/sparse-checkout",
            ],
        )?;
        anyhow::ensure!(
            output.status.success(),
            "Could not locate sparse-checkout patterns in {}: {}",
            child.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        );
        let sparse_file = crate::utilities::git_path_from_stdout(output.stdout)?;
        let patterns = match std::fs::read_to_string(sparse_file) {
            Ok(content) => content.lines().map(str::to_string).collect(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error.into()),
        };
        Ok((enabled, cone, patterns))
    }

    fn sparse_checkout_matches(&self, path: &str, expected: &[String]) -> Result<bool> {
        let (enabled, cone, actual) = self.sparse_checkout_state(path)?;
        if expected.is_empty() {
            Ok(!enabled)
        } else {
            Ok(enabled && !cone && actual == expected)
        }
    }

    /// Validate a sparse policy change before any module in the command mutates.
    pub fn preflight_sparse_checkout(&self, path: &str, expected: &[String]) -> Result<()> {
        for pattern in expected {
            anyhow::ensure!(
                !pattern.contains(['\0', '\n', '\r']),
                "Sparse pattern contains an invalid record boundary"
            );
        }
        if self.sparse_checkout_matches(path, expected)? {
            return Ok(());
        }
        self.ensure_clean_checkout(path)?;
        let child = self.worktree.join(self.validated_child(Path::new(path))?);
        let context = crate::utilities::RepositoryContext::discover(&child, None)?;
        for lock in [
            context.git_dir.join("index.lock"),
            context.common_dir.join("config.lock"),
            context.git_dir.join("config.worktree.lock"),
        ] {
            match std::fs::symlink_metadata(&lock) {
                Ok(_) => anyhow::bail!(
                    "Git lock exists at {}; sparse checkout was not changed",
                    lock.display()
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        let sparse_file = crate::utilities::git_path(
            &child,
            &[
                "--path-format=absolute",
                "--git-path",
                "info/sparse-checkout",
            ],
        )?;
        match std::fs::symlink_metadata(&sparse_file) {
            Ok(metadata) if metadata.file_type().is_symlink() => anyhow::bail!(
                "Refusing to write sparse policy through symlink {}",
                sparse_file.display()
            ),
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Ok(())
    }

    /// Apply native non-cone sparse policy, or disable it for a full checkout.
    pub fn reconcile_sparse_checkout(&self, path: &str, expected: &[String]) -> Result<bool> {
        self.preflight_sparse_checkout(path, expected)?;
        if self.sparse_checkout_matches(path, expected)? {
            return Ok(false);
        }
        if expected.is_empty() {
            self.child_git(path, ["sparse-checkout", "disable"])?;
        } else {
            self.set_sparse_patterns(path, expected)?;
        }
        anyhow::ensure!(
            self.sparse_checkout_matches(path, expected)?,
            "Git sparse-checkout state did not reach the requested policy"
        );
        Ok(true)
    }

    /// Reconcile the managed policy fields for one exact registration.
    pub fn sync_submodule_settings(&self, path: &str, entry: &SubmoduleEntry) -> Result<()> {
        self.preflight_submodule_settings(path, entry)?;
        let path = self.validated_path(Path::new(path))?;
        let name = self.registered_name_for_path(&path)?.with_context(|| {
            format!(
                "No exact submodule registration exists for {}",
                path.display()
            )
        })?;
        let prefix = format!("submodule.{name}");
        let fields = Self::managed_portable_fields(entry);
        let worktree_config = self.worktree_config_enabled()?;
        let mut url_requires_sync = false;
        for (field, desired) in &fields {
            let key = format!("{prefix}.{field}");
            if self.preflight_portable_key(&key, desired.as_deref())? != *desired {
                self.set_config_value_exact("portable", &key, desired.as_deref())?;
                url_requires_sync |= *field == "url";
            }
            if *field != "url" && self.config_value("local", &key)? != *desired {
                self.set_config_value_exact("local", &key, desired.as_deref())?;
            }
            if worktree_config && self.config_value("worktree", &key)?.is_some() {
                self.set_config_value_exact("worktree", &key, None)?;
                url_requires_sync |= *field == "url";
            }
        }

        let url_key = format!("{prefix}.url");
        if let Some(url) = &entry.url {
            let (expected_local_url, expected_child_url) = self.expected_synced_urls(&path, url)?;
            let local_url = self.config_value("local", &url_key)?;
            url_requires_sync |= local_url.as_deref() != Some(expected_local_url.as_str());
            if let Some((remote, expected)) = &expected_child_url {
                let child = self.worktree.join(&path);
                url_requires_sync |=
                    Self::config_value_in(&child, &format!("remote.{remote}.url"))?.as_deref()
                        != Some(expected.as_str());
            }
            if url_requires_sync {
                if self
                    .index_gitlink_oid(path.to_str().context("Submodule path is not valid UTF-8")?)?
                    .is_none()
                {
                    // Native `submodule sync -- <path>` ignores a portable
                    // registration that has no stage-0 gitlink.  This state is
                    // valid for disabled declarations, so reconcile the parent
                    // cache directly while retaining the portable declaration.
                    self.set_config_value_exact(
                        "local",
                        &url_key,
                        Some(expected_local_url.as_str()),
                    )?;
                } else {
                    let active_override = format!("{prefix}.active=true");
                    self.git([
                        OsStr::new("-c"),
                        OsStr::new(&active_override),
                        OsStr::new("submodule"),
                        OsStr::new("sync"),
                        OsStr::new("--"),
                        Self::literal_pathspec(&path).as_os_str(),
                    ])?;
                }
            }
        } else if self.config_value("local", &url_key)?.is_some() {
            self.set_config_value_exact("local", &url_key, None)?;
        }

        let active_key = format!("{prefix}.active");
        let active = entry.active.map(|value| value.to_string());
        if self.config_value("local", &active_key)? != active {
            self.set_config_value_exact("local", &active_key, active.as_deref())?;
        }
        if worktree_config && self.config_value("worktree", &active_key)?.is_some() {
            self.set_config_value_exact("worktree", &active_key, None)?;
        }

        for (field, desired) in &fields {
            let key = format!("{prefix}.{field}");
            anyhow::ensure!(
                self.config_value("portable", &key)? == *desired,
                "Git did not retain intended .gitmodules value for {key}"
            );
            if *field == "url" && desired.is_some() {
                let (expected_local, expected_child) =
                    self.expected_synced_urls(&path, desired.as_deref().expect("URL is present"))?;
                let resolved = self.config_value("local", &key)?;
                anyhow::ensure!(
                    resolved.as_deref() == Some(expected_local.as_str()),
                    "Git did not resolve local URL for {key}"
                );
                if let Some((remote, expected)) = expected_child {
                    let child = self.worktree.join(&path);
                    let actual = Self::config_value_in(&child, &format!("remote.{remote}.url"))?;
                    anyhow::ensure!(
                        actual.as_deref() == Some(expected.as_str()),
                        "Git did not synchronize the selected child URL for {key}: expected remote.{remote}.url {expected:?}, found {actual:?}"
                    );
                }
            } else {
                anyhow::ensure!(
                    self.config_value("local", &key)? == *desired,
                    "Git did not retain intended local value for {key}"
                );
            }
            if worktree_config {
                anyhow::ensure!(
                    self.config_value("worktree", &key)?.is_none(),
                    "Git did not remove worktree override for {key}"
                );
            }
        }
        anyhow::ensure!(
            self.config_value("local", &active_key)? == active,
            "Git did not retain intended activation for {active_key}"
        );
        Ok(())
    }

    /// Finish metadata reconciliation as part of a structural registration.
    /// Registration already owns staging `.gitmodules`, so stage only that
    /// exact file after applying effective managed settings.
    pub(crate) fn sync_added_submodule_settings(
        &self,
        path: &str,
        entry: &SubmoduleEntry,
    ) -> Result<()> {
        self.sync_submodule_settings(path, entry)?;
        self.git(["add", "--", ".gitmodules"])?;
        let unstaged = self.git_output(["diff", "--quiet", "--", ".gitmodules"])?;
        anyhow::ensure!(
            unstaged.status.success(),
            "Git registration left an unstaged .gitmodules delta"
        );
        Ok(())
    }

    /// Validate an initialized submodule move without changing checkout or metadata.
    pub fn preflight_move_submodule(&self, old_path: &str, new_path: &str) -> Result<()> {
        let old = self.validated_path(Path::new(old_path))?;
        let new = self.validated_path(Path::new(new_path))?;
        let old_str = old.to_str().context("Submodule path is not valid UTF-8")?;
        self.validated_child(&old)?;
        self.ensure_clean_checkout(old_str)?;
        if std::fs::symlink_metadata(self.worktree.join(&new)).is_ok() {
            anyhow::bail!(
                "Destination {} is occupied; move never replaces existing content",
                self.worktree.join(&new).display()
            );
        }
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        self.preflight_gitmodules_layers()?;
        Ok(())
    }

    /// Move an initialized submodule through native Git and verify both registration and index.
    pub fn move_submodule(&mut self, old_path: &str, new_path: &str) -> Result<()> {
        self.preflight_move_submodule(old_path, new_path)?;
        let old = self.validated_path(Path::new(old_path))?;
        let new = self.validated_path(Path::new(new_path))?;
        self.git([
            OsStr::new("mv"),
            OsStr::new("--"),
            old.as_os_str(),
            new.as_os_str(),
        ])?;
        if self.registered_name_for_path(&old)?.is_some()
            || self.registered_name_for_path(&new)?.is_none()
        {
            anyhow::bail!("Git moved the checkout but did not update its exact registration");
        }
        let new_pathspec = Self::literal_pathspec(&new);
        let index = self.git([
            OsStr::new("ls-files"),
            OsStr::new("--stage"),
            OsStr::new("--"),
            new_pathspec.as_os_str(),
        ])?;
        if !String::from_utf8_lossy(&index.stdout).starts_with("160000 ") {
            anyhow::bail!("Git moved the checkout but did not create the new gitlink");
        }
        Ok(())
    }

    /// Reopen the repository from the working directory to refresh any cached state.
    /// This is needed after destructive operations (e.g., submodule delete) so that the
    /// in-memory git2 repository object reflects the updated on-disk state.
    ///
    /// Returns an error if the git2 repository (the required backend) cannot be reopened.
    /// A gix reopen failure is non-fatal since gix is an optional optimistic backend.
    /// A manager constructed without gix (e.g. via [`GitOpsManager::without_gix`])
    /// stays without it: reopen preserves the backend policy, it never upgrades one.
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

        // gix is an optional optimistic backend — log failures but don't fail,
        // and never enable it on a manager that was constructed without it.
        if self.gix_ops.is_some() {
            match GixOperations::new(Some(&workdir)) {
                Ok(new_gix) => {
                    self.gix_ops = Some(new_gix);
                }
                Err(e) => {
                    if self.verbose {
                        eprintln!(
                            "Warning: failed to reopen gix repository at {}: {}",
                            crate::utilities::safe_human_text(&workdir.to_string_lossy()),
                            crate::utilities::safe_human_text(&e.to_string())
                        );
                    }
                }
            }
        }

        Ok(())
    }

    fn git_output<I, S>(&self, args: I) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        Command::new("git")
            .args(["-c", "diff.autoRefreshIndex=false"])
            .args(args)
            .current_dir(&self.worktree)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .context("Failed to execute Git")
    }

    fn git<I, S>(&self, args: I) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let output = self.git_output(args)?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(anyhow::anyhow!(
                "Git exited {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    fn git_path(&self, path: &str) -> Result<PathBuf> {
        crate::utilities::git_path(
            &self.worktree,
            &["--path-format=absolute", "--git-path", path],
        )
    }

    fn literal_pathspec(path: &Path) -> OsString {
        let mut pathspec = OsString::from(":(literal)");
        pathspec.push(path.as_os_str());
        pathspec
    }

    fn validated_module_storage(&self, name: &str) -> Result<PathBuf> {
        Self::validate_name(name)?;
        let relative = Path::new("modules").join(name);
        crate::utilities::validate_submodule_path(&self.git_dir, &relative)
            .context("Unsafe submodule storage path")?;
        let expected = self.git_dir.join(&relative);
        let reported = self.git_path(
            relative
                .to_str()
                .context("Submodule storage name is not valid UTF-8")?,
        )?;
        // `git --git-path` resolves storage through the filesystem, so its
        // spelling can differ from the joined path (8.3 short names and
        // separator style on Windows) while naming the same directory.
        // Compare canonical forms so the containment check is not defeated by
        // spelling; both spellings are reported when they still disagree.
        let canonical_reported = Self::canonical_storage_form(&reported)?;
        let canonical_expected = Self::canonical_storage_form(&expected)?;
        if canonical_reported != canonical_expected {
            anyhow::bail!(
                "Git resolved submodule storage outside the expected Git directory: reported {}, expected {}",
                reported.display(),
                expected.display()
            );
        }
        Ok(reported)
    }

    /// Canonicalize a repository top-level reported by Git for identity
    /// comparison against an already-canonical checkout root.
    ///
    /// Git spells the same directory differently per platform (forward
    /// slashes on Windows, unresolved symlinks on macOS), while
    /// `canonicalize` uses verbatim UNC paths there, so comparing the raw
    /// report rejects every checkout. A missing report is fail-closed: the
    /// top-level Git just named must exist.
    fn canonical_worktree_root(reported: &Path, intended: &Path) -> Result<PathBuf> {
        reported.canonicalize().with_context(|| {
            format!(
                "Submodule path {} resolves to unrelated worktree {}",
                intended.display(),
                reported.display()
            )
        })
    }

    /// Canonicalize a submodule storage path for containment comparison,
    /// resolving through the nearest existing ancestor when the path itself
    /// does not exist yet (preflight runs before Git creates storage).
    fn canonical_storage_form(path: &Path) -> Result<PathBuf> {
        match std::fs::canonicalize(path) {
            Ok(canonical) => Ok(canonical),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let mut missing: Vec<OsString> = Vec::new();
                let mut current = path;
                loop {
                    match std::fs::canonicalize(current) {
                        Ok(base) => {
                            let mut canonical = base;
                            for component in missing.iter().rev() {
                                canonical.push(component);
                            }
                            return Ok(canonical);
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                            match (current.file_name(), current.parent()) {
                                (Some(name), Some(parent)) => {
                                    missing.push(name.to_os_string());
                                    current = parent;
                                }
                                _ => return Err(error.into()),
                            }
                        }
                        Err(error) => return Err(error.into()),
                    }
                }
            }
            Err(error) => Err(error.into()),
        }
    }

    fn retained_repo_matches_url(&self, git_dir: &Path, url: &str) -> Result<bool> {
        let config = git_dir.join("config");
        let metadata = match std::fs::symlink_metadata(&config) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => return Err(error.into()),
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            anyhow::bail!(
                "Refusing retained submodule repository with unsafe config at {}",
                config.display()
            );
        }

        let output = Command::new("git")
            .current_dir(&self.worktree)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .args(["config", "--no-includes", "--file"])
            .arg(&config)
            .args(["--get", "remote.origin.url"])
            .output()
            .context("Failed to inspect retained submodule repository")?;
        if !output.status.success() {
            return match output.status.code() {
                Some(1) => Ok(false),
                _ => anyhow::bail!(
                    "Could not inspect retained submodule origin: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ),
            };
        }
        let configured = output.stdout.strip_suffix(b"\n").unwrap_or(&output.stdout);
        Ok(configured == url.as_bytes())
    }

    fn validated_path(&self, path: &Path) -> Result<PathBuf> {
        let path = crate::utilities::normalize_submodule_path(path)?;
        crate::utilities::validate_submodule_path(&self.worktree, &path)?;
        Ok(path)
    }

    fn validate_name(name: &str) -> Result<()> {
        let normalized = crate::utilities::normalize_submodule_path(Path::new(name));
        if name.contains(['\0', '\n', '\r', '\\'])
            || normalized
                .as_ref()
                .map_or(true, |normalized| normalized != Path::new(name))
        {
            anyhow::bail!("Invalid submodule administrative name: {name:?}");
        }
        Ok(())
    }

    fn config_key_regex_literal(value: &str) -> String {
        let mut escaped = String::with_capacity(value.len());
        for character in value.chars() {
            if matches!(
                character,
                '\\' | '.' | '^' | '$' | '|' | '?' | '*' | '+' | '(' | ')' | '[' | ']' | '{' | '}'
            ) {
                escaped.push('\\');
            }
            escaped.push(character);
        }
        escaped
    }

    fn registered_name_for_path(&self, path: &Path) -> Result<Option<String>> {
        if !self.worktree.join(".gitmodules").is_file() {
            return Ok(None);
        }
        let output = self.git_output([
            OsStr::new("config"),
            OsStr::new("-z"),
            OsStr::new("--file"),
            OsStr::new(".gitmodules"),
            OsStr::new("--get-regexp"),
            OsStr::new(r"^submodule\..*\.path$"),
        ])?;
        if !output.status.success() {
            return match output.status.code() {
                Some(1) => Ok(None),
                _ => Err(anyhow::anyhow!(
                    "Could not read .gitmodules registrations: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                )),
            };
        }
        let requested = path.as_os_str();
        let mut found = Vec::new();
        for record in output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|r| !r.is_empty())
        {
            let Some(newline) = record.iter().position(|byte| *byte == b'\n') else {
                anyhow::bail!("Git returned a malformed .gitmodules registration");
            };
            let key = std::str::from_utf8(&record[..newline])?;
            let value = &record[newline + 1..];
            if Self::gitmodules_path_matches(value, requested) {
                let name = key
                    .strip_prefix("submodule.")
                    .and_then(|key| key.strip_suffix(".path"))
                    .context("Git returned an invalid submodule path key")?;
                found.push(name.to_string());
            }
        }
        match found.as_slice() {
            [] => Ok(None),
            [name] => Ok(Some(name.clone())),
            _ => anyhow::bail!(
                "Multiple .gitmodules registrations use path {}",
                path.display()
            ),
        }
    }

    fn read_native_gitmodules(&self) -> Result<SubmoduleEntries> {
        if !self.worktree.join(".gitmodules").is_file() {
            return Ok(SubmoduleEntries::default());
        }
        let output = self.git(["config", "-z", "--file", ".gitmodules", "--list"])?;
        let mut entries: HashMap<String, HashMap<String, String>> = HashMap::new();
        for record in output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|r| !r.is_empty())
        {
            let newline = record
                .iter()
                .position(|byte| *byte == b'\n')
                .context("Git returned malformed .gitmodules data")?;
            let key = std::str::from_utf8(&record[..newline])?;
            let value = std::str::from_utf8(&record[newline + 1..])?;
            let Some(key) = key.strip_prefix("submodule.") else {
                continue;
            };
            let Some((name, field)) = key.rsplit_once('.') else {
                continue;
            };
            Self::validate_name(name)?;
            let field = match field.to_ascii_lowercase().as_str() {
                "path" => "path",
                "url" => "url",
                "branch" => "branch",
                "ignore" => "ignore",
                "update" => {
                    if value.starts_with('!') {
                        anyhow::bail!(
                            "Custom submodule update commands are not supported for {name:?}"
                        );
                    }
                    "update"
                }
                "fetchrecursesubmodules" => "fetchRecurseSubmodules",
                "shallow" => "shallow",
                _ => continue,
            };
            entries
                .entry(name.to_string())
                .or_default()
                .insert(field.to_string(), value.to_string());
        }
        Ok(SubmoduleEntries::from_gitmodules(entries))
    }

    fn validated_child(&self, path: &Path) -> Result<PathBuf> {
        let path = self.validated_path(path)?;
        let registered_name = self.registered_name_for_path(&path)?.with_context(|| {
            format!(
                "No exact .gitmodules registration exists for {}",
                path.display()
            )
        })?;
        self.validated_module_storage(&registered_name)?;
        let intended = self.worktree.join(&path);
        let admin = intended.join(".git");
        let mut declared_git_dir = None;
        if let Ok(metadata) = std::fs::symlink_metadata(&admin)
            && metadata.is_file()
        {
            let bytes = std::fs::read(&admin)
                .with_context(|| format!("Could not read submodule gitfile {}", admin.display()))?;
            let line = bytes.strip_suffix(b"\n").unwrap_or(&bytes);
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            let raw_target = line
                .strip_prefix(b"gitdir: ")
                .with_context(|| format!("Malformed submodule gitfile at {}", admin.display()))?;
            #[cfg(unix)]
            let target = {
                use std::os::unix::ffi::OsStringExt;
                PathBuf::from(OsString::from_vec(raw_target.to_vec()))
            };
            #[cfg(not(unix))]
            let target =
                PathBuf::from(String::from_utf8(raw_target.to_vec()).with_context(|| {
                    format!(
                        "Submodule gitfile target at {} is not valid Unicode",
                        admin.display()
                    )
                })?);
            let target = if target.is_absolute() {
                target
            } else {
                intended.join(target)
            };
            match std::fs::symlink_metadata(&target) {
                Ok(_) if target.is_dir() => {
                    declared_git_dir = Some(target);
                }
                Ok(_) => anyhow::bail!(
                    "Submodule gitfile {} does not point to a Git directory",
                    admin.display()
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => anyhow::bail!(
                    "Submodule gitfile {} points to missing Git directory {}",
                    admin.display(),
                    target.display()
                ),
                Err(error) => return Err(error.into()),
            }
        }
        let intended_root = intended
            .canonicalize()
            .with_context(|| format!("Submodule checkout is missing: {}", intended.display()))?;
        let worktree_root = crate::utilities::git_path(&intended_root, &["--show-toplevel"])
            .with_context(|| format!("Invalid submodule repository at {}", intended.display()))?;
        if Self::canonical_worktree_root(&worktree_root, &intended)? != intended_root {
            anyhow::bail!(
                "Submodule path {} resolves to unrelated worktree {}",
                intended.display(),
                worktree_root.display()
            );
        }
        let metadata = std::fs::symlink_metadata(intended_root.join(".git"))?;
        let git_dir_is_valid = if metadata.is_file() {
            declared_git_dir.is_some_and(|path| path.is_dir())
        } else {
            metadata.is_dir()
        };
        if metadata.file_type().is_symlink() || !git_dir_is_valid {
            anyhow::bail!(
                "Invalid submodule Git administrative identity at {}",
                intended.display()
            );
        }
        Ok(path)
    }

    fn has_only_git_admin(&self, path: &Path) -> Result<bool> {
        let path = self.validated_child(path)?;
        let child = self.worktree.join(&path);
        for entry in child.read_dir()? {
            if entry?.file_name() != OsStr::new(".git") {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// A failed native initialization can leave a gitfile, no child index, and
    /// staged deletions for the remote tip. No user worktree content exists.
    fn is_incomplete_materialization(&self, path: &Path) -> Result<bool> {
        if !self.has_only_git_admin(path)? {
            return Ok(false);
        }
        let path = self.validated_child(path)?;
        let status = self.child_git(
            path.to_str().context("Submodule path is not valid UTF-8")?,
            [
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
                "--ignored",
                "--ignore-submodules=none",
            ],
        )?;
        let index = self.child_git(
            path.to_str().context("Submodule path is not valid UTF-8")?,
            ["ls-files", "--stage"],
        )?;
        Ok(!status.stdout.is_empty()
            && index.stdout.is_empty()
            && String::from_utf8_lossy(&status.stdout)
                .lines()
                .all(|line| line.starts_with("D  ")))
    }

    /// Validate an exact registration and any checkout already occupying its path.
    /// A missing or empty directory is a valid materialization target; anything
    /// populated must be the intended Git worktree before native Git can touch it.
    fn validated_materialization_target(&self, path: &Path) -> Result<PathBuf> {
        let path = self.validated_path(path)?;
        let registered_name = self.registered_name_for_path(&path)?.with_context(|| {
            format!(
                "No exact .gitmodules registration exists for {}",
                path.display()
            )
        })?;
        self.validated_module_storage(&registered_name)?;
        let target = self.worktree.join(&path);
        match std::fs::symlink_metadata(&target) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
            Err(error) => Err(error.into()),
            Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
                anyhow::bail!(
                    "Submodule materialization target {} is occupied",
                    target.display()
                )
            }
            Ok(_) => match std::fs::symlink_metadata(target.join(".git")) {
                Ok(_) => self.validated_child(&path),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    if target.read_dir()?.next().is_none() {
                        Ok(path)
                    } else {
                        anyhow::bail!(
                            "Submodule materialization target {} contains unrelated data",
                            target.display()
                        )
                    }
                }
                Err(error) => Err(error.into()),
            },
        }
    }

    fn preflight_native_locks(&self) -> Result<()> {
        let mut locks = vec![
            self.git_path("index.lock")?,
            self.git_path("config.lock")?,
            self.git_path("config.worktree.lock")?,
        ];
        locks.push(self.worktree.join(".gitmodules.lock"));
        for lock in locks {
            match std::fs::symlink_metadata(&lock) {
                Ok(_) => anyhow::bail!(
                    "Git lock exists at {}; finish the other Git operation or remove a verified stale lock",
                    lock.display()
                ),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    fn preflight_native_metadata(&self) -> Result<()> {
        for path in [
            self.worktree.join(".gitmodules"),
            self.git_path("config")?,
            self.git_path("config.worktree")?,
        ] {
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) if metadata.file_type().is_symlink() => anyhow::bail!(
                    "Refusing to write Git metadata through symlink {}",
                    path.display()
                ),
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    fn preflight_gitmodules_layers(&self) -> Result<()> {
        if !self.worktree.join(".gitmodules").exists() {
            return Ok(());
        }
        let output = self.git_output(["diff", "--quiet", "--", ".gitmodules"])?;
        match output.status.code() {
            Some(0) => Ok(()),
            Some(1) => anyhow::bail!(
                ".gitmodules has unstaged edits; stage or stash them before changing submodule registration"
            ),
            _ => Err(anyhow::anyhow!(
                "Could not inspect .gitmodules layers: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )),
        }
    }

    fn child_git_output<I, S>(&self, path: &str, args: I) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let path = self.validated_child(Path::new(path))?;
        self.child_git_output_at_validated(&path, args)
    }

    /// Run a child Git command after the caller has validated the exact checkout.
    fn child_git_output_at_validated<I, S>(&self, path: &Path, args: I) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let child = self.worktree.join(path);
        Command::new("git")
            .args(["-c", "diff.autoRefreshIndex=false"])
            .args(args)
            .current_dir(&child)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .output()
            .with_context(|| format!("Failed to execute Git in {}", child.display()))
    }

    fn child_git<I, S>(&self, path: &str, args: I) -> Result<Output>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let path = self.validated_path(Path::new(path))?;
        let child = self.worktree.join(&path);
        let output = self.child_git_output(
            path.to_str().context("Submodule path is not valid UTF-8")?,
            args,
        )?;
        if output.status.success() {
            Ok(output)
        } else {
            Err(anyhow::anyhow!(
                "Git in {} exited {}: {}",
                child.display(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }

    fn child_ref_oid(&self, path: &str, reference: &str) -> Result<Option<String>> {
        let output =
            self.child_git_output(path, ["rev-parse", "--verify", "--quiet", reference])?;
        match output.status.code() {
            Some(0) => Ok(Some(
                String::from_utf8(output.stdout)?.trim_end().to_string(),
            )),
            Some(1) => Ok(None),
            _ => anyhow::bail!(
                "Could not inspect {reference} in submodule {path}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        }
    }

    /// Build a shell-safe, in-directory recovery command for an exact stash.
    ///
    /// `git stash branch` checks out the stash's original base before applying
    /// its index and worktree state, so recovery remains correct after reset
    /// moved the checkout to a different parent pin.
    pub fn stash_recovery_command(&self, path: &str, oid: &str) -> Result<String> {
        anyhow::ensure!(
            !oid.is_empty() && oid.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "Invalid stash object ID returned by Git"
        );
        self.child_git(path, ["cat-file", "-e", &format!("{oid}^{{commit}}")])?;
        let short = &oid[..oid.len().min(12)];
        for suffix in 0..1000 {
            let branch = if suffix == 0 {
                format!("submod-recovery-{short}")
            } else {
                format!("submod-recovery-{short}-{suffix}")
            };
            if self
                .child_ref_oid(path, &format!("refs/heads/{branch}"))?
                .is_none()
            {
                return Ok(format!("git stash branch {branch} {oid}"));
            }
        }
        anyhow::bail!("Could not choose an unused recovery branch for stash {oid}")
    }

    fn ensure_clean_checkout(&self, path: &str) -> Result<()> {
        let path = self.validated_path(Path::new(path))?;
        let child = self.worktree.join(&path);
        if !child.join(".git").exists() {
            if std::fs::symlink_metadata(&child).is_ok() && child.read_dir()?.next().is_some() {
                anyhow::bail!(
                    "Submodule path {} contains data but is not an initialized checkout",
                    child.display()
                );
            }
            return Ok(());
        }
        let output = self.child_git(
            path.to_str().context("Submodule path is not valid UTF-8")?,
            [
                "status",
                "--porcelain=v1",
                "--untracked-files=all",
                "--ignored",
                "--ignore-submodules=none",
            ],
        )?;
        if !output.stdout.is_empty() {
            anyhow::bail!(
                "Submodule checkout at {} is dirty or contains local changes (tracked, untracked, or ignored); preserve it before removal",
                child.display()
            );
        }
        Ok(())
    }

    fn ensure_no_discardable_checkout_data(&self, path: &str) -> Result<()> {
        self.ensure_clean_checkout(path)?;
        let path = self.validated_path(Path::new(path))?;
        let child = self.worktree.join(&path);
        if !child.join(".git").exists() {
            return Ok(());
        }
        let nested = self.child_git(
            path.to_str().context("Submodule path is not valid UTF-8")?,
            ["ls-files", "--stage", "-z"],
        )?;
        if nested
            .stdout
            .split(|byte| *byte == 0)
            .any(|record| record.starts_with(b"160000 "))
        {
            anyhow::bail!(
                "Submodule checkout at {} contains nested registrations whose ignored data cannot be proven safe; inspect them and use --force only if their worktree data may be discarded",
                child.display()
            );
        }
        Ok(())
    }

    fn ensure_lock_absent(lock: &Path) -> Result<()> {
        match std::fs::symlink_metadata(lock) {
            Ok(_) => anyhow::bail!(
                "Git lock exists at {}; finish the other Git operation or remove a verified stale lock",
                lock.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }

    fn preflight_gitdir_locks(git_dir: &Path, include_stash: bool) -> Result<()> {
        let mut locks = vec![
            git_dir.join("index.lock"),
            git_dir.join("config.lock"),
            git_dir.join("config.worktree.lock"),
        ];
        if include_stash {
            locks.push(git_dir.join("refs/stash.lock"));
        }
        for lock in locks {
            Self::ensure_lock_absent(&lock)?;
        }
        Ok(())
    }

    fn preflight_child_locks(&self, path: &Path, include_stash: bool) -> Result<()> {
        let path = self.validated_child(path)?;
        let child = self.worktree.join(&path);
        let mut names = vec!["index.lock", "config.lock", "config.worktree.lock"];
        if include_stash {
            names.push("refs/stash.lock");
        }
        for name in names {
            let lock = crate::utilities::git_path(
                &child,
                &["--path-format=absolute", "--git-path", name],
            )?;
            Self::ensure_lock_absent(&lock)?;
        }
        Ok(())
    }

    /// Validate exact native removal without changing registration, checkout, or config.
    pub fn preflight_delete_submodule(&self, path: &str, force: bool) -> Result<()> {
        let path = self.validated_materialization_target(Path::new(path))?;
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        self.preflight_gitmodules_layers()?;
        self.index_gitlink_oid(path.to_str().context("Submodule path is not valid UTF-8")?)?
            .context("Registered submodule has no stage-0 gitlink to delete")?;
        if self.worktree.join(&path).join(".git").exists() {
            self.preflight_child_locks(&path, true)?;
            if !force {
                self.ensure_no_discardable_checkout_data(
                    path.to_str().context("Submodule path is not valid UTF-8")?,
                )?;
            }
        }
        Ok(())
    }

    fn reset_target(&self, path: &Path) -> Result<String> {
        self.index_gitlink_oid(path.to_str().context("Submodule path is not valid UTF-8")?)?
            .with_context(|| {
                format!(
                    "Parent index has no stage-0 mode-160000 target for {}",
                    path.display()
                )
            })
    }

    fn preflight_reset_preservation(&self, path: &str) -> Result<()> {
        let path = self.validated_child(Path::new(path))?;
        let child = self.worktree.join(&path);
        let stash_lock = crate::utilities::git_path(
            &child,
            &["--path-format=absolute", "--git-path", "refs/stash.lock"],
        )?;
        match std::fs::symlink_metadata(&stash_lock) {
            Ok(_) => anyhow::bail!(
                "Git stash lock exists at {}; finish the other Git operation or remove a verified stale lock",
                stash_lock.display()
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }

        let target = self.reset_target(&path)?;
        let target_tree = self.child_git(
            path.to_str().context("Submodule path is not valid UTF-8")?,
            ["ls-tree", "-r", "-z", "--name-only", &target],
        )?;
        let tracked: Vec<&[u8]> = target_tree
            .stdout
            .split(|byte| *byte == 0)
            .filter(|record| !record.is_empty())
            .collect();
        let status = self.child_git(
            path.to_str().context("Submodule path is not valid UTF-8")?,
            [
                "status",
                "--porcelain=v1",
                "-z",
                "--untracked-files=all",
                "--ignored",
            ],
        )?;
        for record in status.stdout.split(|byte| *byte == 0) {
            if record.len() < 4 || (&record[..3] != b"!! " && &record[..3] != b"?? ") {
                continue;
            }
            let local = record[3..]
                .strip_suffix(b"/")
                .unwrap_or_else(|| &record[3..]);
            let is_ignored = &record[..3] == b"!! ";
            let is_nested_repo = if is_ignored {
                true
            } else {
                #[cfg(unix)]
                let relative = {
                    use std::os::unix::ffi::OsStringExt;
                    PathBuf::from(OsString::from_vec(local.to_vec()))
                };
                #[cfg(not(unix))]
                let relative = PathBuf::from(String::from_utf8(local.to_vec())?);
                child.join(relative).join(".git").exists()
            };
            if !is_ignored && !is_nested_repo {
                continue;
            }
            let collides = tracked.iter().any(|target_path| {
                *target_path == local
                    || target_path
                        .strip_prefix(local)
                        .is_some_and(|suffix| suffix.starts_with(b"/"))
                    || local
                        .strip_prefix(*target_path)
                        .is_some_and(|suffix| suffix.starts_with(b"/"))
            });
            if collides {
                anyhow::bail!(
                    "Reset target {target} would overwrite ignored content or a nested repository at {}",
                    String::from_utf8_lossy(local)
                );
            }
        }
        Ok(())
    }

    /// Validate a reset target and every preservation precondition without mutation.
    pub fn preflight_reset_submodule(&self, path: &str) -> Result<String> {
        self.read_native_gitmodules()?;
        let path = self.validated_child(Path::new(path))?;
        let path_text = path.to_str().context("Submodule path is not valid UTF-8")?;
        self.preflight_native_locks()?;
        self.preflight_child_locks(&path, true)?;
        self.preflight_reset_preservation(path_text)?;
        self.reset_target(&path)
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
                        eprintln!(
                            "gix operation failed, falling back to git2: {}",
                            crate::utilities::safe_human_text(&e.to_string())
                        );
                    }
                }
            }
        }

        git2_op(&self.git2_ops)
    }
}

/// Native Git mutations with supported repository inspection backends.
impl GitOperations for GitOpsManager {
    fn read_gitmodules(&self) -> Result<SubmoduleEntries> {
        self.read_native_gitmodules()
    }

    fn write_gitmodules(&mut self, config: &SubmoduleEntries) -> Result<()> {
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        self.preflight_gitmodules_layers()?;
        for (name, entry) in config.submodule_iter() {
            Self::validate_name(name)?;
            let prefix = format!("submodule.{name}");
            if let Some(path) = &entry.path {
                self.validated_path(Path::new(path))?;
                self.git([
                    "config",
                    "--file",
                    ".gitmodules",
                    &format!("{prefix}.path"),
                    path,
                ])?;
            }
            if let Some(url) = &entry.url {
                self.git([
                    "config",
                    "--file",
                    ".gitmodules",
                    &format!("{prefix}.url"),
                    url,
                ])?;
            }
            for (key, value) in [
                (
                    "branch",
                    entry.branch.as_ref().map(GitmodulesConvert::to_gitmodules),
                ),
                ("ignore", entry.ignore.map(|value| value.to_string())),
                ("update", entry.update.as_ref().map(ToString::to_string)),
                (
                    "fetchRecurseSubmodules",
                    entry.fetch_recurse.map(|value| value.to_gitmodules()),
                ),
            ] {
                if let Some(value) = value.filter(|value| !value.is_empty()) {
                    self.git([
                        "config",
                        "--file",
                        ".gitmodules",
                        &format!("{prefix}.{key}"),
                        &value,
                    ])?;
                }
            }
        }
        Ok(())
    }

    fn read_git_config(&self, level: ConfigLevel) -> Result<GitConfig> {
        self.try_with_fallback(
            |gix| gix.read_git_config(level),
            |git2| git2.read_git_config(level),
        )
    }

    fn write_git_config(&self, config: &GitConfig, level: ConfigLevel) -> Result<()> {
        if level != ConfigLevel::Local {
            anyhow::bail!("Only repository-local Git config writes are supported");
        }
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        for (key, value) in &config.entries {
            self.git(["config", "--local", key, value])?;
        }
        Ok(())
    }

    fn set_config_value(&self, key: &str, value: &str, level: ConfigLevel) -> Result<()> {
        if level != ConfigLevel::Local {
            anyhow::bail!("Only repository-local Git config writes are supported");
        }
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        self.git(["config", "--local", key, value])?;
        Ok(())
    }

    fn add_submodule(&mut self, opts: &SubmoduleAddOptions) -> Result<()> {
        let path = self.validated_path(&opts.path)?;
        let reuse_retained = self.add_preconditions(opts)?;

        let mut command = Command::new("git");
        command
            .current_dir(&self.worktree)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .args(["submodule", "add", "--name"])
            .arg(&opts.name);
        if reuse_retained {
            command.arg("--force");
        }
        let stored_branch = opts.branch.as_ref().map(GitmodulesConvert::to_gitmodules);
        if let Some(branch) = stored_branch.as_deref() {
            if branch == "." {
                let current = self.git(["symbolic-ref", "--quiet", "--short", "HEAD"])?;
                command
                    .arg("--branch")
                    .arg(String::from_utf8(current.stdout)?.trim());
            } else if branch != "HEAD" {
                command.arg("--branch").arg(branch);
            }
        }
        if opts.shallow {
            command.args(["--depth", "1"]);
        }
        command.arg("--").arg(&opts.url).arg(&path);
        let output = command
            .output()
            .context("Failed to execute git submodule add")?;
        if !output.status.success() {
            anyhow::bail!(
                "git submodule add failed; partial state was preserved for recovery: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        self.validated_child(&path)
            .context("Git created a submodule checkout outside the requested path")?;

        let prefix = format!("submodule.{}", opts.name);
        for (key, value) in [
            ("branch", stored_branch),
            ("ignore", opts.ignore.map(|value| value.to_string())),
            ("update", opts.update.as_ref().map(ToString::to_string)),
            (
                "fetchRecurseSubmodules",
                opts.fetch_recurse.map(|value| value.to_gitmodules()),
            ),
            ("shallow", opts.shallow.then(|| "true".to_string())),
        ] {
            if let Some(value) = value.filter(|value| !value.is_empty()) {
                self.git([
                    "config",
                    "--file",
                    ".gitmodules",
                    &format!("{prefix}.{key}"),
                    &value,
                ])?;
            }
        }
        let pathspec = Self::literal_pathspec(&path);
        self.git([
            OsStr::new("add"),
            OsStr::new("--"),
            OsStr::new(".gitmodules"),
            pathspec.as_os_str(),
        ])?;

        let registered = self.git([
            "config",
            "--file",
            ".gitmodules",
            "--get",
            &format!("{prefix}.path"),
        ])?;
        let registered = registered
            .stdout
            .strip_suffix(b"\n")
            .unwrap_or(&registered.stdout);
        if !Self::gitmodules_path_matches(registered, path.as_os_str()) {
            anyhow::bail!("git submodule add did not record the requested path");
        }
        let index = self.git([
            OsStr::new("ls-files"),
            OsStr::new("--stage"),
            OsStr::new("--"),
            pathspec.as_os_str(),
        ])?;
        if !String::from_utf8_lossy(&index.stdout).starts_with("160000 ") {
            anyhow::bail!("git submodule add did not create a mode-160000 gitlink");
        }
        Ok(())
    }

    fn init_submodule(&mut self, path: &str) -> Result<()> {
        let path = self.validated_materialization_target(Path::new(path))?;
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        let pathspec = Self::literal_pathspec(&path);
        self.git([
            OsStr::new("submodule"),
            OsStr::new("init"),
            OsStr::new("--"),
            pathspec.as_os_str(),
        ])?;
        Ok(())
    }

    fn update_submodule(&mut self, path: &str, opts: &SubmoduleUpdateOptions) -> Result<()> {
        if opts.strategy == SerializableUpdate::None {
            return Ok(());
        }
        self.preflight_update_submodule(path, opts)?;
        let path = self.validated_materialization_target(Path::new(path))?;
        let repair_incomplete = self.worktree.join(&path).join(".git").exists()
            && self.is_incomplete_materialization(&path)?;
        if !opts.remote && !opts.recursive && self.update_postcondition_holds(&path, opts)? {
            return Ok(());
        }
        let mut args: Vec<OsString> = ["submodule", "update", "--init"]
            .into_iter()
            .map(OsString::from)
            .collect();
        match opts.strategy {
            SerializableUpdate::Checkout | SerializableUpdate::Unspecified => {
                args.push("--checkout".into());
            }
            SerializableUpdate::Merge => args.push("--merge".into()),
            SerializableUpdate::Rebase => args.push("--rebase".into()),
            SerializableUpdate::None => return Ok(()),
        }
        if opts.recursive {
            args.push("--recursive".into());
        }
        if opts.force || repair_incomplete {
            args.push("--force".into());
        }
        if opts.remote {
            args.push("--remote".into());
        }
        args.push("--".into());
        args.push(Self::literal_pathspec(&path));
        self.git(args)?;
        anyhow::ensure!(
            self.update_postcondition_holds(&path, opts)?,
            "Git submodule update did not reach the required parent-pin postcondition"
        );
        Ok(())
    }

    fn delete_submodule(&mut self, path: &str, force: bool) -> Result<()> {
        let path = self.validated_path(Path::new(path))?;
        let path_str = path.to_str().context("Submodule path is not valid UTF-8")?;
        let registered_name = self
            .registered_name_for_path(&path)?
            .context("No exact .gitmodules registration exists for deletion")?;
        self.preflight_delete_submodule(path_str, force)?;
        let pathspec = Self::literal_pathspec(&path);
        let head_entry = self.git_output([
            OsStr::new("ls-tree"),
            OsStr::new("-z"),
            OsStr::new("HEAD"),
            OsStr::new("--"),
            pathspec.as_os_str(),
        ])?;
        if !head_entry.status.success() {
            anyhow::bail!(
                "Could not inspect the parent commit before deletion: {}",
                String::from_utf8_lossy(&head_entry.stderr).trim()
            );
        }
        if head_entry.stdout.is_empty() {
            // A newly added gitlink has no HEAD entry, so ordinary `git rm` refuses it as
            // staged. Deinitialize the already-verified clean checkout, remove only the exact
            // cached gitlink, then remove its exact portable section with Git's config parser.
            self.git([
                OsStr::new("submodule"),
                OsStr::new("deinit"),
                OsStr::new("--force"),
                OsStr::new("--"),
                pathspec.as_os_str(),
            ])?;
            self.git([
                OsStr::new("rm"),
                OsStr::new("--cached"),
                OsStr::new("--"),
                pathspec.as_os_str(),
            ])?;
            self.git([
                OsStr::new("config"),
                OsStr::new("--file"),
                OsStr::new(".gitmodules"),
                OsStr::new("--remove-section"),
                OsStr::new(&format!("submodule.{registered_name}")),
            ])?;
            self.git(["add", "--", ".gitmodules"])?;
        } else {
            let mut args: Vec<OsString> = vec!["rm".into()];
            if force {
                args.push("--force".into());
            }
            args.push("--".into());
            args.push(pathspec.clone());
            self.git(args)?;
        }
        let local_pattern = format!(
            r"^submodule\.{}\.",
            Self::config_key_regex_literal(&registered_name)
        );
        let local_section_exists = self.git_output([
            OsStr::new("config"),
            OsStr::new("--local"),
            OsStr::new("--name-only"),
            OsStr::new("--get-regexp"),
            OsStr::new(&local_pattern),
        ])?;
        match local_section_exists.status.code() {
            Some(0) => {
                self.git([
                    OsStr::new("config"),
                    OsStr::new("--local"),
                    OsStr::new("--remove-section"),
                    OsStr::new(&format!("submodule.{registered_name}")),
                ])?;
            }
            Some(1) => {}
            _ => {
                anyhow::bail!(
                    "Could not inspect the exact local Git registration: {}",
                    String::from_utf8_lossy(&local_section_exists.stderr).trim()
                );
            }
        }
        match std::fs::symlink_metadata(self.worktree.join(&path)) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Ok(metadata)
                if metadata.is_dir() && self.worktree.join(&path).read_dir()?.next().is_none() =>
            {
                std::fs::remove_dir(self.worktree.join(&path)).with_context(|| {
                    format!(
                        "Could not remove Git's empty checkout placeholder at {}",
                        self.worktree.join(&path).display()
                    )
                })?;
            }
            Ok(_) => anyhow::bail!(
                "Git removed the registration but left content at {}; preserving it for recovery",
                self.worktree.join(&path).display()
            ),
            Err(error) => return Err(error.into()),
        }
        let index = self.git([
            OsStr::new("ls-files"),
            OsStr::new("--stage"),
            OsStr::new("--"),
            pathspec.as_os_str(),
        ])?;
        if !index.stdout.is_empty() {
            anyhow::bail!("Git reported removal but the exact gitlink remains in the index");
        }
        if self.registered_name_for_path(&path)?.is_some() {
            anyhow::bail!("Git reported removal but the exact .gitmodules registration remains");
        }
        Ok(())
    }

    fn deinit_submodule(&mut self, path: &str, force: bool) -> Result<()> {
        let path = self.validated_materialization_target(Path::new(path))?;
        let path_str = path.to_str().context("Submodule path is not valid UTF-8")?;
        self.preflight_native_locks()?;
        self.preflight_native_metadata()?;
        if !force {
            self.ensure_no_discardable_checkout_data(path_str)?;
        }
        let mut args: Vec<OsString> = vec!["submodule".into(), "deinit".into()];
        if force {
            args.push("--force".into());
        }
        args.push("--".into());
        args.push(Self::literal_pathspec(&path));
        self.git(args)?;
        Ok(())
    }

    fn get_submodule_status(&self, path: &str) -> Result<DetailedSubmoduleStatus> {
        self.git2_ops.get_submodule_status(path)
    }

    fn list_submodules(&self) -> Result<Vec<String>> {
        self.try_with_fallback(
            GixOperations::list_submodules,
            Git2Operations::list_submodules,
        )
    }

    fn fetch_submodule(&self, path: &str) -> Result<()> {
        self.preflight_native_locks()?;
        self.child_git(path, ["fetch"])?;
        Ok(())
    }

    fn reset_submodule(&self, path: &str, hard: bool) -> Result<()> {
        let target = self.preflight_reset_submodule(path)?;
        let mut args = vec!["submodule", "update", "--init", "--checkout"];
        if hard {
            args.push("--force");
        }
        args.push("--");
        let mut args: Vec<OsString> = args.into_iter().map(OsString::from).collect();
        args.push(Self::literal_pathspec(Path::new(path)));
        self.git(args)?;
        let head = self.child_git(path, ["rev-parse", "HEAD"])?;
        anyhow::ensure!(
            String::from_utf8(head.stdout)?.trim_end() == target,
            "Reset completed without moving {path} to parent pin {target}"
        );
        Ok(())
    }

    fn clean_submodule(&self, path: &str, force: bool, remove_directories: bool) -> Result<()> {
        self.preflight_native_locks()?;
        let mut args = vec!["clean"];
        if force {
            args.push("-f");
        }
        if remove_directories {
            args.push("-d");
        }
        args.push("--");
        self.child_git(path, args)?;
        Ok(())
    }

    fn stash_submodule(&self, path: &str, include_untracked: bool) -> Result<Option<String>> {
        self.read_native_gitmodules()?;
        self.preflight_reset_preservation(path)?;
        self.preflight_native_locks()?;
        let before = self.child_ref_oid(path, "refs/stash")?;
        let mut args = vec!["stash", "push"];
        if include_untracked {
            args.push("--include-untracked");
        }
        args.extend(["--message", "submod reset preservation"]);
        self.child_git(path, args)?;
        let after = self.child_ref_oid(path, "refs/stash")?;
        if after == before {
            return Ok(None);
        }
        let oid = after.context("Git reported a stash but refs/stash is missing")?;
        self.child_git(path, ["cat-file", "-e", &format!("{oid}^{{commit}}")])?;
        Ok(Some(oid))
    }

    fn enable_sparse_checkout(&self, path: &str) -> Result<()> {
        self.preflight_native_locks()?;
        self.child_git(path, ["sparse-checkout", "init", "--no-cone"])?;
        Ok(())
    }

    fn set_sparse_patterns(&self, path: &str, patterns: &[String]) -> Result<()> {
        let path = self.validated_child(Path::new(path))?;
        self.preflight_native_locks()?;
        let child = self.worktree.join(path);
        let mut command = Command::new("git")
            .args(["sparse-checkout", "set", "--no-cone", "--stdin"])
            .current_dir(&child)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to execute Git in {}", child.display()))?;
        {
            let stdin = command
                .stdin
                .as_mut()
                .context("Git sparse-checkout stdin unavailable")?;
            for pattern in patterns {
                writeln!(stdin, "{pattern}")?;
            }
        }
        let output = command.wait_with_output()?;
        if !output.status.success() {
            anyhow::bail!(
                "git sparse-checkout set failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(())
    }

    fn get_sparse_patterns(&self, path: &str) -> Result<Vec<String>> {
        let output = self.child_git(
            path,
            [
                "rev-parse",
                "--path-format=absolute",
                "--git-path",
                "info/sparse-checkout",
            ],
        )?;
        let sparse_file = crate::utilities::git_path_from_stdout(output.stdout)?;
        match std::fs::read_to_string(sparse_file) {
            Ok(content) => Ok(content.lines().map(str::to_string).collect()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(error.into()),
        }
    }

    fn apply_sparse_checkout(&self, path: &str) -> Result<()> {
        self.preflight_native_locks()?;
        self.child_git(path, ["sparse-checkout", "reapply"])?;
        Ok(())
    }
}

#[cfg(test)]
mod storage_path_tests {
    use super::*;

    #[test]
    fn canonical_storage_form_resolves_existing_directory() {
        let dir = tempfile::TempDir::new().unwrap();
        let target = dir.path().join("modules").join("child");
        std::fs::create_dir_all(&target).unwrap();
        assert_eq!(
            GitOpsManager::canonical_storage_form(&target).unwrap(),
            std::fs::canonicalize(&target).unwrap()
        );
    }

    #[test]
    fn canonical_storage_form_keeps_missing_leaf_below_canonical_parent() {
        let dir = tempfile::TempDir::new().unwrap();
        let parent = dir.path().join("modules");
        std::fs::create_dir_all(&parent).unwrap();
        let missing = parent.join("child");
        assert_eq!(
            GitOpsManager::canonical_storage_form(&missing).unwrap(),
            std::fs::canonicalize(&parent).unwrap().join("child")
        );
    }

    #[test]
    fn gitmodules_path_matches_exact_spelling() {
        assert!(GitOpsManager::gitmodules_path_matches(
            b"vendor/checkout",
            OsStr::new("vendor/checkout")
        ));
        assert!(!GitOpsManager::gitmodules_path_matches(
            b"vendor/other",
            OsStr::new("vendor/checkout")
        ));
    }

    #[test]
    fn gitmodules_path_separator_matching_follows_platform_rules() {
        // On Windows both separators name the same file, so storage using
        // forward slashes matches a normalized backslash request; elsewhere
        // a backslash is a distinct filename character and must not match.
        assert_eq!(
            GitOpsManager::gitmodules_path_matches(
                b"vendor/checkout",
                OsStr::new("vendor\\checkout")
            ),
            cfg!(windows)
        );
    }

    #[test]
    fn canonical_storage_form_walks_multiple_missing_levels() {
        let dir = tempfile::TempDir::new().unwrap();
        let missing = dir.path().join("a").join("b").join("c");
        assert_eq!(
            GitOpsManager::canonical_storage_form(&missing).unwrap(),
            std::fs::canonicalize(dir.path())
                .unwrap()
                .join("a")
                .join("b")
                .join("c")
        );
    }

    #[test]
    fn resolve_relative_url_normalizes_absolute_url_separators() {
        // A Windows-spelled file base must resolve to a canonical URL so
        // exact-match remote lookups keep working; relative results are
        // untouched.
        assert_eq!(
            GitOpsManager::resolve_relative_submodule_url(
                "file://C:\\Users\\me\\super.git",
                "../reachable.git",
                None
            )
            .unwrap(),
            "file://C:/Users/me/reachable.git"
        );
        assert_eq!(
            GitOpsManager::resolve_relative_submodule_url(
                "file://C:/Users/me/super.git",
                "../reachable.git",
                None
            )
            .unwrap(),
            "file://C:/Users/me/reachable.git"
        );
        assert_eq!(
            GitOpsManager::resolve_relative_submodule_url("../base.git", "../sib.git", None)
                .unwrap(),
            "../sib.git"
        );
    }

    #[test]
    fn canonical_worktree_root_ignores_report_spelling() {
        let dir = tempfile::TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        std::fs::create_dir(dir.path().join("other")).unwrap();
        // An `..` segment spells the same directory with different components
        // on every platform, the way Git's forward slashes differ from
        // verbatim UNC reports on Windows. (`Path` equality itself skips `.`
        // segments, so a dot spelling cannot prove anything here.)
        let winding = PathBuf::from(format!("{}/other/../sub", dir.path().display()));
        assert_ne!(winding, std::fs::canonicalize(&sub).unwrap());
        assert_eq!(
            GitOpsManager::canonical_worktree_root(&winding, &sub).unwrap(),
            std::fs::canonicalize(&sub).unwrap()
        );
    }

    #[test]
    fn canonical_worktree_root_refuses_other_and_missing_reports() {
        let dir = tempfile::TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        let other = dir.path().join("other");
        std::fs::create_dir(&sub).unwrap();
        std::fs::create_dir(&other).unwrap();
        let reported = std::fs::canonicalize(&other).unwrap();
        // An existing but different directory normalizes fine; the caller
        // rejects the mismatch against the intended root.
        assert_eq!(
            GitOpsManager::canonical_worktree_root(&reported, &sub).unwrap(),
            reported
        );
        assert_ne!(
            GitOpsManager::canonical_worktree_root(&reported, &sub).unwrap(),
            std::fs::canonicalize(&sub).unwrap()
        );
        assert!(
            GitOpsManager::canonical_worktree_root(&dir.path().join("absent"), &sub)
                .unwrap_err()
                .to_string()
                .contains("unrelated worktree")
        );
    }
}
