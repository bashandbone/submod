// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

use super::{DetailedSubmoduleStatus, GitConfig, SubmoduleStatusFlags};
use crate::config::{SubmoduleEntries, SubmoduleEntry};
use crate::options::{ConfigLevel, SerializableBranch, SerializableFetchRecurse};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
/// Read-only Git2 repository inspection and fallback reads
pub struct Git2Operations {
    repo: git2::Repository,
}
impl Git2Operations {
    /// Create a new `Git2Operations` instance
    pub fn new(repo_path: Option<&Path>) -> Result<Self> {
        let repo = match repo_path {
            Some(path) => git2::Repository::open(path)
                .with_context(|| format!("Failed to open repository at {}", path.display()))?,
            None => git2::Repository::open_from_env()
                .with_context(|| "Failed to open repository from environment")?,
        };
        Ok(Self { repo })
    }

    /// Return the working directory of the repository, if any.
    pub(super) fn workdir(&self) -> Option<&std::path::Path> {
        self.repo.workdir()
    }

    /// Resolve the portable `.gitmodules` section name for an exact checkout path.
    ///
    /// libgit2 can report `Submodule::name()` as the checkout path when the section
    /// nickname differs from that path. Configuration and status APIs, however,
    /// are keyed by the portable section name. Read that identity from
    /// `.gitmodules` instead of conflating the two namespaces.
    fn registered_name_for_path(&self, path: &str) -> Result<Option<String>> {
        let Some(workdir) = self.repo.workdir() else {
            return Ok(None);
        };
        let gitmodules = workdir.join(".gitmodules");
        if !gitmodules.is_file() {
            return Ok(None);
        }

        let config = git2::Config::open(&gitmodules)?;
        let mut matches = Vec::new();
        let mut entries = config.entries(Some(r"^submodule\..*\.path$"))?;
        while let Some(entry) = entries.next() {
            let entry = entry?;
            if entry.value().ok() != Some(path) {
                continue;
            }
            let key = entry.name()?;
            let Some(name) = key
                .strip_prefix("submodule.")
                .and_then(|key| key.strip_suffix(".path"))
            else {
                continue;
            };
            matches.push(name.to_string());
        }
        matches.sort();
        matches.dedup();
        match matches.as_slice() {
            [] => Ok(None),
            [name] => Ok(Some(name.clone())),
            _ => anyhow::bail!("Multiple .gitmodules sections register path {path:?}"),
        }
    }

