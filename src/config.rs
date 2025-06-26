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
"]

use std::path::PathBuf;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};
use crate::options::{
    SerializableFetchRecurse, SerializableIgnore, SerializableUpdate,
};
use crate::options::SerializableBranch;
// TODO: Implement figment::Profile for modular configs
use figment::{Figment, Metadata, providers::{Serialized, Toml, Format}, value::{Value, Map, Dict}, Provider, Result as FigmentResult};

/// Returns true. Used as a serde default for boolean fields.
fn default_true() -> bool {
    true
}

/// Returns false. Used as a serde default for boolean fields.
fn default_false() -> bool {
    false
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

    /// Optional nickname, used for submod.toml and the cli as an easy reference; otherwise the relative path is used.
    pub name: Option<String>,

    /// Git-specific options for this submodule
    #[serde(flatten)]
    pub git_options: SubmoduleGitOptions,
    /// Whether this submodule is active
    #[serde(default = "default_true")]
    pub active: bool,
    /// Whether to perform a shallow clone (depth == 1). Default is False.
    /// When true, only the last commit will be included in the submodule's history.
    #[serde(default = "default_false")]
    pub shallow: bool,

    /// Sparse checkout paths for this submodule (relative paths)
    pub sparse_paths: Option<Vec<String>>,
}

impl SubmoduleConfig {
    /// Create a new submodule configuration with defaults
    pub fn new(url: String, path: Option<String>, name: Option<String>, git_options: SubmoduleGitOptions, active: Option<bool>, shallow: Option<bool>, sparse_paths: Option<Vec<String>>) -> Self {
        Self {
            url: url.clone(),
            path: path.clone().or_else(|| Some(Self::name_from_url(&url))),
            name: name.or_else(|| path.clone().or_else(|| Some(Self::name_from_url(&url)))),
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

    pub fn default() -> Self {
        Self {
            defaults: SubmoduleDefaults::default(),
            submodules: HashMap::new(),
        }
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
                    .or(self.defaults.ignore.as_ref())
                    .map(|s| format!("{s:?}")) // Convert to string representation
            }
            "update" => submodule
                .git_options
                .update
                .as_ref()
                .or(self.defaults.update.as_ref())
                .map(|s| format!("{s:?}")),
            "fetchRecurse" => submodule
                .git_options
                .fetch_recurse
                .as_ref()
                .or(self.defaults.fetch_recurse.as_ref())
                .map(|s| format!("{s:?}")),
            _ => None,
        }
    }
}

// Default figment profile for the repository configuration (i.e. .gitmodules)
const REPO: figment::Profile = figment::Profile::const_new("repo");

// TODO: Implement figment::Profile for modular configs
/**
const USER: figment::Profile = figment::Profile::const_new("user");
const DEVELOPER: figment::Profile = figment::Profile::const_new("developer");
*/

impl Provider for Config {
    /// We now know where the settings came from
    fn metadata(&self) -> Metadata {
        Metadata::named("CLI arguments")
            .source("cli")
    }

    /// Serialize the configuration to a Figment Value
    fn data(&self) -> FigmentResult<Map<figment::Profile, Dict>> {
        let value = Value::serialize(self)?;
        let profile = self.profile().unwrap_or_default();

        if let Value::Dict(_, dict) = value {
            let mut map = Map::new();
            map.insert(profile, dict);
            Ok(map)
        } else {
            Err(figment::Error::from(figment::error::Kind::InvalidType(
                value.to_actual(),
                "dictionary".into()
            )))
        }
   }

    /// Return the profile for this configuration
    ///
    /// This is used to identify the source of the configuration (e.g., repo, user, developer)
    /// In this case, we use a constant profile for the repository configuration.
    // TODO: This will likely need to change to add developer/user profiles
   fn profile(&self) -> Option<figment::Profile> {
        Some(REPO)
    }
}

/// Returns the resolved configuration from defaults, TOML file, and CLI arguments.
pub fn get_configuration(cli_config: &Config, path: &Path) -> Result<Config> {
    let figment = Figment::new()
        .merge(Toml::file(path).nested())
        .merge(Serialized::defaults(cli_config));

    // Extract the configuration from the Figment instance
    let config: Config = figment.extract()?;
    Ok(config)
}
