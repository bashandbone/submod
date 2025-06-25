// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: MIT
// Licensed under the [Plain MIT License][../LICENSE.md]
#![doc = r"
Configuration types and utilities for submod.

Defines project-level defaults, and submodule
configuration management. Supports loading and saving configuration in TOML format.

Main Types:
- SubmoduleGitOptions: Git-specific options for a submodule.
- SubmoduleDefaults: Project-level default submodule options.
- SubmoduleConfig: Configuration for a single submodule.
- Config: Main configuration structure, containing defaults and all submodules.

Features:
- Load and save configuration from/to TOML files.
- Serialize/deserialize submodule options for config files.
- Manage submodule entries and defaults programmatically.

TODO:
- Add validation for config values when loading from file.
"]

use std::path::PathBuf;
use anyhow::{Context, Result};
use gix::index::extension::sparse;
use serde::{Deserialize, Serialize};
use std::fs;
use std::{collections::HashMap, path::Path};
use toml_edit::{DocumentMut, Item, Table, value};
use crate::options::{
    SerializableFetchRecurse, SerializableIgnore, SerializableUpdate,
};
use crate::options::SerializableBranch;

/// Returns true. Used as a serde default for boolean fields.
fn default_true() -> bool {
    true
}

/// Git options for a submodule
#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SubmoduleGitOptions {
    /// How to handle dirty files when updating submodules
    #[serde(default)]
    pub ignore: Option<SerializableIgnore>,
    /// Whether to fetch submodules recursively
    #[serde(default)]
    pub fetch_recurse: Option<SerializableFetchRecurse>,
    /// Branch to track for the submodule
    #[serde(default)]
    pub branch: Option<SerializableBranch>,
    /// Update strategy for the submodule
    #[serde(default)]
    pub update: Option<SerializableUpdate>,
}

/// Convert git submodule options to git2-compatible options
pub struct Git2SubmoduleOptions {
    ignore: git2::SubmoduleIgnore,
    update: git2::SubmoduleUpdate,
    branch: Option<String>,
    fetch_recurse: Option<String>,
}

/// Implementation for converting git2 submodule options
impl Git2SubmoduleOptions {
    pub fn new(
        ignore: git2::SubmoduleIgnore,
        update: git2::SubmoduleUpdate,
        branch: Option<String>,
        fetch_recurse: Option<String>,
    ) -> Self {
        Self {
            ignore,
            update,
            branch,
            fetch_recurse,
        }
    }
}

// Default implementation for [`SubmoduleGitOptions`]
impl SubmoduleGitOptions {

}

impl TryFrom<SubmoduleGitOptions> for Git2SubmoduleOptions {
    type Error = String;

    fn try_from(options: SubmoduleGitOptions) -> Result<Self, Self::Error> {
        let ignore = match options.ignore {
            Some(i) => git2::SubmoduleIgnore::try_from(i).map_err(|_| "Failed to convert SerializableIgnore to git2::SubmoduleIgnore".to_string())?,
            None => git2::SubmoduleIgnore::Unspecified,
        };
        let update = match options.update {
            Some(u) => git2::SubmoduleUpdate::try_from(u).map_err(|_| "Failed to convert SerializableUpdate to git2::SubmoduleUpdate".to_string())?,
            None => git2::SubmoduleUpdate::Default,
        };
        let branch = options.branch.map(|b| b.to_string());
        let fetch_recurse = options.fetch_recurse.map(|fr| fr.to_git_options());
        Ok(Self::new(ignore, update, branch, fetch_recurse))
    }
}

/// Project-level defaults for git submodule options (for all submodules)
/// Can be used to set global defaults for submodule behavior in the repository
/// And overridden by submodule-specific configurations
#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SubmoduleDefaults {
    /// [`Ignore`][SerializableIgnore] setting for submodules
    pub ignore: Option<SerializableIgnore>,
    /// [`Update`][SerializableUpdate] setting for submodules
    pub fetch_recurse: Option<SerializableFetchRecurse>,
    /// [`Update`][SerializableUpdate] setting for submodules
    pub update: Option<SerializableUpdate>,
}
impl SubmoduleDefaults {


    /// Returns a vector of SubmoduleDefaults with the current values (for comparison)
    pub fn get_values(&self) -> Vec<SubmoduleDefaults> {
        vec![self.clone()]
    }

    /// Merge another SubmoduleDefaults into this one. Only updates fields that are set in the other. Returns a new instance with the merged values.
    pub fn merge_from(&self, other: SubmoduleDefaults) -> Self {
        let mut mut_self = self.clone();
        if other.ignore.is_some() {
            mut_self.ignore = other.ignore;
        }
        if other.fetch_recurse.is_some() {
            mut_self.fetch_recurse = other.fetch_recurse;
        }
        if other.update.is_some() {
            mut_self.update = other.update;
        }
        {
            let ignore = mut_self.ignore;
            let update = mut_self.update;
            Self {
                ignore: ignore.or_else(|| Some(SerializableIgnore::default())),
                fetch_recurse: mut_self.fetch_recurse.or_else(|| Some(SerializableFetchRecurse::default())),
                update: update.or_else(|| Some(SerializableUpdate::default())),
            }
        }
    }
}

