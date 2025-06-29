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
    ConfigLevel,
};
use crate::config::{SubmoduleAddOptions, SubmoduleEntry, SubmoduleEntries, SubmoduleUpdateOptions};

/// Primary implementation using gix (gitoxide)
pub struct GixOperations {
    repo: gix::Repository,
}
impl GixOperations {
    /// Create a new GixOperations instance
    pub fn new(repo_path: Option<&Path>) -> Result<Self> {
        let repo = match repo_path {
            Some(path) => gix::open(path)
                .with_context(|| format!("Failed to open repository at {}", path.display()))?,
            None => gix::discover(".")
                .with_context(|| "Failed to discover repository in current directory")?,
        };
        Ok(Self { repo })
    }
    /// Try to perform operation with gix, return error if not supported
    fn try_gix_operation<T, F>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&gix::Repository) -> Result<T>,
    {
        operation(&self.repo)
    }
    /// Convert gix submodule to our SubmoduleEntry format
    fn convert_gix_submodule_to_entry(
        &self,
        submodule: &gix::Submodule,
    ) -> Result<SubmoduleEntry> {
        let _name = submodule.name().to_string();
        let path = submodule.path()?.to_string();
        let url = submodule.url()?.to_string();

        // TODO: gix doesn't expose submodule config directly yet
        // For now, use default values - this will fall back to git2
        let branch = None;
        let ignore = None;
        let update = None;
        let fetch_recurse = None;
        let active = true; // Default to active
        Ok(SubmoduleEntry {
            path: Some(path),
            url: Some(url),
            branch,
            ignore,
            update,
            fetch_recurse,
            active: Some(active),
            shallow: Some(false), // gix doesn't expose shallow info directly
        })
    }
    /// Convert gix submodule status to our status flags
    fn convert_gix_status_to_flags(&self, status: &gix::submodule::Status) -> SubmoduleStatusFlags {
        let mut flags = SubmoduleStatusFlags::empty();
        // Map gix status to our flags
        // Note: This is a simplified mapping as gix status structure may differ
        if status.is_dirty() == Some(true) {
            flags |= SubmoduleStatusFlags::WD_WD_MODIFIED;
        }
        // Add more mappings as needed based on gix::submodule::Status structure
        flags
    }
}
impl GitOperations for GixOperations {
    fn read_gitmodules(&self) -> Result<SubmoduleEntries> {
        self.try_gix_operation(|repo| {
            let mut submodules = HashMap::new();
            // Use gix::Repository::submodules() to get iterator over submodules
            if let Some(submodule_iter) = repo.submodules()? {
                for submodule in submodule_iter {
                    let name = submodule.name().to_string();
                    let entry = self.convert_gix_submodule_to_entry(&submodule)?;
                    submodules.insert(name, entry);
                }
            }
            Ok(SubmoduleEntries::new(
                if submodules.is_empty() { None } else { Some(submodules) },
                None, // Will be populated separately if needed
            ))
        })
    }
    fn write_gitmodules(&mut self, _config: &SubmoduleEntries) -> Result<()> {
        // gix doesn't have direct .gitmodules writing yet
        Err(anyhow::anyhow!(
            "gix .gitmodules writing not yet supported, falling back to git2"
        ))
    }
    fn read_git_config(&self, _level: ConfigLevel) -> Result<GitConfig> {
        // gix config reading is complex and not fully implemented yet
        Err(anyhow::anyhow!(
            "gix config reading not yet fully supported, falling back to git2"
        ))
    }
    fn write_git_config(&self, _config: &GitConfig, _level: ConfigLevel) -> Result<()> {
        // gix config writing is limited, fall back to git2
        Err(anyhow::anyhow!(
            "gix config writing not yet fully supported, falling back to git2"
        ))
    }
    fn set_config_value(&self, _key: &str, _value: &str, _level: ConfigLevel) -> Result<()> {
        // gix config writing is limited, fall back to git2
        Err(anyhow::anyhow!(
            "gix config value setting not yet fully supported, falling back to git2"
        ))
    }
    fn add_submodule(&mut self, _opts: &SubmoduleAddOptions) -> Result<()> {
        // gix doesn't support submodule addition yet
        Err(anyhow::anyhow!(
            "gix submodule addition not yet supported, falling back to git2"
        ))
    }
    fn init_submodule(&mut self, _path: &str) -> Result<()> {
        // gix doesn't support submodule initialization yet
        Err(anyhow::anyhow!(
            "gix submodule initialization not yet supported, falling back to git2"
        ))
    }
    fn update_submodule(&mut self, _path: &str, _opts: &SubmoduleUpdateOptions) -> Result<()> {
        // gix doesn't support submodule updates yet
        Err(anyhow::anyhow!(
            "gix submodule updates not yet supported, falling back to git2"
        ))
    }
    fn delete_submodule(&self, _path: &str) -> Result<()> {
        // gix doesn't support submodule deletion yet
        Err(anyhow::anyhow!(
            "gix submodule deletion not yet supported, falling back to git2"
        ))
    }
    fn deinit_submodule(&self, _path: &str, _force: bool) -> Result<()> {
        // gix doesn't support submodule deinitialization yet
        Err(anyhow::anyhow!(
            "gix submodule deinitialization not yet supported, falling back to git2"
        ))
    }
    fn get_submodule_status(&self, _path: &str) -> Result<DetailedSubmoduleStatus> {
        // gix submodule status is complex and not fully implemented yet
        // Return an error to trigger git2 fallback
        Err(anyhow::anyhow!("gix submodule status not yet fully supported"))
    }
    fn list_submodules(&self) -> Result<Vec<String>> {
        self.try_gix_operation(|repo| {
            let mut submodule_paths = Vec::new();
            if let Some(submodule_iter) = repo.submodules()? {
                for submodule in submodule_iter {
                    let path = submodule.path()?.to_string();
                    submodule_paths.push(path);
                }
            }
            Ok(submodule_paths)
        })
    }
    fn fetch_submodule(&self, _path: &str) -> Result<()> {
        // gix doesn't support submodule fetching yet
        Err(anyhow::anyhow!(
            "gix submodule fetching not yet supported, falling back to git2"
        ))
    }
    fn reset_submodule(&self, _path: &str, _hard: bool) -> Result<()> {
        // gix doesn't support submodule reset yet
        Err(anyhow::anyhow!(
            "gix submodule reset not yet supported, falling back to git2"
        ))
    }
    fn clean_submodule(&self, _path: &str, _force: bool, _remove_directories: bool) -> Result<()> {
        // gix doesn't support submodule cleaning yet
        Err(anyhow::anyhow!(
            "gix submodule cleaning not yet supported, falling back to git2"
        ))
    }
    fn stash_submodule(&self, _path: &str, _include_untracked: bool) -> Result<()> {
        // gix doesn't support stashing yet
        Err(anyhow::anyhow!(
            "gix stashing not yet supported, falling back to git2"
        ))
    }
    fn enable_sparse_checkout(&self, _path: &str) -> Result<()> {
        // gix doesn't support sparse checkout operations yet
        Err(anyhow::anyhow!(
            "gix sparse checkout not yet supported, falling back to git2"
        ))
    }
    fn set_sparse_patterns(&self, _path: &str, _patterns: &[String]) -> Result<()> {
        // gix doesn't support sparse checkout operations yet
        Err(anyhow::anyhow!(
            "gix sparse checkout not yet supported, falling back to git2"
        ))
    }
    fn get_sparse_patterns(&self, _path: &str) -> Result<Vec<String>> {
        // gix doesn't support sparse checkout operations yet
        Err(anyhow::anyhow!(
            "gix sparse checkout not yet supported, falling back to git2"
        ))
    }
    fn apply_sparse_checkout(&self, _path: &str) -> Result<()> {
        // gix doesn't support sparse checkout operations yet
        Err(anyhow::anyhow!(
            "gix sparse checkout not yet supported, falling back to git2"
        ))
    }
}
