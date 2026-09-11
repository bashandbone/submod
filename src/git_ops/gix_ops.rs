// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
use anyhow::{Context, Result};

use std::collections::HashMap;
use std::path::Path;

use super::GitConfig;
use crate::config::SubmoduleEntries;
use crate::options::ConfigLevel;

/// Primary implementation using gix (gitoxide)
#[derive(Debug, Clone, PartialEq)]
pub struct GixOperations {
    repo: gix::Repository,
}
impl GixOperations {
    /// Create a read-only `GixOperations` instance.
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

    /// Try to perform ops with gix using a mutable reference
    fn try_gix_operation_mut<T, F>(&mut self, operation: F) -> Result<T>
    where
        F: FnOnce(&mut gix::Repository) -> Result<T>,
    {
        operation(&mut self.repo)
    }

    /// Convert gix submodule file to `SubmoduleEntries`
    #[allow(clippy::unused_self, clippy::redundant_closure_for_method_calls)]
    fn convert_gitmodules_to_entries(&self, gitmodules: gix_submodule::File) -> SubmoduleEntries {
        let as_config_file = gitmodules.into_config();
        let mut sections_map = std::collections::HashMap::new();
        for section in as_config_file.sections() {
            // we need to convert everything to String and add to map
            let mut section_entries = std::collections::HashMap::new();
            let name = section.header().subsection_name().map_or_else(
                || section.header().name().to_string(),
                |subsection| subsection.to_string(),
            );
            for (key, value) in section.body() {
                section_entries.insert(key.clone(), value.to_string());
            }
            sections_map.insert(name, section_entries);
        }
        crate::config::SubmoduleEntries::from_gitmodules(sections_map)
    }
}

impl GixOperations {
    /// Read `.gitmodules` registrations through gix (no mutation, no fetch).
    pub fn read_gitmodules(&self) -> Result<SubmoduleEntries> {
        let mutable_self = self.clone();
        mutable_self.try_gix_operation(|repo| {
            let gitmodules_path = repo
                .workdir()
                .ok_or_else(|| anyhow::anyhow!("Repository has no working directory"))?
                .join(".gitmodules");

            if !gitmodules_path.exists() {
                return Ok(SubmoduleEntries::default());
            }

            let content = std::fs::read(&gitmodules_path)?;
            let config = repo.config_snapshot();
            let submodule_file =
                gix_submodule::File::from_bytes(&content, Some(gitmodules_path), &config)?;

            Ok(mutable_self.convert_gitmodules_to_entries(submodule_file))
        })
    }

    /// Read Git configuration at `level` through gix (no mutation).
    pub fn read_git_config(&self, level: ConfigLevel) -> Result<GitConfig> {
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
                    for (key, value) in section.body() {
                        entries.insert(format!("{section_name}.{key}"), value.to_string());
                    }
                }
            }

            Ok(GitConfig { entries })
        })
    }

    /// List registered submodule paths through gix (no mutation, no fetch).
    pub fn list_submodules(&self) -> Result<Vec<String>> {
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
}