/// Configuration for a single submodule
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SubmoduleConfig {
    /// URL of the submodule repository. This can be either a remote (https, ssh, etc) or a *relative* local path like `../some/other/repo`.
    pub url: String,
    /// Where to put the submodule in the working directory (relative path)
    pub path: Option<String>,

    /// Git-specific options for this submodule
    #[serde(flatten)]
    pub git_options: SubmoduleGitOptions,
    /// Whether this submodule is active
    #[serde(default = "default_true")]
    pub active: bool,
    /// Whether to perform a shallow clone (depth == 1). Default is False.
    /// When true, only the last commit will be included in the submodule's history.
    pub shallow: bool,

    /// Sparse checkout paths for this submodule (relative paths)
    pub sparse_paths: Option<Vec<String>>,
}

impl SubmoduleConfig {
    /// Create a new submodule configuration with defaults
    pub fn new(url: String, path: Option<String>, git_options: SubmoduleGitOptions, active: Option<bool>, shallow: Option<bool>, sparse_paths: Option<Vec<String>>) -> Self {
        Self {
            url: url.clone(),
            path: path.or_else(|| Some(Self::name_from_url(&url))),
            git_options: SubmoduleGitOptions {
                ignore: git_options.ignore,
                fetch_recurse: git_options.fetch_recurse,
                branch: git_options.branch,
                update: git_options.update,
            },
            active: active.unwrap_or(true), // Default to true if not specified
            sparse_paths: sparse_paths.or_else(|| Some(Vec::new())),
            shallow: shallow.unwrap_or(false), // Default to false if not specified
        }
    }

    /// Derive the submodule name from the URL (e.g., last path component, stripping .git)
    pub fn name(&self) -> Option<String> {
        Some(Self::name_from_url(&self.url))
    }

    /// Returns true if the url is a local path (relative or absolute)
    pub fn is_local(&self) -> bool {
        let url = self.url.as_str();
        url.starts_with("./") || url.starts_with("../") || url.starts_with('/')
    }

    /// Returns true if the url is a remote repository (http, ssh, git, etc)
    pub fn is_remote(&self) -> bool {
        let url = self.url.as_str();
        url.starts_with("http://") || url.starts_with("https://") || url.starts_with("ssh://") || url.starts_with("git@") || url.starts_with("git://")
    }

    /// Helper to derive a default path from the url (e.g., last path component)
    fn name_from_url(url: &str) -> String {
        let url = url.trim_end_matches('/')
            .trim_end_matches(".git");
        url.rsplit(&['/', ':'][..]).next().unwrap_or("").to_string()
    }

    /// Convert this submodule configuration to git2 options
    pub fn to_git2_options(&self) -> Result<Git2SubmoduleOptions> {
        Git2SubmoduleOptions::try_from(self.git_options.clone()).map_err(|e| anyhow::anyhow!(e))
    }
    /// Check if our active setting matches what git would report
    /// `git_active_state` should be the result of calling `gix_submodule::IsActivePlatform`
    #[allow(dead_code)]
    #[must_use]
    pub const fn active_setting_matches_git(&self, git_active_state: bool) -> bool {
        self.active == git_active_state
    }

    /// convert path to PathBuf for filesystem operations
    pub fn path_as_pathbuf(&self) -> Option<PathBuf> {
        self.path.as_ref().map(PathBuf::from)
    }

    /// Convert the submodule's URL to a string
    pub fn url_as_string(&self) -> String {
        self.url.clone()
    }
}

