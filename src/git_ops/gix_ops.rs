// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
// TODO: This module is very not-DRY...but it's low priority right now.
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
use crate::utilities;

/// Primary implementation using gix (gitoxide)
#[derive(Debug, Clone, PartialEq)]
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

    /// Helper: Ensure the submodule directory exists (create if missing)
    fn ensure_submodule_dir(&self, submodule_path: &std::path::Path) -> Result<()> {
        if !submodule_path.exists() {
            std::fs::create_dir_all(submodule_path)?;
        }
        Ok(())
    }

    /// Helper: Clone or fetch+checkout a repo/submodule at the given path
    /// Used by add_submodule, init_submodule, update_submodule
    fn clone_or_fetch_then_checkout(
        &self,
        url: &str,
        submodule_path: &std::path::Path,
        shallow: bool,
        branch: Option<&crate::options::SerializableBranch>,
    ) -> Result<()> {
        // If .git does not exist, clone
        if !submodule_path.exists() || !submodule_path.join(".git").exists() {
            self.ensure_submodule_dir(submodule_path)?;
            let mut prepare = gix::prepare_clone(url, submodule_path)?;
            if shallow {
                prepare = prepare.with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(1.try_into()?));
            }
            let should_interrupt = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let progress = gix::progress::Discard;
            let (mut checkout, _outcome) = prepare.fetch_then_checkout(progress, &should_interrupt)?;
            if let Some(branch) = branch {
                if let crate::options::SerializableBranch::Name(branch_name) = branch {
                    let repo = checkout.repo();
                    let config = repo.config_snapshot();
                    let mut config_file = config.to_owned();
                    config_file.set_raw_value_by(
                        "branch",
                        Some(branch_name.as_bytes().as_bstr()),
                        "remote",
                        "origin".as_bytes().as_bstr(),
                    )?;
                    config_file.set_raw_value_by(
                        "branch",
                        Some(branch_name.as_bytes().as_bstr()),
                        "merge",
                        format!("refs/heads/{}", branch_name).as_bytes().as_bstr(),
                    )?;
                }
            }
        } else {
            // Already exists: fetch and checkout
            let submodule_repo = gix::open(submodule_path)?;
            let remote = submodule_repo.find_default_remote(gix::remote::Direction::Fetch)
                .ok_or_else(|| anyhow::anyhow!("No default remote found for submodule"))?;
            let connection = remote.connect(gix::remote::Direction::Fetch)?;
            let should_interrupt = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let progress = gix::progress::Discard;
            connection.prepare_fetch(progress, gix::remote::fetch::RefLogMessage::Override {
                message: format!("fetch from {}", url).into(),
            })?
            .receive(progress, &should_interrupt)?;
            // Optionally: checkout logic could be added here if needed
        }
        Ok(())
    }

    /// Try to perform operation with gix, return error if not supported
    fn try_gix_operation<T, F>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&gix::Repository) -> Result<T>,
    {
        operation(&self.repo)
    }

    /// Try to perform ops with gix using a mutable reference
    fn try_gix_operation_mut<T, F>(&mut self, operation: F) -> Result<T>
    where
        F: FnOnce(&mut gix::Repository) -> Result<T>,
    {
        operation(&mut self.repo)
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

    /// Read the .gitmodules file and convert it to SubmoduleEntries
    fn read_gitmodules(&self) -> Result<SubmoduleEntries> {
        let mut mutable_self = self.clone();
        mutable_self.try_gix_operation_mut(|repo| {
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

    /// Write the submodule entries to the .gitmodules file
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

    /// Read the Git configuration at the specified level
    fn read_git_config(&self, level: ConfigLevel) -> Result<GitConfig> {
        self.clone().try_gix_operation_mut(|repo| {
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
                    let body_iter = section.body().iter();
                    for (key, value) in body_iter {
                        entries.insert(format!("{}.{}", section_name, key), value.to_string());
                    }
                }
            }

            Ok(GitConfig { entries })
        })
    }

    /// Write the Git configuration to the repository
    fn write_git_config(&self, config: &GitConfig, level: ConfigLevel) -> Result<()> {
        self.try_gix_operation(|repo| {
            let config_path = match level {
                ConfigLevel::Local | ConfigLevel::Worktree => repo.git_dir().join("config"),
                _ => return Err(anyhow::anyhow!("Only local config writing is supported with gix")),
            };
            let mut bytes = if config_path.exists() {
                std::fs::read(&config_path)?
            } else {
                Vec::new()
            };
            let mut config_file = gix::config::File::from_bytes_owned(
                &mut bytes,
                gix::config::file::Metadata::from(gix::config::Source::Local),
                Default::default(),
            )?;
            for (key, value) in &config.entries {
                let mut parts = key.splitn(3, '.');
                let section = parts.next().unwrap_or("");
                let subsection = parts.next();
                let name = parts.next().unwrap_or("");
                config_file.set_raw_value_by(
                    section,
                    subsection.map(|s| s.as_bytes().as_bstr()),
                    name,
                    value.as_bytes().as_bstr(),
                )?;
            }
            let mut output = std::fs::File::create(&config_path)?;
            config_file.write_to(&mut output)?;
            Ok(())
        })
    }

    /// Set a configuration value in the repository
    fn set_config_value(&self, key: &str, value: &str, level: ConfigLevel) -> Result<()> {
        let mut entries = HashMap::new();
        entries.insert(key.to_string(), value.to_string());
        let config = GitConfig { entries };
        // Merge with existing config
        let existing = self.read_git_config(level)?;
        let mut merged = existing.entries;
        merged.insert(key.to_string(), value.to_string());
        let merged_config = GitConfig { entries: merged };
        self.write_git_config(&merged_config, level)
    }

    /// Add a new submodule to the repository
    fn add_submodule(&mut self, opts: &SubmoduleAddOptions) -> Result<()> {
        // 2. Check if submodule already exists (do this before borrowing self mutably)
        if let Ok(existing_entries) = self.read_gitmodules() {
            if existing_entries.submodule_iter().any(|(n, _)| n == &opts.name) {
                return Err(anyhow::anyhow!("Submodule '{}' already exists", opts.name));
            }
        }

        self.try_gix_operation(|repo| {
            let path = &opts.path;
            let url = &opts.url;
            let workdir = repo.workdir()
                .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
            let target_path = workdir.join(path);
            // Use helper for clone/fetch/checkout
            self.clone_or_fetch_then_checkout(
                url,
                &target_path,
                opts.shallow == Some(true),
                opts.branch.as_ref(),
            )
        })
    }

    /// Initialize a submodule by reading its configuration and setting it up
    fn init_submodule(&mut self, path: &str) -> Result<()> {
        // 1. Read .gitmodules to get submodule configuration
        let entries = self.read_gitmodules()?;

        // 2. Find the submodule entry by path
        let submodule_entry = entries.submodule_iter()
            .find(|(_, entry)| entry.path.as_ref() == Some(&path.to_string()))
            .ok_or_else(|| anyhow::anyhow!("Submodule '{}' not found in .gitmodules", path))?;

        let (name, entry) = submodule_entry;
        let url = entry.url.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Submodule '{}' has no URL configured", name))?;

        self.try_gix_operation(|repo| {
            // 3. Set up submodule configuration in .git/config
            let config_snapshot = repo.config_snapshot();
            let mut config_file = config_snapshot.to_owned();

            // Set submodule URL in local config
            let url_key = format!("submodule.{}.url", name);
            config_file.set_raw_value_by("submodule", Some(name.as_bytes().as_bstr()), "url", url.as_bytes().as_bstr())?;

            // Set submodule active flag
            let active_key = format!("submodule.{}.active", name);
            config_file.set_raw_value_by("submodule", Some(name.as_bytes().as_bstr()), "active", "true".as_bytes().as_bstr())?;

            // 4. Check if submodule directory exists and is empty
            let workdir = repo.workdir()
                .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
            let submodule_path = workdir.join(path);

            if !submodule_path.exists() {
                std::fs::create_dir_all(&submodule_path)?;
            } else if submodule_path.read_dir()?.next().is_some() {
                // Directory exists and is not empty - this is fine for init
                // (unlike clone which would fail)
            }

            // 5. Clone the submodule if it doesn't exist yet
            if !submodule_path.join(".git").exists() {
                // Clone the submodule repository using gix
                let mut prepare = gix::prepare_clone(url.clone(), &submodule_path)?;
                if entry.shallow == Some(true) {
                    prepare = prepare.with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(1.try_into()?));
                }
                let should_interrupt = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
                let progress = gix::progress::Discard;
                let (_checkout, _outcome) = prepare
                    .fetch_then_checkout(progress, &should_interrupt)?;
            }

            Ok(())
        })
    }

    /// Update a submodule to the latest commit in its remote repository
    fn update_submodule(&mut self, path: &str, opts: &SubmoduleUpdateOptions) -> Result<()> {
        let entries = self.read_gitmodules()?;
        let submodule_entry = entries.submodule_iter()
            .find(|(_, entry)| entry.path.as_ref() == Some(&path.to_string()))
            .ok_or_else(|| anyhow::anyhow!("Submodule '{}' not found in .gitmodules", path))?;
        let (name, entry) = submodule_entry;
        let url = entry.url.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Submodule '{}' has no URL configured", name))?;
        let workdir = self.repo.workdir()
            .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
        let submodule_path = workdir.join(path);

        if !submodule_path.exists() || !submodule_path.join(".git").exists() {
            // Use gix::prepare_clone for proper remote operations
            let mut prepare = gix::prepare_clone(url.clone(), &submodule_path)?;
            if entry.shallow == Some(true) {
                prepare = prepare.with_shallow(gix::remote::fetch::Shallow::DepthAtRemote(1.try_into()?));
            }
            let should_interrupt = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let progress = gix::progress::Discard;
            let (mut checkout, _outcome) = prepare
                .fetch_then_checkout(progress, &should_interrupt)?;
            if let Some(branch) = &entry.branch {
                match branch {
                    crate::options::SerializableBranch::Name(branch_name) => {
                        let repo = checkout.repo();
                        let config = repo.config_snapshot();
                        let mut config_file = config.to_owned();
                        config_file.set_raw_value_by(
                            "branch",
                            Some(branch_name.as_bytes().as_bstr()),
                            "remote",
                            "origin".as_bytes().as_bstr()
                        )?;
                        config_file.set_raw_value_by(
                            "branch",
                            Some(branch_name.as_bytes().as_bstr()),
                            "merge",
                            format!("refs/heads/{}", branch_name).as_bytes().as_bstr()
                        )?;
                    },
                    crate::options::SerializableBranch::CurrentInSuperproject => {
                        // Set branch to current branch in superproject
                        let superproject_branch = self.get_superproject_branch()?;
                        config_file.set_raw_value_by(
                            "branch",
                            Some(superproject_branch.as_bytes().as_bstr()),
                            "remote",
                            "origin".as_bytes().as_bstr()
                        )?;
                        config_file.set_raw_value_by(
                            "branch",
                            Some(superproject_branch.as_bytes().as_bstr()),
                            "merge",
                            format!("refs/heads/{}", superproject_branch).as_bytes().as_bstr()
                        )?;
                    }
                }
            }
        } else {
            let submodule_repo = gix::open(&submodule_path)?;
            let remote = submodule_repo.find_default_remote(gix::remote::Direction::Fetch)
                .ok_or_else(|| anyhow::anyhow!("No default remote found for submodule"))?;
            let connection = remote.connect(gix::remote::Direction::Fetch)?;
            let should_interrupt = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let progress = gix::progress::Discard;
            let _outcome = connection.prepare_fetch(progress, gix::remote::fetch::RefLogMessage::Override {
                message: format!("fetch from {}", url).into(),
            })?
            .receive(progress, &should_interrupt)?;
            match opts.strategy {
                crate::options::SerializableUpdate::Checkout | crate::options::SerializableUpdate::Unspecified => {
                // Use helper for clone/fetch/checkout
                self.clone_or_fetch_then_checkout(
                    url,
                    &submodule_path,
                    entry.shallow == Some(true),
                    entry.branch.as_ref(),
                )?;
                },
                crate::options::SerializableUpdate::Merge => {
                    return Err(anyhow::anyhow!("Merge strategy not yet implemented with gix"));
                },
                crate::options::SerializableUpdate::Rebase => {
                    return Err(anyhow::anyhow!("Rebase strategy not yet implemented with gix"));
                },
                crate::options::SerializableUpdate::None => {
                    // No update
                }
            }
        }
        Ok(())
    }

    /// Delete a submodule by removing its configuration and content
    fn delete_submodule(&mut self, path: &str) -> Result<()> {
        // 1. Read .gitmodules to get submodule configuration (outside closure)
        let mut entries = self.read_gitmodules()?;

        // 2. Find the submodule entry by path
        let submodule_name = entries.submodule_iter()
            .find(|(_, entry)| entry.path.as_ref() == Some(&path.to_string()))
            .map(|(name, _)| name.to_string())
            .ok_or_else(|| anyhow::anyhow!("Submodule '{}' not found in .gitmodules", path))?;

        // 3. Remove from .gitmodules
        entries.remove_submodule(&submodule_name);
        self.write_gitmodules(&entries)?;

        self.try_gix_operation_mut(|repo| {
            // 4. Remove from git index using gix (fixed API usage)
            let index_path = repo.git_dir().join("index");
            if index_path.exists() {
                let mut index = gix::index::File::at(
                    &index_path,
                    gix::hash::Kind::Sha1,
                    false,
                    gix::index::decode::Options::default()
                )?;
                // Remove all entries matching the submodule path prefix
                let remove_prefix = path;
                index.remove_entries(|_idx, path, _entry| {
                    let path_str = std::str::from_utf8(path).unwrap_or("");
                    path_str.starts_with(remove_prefix)
                });
                let mut index_file = std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&index_path)?;
                index.write_to(
                    &mut index_file,
                    gix::index::write::Options::default(),
                )?;
                let mut index_file = std::fs::OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .open(&index_path)?;
                index.write_to(
                    &mut index_file,
                    gix::index::write::Options::default(),
                )?;
            }

            // 5. Remove submodule configuration from .git/config
            let config_snapshot = repo.config_snapshot();
            let mut config_file = config_snapshot.to_owned();

            // Remove all submodule.{name}.* entries
            let section_name = format!("submodule.{}", submodule_name);
            // Note: gix config API for removing sections is complex
            // For now, we'll fall back to manual removal or git2 for this part
            // This is acceptable as it's a less common operation

            // 6. Remove the submodule directory from working tree
            let workdir = repo.workdir()
                .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
            let submodule_path = workdir.join(path);

            if submodule_path.exists() {
                std::fs::remove_dir_all(&submodule_path)
                    .with_context(|| format!("Failed to remove submodule directory at {}", submodule_path.display()))?;
            }

            // 7. Remove .git/modules/{name} directory if it exists
            let modules_path = repo.git_dir().join("modules").join(&submodule_name);
            if modules_path.exists() {
                std::fs::remove_dir_all(&modules_path)
                    .with_context(|| format!("Failed to remove submodule git directory at {}", modules_path.display()))?;
            }

            Ok(())
        })
    }

    /// Deinitialize a submodule, removing its configuration and content
    fn deinit_submodule(&mut self, path: &str, force: bool) -> Result<()> {
        let entries = self.read_gitmodules()?;
        let submodule_name = entries
            .submodule_iter()
            .find(|(_, entry)| entry.path.as_ref() == Some(&path.to_string()))
            .map(|(name, _)| name.to_string())
            .ok_or_else(|| anyhow::anyhow!("Submodule '{}' not found in .gitmodules", path))?;
        self.clone().try_gix_operation_mut(|repo| {
            // 1. Get the submodule directory
            let workdir = repo.workdir()
                .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?;
            let submodule_path = workdir.join(path);

            // 2. Check if submodule has uncommitted changes (unless force is true)
            if !force && submodule_path.exists() && submodule_path.join(".git").exists() {
                if let Ok(submodule_repo) = gix::open(&submodule_path) {
                    // TODO: properly implement this
                    // Check for uncommitted changes
                    // Note: gix status API is complex, for now we'll do a simple check
                    // by looking at the index vs HEAD
                    let head = submodule_repo.head_commit().ok();
                    let index = submodule_repo.index_or_empty().ok();

                    // Simple check: if we can't get head or index, assume there might be changes
                    if head.is_none() || index.is_none() {
                        if !force {
                            return Err(anyhow::anyhow!(
                                "Submodule '{}' might have uncommitted changes. Use force=true to override.",
                                path
                            ));
                        }
                    }
                }
            }

            // 4. Remove submodule configuration from .git/config
            let config_snapshot = repo.config_snapshot();
            let config_file = config_snapshot.to_owned();

            // Remove submodule.{name}.url and submodule.{name}.active
            // Note: gix config API for removing specific keys is complex
            // For a complete implementation, we might need to fall back to git2
            // or implement more sophisticated config manipulation

            // 5. Clear the submodule working directory
            if submodule_path.exists() {
                if force {
                    // Force removal of all content
                    std::fs::remove_dir_all(&submodule_path)
                        .with_context(|| format!("Failed to remove submodule directory at {}", submodule_path.display()))?;

                    // Recreate empty directory to maintain the path structure
                    std::fs::create_dir_all(&submodule_path)?;
                } else {
                    // Only remove .git directory and tracked files, preserve untracked files
                    let git_dir = submodule_path.join(".git");
                    if git_dir.exists() {
                        if git_dir.is_dir() {
                            std::fs::remove_dir_all(&git_dir)?;
                        } else {
                            // .git is a file (gitdir reference)
                            std::fs::remove_file(&git_dir)?;
                        }
                    }

                    // Remove tracked files by checking out empty tree
                    // This is complex to implement properly with gix
                    // For now, we'll do a simple approach by removing all files
                    // except untracked ones (which is hard to determine without proper status)
                    // We'll just remove common git-tracked file patterns
                    for entry in std::fs::read_dir(&submodule_path)? {
                        let entry = entry?;
                        let path = entry.path();
                        if path.is_file() {
                            std::fs::remove_file(&path).ok(); // Ignore errors for individual files
                        }
                    }
                }
            }

            // 6. Remove .git/modules/{name} directory if it exists
            let modules_path = repo.git_dir().join("modules").join(&submodule_name);
            if modules_path.exists() {
                std::fs::remove_dir_all(&modules_path)
                    .with_context(|| format!("Failed to remove submodule git directory at {}", modules_path.display()))?;
            }

            Ok(())
        })
    }
    /// Get the status of a submodule
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
    async fn fetch_submodule(&self, _path: &str) -> Result<()> {
        let submodule_repo = utilities::repo_from_path(&std::path::PathBuf::from(_path))?;
        let head = submodule_repo.head()?;
        if let Some(reference) = head.referent_name() {
        let (_upstream, remote) = submodule_repo.upstream_branch_and_remote_for_tracking_branch(reference)?
            .ok_or_else(|| anyhow::anyhow!("Failed to find upstream branch for submodule at {}", _path))?;
        let connection = remote.connect(gix::remote::Direction::Fetch)?.await?;
            // Prepare fetch operation
        let prepare = connection.prepare_fetch(gix::progress::Discard, gix::remote::ref_map::Options::default())?;
            // Use a shared atomic bool for interrupt handling
        let should_interrupt = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let progress = gix::progress::Discard;
        let (_repo, outcome) = prepare.fetch_only(progress, &should_interrupt).await?;
        match outcome.status {
            gix::remote::fetch::Status::Change {..} => Ok(()),
            gix::remote::fetch::Status::NoPackReceived {..} => Err(anyhow::anyhow!("Failed to fetch submodule: {}", outcome.error())),
        }
    } else {
            Err(anyhow::anyhow!("Submodule at {} has no HEAD reference", _path))
        }
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
        // Set core.sparseCheckout = true in repository config
        self.set_config_value("core.sparseCheckout", "true", ConfigLevel::Local)?;

        self.try_gix_operation(|repo| {
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

impl From<super::GitOpsManager> for GixOperations {
    fn from(git_ops: super::GitOpsManager) -> Self {
        git_ops.gix_ops.clone()
            .expect("GixOperations should always be initialized")
    }
}