    /// Convert git2 submodule to our `SubmoduleEntry` format
    fn convert_git2_submodule_to_entry(
        &self,
        submodule: &git2::Submodule,
    ) -> Result<(String, SubmoduleEntry)> {
        let path = submodule.path().to_string_lossy().to_string();
        let name = self
            .registered_name_for_path(&path)?
            .unwrap_or_else(|| submodule.name().unwrap_or("").to_string());
        // git2 0.21 splits what used to be one Option: the Result is UTF-8
        // validity, the Option is whether a URL is configured at all. Both still
        // collapse to "" here, matching the previous behavior.
        let url = submodule.url().ok().flatten().unwrap_or("").to_string();
        // Get branch from config
        let branch = self.get_submodule_branch(&name)?;
        // Get ignore setting
        let ignore = submodule.ignore_rule().try_into().ok();
        // Get update setting
        let update = submodule.update_strategy().try_into().ok();
        // Get fetch recurse setting from config
        let fetch_recurse = self.get_submodule_fetch_recurse(&name)?;
        // Check if submodule is active
        let active = self.is_submodule_active(&name)?;
        // Check if submodule is shallow
        let shallow = self.is_submodule_shallow(&path)?;
        let entry = SubmoduleEntry {
            path: Some(path),
            url: Some(url),
            branch,
            ignore,
            update,
            fetch_recurse,
            active: Some(active),
            shallow: Some(shallow),
            no_init: Some(false), // not used here
            sparse_paths: None,
            use_git_default_sparse_checkout: None,
        };
        Ok((name, entry))
    }
    /// Get branch configuration for a submodule
    fn get_submodule_branch(&self, name: &str) -> Result<Option<SerializableBranch>> {
        let config = self.repo.config()?;
        let key = format!("submodule.{name}.branch");

        config.get_string(&key).map_or_else(
            |_| Ok(None),
            |branch_str| {
                if branch_str == "." {
                    Ok(Some(SerializableBranch::CurrentInSuperproject))
                } else {
                    Ok(Some(SerializableBranch::Name(branch_str)))
                }
            },
        )
    }
    /// Get fetch recurse configuration for a submodule
    fn get_submodule_fetch_recurse(&self, name: &str) -> Result<Option<SerializableFetchRecurse>> {
        let config = self.repo.config()?;
        let key = format!("submodule.{name}.fetchRecurseSubmodules");

        config.get_string(&key).map_or_else(
            |_| Ok(None),
            |fetch_str| match fetch_str.as_str() {
                "true" => Ok(Some(SerializableFetchRecurse::Always)),
                "on-demand" => Ok(Some(SerializableFetchRecurse::OnDemand)),
                "false" | "no" => Ok(Some(SerializableFetchRecurse::Never)),
                _ => Ok(None),
            },
        )
    }
    /// Check if a submodule is active
    fn is_submodule_active(&self, name: &str) -> Result<bool> {
        let config = self.repo.config()?;
        let key = format!("submodule.{name}.active");

        config.get_bool(&key).map_or(Ok(true), Ok)
    }
    /// Check if a submodule is shallow
    fn is_submodule_shallow(&self, path: &str) -> Result<bool> {
        let submodule_path = self
            .repo
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?
            .join(path);

        if !submodule_path.exists() {
            return Ok(false);
        }
        // Check if .git/shallow exists in the submodule
        let shallow_file = submodule_path.join(".git").join("shallow");
        Ok(shallow_file.exists())
    }
    /// Convert git2 status flags to our status flags
    #[allow(dead_code, clippy::unused_self)]
    fn convert_git2_status_to_flags(&self, status: git2::SubmoduleStatus) -> SubmoduleStatusFlags {
        let mut flags = SubmoduleStatusFlags::empty();
        if status.contains(git2::SubmoduleStatus::IN_HEAD) {
            flags |= SubmoduleStatusFlags::IN_HEAD;
        }
        if status.contains(git2::SubmoduleStatus::IN_INDEX) {
            flags |= SubmoduleStatusFlags::IN_INDEX;
        }
        if status.contains(git2::SubmoduleStatus::IN_CONFIG) {
            flags |= SubmoduleStatusFlags::IN_CONFIG;
        }
        if status.contains(git2::SubmoduleStatus::IN_WD) {
            flags |= SubmoduleStatusFlags::IN_WD;
        }
        if status.contains(git2::SubmoduleStatus::INDEX_ADDED) {
            flags |= SubmoduleStatusFlags::INDEX_ADDED;
        }
        if status.contains(git2::SubmoduleStatus::INDEX_DELETED) {
            flags |= SubmoduleStatusFlags::INDEX_DELETED;
        }
        if status.contains(git2::SubmoduleStatus::INDEX_MODIFIED) {
            flags |= SubmoduleStatusFlags::INDEX_MODIFIED;
        }
        if status.contains(git2::SubmoduleStatus::WD_UNINITIALIZED) {
            flags |= SubmoduleStatusFlags::WD_UNINITIALIZED;
        }
        if status.contains(git2::SubmoduleStatus::WD_ADDED) {
            flags |= SubmoduleStatusFlags::WD_ADDED;
        }
        if status.contains(git2::SubmoduleStatus::WD_DELETED) {
            flags |= SubmoduleStatusFlags::WD_DELETED;
        }
        if status.contains(git2::SubmoduleStatus::WD_MODIFIED) {
            flags |= SubmoduleStatusFlags::WD_MODIFIED;
        }
        if status.contains(git2::SubmoduleStatus::WD_INDEX_MODIFIED) {
            flags |= SubmoduleStatusFlags::WD_INDEX_MODIFIED;
        }
        if status.contains(git2::SubmoduleStatus::WD_WD_MODIFIED) {
            flags |= SubmoduleStatusFlags::WD_WD_MODIFIED;
        }
        if status.contains(git2::SubmoduleStatus::WD_UNTRACKED) {
            flags |= SubmoduleStatusFlags::WD_UNTRACKED;
        }
        flags
    }
    /// Get git config at specified level
    #[allow(dead_code)]
    fn get_config_at_level(&self, level: ConfigLevel) -> Result<git2::Config> {
        match level {
            ConfigLevel::Local => self.repo.config(),
            ConfigLevel::Global => git2::Config::open_default()
                .and_then(|config| config.open_level(git2::ConfigLevel::Global)),
            ConfigLevel::System => git2::Config::open_default()
                .and_then(|config| config.open_level(git2::ConfigLevel::System)),
            ConfigLevel::Worktree => {
                // Worktree config is typically handled as local config
                self.repo.config()
            }
        }
        .with_context(|| format!("Failed to open config at level {level:?}"))
    }
}
impl Git2Operations {
    /// Read `.gitmodules` registrations through libgit2 (no mutation, no fetch).
    pub fn read_gitmodules(&self) -> Result<SubmoduleEntries> {
        let mut submodules = HashMap::new();
        // Iterate through all submodules
        self.repo
            .submodules()?
            .into_iter()
            .try_for_each(|submodule| -> Result<()> {
                let (name, entry) = self.convert_git2_submodule_to_entry(&submodule)?;
                submodules.insert(name, entry);
                Ok(())
            })?;
        Ok(SubmoduleEntries::new(
            if submodules.is_empty() {
                None
            } else {
                Some(submodules)
            },
            None, // sparse_checkouts will be populated separately if needed
        ))
    }

    /// Read Git configuration at `level` through libgit2 (no mutation).
    pub fn read_git_config(&self, level: ConfigLevel) -> Result<GitConfig> {
        let config = self.get_config_at_level(level)?;
        let mut entries = HashMap::new();
        // Iterate through config entries
        config.entries(None)?.for_each(|entry| {
            if let (Ok(name), Ok(value)) = (entry.name(), entry.value()) {
                entries.insert(name.to_string(), value.to_string());
            }
        })?;
        Ok(GitConfig { entries })
    }

