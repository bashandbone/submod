// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use gix::bstr::ByteSlice;

/// Simple glob pattern matching for sparse checkout patterns
fn simple_glob_match(pattern: &str, text: &str) -> bool {
    // Very basic glob matching - just handle * wildcard
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let prefix = parts[0];
            let suffix = parts[1];
            text.starts_with(prefix) && text.ends_with(suffix)
        } else {
            false // More complex patterns not supported
        }
    } else {
        text == pattern
    }
}
use super::{
    DetailedSubmoduleStatus, GitConfig, GitOperations, SubmoduleStatusFlags,
};
use crate::options::{
    ConfigLevel, GitmodulesConvert,
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
    /// Convert gix_submodule File + name to our SubmoduleEntry format
    fn convert_gix_submodule_to_entry(
        &self,
        submodule_file: &gix_submodule::File,
        name: &str,
    ) -> Result<SubmoduleEntry> {
        // Extract basic information from the submodule file
        let name_bstr = name.as_bytes().as_bstr();
        let path = submodule_file.path(name_bstr).ok().map(|p| p.to_string());
        let url = submodule_file.url(name_bstr).ok().map(|u| u.to_string());

        // Convert gix_submodule types to our serializable types
        let branch = submodule_file.branch(name_bstr).ok().and_then(|b| {
            use crate::options::SerializableBranch;
            match b {
                Some(gix_submodule::config::Branch::Name(name)) => Some(SerializableBranch::Name(name.to_string())),
                Some(gix_submodule::config::Branch::CurrentInSuperproject) => Some(SerializableBranch::CurrentInSuperproject),
                None => None,
            }
        });
        
        let ignore = submodule_file.ignore(name_bstr).ok().and_then(|i| {
            use crate::options::SerializableIgnore;
            match i {
                Some(gix_submodule::config::Ignore::None) => Some(SerializableIgnore::None),
                Some(gix_submodule::config::Ignore::Untracked) => Some(SerializableIgnore::Untracked),
                Some(gix_submodule::config::Ignore::Dirty) => Some(SerializableIgnore::Dirty),
                Some(gix_submodule::config::Ignore::All) => Some(SerializableIgnore::All),
                None => None,
            }
        });
        
        let update = submodule_file.update(name_bstr).ok().and_then(|u| {
            use crate::options::SerializableUpdate;
            match u {
                Some(gix_submodule::config::Update::Checkout) => Some(SerializableUpdate::Checkout),
                Some(gix_submodule::config::Update::Rebase) => Some(SerializableUpdate::Rebase),
                Some(gix_submodule::config::Update::Merge) => Some(SerializableUpdate::Merge),
                Some(gix_submodule::config::Update::None) => Some(SerializableUpdate::None),
                Some(gix_submodule::config::Update::Command(_)) => Some(SerializableUpdate::Unspecified),
                None => None,
            }
        });
        
        let fetch_recurse = submodule_file.fetch_recurse(name_bstr).ok().and_then(|fr| {
            use crate::options::SerializableFetchRecurse;
            match fr {
                Some(gix_submodule::config::FetchRecurse::Always) => Some(SerializableFetchRecurse::Always),
                Some(gix_submodule::config::FetchRecurse::OnDemand) => Some(SerializableFetchRecurse::OnDemand),
                Some(gix_submodule::config::FetchRecurse::Never) => Some(SerializableFetchRecurse::Never),
                None => None,
            }
        });

        // Check if submodule is active and shallow
        let active = Some(true); // TODO: Implement proper active check
        let shallow = submodule_file.shallow(name_bstr).ok().flatten();

        Ok(SubmoduleEntry {
            path,
            url,
            branch,
            ignore,
            update,
            fetch_recurse,
            active,
            shallow,
            no_init: Some(false), // This is a runtime flag, not stored in .gitmodules
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
            let gitmodules_path = repo.workdir()
                .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?
                .join(".gitmodules");

            if !gitmodules_path.exists() {
                return Ok(SubmoduleEntries::default());
            }

            let content = std::fs::read(&gitmodules_path)?;
            let config = repo.config_snapshot();
            let submodule_file = gix_submodule::File::from_bytes(&content, Some(gitmodules_path), &config)?;

            // Convert gix_submodule entries to our SubmoduleEntry format
            let mut submodules = HashMap::new();
            
            // Iterate through submodule names and get their properties
            for name in submodule_file.names() {
                let name_str = name.to_str().map_err(|_| anyhow::anyhow!("Invalid UTF-8 in submodule name"))?;
                let entry = self.convert_gix_submodule_to_entry(&submodule_file, name_str)?;
                submodules.insert(name_str.to_string(), entry);
            }

            Ok(SubmoduleEntries::new(
                if submodules.is_empty() { None } else { Some(submodules) },
                None, // Will be populated separately if needed
            ))
        })
    }
    fn write_gitmodules(&mut self, config: &SubmoduleEntries) -> Result<()> {
        self.try_gix_operation(|repo| {
            let mut git_config = gix::config::File::new(gix::config::file::Metadata::api());

            // Convert SubmoduleEntries to gix config format
            for (name, entry) in config.submodule_iter() {
                let subsection_name = name.as_bytes().as_bstr();
                
                if let Some(path) = &entry.path {
                    git_config.set_raw_value_by("submodule", Some(subsection_name), "path", path.as_bytes().as_bstr())?;
                }
                if let Some(url) = &entry.url {
                    git_config.set_raw_value_by("submodule", Some(subsection_name), "url", url.as_bytes().as_bstr())?;
                }
                if let Some(branch) = &entry.branch {
                    let value = branch.to_string();
                    git_config.set_raw_value_by("submodule", Some(subsection_name), "branch", value.as_bytes().as_bstr())?;
                }
                if let Some(update) = &entry.update {
                    let value = update.to_gitmodules();
                    git_config.set_raw_value_by("submodule", Some(subsection_name), "update", value.as_bytes().as_bstr())?;
                }
                if let Some(ignore) = &entry.ignore {
                    let value = ignore.to_gitmodules();
                    git_config.set_raw_value_by("submodule", Some(subsection_name), "ignore", value.as_bytes().as_bstr())?;
                }
                if let Some(fetch_recurse) = &entry.fetch_recurse {
                    let value = fetch_recurse.to_gitmodules();
                    git_config.set_raw_value_by("submodule", Some(subsection_name), "fetchRecurseSubmodules", value.as_bytes().as_bstr())?;
                }
            }

            // Write to .gitmodules file
            let gitmodules_path = repo.workdir()
                .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?
                .join(".gitmodules");

            let mut file = std::fs::File::create(&gitmodules_path)?;
            git_config.write_to(&mut file)?;
            Ok(())
        })
    }
    fn read_git_config(&self, level: ConfigLevel) -> Result<GitConfig> {
        self.try_gix_operation(|repo| {
            let config_snapshot = repo.config_snapshot();
            let mut entries = HashMap::new();

            // Filter by configuration level
            let source_filter = match level {
                ConfigLevel::System => gix::config::Source::System,
                ConfigLevel::Global => gix::config::Source::User,
                ConfigLevel::Local => gix::config::Source::Local,
                ConfigLevel::Worktree => gix::config::Source::Worktree,
            };

            // Extract entries from the specified level
            for section in config_snapshot.sections() {
                if section.meta().source == source_filter {
                    let section_name = section.header().name();
                    // For now, use a simplified approach to extract key-value pairs
                    // The exact iteration method may vary based on gix version
                    // This is a placeholder that will need adjustment based on actual API
                    entries.insert(format!("{}.placeholder", section_name), "placeholder".to_string());
                }
            }

            Ok(GitConfig { entries })
        })
    }
    fn write_git_config(&self, _config: &GitConfig, _level: ConfigLevel) -> Result<()> {
        // For now, fall back to git2 to avoid complex lifetime issues with gix_config
        // This can be improved later when gix_config API is more stable
        Err(anyhow::anyhow!(
            "gix config writing has lifetime complexities, falling back to git2"
        ))
    }
    fn set_config_value(&self, _key: &str, _value: &str, _level: ConfigLevel) -> Result<()> {
        // For now, fall back to git2 to avoid complex lifetime issues with gix_config
        // This can be improved later when gix_config API is more stable
        Err(anyhow::anyhow!(
            "gix config writing has lifetime complexities, falling back to git2"
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
        self.try_gix_operation(|repo| {
            // Set core.sparseCheckout = true in repository config
            self.set_config_value("core.sparseCheckout", "true", ConfigLevel::Local)?;

            // Create sparse-checkout file if it doesn't exist
            let sparse_checkout_path = repo.git_dir().join("info").join("sparse-checkout");
            if !sparse_checkout_path.exists() {
                std::fs::create_dir_all(sparse_checkout_path.parent().unwrap())?;
                std::fs::write(&sparse_checkout_path, "/*\n")?; // Default to include everything
            }

            Ok(())
        })
    }
    fn set_sparse_patterns(&self, _path: &str, patterns: &[String]) -> Result<()> {
        self.try_gix_operation(|repo| {
            let sparse_checkout_path = repo.git_dir().join("info").join("sparse-checkout");
            let content = patterns.join("\n") + "\n";
            std::fs::write(&sparse_checkout_path, content)?;
            Ok(())
        })
    }
    fn get_sparse_patterns(&self, _path: &str) -> Result<Vec<String>> {
        self.try_gix_operation(|repo| {
            let sparse_checkout_path = repo.git_dir().join("info").join("sparse-checkout");
            if !sparse_checkout_path.exists() {
                return Ok(vec![]);
            }

            let content = std::fs::read_to_string(&sparse_checkout_path)?;
            Ok(content.lines().map(|s| s.to_string()).collect())
        })
    }
    fn apply_sparse_checkout(&self, _path: &str) -> Result<()> {
        self.try_gix_operation(|repo| {
            // Get sparse checkout patterns
            let patterns = self.get_sparse_patterns(_path)?;
            if patterns.is_empty() {
                return Ok(()); // No patterns to apply
            }

            // Load the index
            let index_path = repo.git_dir().join("index");
            let _index = gix::index::File::at(
                &index_path,
                gix::hash::Kind::Sha1,
                false,
                gix::index::decode::Options::default(),
            )?;

            // Use a simpler approach since remove_entries closure signature is complex
            // Fall back to git2 for now for sparse checkout application
            Err(anyhow::anyhow!(
                "gix sparse checkout application is complex, falling back to git2"
            ))
        })
    }
}