/// Main configuration structure for the submod tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Global default settings that apply to all submodules
    #[serde(default)]
    pub defaults: SubmoduleDefaults,
    /// Individual submodule configurations, keyed by submodule name
    #[serde(flatten)]
    pub submodules: HashMap<String, SubmoduleConfig>,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self {
                defaults: SubmoduleDefaults::default(),
                submodules: HashMap::new(),
            });
        }

        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;

        toml::from_str(&content).with_context(|| "Failed to parse TOML config")
    }

    /// Save configuration to a TOML file
    pub fn save(&self, path: &Path) -> Result<()> {
        self.save_with_toml_edit(path)
    }

    /// Save configuration using `toml_edit` for better formatting and comments
    fn save_with_toml_edit(&self, path: &Path) -> Result<()> {
        // Load existing document or create new one
        let mut doc = if path.exists() {
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read existing config: {}", path.display()))?;
            content
                .parse::<DocumentMut>()
                .with_context(|| "Failed to parse existing TOML document")?
        } else {
            // Create a beautiful new document with header comment
            let mut doc = DocumentMut::new();
            doc.insert(
                "# Submodule configuration for `submod`",
                Item::None,
            );
            doc.insert( "# For configuration options, see [the submod docs](https://docs.rs/submod/latest/submod/options/) or our [example config](https://github.com/bashandbone/submod/blob/main/sample_config/submod.toml)", Item::None);
            doc.insert("# Each section [name] defines a submodule; the [name] should be the exact name of the repo (without `.git` or its path)", Item::None);
            doc.insert("", Item::None); // Empty line for spacing
            doc
        };
        let mut defaults_table = Table::default();
        let non_defaults = self.get_non_defaults();
        // Handle defaults section
        if !non_defaults.is_empty() {
            // Add each default field to the defaults section
            // if it doesn't match the default value (which we already filtered for)
            for default in non_defaults {
                if let Some(ref ignore) = default.ignore {
                    defaults_table["ignore"] = value(ignore);
                }
                if let Some(ref fetch_recurse) = default.fetch_recurse {
                    defaults_table["fetchRecurse"] = value(fetch_recurse);
                }
                if let Some(ref update) = default.update {
                    defaults_table["update"] = value(update);
                }
            }
        }
        if doc["defaults"].is_some() {
            if let Some(existing_defaults) = doc["defaults"].as_table_mut() {
                // Merge existing defaults with new ones
                for (key, value) in defaults_table.iter() {
                    existing_defaults.insert(key.clone(), value.clone());
                }
            }
        }
        }

        for key in keys_to_remove {
            doc.remove(&key);
        }

        // Add each submodule as its own section
        for (submodule_name, submodule) in &self.submodules {
            let mut submodule_table = Table::new();

            // Required fields
            if let Some(ref path) = submodule.path {
                submodule_table["path"] = value(path);
            }
            submodule_table["url"] = value(&submodule.url);

            // Active state
            submodule_table["active"] = value(submodule.active);

            // Optional sparse_paths
            if let Some(ref sparse_paths) = submodule.sparse_paths {
                let mut sparse_array = Array::new();
                for path in sparse_paths {
                    sparse_array.push(path);
                }
                submodule_table["sparse_paths"] = value(sparse_array);
            }

            // Git options (flattened)
            if let Some(ref ignore) = submodule.git_options.ignore {
                let serialized = serde_json::to_string(ignore).unwrap_or_default();
                let clean_value = serialized.trim_matches('"');
                submodule_table["ignore"] = value(clean_value);
            }
            if let Some(ref update) = submodule.git_options.update {
                let serialized = serde_json::to_string(update).unwrap_or_default();
                let clean_value = serialized.trim_matches('"');
                submodule_table["update"] = value(clean_value);
            }
            if let Some(ref branch) = submodule.git_options.branch {
                let serialized = serde_json::to_string(branch).unwrap_or_default();
                let clean_value = serialized.trim_matches('"');
                submodule_table["branch"] = value(clean_value);
            }
            if let Some(ref fetch_recurse) = submodule.git_options.fetch_recurse {
                let serialized = serde_json::to_string(fetch_recurse).unwrap_or_default();
                let clean_value = serialized.trim_matches('"');
                submodule_table["fetchRecurse"] = value(clean_value);
            }

            doc[submodule_name] = Item::Table(submodule_table);
        }

        fs::write(path, doc.to_string())
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;

        Ok(())
    }

    /// Get provided values for global defaults that are not a default value
    fn get_non_defaults(&self) -> Vec<Partial<SubmoduleDefaults>> {
        let default_values = SubmoduleDefaults {
                ignore: ignore.or_else(|| Some(SerializableIgnore::default())),
                fetch_recurse: fetch_recurse.or_else(|| Some(SerializableFetchRecurse::default())),
                update: update.or_else(|| Some(SerializableUpdate::default())),
            }.get_values();
        for default in self.defaults.get_values().iter() {
            if !default_values.contains(default) {
                return vec![default.clone()];
            }
        }
        vec![]
    }

    /// Add a submodule configuration
    pub fn add_submodule(&mut self, name: String, submodule: SubmoduleConfig) {
        self.submodules.insert(name, submodule);
    }

    /// Get an iterator over all submodule configurations
    pub fn get_submodules(&self) -> impl Iterator<Item = (&String, &SubmoduleConfig)> {
        self.submodules.iter()
    }

    /// Get the effective setting for a submodule, falling back to defaults
    #[must_use]
    pub fn get_effective_setting(
        &self,
        submodule: &SubmoduleConfig,
        setting: &str,
    ) -> Option<String> {
        // Check submodule-specific setting first, then fall back to defaults
        match setting {
            "ignore" => {
                submodule
                    .git_options
                    .ignore
                    .as_ref()
                    .or(self.defaults.0.ignore.as_ref())
                    .map(|s| format!("{s:?}")) // Convert to string representation
            }
            "update" => submodule
                .git_options
                .update
                .as_ref()
                .or(self.defaults.0.update.as_ref())
                .map(|s| format!("{s:?}")),
            "fetchRecurse" => submodule
                .git_options
                .fetch_recurse
                .as_ref()
                .or(self.defaults.0.fetch_recurse.as_ref())
                .map(|s| format!("{s:?}")),
            _ => None,
        }
    }
}