    /// Detailed libgit2 submodule status (no mutation, no fetch).
    pub fn get_submodule_status(&self, path: &str) -> Result<DetailedSubmoduleStatus> {
        let submodule = self
            .repo
            .find_submodule(path)
            .with_context(|| format!("Submodule not found: {path}"))?;
        let name = self
            .registered_name_for_path(path)?
            .unwrap_or_else(|| submodule.name().unwrap_or(path).to_string());
        let url = submodule
            .url()
            .ok()
            .flatten()
            .map(std::string::ToString::to_string);

        // Get status
        let status = self
            .repo
            .submodule_status(&name, git2::SubmoduleIgnore::Unspecified)?;
        let mut status_flags = self.convert_git2_status_to_flags(status);
        if !status_flags.contains(SubmoduleStatusFlags::IN_CONFIG)
            && self.registered_name_for_path(path)?.as_deref() == Some(name.as_str())
        {
            status_flags.insert(SubmoduleStatusFlags::IN_CONFIG);
        }
        // Get OIDs
        let head_oid = submodule.head_id().map(|oid| oid.to_string());
        let index_oid = submodule.index_id().map(|oid| oid.to_string());
        let workdir_oid = submodule.workdir_id().map(|oid| oid.to_string());
        // Get configuration
        let branch = self.get_submodule_branch(&name)?;
        let ignore_rule = submodule.ignore_rule().try_into().unwrap_or_default();
        let update_rule = submodule.update_strategy().try_into().unwrap_or_default();
        let fetch_recurse_rule = self.get_submodule_fetch_recurse(&name)?.unwrap_or_default();
        // Check status flags
        let is_initialized = !status.contains(git2::SubmoduleStatus::WD_UNINITIALIZED);
        let is_active = self.is_submodule_active(&name)?;
        let has_modifications = status.intersects(
            git2::SubmoduleStatus::WD_MODIFIED
                | git2::SubmoduleStatus::WD_INDEX_MODIFIED
                | git2::SubmoduleStatus::WD_WD_MODIFIED,
        );
        // Check sparse checkout
        let (sparse_checkout_enabled, sparse_patterns) = self.get_sparse_checkout_info(path)?;
        Ok(DetailedSubmoduleStatus {
            path: path.to_string(),
            name,
            url,
            head_oid,
            index_oid,
            workdir_oid,
            status_flags,
            ignore_rule,
            update_rule,
            fetch_recurse_rule,
            branch,
            is_initialized,
            is_active,
            has_modifications,
            sparse_checkout_enabled,
            sparse_patterns,
        })
    }

    /// List registered submodule paths through libgit2 (no mutation, no fetch).
    pub fn list_submodules(&self) -> Result<Vec<String>> {
        let submodules = self.repo.submodules()?;
        let paths = submodules
            .iter()
            .map(|sm| sm.path().to_string_lossy().to_string())
            .collect();
        Ok(paths)
    }

    /// Read the sparse-checkout patterns of an initialized child (no mutation).
    pub fn get_sparse_patterns(&self, path: &str) -> Result<Vec<String>> {
        let submodule = self
            .repo
            .find_submodule(path)
            .with_context(|| format!("Submodule not found: {path}"))?;
        // Open the submodule repository
        let sub_repo = submodule
            .open()
            .with_context(|| format!("Failed to open submodule repository: {path}"))?;
        // Read patterns from .git/info/sparse-checkout
        let git_dir = sub_repo.path();
        let sparse_checkout_file = git_dir.join("info").join("sparse-checkout");
        if !sparse_checkout_file.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&sparse_checkout_file).with_context(|| {
            format!("Failed to read sparse checkout patterns for submodule: {path}")
        })?;
        let patterns = content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(std::string::ToString::to_string)
            .collect();
        Ok(patterns)
    }
}

impl Git2Operations {
    /// Get sparse checkout information for a submodule
    #[allow(dead_code)]
    fn get_sparse_checkout_info(&self, path: &str) -> Result<(bool, Vec<String>)> {
        let submodule = self
            .repo
            .find_submodule(path)
            .with_context(|| format!("Submodule not found: {path}"))?;
        // Open the submodule repository
        let sub_repo = submodule
            .open()
            .with_context(|| format!("Failed to open submodule repository: {path}"))?;
        // Check if sparse checkout is enabled
        let config = sub_repo.config()?;
        let sparse_enabled = config.get_bool("core.sparseCheckout").unwrap_or(false);
        if !sparse_enabled {
            return Ok((false, Vec::new()));
        }
        // Get sparse patterns
        let patterns = self.get_sparse_patterns(path)?;
        Ok((true, patterns))
    }
}

impl From<super::GitOpsManager> for Git2Operations {
    fn from(ops: super::GitOpsManager) -> Self {
        ops.git2_ops
    }
}
