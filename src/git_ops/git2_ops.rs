// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use super::{
    DetailedSubmoduleStatus, GitConfig, GitOperations, SubmoduleStatusFlags,
};
use crate::options::{
    ConfigLevel, SerializableBranch, SerializableFetchRecurse, SerializableIgnore, SerializableUpdate,
};
use crate::config::{SubmoduleAddOptions, SubmoduleEntries, SubmoduleEntry, SubmoduleUpdateOptions};
/// Git2 implementation providing complete fallback coverage
pub struct Git2Operations {
    repo: git2::Repository,
}
impl Git2Operations {
    /// Create a new Git2Operations instance
    pub fn new(repo_path: Option<&Path>) -> Result<Self> {
        let repo = match repo_path {
            Some(path) => git2::Repository::open(path)
                .with_context(|| format!("Failed to open repository at {}", path.display()))?,
            None => git2::Repository::open_from_env()
                .with_context(|| "Failed to open repository from environment")?,
        };
        Ok(Self { repo })
    }
    /// Convert git2 submodule to our SubmoduleEntry format
    fn convert_git2_submodule_to_entry(&self, submodule: &git2::Submodule) -> Result<(String, SubmoduleEntry)> {
        let name = submodule.name().unwrap_or("").to_string();
        let path = submodule.path().to_string_lossy().to_string();
        let url = submodule.url().unwrap_or("").to_string();
        // Get branch from config
        let branch = self.get_submodule_branch(&name)?;
        // Get ignore setting
        let ignore = submodule.ignore_rule()
            .try_into()
            .ok();
        // Get update setting
        let update = submodule.update_strategy()
            .try_into()
            .ok();
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
        };
        Ok((name, entry))
    }
    /// Get branch configuration for a submodule
    fn get_submodule_branch(&self, name: &str) -> Result<Option<SerializableBranch>> {
        let config = self.repo.config()?;
        let key = format!("submodule.{}.branch", name);

        match config.get_string(&key) {
            Ok(branch_str) => {
                if branch_str == "." {
                    Ok(Some(SerializableBranch::CurrentInSuperproject))
                } else {
                    Ok(Some(SerializableBranch::Name(branch_str)))
                }
            }
            Err(_) => Ok(None),
        }
    }
    /// Get fetch recurse configuration for a submodule
    fn get_submodule_fetch_recurse(&self, name: &str) -> Result<Option<SerializableFetchRecurse>> {
        let config = self.repo.config()?;
        let key = format!("submodule.{}.fetchRecurseSubmodules", name);

        match config.get_string(&key) {
            Ok(fetch_str) => match fetch_str.as_str() {
                "true" | "on-demand" => Ok(Some(SerializableFetchRecurse::OnDemand)),
                "false" | "no" => Ok(Some(SerializableFetchRecurse::Never)),
                _ => Ok(None),
            },
            Err(_) => Ok(None),
        }
    }
    /// Check if a submodule is active
    fn is_submodule_active(&self, name: &str) -> Result<bool> {
        let config = self.repo.config()?;
        let key = format!("submodule.{}.active", name);

        match config.get_bool(&key) {
            Ok(active) => Ok(active),
            Err(_) => Ok(true), // Default to active if not specified
        }
    }
    /// Check if a submodule is shallow
    fn is_submodule_shallow(&self, path: &str) -> Result<bool> {
        let submodule_path = self.repo.workdir()
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
        .with_context(|| format!("Failed to open config at level {:?}", level))
    }
}
impl GitOperations for Git2Operations {
    fn read_gitmodules(&self) -> Result<SubmoduleEntries> {
        let mut submodules = HashMap::new();
        // Iterate through all submodules
        self.repo.submodules()?
            .into_iter()
            .try_for_each(|submodule| -> Result<()> {
                let (name, entry) = self.convert_git2_submodule_to_entry(&submodule)?;
                submodules.insert(name, entry);
                Ok(())
            })?;
            Ok(SubmoduleEntries::new(
                if submodules.is_empty() { None } else { Some(submodules) },
                None, // sparse_checkouts will be populated separately if needed
            ))
    }
    fn write_gitmodules(&mut self, config: &SubmoduleEntries) -> Result<()> {
        // git2 doesn't have direct .gitmodules writing, but we can manipulate submodules
        // For now, we'll update individual submodule configurations
        if let Some(submodules) = config.submodules().as_ref() {
            for (name, entry) in submodules.iter() {
                // Find or create the submodule
                match self.repo.find_submodule(&entry.path.as_ref().map(|p| p.to_string()).unwrap_or(name.clone())) {
                    Ok(mut submodule) => {
                        // Update existing submodule configuration through git config
                        let mut config = self.repo.config()?;
                        if let Some(ignore) = &entry.ignore {
                            let ignore_str = match ignore {
                                SerializableIgnore::All => "all",
                                SerializableIgnore::Dirty => "dirty",
                                SerializableIgnore::Untracked => "untracked",
                                SerializableIgnore::None => "none",
                                SerializableIgnore::Unspecified => continue, // Skip unspecified
                            };
                            config.set_str(&format!("submodule.{}.ignore", name), ignore_str)?;
                        }
                        if let Some(update) = &entry.update {
                            let update_str = match update {
                                SerializableUpdate::Checkout => "checkout",
                                SerializableUpdate::Rebase => "rebase",
                                SerializableUpdate::Merge => "merge",
                                SerializableUpdate::None => "none",
                                SerializableUpdate::Unspecified => continue, // Skip unspecified
                            };
                            config.set_str(&format!("submodule.{}.update", name), update_str)?;
                        }
                        // Set URL if different
                        if let Some(url) = &entry.url {
                            if submodule.url() != Some(url.as_str()) {
                                config.set_str(&format!("submodule.{}.url", name), url)?;
                            }
                        }
                        // Sync changes
                        submodule.sync()?;
                    }
                    Err(_) => {
                        // Submodule doesn't exist, we'd need to add it
                        // This is handled by add_submodule method
                        continue;
                    }
                }
            }
        }
        Ok(())
    }
    fn read_git_config(&self, level: ConfigLevel) -> Result<GitConfig> {
        let config = self.get_config_at_level(level)?;
        let mut entries = HashMap::new();
        // Iterate through config entries
        config.entries(None)?.for_each(|entry| {
            if let (Some(name), Some(value)) = (entry.name(), entry.value()) {
                entries.insert(name.to_string(), value.to_string());
            }
        });
        Ok(GitConfig { entries })
    }
    fn write_git_config(&self, config: &GitConfig, level: ConfigLevel) -> Result<()> {
        let mut git_config = self.get_config_at_level(level)?;
        for (key, value) in &config.entries {
            git_config.set_str(key, value)?;
        }
        Ok(())
    }
    fn set_config_value(&self, key: &str, value: &str, level: ConfigLevel) -> Result<()> {
        let mut config = self.get_config_at_level(level)?;
        config.set_str(key, value)
            .with_context(|| format!("Failed to set config value {}={}", key, value))?;
        Ok(())
    }
    fn add_submodule(&mut self, opts: &SubmoduleAddOptions) -> Result<()> {
        // Add the submodule
        {
            let _submodule = self.repo.submodule(
                &opts.url,
                &opts.path,
                true, // use_gitlink
            )?;
        } // submodule is dropped here
        // Configure the submodule (after dropping the submodule reference)
        if let Some(ignore) = &(*opts).ignore {
            let git2_ignore: git2::SubmoduleIgnore = ignore.clone().try_into()
                .map_err(|_| anyhow::anyhow!("Failed to convert ignore setting"))?;
            self.repo.submodule_set_ignore(&opts.path.to_string_lossy(), git2_ignore)?;
        }
        if let Some(update) = &opts.update {
            let git2_update: git2::SubmoduleUpdate = update.clone().try_into()
                .map_err(|_| anyhow::anyhow!("Failed to convert update setting"))?;
            self.repo.submodule_set_update(&opts.path.to_string_lossy(), git2_update)?;
        }
        // Set branch if specified
        if let Some(branch) = &opts.branch {
            let branch_str = match branch {
                SerializableBranch::CurrentInSuperproject => ".".to_string(),
                SerializableBranch::Name(name) => name.clone(),
            };

            let mut config = self.repo.config()?;
            let key = format!("submodule.{}.branch", opts.name);
            config.set_str(&key, &branch_str)?;
        }
        // Set fetch recurse if specified
        if let Some(fetch_recurse) = &opts.fetch_recurse {
            let fetch_str = match fetch_recurse {
                SerializableFetchRecurse::OnDemand => "on-demand",
                SerializableFetchRecurse::Always => "true",
                SerializableFetchRecurse::Never => "false",
                SerializableFetchRecurse::Unspecified => return Ok(()), // Skip setting
            };

            let mut config = self.repo.config()?;
            let key = format!("submodule.{}.fetchRecurseSubmodules", opts.name);
            config.set_str(&key, fetch_str)?;
        }
        // Initialize the submodule if not skipped
        if !opts.no_init {
            let mut submodule = self.repo.find_submodule(opts.path.to_str().unwrap())?;
            submodule.init(false)?; // false = don't overwrite existing config
            submodule.update(true, None)?; // true = init, None = use default options
        submodule.sync()?;
        }
        // Sync changes
        Ok(())
    }
    fn init_submodule(&mut self, path: &str) -> Result<()> {
        let mut submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;

        submodule.init(false)?; // false = don't overwrite existing config
        Ok(())
    }
    fn update_submodule(&mut self, path: &str, opts: &SubmoduleUpdateOptions) -> Result<()> {
        let mut submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;
        // Create update options
        let mut update_opts = git2::SubmoduleUpdateOptions::new();
        update_opts.allow_fetch(true);
        // Set update strategy (git2 has limited support for different strategies)
        match opts.strategy {
            SerializableUpdate::Checkout => {
                // Default behavior
            }
            SerializableUpdate::Rebase | SerializableUpdate::Merge => {
                // git2 doesn't support rebase/merge directly, use checkout
                eprintln!("Warning: git2 doesn't support rebase/merge update strategies, using checkout");
            }
            SerializableUpdate::None => return Ok(()),
            SerializableUpdate::Unspecified => {
                // Use default
            }
        }
        submodule.update(true, Some(&mut update_opts))?;
        Ok(())
    }
    fn delete_submodule(&self, path: &str) -> Result<()> {
        // git2 doesn't have direct submodule deletion, so we need to do it manually

        // 1. Deinitialize the submodule
        self.deinit_submodule(path, true)?;
        // 2. Remove from index
        let mut index = self.repo.index()?;
        index.remove_path(Path::new(path))?;
        index.write()?;
        // 3. Remove the directory
        let workdir = self.repo.workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
        let submodule_path = workdir.join(path);

        if submodule_path.exists() {
            std::fs::remove_dir_all(&submodule_path)
                .with_context(|| format!("Failed to remove submodule directory: {}", path))?;
        }
        // 4. Remove from .gitmodules (this is complex with git2, might need manual file editing)
        // For now, we'll leave this to be handled by higher-level logic
        Ok(())
    }
    fn deinit_submodule(&self, path: &str, force: bool) -> Result<()> {
        let submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;
        // git2 doesn't have a direct deinit method, so we need to:
        // 1. Remove the submodule's config entries
        // 2. Remove the submodule's working directory if force is true
        let mut config = self.repo.config()?;
        let name = submodule.name().unwrap_or(path);
        // Remove config entries
        let keys_to_remove = [
            format!("submodule.{}.url", name),
            format!("submodule.{}.active", name),
            format!("submodule.{}.branch", name),
            format!("submodule.{}.fetchRecurseSubmodules", name),
        ];
        for key in &keys_to_remove {
            let _ = config.remove(key); // Ignore errors if key doesn't exist
        }
        // Remove working directory if force is true
        if force {
            let workdir = self.repo.workdir()
                .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
            let submodule_path = workdir.join(path);

            if submodule_path.exists() {
                std::fs::remove_dir_all(&submodule_path)
                    .with_context(|| format!("Failed to remove submodule directory: {}", path))?;
            }
        }
        Ok(())
    }
    fn get_submodule_status(&self, path: &str) -> Result<DetailedSubmoduleStatus> {
        let submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;
        let name = submodule.name().unwrap_or(path).to_string();
        let url = submodule.url().map(|u| u.to_string());

        // Get status
        let status = self.repo.submodule_status(path, git2::SubmoduleIgnore::Unspecified)?;
        let status_flags = self.convert_git2_status_to_flags(status);
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
            git2::SubmoduleStatus::WD_MODIFIED |
            git2::SubmoduleStatus::WD_INDEX_MODIFIED |
            git2::SubmoduleStatus::WD_WD_MODIFIED
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
    fn list_submodules(&self) -> Result<Vec<String>> {
        let submodules = self.repo.submodules()?;
        let paths = submodules
            .iter()
            .map(|sm| sm.path().to_string_lossy().to_string())
            .collect();
        Ok(paths)
    }
    fn fetch_submodule(&self, path: &str) -> Result<()> {
        let submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;
        // Open the submodule repository
        let sub_repo = submodule.open()
            .with_context(|| format!("Failed to open submodule repository: {}", path))?;
        // Find the origin remote
        let mut remote = sub_repo.find_remote("origin")
            .with_context(|| format!("Failed to find origin remote for submodule: {}", path))?;
        // Fetch from origin
        remote.fetch(&[] as &[&str], None, None)
            .with_context(|| format!("Failed to fetch submodule: {}", path))?;
        Ok(())
    }
    fn reset_submodule(&self, path: &str, hard: bool) -> Result<()> {
        let submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;
        // Open the submodule repository
        let sub_repo = submodule.open()
            .with_context(|| format!("Failed to open submodule repository: {}", path))?;
        // Get HEAD commit
        let head = sub_repo.head()?;
        let commit = head.peel_to_commit()?;
        // Reset to HEAD
        let reset_type = if hard {
            git2::ResetType::Hard
        } else {
            git2::ResetType::Soft
        };
        sub_repo.reset(&commit.as_object(), reset_type, None)
            .with_context(|| format!("Failed to reset submodule: {}", path))?;
        Ok(())
    }
    fn clean_submodule(&self, path: &str, force: bool, remove_directories: bool) -> Result<()> {
        let submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;
        // Open the submodule repository
        let sub_repo = submodule.open()
            .with_context(|| format!("Failed to open submodule repository: {}", path))?;
        // Get status to find untracked files
        let mut status_opts = git2::StatusOptions::new();
        status_opts.include_untracked(true);
        status_opts.include_ignored(false);
        let statuses = sub_repo.statuses(Some(&mut status_opts))?;
        // Remove untracked files
        for entry in statuses.iter() {
            if entry.status().is_wt_new() {
                if let Some(file_path) = entry.path() {
                    let full_path = sub_repo.workdir()
                        .ok_or_else(|| anyhow::anyhow!("Submodule has no working directory"))?
                        .join(file_path);
                    if full_path.is_file() {
                        if force {
                            std::fs::remove_file(&full_path)
                                .with_context(|| format!("Failed to remove file: {}", full_path.display()))?;
                        }
                    } else if full_path.is_dir() && remove_directories {
                        if force {
                            std::fs::remove_dir_all(&full_path)
                                .with_context(|| format!("Failed to remove directory: {}", full_path.display()))?;
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn stash_submodule(&self, path: &str, include_untracked: bool) -> Result<()> {
        let submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;
        // Open the submodule repository
        let mut sub_repo = submodule.open()
            .with_context(|| format!("Failed to open submodule repository: {}", path))?;
        // Create stash
        let signature = sub_repo.signature()
            .or_else(|_| git2::Signature::now("submod", "submod@localhost"))?;
        let mut stash_flags = git2::StashFlags::DEFAULT;
        if include_untracked {
            stash_flags |= git2::StashFlags::INCLUDE_UNTRACKED;
        }
        sub_repo.stash_save(&signature, "submod stash", Some(stash_flags))
            .with_context(|| format!("Failed to stash changes in submodule: {}", path))?;
        Ok(())
    }
    fn enable_sparse_checkout(&self, path: &str) -> Result<()> {
        let submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;
        // Open the submodule repository
        let sub_repo = submodule.open()
            .with_context(|| format!("Failed to open submodule repository: {}", path))?;
        // Enable sparse checkout in config
        let mut config = sub_repo.config()?;
        config.set_bool("core.sparseCheckout", true)
            .with_context(|| format!("Failed to enable sparse checkout for submodule: {}", path))?;
        Ok(())
    }
    fn set_sparse_patterns(&self, path: &str, patterns: &[String]) -> Result<()> {
        let submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;
        // Open the submodule repository
        let sub_repo = submodule.open()
            .with_context(|| format!("Failed to open submodule repository: {}", path))?;
        // Write patterns to .git/info/sparse-checkout
        let git_dir = sub_repo.path();
        let sparse_checkout_file = git_dir.join("info").join("sparse-checkout");
        // Create info directory if it doesn't exist
        if let Some(parent) = sparse_checkout_file.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create info directory for submodule: {}", path))?;
        }
        // Write patterns
        let content = patterns.join("\n");
        std::fs::write(&sparse_checkout_file, content)
            .with_context(|| format!("Failed to write sparse checkout patterns for submodule: {}", path))?;
        Ok(())
    }
    fn get_sparse_patterns(&self, path: &str) -> Result<Vec<String>> {
        let submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;
        // Open the submodule repository
        let sub_repo = submodule.open()
            .with_context(|| format!("Failed to open submodule repository: {}", path))?;
        // Read patterns from .git/info/sparse-checkout
        let git_dir = sub_repo.path();
        let sparse_checkout_file = git_dir.join("info").join("sparse-checkout");
        if !sparse_checkout_file.exists() {
            return Ok(Vec::new());
        }
        let content = std::fs::read_to_string(&sparse_checkout_file)
            .with_context(|| format!("Failed to read sparse checkout patterns for submodule: {}", path))?;
        let patterns = content
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();
        Ok(patterns)
    }
    fn apply_sparse_checkout(&self, _path: &str) -> Result<()> {
        // git2 doesn't have direct sparse checkout application
        // We need to use gix_command or implement it manually
        // For now, return an error to indicate this needs manual implementation
        Err(anyhow::anyhow!(
            "git2 sparse checkout application not implemented, consider using gix_command"
        ))
    }
}
impl Git2Operations {
    /// Get sparse checkout information for a submodule
    fn get_sparse_checkout_info(&self, path: &str) -> Result<(bool, Vec<String>)> {
        let submodule = self.repo.find_submodule(path)
            .with_context(|| format!("Submodule not found: {}", path))?;
        // Open the submodule repository
        let sub_repo = submodule.open()
            .with_context(|| format!("Failed to open submodule repository: {}", path))?;
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
