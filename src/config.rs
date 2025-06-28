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
    ConfigLevel, GitmodulesConvert, SerializableFetchRecurse, SerializableIgnore, SerializableUpdate
};
use crate::options::SerializableBranch;
use crate::git_ops::{GitOperations};
// TODO: Implement figment::Profile for modular configs
use figment::{Figment, Metadata, providers::{Toml, Format}, value::{Value, Map, Dict}, Provider, Result as FigmentResult};

/// Returns true. Used as a serde default for boolean fields.
fn default_true() -> bool {
    true
}

/// Returns false. Used as a serde default for boolean fields.
fn default_false() -> bool {
    false
}

/// Serialization skipping filter for `shallow` -- only serialize if the value is true,
/// So the function inverts falsey values to true.
fn shallow_filter(shallow: &bool) -> bool {
    // We skip if false
    !shallow
}

// Just a type wrapper around str to make it clear what we're working with
pub type SubmoduleName = String;

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

impl SubmoduleGitOptions {
    /// Create a new instance with defaults
    pub fn new(
        ignore: Option<SerializableIgnore>,
        fetch_recurse: Option<SerializableFetchRecurse>,
        branch: Option<SerializableBranch>,
        update: Option<SerializableUpdate>,
    ) -> Self {
        Self {
            ignore,
            fetch_recurse,
            branch,
            update,
        }
    }

    /// new with defaults
    pub fn default() -> Self {
        Self {
            ignore: Some(SerializableIgnore::default()),
            fetch_recurse: Some(SerializableFetchRecurse::default()),
            branch: Some(SerializableBranch::default()),
            update: Some(SerializableUpdate::default()),
        }
    }
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
        let fetch_recurse = options.fetch_recurse.map(|fr| fr.to_gitmodules());
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

impl Iterator for SubmoduleDefaults {
    type Item = SubmoduleDefaults;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.clone())
    }
}

impl SubmoduleDefaults {

    /// Returns a vector of SubmoduleDefaults with the current values (for comparison)
    pub fn get_values(&self) -> Vec<SubmoduleDefaults> {
        vec![self.clone()].into_iter().flatten().collect()
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

/// Options for adding a submodule
#[derive(Debug, Clone)]
pub struct SubmoduleAddOptions {
    /// Name of the submodule
    pub name: SubmoduleName,
    /// Local path where the submodule will be checked out
    pub path: PathBuf,
    /// URL of the submodule repository
    pub url: String,
    /// Branch to track (optional)
    pub branch: Option<SerializableBranch>,
    /// Ignore rule for the submodule (optional)
    pub ignore: Option<SerializableIgnore>,
    /// Update strategy for the submodule (optional)
    pub update: Option<SerializableUpdate>,
    /// Fetch recurse setting (optional)
    pub fetch_recurse: Option<SerializableFetchRecurse>,
    /// Whether to create a shallow clone
    pub shallow: bool,
    /// Whether to skip initialization after adding
    pub no_init: bool,
}

/// Options for updating a submodule
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubmoduleUpdateOptions {
    /// Update strategy to use
    pub strategy: SerializableUpdate,
    /// Whether to update recursively
    pub recursive: bool,
    /// Whether to force the update
    pub force: bool,
}

impl SubmoduleUpdateOptions {
    /// Create a new instance with defaults
    pub fn new(strategy: SerializableUpdate, recursive: bool, force: bool) -> Self {
        Self {
            strategy,
            recursive,
            force,
        }
    }

    pub fn default() -> Self {
        Self {
            strategy: SerializableUpdate::default(),
            recursive: false, // Default to not recursive
            force: false,     // Default to not force
        }
    }

    pub fn forced(&self) -> Self {
        Self {
            strategy: self.strategy.clone(),
            recursive: self.recursive,
            force: true, // Set force to true
        }
    }

    pub fn from_options(
        options: SubmoduleGitOptions,
    ) -> Self {
        Self {
            strategy: options.update.unwrap_or(SerializableUpdate::default()),
            recursive: match options.fetch_recurse {
                Some(SerializableFetchRecurse::Always) => true,
                _ => false,
            },
            force: false, // Default to not force
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct OtherSubmoduleSettings {
    /// URL of the submodule repository. This can be either a remote (https, ssh, etc) or a *relative* local path like `../some/other/repo`.
    pub url: Option<String>,

    /// Where to put the submodule in the working directory (relative path)
    pub path: Option<String>,

    /// Optional nickname, used for submod.toml and the cli as an easy reference; otherwise the relative path is used.
    pub name: Option<SubmoduleName>,

    /// Whether this submodule is active
    #[serde(default = "default_true")]
    pub active: bool,

    /// Whether to perform a shallow clone (depth == 1). Default is False.
    /// When true, only the last commit will be included in the submodule's history.
    #[serde(default = "default_false", skip_serializing_if = "shallow_filter")]
    pub shallow: bool,
}

impl OtherSubmoduleSettings {

    fn default() -> Self {
        Self {
            url: None, // Default to None, which makes it easier to identify missing values
            path: None, // Default to None
            name: None, // Default to None
            active: true,           // Default to active
            shallow: false,         // Default to not shallow
        }
    }

    fn new(url: Option<String>, path: Option<String>, name: Option<String>, active: Option<bool>, shallow: Option<bool>) -> Self {
        Self {
            url: url.clone().or_else(|| Some(".".to_string())),
            path: path.clone().or_else(|| if let Some(ref u) = url {
                Some(Self::name_from_url(u))
            } else {
                None
            }),
            name: name.or_else(|| if let Some(ref p) = path {
                Some(p.clone())
            } else if let Some(ref u) = url {
                Some(Self::name_from_url(u))
            } else {
                None
            }),
            active: active.unwrap_or(true), // Default to true if not specified
            shallow: shallow.unwrap_or(false), // Default to false if not specified
        }
    }

    /// Helper to derive a default path from the url (e.g., last path component)
    fn name_from_url(url: &str) -> String {
        let url = url.trim_end_matches('/')
            .trim_end_matches(".git");
        url.rsplit(&['/', ':'][..]).next().unwrap_or("").to_string()
    }

    /// Create a new instance from SubmoduleEntry, optionally providing a name
    pub fn from_entry(entry: &SubmoduleEntry, name: Option<String>) -> Self {
        Self::new(
            entry.url.clone(),
            entry.path.clone(),
            name,
            entry.active,
            entry.shallow,
        )
    }

    /// Get a new instance with an updated name
    pub fn update_with_name(
        &self,
        name: SubmoduleName,
    ) -> Self {
        let mut new_self = self.clone();
        new_self.name = Some(name);
        new_self
    }
}

/// A single submodule entry in .gitmodules and in our config
///
/// We have to keep all of the properties as `Option` because we
/// need to create and merge objects before we have all the data
/// this just means we need to validate before serializing or before
/// an action
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SubmoduleEntry {
    /// Path where the submodule is checked out
    pub path: Option<String>,
    /// URL of the submodule repository
    pub url: Option<String>,
    /// Branch to track (optional)
    pub branch: Option<SerializableBranch>,
    /// Ignore rule (optional)
    pub ignore: Option<SerializableIgnore>,
    /// Update strategy (optional)
    pub update: Option<SerializableUpdate>,
    /// Fetch recurse setting (optional)
    pub fetch_recurse: Option<SerializableFetchRecurse>,
    /// Whether the submodule is active
    pub active: Option<bool>,
    /// Whether the submodule is shallow (depth == 1)
    pub shallow: Option<bool>
}

impl SubmoduleEntry {
    /// Create a new submodule entry with defaults
    pub fn new(
        url: Option<String>,
        path: Option<String>,
        branch: Option<SerializableBranch>,
        ignore: Option<SerializableIgnore>,
        update: Option<SerializableUpdate>,
        fetch_recurse: Option<SerializableFetchRecurse>,
        active: Option<bool>,
        shallow: Option<bool>,
    ) -> Self {
        Self {
            url, // keep url explicitly None if we can't get it right now
            path,
            branch,
            ignore,
            update,
            fetch_recurse,
            active: active,
            shallow: shallow,
        }
    }

    pub fn from_options_and_settings(
        options: SubmoduleGitOptions,
        other_settings: OtherSubmoduleSettings,
    ) -> Self {
        Self::new(
            other_settings.url,
            other_settings.path,
            options.branch,
            options.ignore,
            options.update,
            options.fetch_recurse,
            Some(other_settings.active),
            Some(other_settings.shallow),
        )
    }

    /// Get a new instance with updated options
    pub fn update_with_options(
        &self,
        options: SubmoduleGitOptions,
    ) -> Self {
        let mut new_self = self.clone();
        if let Some(ignore) = options.ignore {
            new_self.ignore = Some(ignore);
        }
        if let Some(fetch_recurse) = options.fetch_recurse {
            new_self.fetch_recurse = Some(fetch_recurse);
        }
        if let Some(branch) = options.branch {
            new_self.branch = Some(branch);
        }
        if let Some(update) = options.update {
            new_self.update = Some(update);
        }
        new_self
    }

    /// Get a new instance with updated settings
    pub fn update_with_settings(
        &self,
        other_settings: OtherSubmoduleSettings,
    ) -> Self {
        let mut new_self = self.clone();
        if let Some(url) = other_settings.url {
            new_self.url = Some(url);
        }
        if let Some(path) = other_settings.path {
            new_self.path = Some(path);
        }
        new_self.active = Some(other_settings.active);
        new_self.shallow = Some(other_settings.shallow);
        new_self
    }

    /// Returns true if the url is a local path (relative or absolute)
    pub fn is_local(&self) -> bool {
        let url = self.url.clone().unwrap_or_else(|| "".to_string());
        url.starts_with("./") || url.starts_with("../") || url.starts_with('/')
    }

    /// Returns true if the url is a remote repository (http, ssh, git, etc)
    pub fn is_remote(&self) -> bool {
        let url = self.url.clone().unwrap_or_else(|| "".to_string());
        url.starts_with("http://") || url.starts_with("https://") || url.starts_with("ssh://") || url.starts_with("git@") || url.starts_with("git://")
    }

    /// Helper to derive a default path from the url (e.g., last path component)
    fn name_from_url(url: &str) -> String {
        let url = url.trim_end_matches('/')
            .trim_end_matches(".git");
        url.rsplit(&['/', ':'][..]).next().unwrap_or("").to_string()
    }

    pub fn git_options(&self) -> SubmoduleGitOptions {
        SubmoduleGitOptions {
            ignore: self.ignore.clone(),
            fetch_recurse: self.fetch_recurse.clone(),
            branch: self.branch.clone(),
            update: self.update.clone(),
        }
    }

    pub fn settings(&self) -> OtherSubmoduleSettings {
        OtherSubmoduleSettings {
            name: None, // We don't have a name in this struct, so we leave it as None
            url: self.url.clone(),
            path: self.path.clone(),
            active: self.active.unwrap_or(true),
            shallow: self.shallow.unwrap_or(false),
        }
    }

    /// Convert this submodule configuration to git2 options
    pub fn to_git2_options(&self) -> Result<Git2SubmoduleOptions> {
        Git2SubmoduleOptions::try_from(self.git_options().clone()).map_err(|e| anyhow::anyhow!(e))
    }

    /// convert path to PathBuf for filesystem operations
    pub fn path_as_pathbuf(&self) -> Option<PathBuf> {
        self.path.as_ref().map(PathBuf::from)
    }

    /// Convert the submodule's URL to a string
    pub fn url_as_string(&self) -> String {
        self.url.clone().unwrap_or_else(|| "".to_string())
    }

    /// Get the configuration active setting
    pub fn is_active(&self) -> bool {
        self.active.unwrap_or(true)
    }
}

impl From<OtherSubmoduleSettings> for SubmoduleEntry {
    fn from(other: OtherSubmoduleSettings) -> Self {
        let default_git_options = SubmoduleGitOptions::default();
        Self {
            url: other.url,
            path: other.path,
            active: Some(other.active),
            shallow: Some(other.shallow),
            ignore: default_git_options.ignore,
            fetch_recurse: default_git_options.fetch_recurse,
            branch: default_git_options.branch,
            update: default_git_options.update,
        }
    }
}


/// A collection of submodule entries, including sparse checkouts
///
/// Revamped to better reflect git's structure so we can use the SubmoduleEntry types directly with gix/git2
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubmoduleEntries {
    submodules: Option<HashMap<SubmoduleName, SubmoduleEntry>>,
    sparse_checkouts: Option<HashMap<SubmoduleName, Vec<String>>>,
}

impl SubmoduleEntries {
    /// Create a new empty SubmoduleEntries
    pub fn new(submodules: Option<HashMap<SubmoduleName, SubmoduleEntry>>, sparse_checkouts: Option<HashMap<SubmoduleName, Vec<String>>>) -> Self {
        Self {
            submodules: submodules.or_else(|| Some(HashMap::new())),
            sparse_checkouts: sparse_checkouts.or_else(|| Some(HashMap::new())),
        }
    }

    pub fn default() -> Self {
        Self {
            submodules: Some(HashMap::new()),
            sparse_checkouts: Some(HashMap::new()),
        }
    }

    /// Add a submodule entry
    pub fn add_submodule(&mut self, name: SubmoduleName, entry: SubmoduleEntry) {
        if let Some(submodules) = &mut self.submodules {
            submodules.insert(name, entry);
        }
    }

    /// Get the submodules map
    pub fn submodules(&self) -> Option<&HashMap<SubmoduleName, SubmoduleEntry>> {
        self.submodules.as_ref()
    }

    /// Get the sparse checkouts map
    pub fn sparse_checkouts(&self) -> Option<&HashMap<SubmoduleName, Vec<String>>> {
        self.sparse_checkouts.as_ref()
    }

    /// Add a sparse checkout
    pub fn add_checkout(&mut self, name: SubmoduleName, checkout: Vec<String>, replace: bool) {
        if let Some(sparse_checkouts) = &mut self.sparse_checkouts {
            if let Some(existing_checkout) = sparse_checkouts.get(&name) {
                match replace {
                    true => {
                        // Replace the existing checkout with the new one
                        sparse_checkouts.insert(name, checkout);
                    },
                    false => {
                        // Append to the existing checkout
                        let mut new_checkout = existing_checkout.clone();
                        new_checkout.extend(checkout);
                        sparse_checkouts.insert(name, new_checkout);
                    }
                }
            } else {
                // No existing checkout, just insert the new one
                sparse_checkouts.insert(name, checkout);
            }
        } else {
            self.sparse_checkouts = Some(HashMap::from([(name, checkout)]));
        }
    }

    /// Remove a sparse checkout by name
    pub fn delete_checkout(&mut self, name: SubmoduleName) {
        if let Some(sparse_checkouts) = &mut self.sparse_checkouts {
            sparse_checkouts.remove(&name);
        }
    }

    /// Remove a sparse checkout path
    pub fn remove_sparse_path(&mut self, name: SubmoduleName, path: String) {
        if let Some(sparse_checkouts) = &mut self.sparse_checkouts {
            if let Some(paths) = sparse_checkouts.get_mut(&name) {
                paths.retain(|p| p != &path);
                if paths.is_empty() {
                    sparse_checkouts.remove(&name); // Remove the entry if no paths left
                }
            }
        }
    }

    /// Add a sparse path
    pub fn add_sparse_path(&mut self, name: SubmoduleName, path: String) {
        if let Some(sparse_checkouts) = &mut self.sparse_checkouts {
            sparse_checkouts.entry(name).or_default().push(path);
        } else {
            self.sparse_checkouts = Some(HashMap::from([(name, vec![path])]));
        }
    }

    /// Get a submodule entry by name
    pub fn get(&self, name: &str) -> Option<&SubmoduleEntry> {
        self.submodules.as_ref()?.get(name)
    }

    pub fn contains_key(&self, name: &str) -> bool {
        self.submodules.as_ref().map_or(false, |s| s.contains_key(name))
    }

    /// Get an iterator over all submodule entries
    pub fn submodule_iter(&self) -> impl Iterator<Item = (&SubmoduleName, &SubmoduleEntry)> {
        self.submodules.as_ref().into_iter().flat_map(|s| s.iter())
    }

    /// Get an iterator over all sparse checkouts
    pub fn sparse_iter(&self) -> impl Iterator<Item = (&SubmoduleName, &Vec<String>)> {
        self.sparse_checkouts.as_ref().into_iter().flat_map(|s| s.iter())
    }

    /// Get an iterator that returns a tuple of submodule and sparse checkout
    pub fn iter(&self) -> impl Iterator<Item = (&SubmoduleName, (&SubmoduleEntry, Vec<String>))> {
        self.submodule_iter().map(move |(name, entry)| {
            let sparse = self.sparse_checkouts
                .as_ref()
                .and_then(|s| s.get(name))
                .cloned()
                .unwrap_or_else(Vec::new);
            (name, (entry, sparse))
        })
    }
}

impl IntoIterator for SubmoduleEntries {
    type Item = (SubmoduleName, SubmoduleEntry);
    type IntoIter = std::collections::hash_map::IntoIter<SubmoduleName, SubmoduleEntry>;

    fn into_iter(self) -> Self::IntoIter {
            self.submodules.unwrap_or_default().into_iter()
        }
}


/// Main configuration structure for the submod tool
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Global default settings that apply to all submodules
    #[serde(default)]
    pub defaults: SubmoduleDefaults,
    /// Individual submodule configurations, keyed by submodule name
    #[serde(flatten)]
    pub submodules: SubmoduleEntries,
}

impl Config {

    /// Create a new configuration with the given defaults and submodules
    pub fn new(defaults: SubmoduleDefaults, submodules: SubmoduleEntries) -> Self {
        Self {
            defaults,
            submodules,
        }
    }

    /// Create a new empty configuration with default values
    pub fn default() -> Self {
        Self {
            defaults: SubmoduleDefaults::default(),
            submodules: SubmoduleEntries::default(),
        }
    }

    fn get_submodule_entry(&self, name: &str) -> Option<&SubmoduleEntry> {
        self.submodules.get(name)
    }
    /// Helper to apply a default if the value is None or Unspecified
    fn apply_option_default<T: Clone + PartialEq>(
        value: &mut Option<T>,
        default: &Option<T>,
        unspecified: T,
    ) {
        if value.is_none() || value.as_ref() == Some(&unspecified) {
            *value = default.clone().or_else(|| Some(unspecified));
        } else {
            *value = value.clone().or_else(|| Some(unspecified));
        }
    }

    /// Create a new configuration, resolving defaults
    pub fn apply_defaults(mut self) -> Self {
        if let Some(submodules) = self.submodules.submodules.as_mut() {
            for sub in submodules.values_mut() {
                Self::apply_option_default(
                    &mut sub.ignore,
                    &self.defaults.ignore,
                    SerializableIgnore::Unspecified,
                );
                Self::apply_option_default(
                    &mut sub.fetch_recurse,
                    &self.defaults.fetch_recurse,
                    SerializableFetchRecurse::Unspecified,
                );
                Self::apply_option_default(
                    &mut sub.update,
                    &self.defaults.update,
                    SerializableUpdate::Unspecified,
                );
                // active is just a bool, no default logic needed
            }
        }
        self
    }

    /// Add a submodule configuration
    pub fn add_submodule(&mut self, name: String, submodule: SubmoduleEntry) {
        self.submodules.add_submodule(name, submodule);
    }

    /// Get an iterator over all submodule configurations
    pub fn get_submodules(&self) -> impl Iterator<Item = (&SubmoduleName, &SubmoduleEntry)> {
        self.submodules.submodule_iter()
    }

    /// Get an iterator over all sparse checkouts
    pub fn get_sparse_checkouts(&self) -> impl Iterator<Item = (&SubmoduleName, &Vec<String>)> {
        self.submodules.sparse_iter()
    }

    /// Get an iterator that returns a tuple of submodule and sparse checkout
    pub fn entries(&self) -> impl Iterator<Item = (&SubmoduleName, (&SubmoduleEntry, Vec<String>))> {
        self.submodules.iter()
    }

    /// Get a submodule configuration by name
    /// Returns None if the submodule does not exist
    pub fn get_submodule(&self, name: &str) -> Option<&SubmoduleEntry> {
        self.submodules.get(name)
    }

    /// Ensure submod.toml and .gitmodules stay in sync
    pub fn sync_with_git_config(&mut self, git_ops: &dyn GitOperations) -> Result<()> {
        // 1. Read current .gitmodules
        let current_gitmodules = git_ops.read_gitmodules()?;

        // 2. Apply our global defaults logic
        let target_gitmodules = self.submodules.clone();

        // 3. Write updated .gitmodules if different
        if current_gitmodules != target_gitmodules {
            git_ops.write_gitmodules(&target_gitmodules)?;
        }

        // 4. Update any git config values that need to be set
        for (name, entry) in target_gitmodules.submodule_iter() {
            if let Some(branch) = &entry.branch {
                git_ops.set_config_value(
                    &format!("submodule.{}.branch", name),
                    branch.to_string().as_str(),
                    ConfigLevel::Local,
                )?;
            }
        }

        Ok(())
    }

    pub fn load(&self, path: impl AsRef<Path>, cli_options: Config) -> anyhow::Result<Self> {
        let fig = Figment::from(Self::default()) // 1) start from Rust-side defaults
        .merge(Toml::file(path).nested())  // 2) file-based overrides
        .merge(cli_options);      // 3) CLI overrides file

        // 4) extract into Config, then post-process submodules
        let cfg: Config = fig.extract()?;
        Ok(cfg.apply_defaults())
    }

    pub fn load_from_file(&self, path: Option<impl AsRef<Path>>) -> anyhow::Result<Self> {
        let p: &dyn AsRef<Path> = match path {
            Some(ref p) => p,
            None => &".",
        };
        let fig = Figment::from(Self::default())
            .merge(Toml::file(p).nested());
        // Extract the configuration from Figment
        let cfg: Config = fig.extract()?;
        Ok(cfg.apply_defaults())
    }


    pub fn load_with_git_sync(&self, path: impl AsRef<Path>, git_ops: &dyn GitOperations, cli_options: Config) -> anyhow::Result<Self> {
        let mut cfg = self.load(path, cli_options)?;
        // Sync with git config
        cfg.sync_with_git_config(git_ops)?;
        Ok(cfg)
    }
}

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
