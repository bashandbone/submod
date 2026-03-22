// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

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

use crate::git_ops::GitOperations;
use crate::options::SerializableBranch;
use crate::options::{
    ConfigLevel, GitmodulesConvert, SerializableFetchRecurse, SerializableIgnore,
    SerializableUpdate,
};
use anyhow::Result;
use serde::de::Deserializer;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::path::PathBuf;
use std::{collections::HashMap, path::Path};
// TODO: Implement figment::Profile for modular configs
use figment::{
    Figment, Metadata, Provider, Result as FigmentResult,
    providers::{Format, Toml},
    value::{Dict, Map, Value},
};

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
/// A type alias for submodule names used throughout the configuration.
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

#[allow(dead_code)]
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
#[allow(dead_code)]
pub struct Git2SubmoduleOptions {
    ignore: git2::SubmoduleIgnore,
    update: git2::SubmoduleUpdate,
    branch: Option<String>,
    fetch_recurse: Option<String>,
}

/// Implementation for converting git2 submodule options
#[allow(dead_code)]
impl Git2SubmoduleOptions {
    /// Create a new `Git2SubmoduleOptions` from individual git2 option values.
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
            Some(i) => git2::SubmoduleIgnore::try_from(i).map_err(|_| {
                "Failed to convert SerializableIgnore to git2::SubmoduleIgnore".to_string()
            })?,
            None => git2::SubmoduleIgnore::Unspecified,
        };
        let update = match options.update {
            Some(u) => git2::SubmoduleUpdate::try_from(u).map_err(|_| {
                "Failed to convert SerializableUpdate to git2::SubmoduleUpdate".to_string()
            })?,
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

#[allow(dead_code)]
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
                fetch_recurse: mut_self
                    .fetch_recurse
                    .or_else(|| Some(SerializableFetchRecurse::default())),
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
    #[allow(dead_code)]
    pub no_init: bool,
}

#[allow(dead_code)]
impl SubmoduleAddOptions {
    /// Create an add options from a SubmoduleEntry
    pub fn into_submodule_entry(self) -> SubmoduleEntry {
        SubmoduleEntry {
            url: Some(self.url),
            path: Some(self.path.to_string_lossy().to_string()),
            branch: self.branch,
            ignore: self.ignore,
            update: self.update,
            fetch_recurse: self.fetch_recurse,
            shallow: Some(self.shallow),
            active: Some(!self.no_init), // we're adding so unless we have a 'no_init" flag, we can assume active
            no_init: Some(self.no_init),
            sparse_paths: None,
        }
    }

    /// Create an add options from a entries tuple (name and SubmoduleEntry)
    pub fn from_submodule_entries_tuple(entry: (SubmoduleName, SubmoduleEntry)) -> Self {
        let (name, submodule_entry) = entry;
        Self {
            name: name.clone(),
            url: submodule_entry
                .url
                .map(|u| u.to_string())
                .unwrap_or_else(|| submodule_entry.path.clone().unwrap_or_else(|| name.clone())),
            path: submodule_entry
                .path
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(name.clone())),
            branch: submodule_entry.branch,
            ignore: submodule_entry.ignore,
            update: submodule_entry.update,
            fetch_recurse: submodule_entry.fetch_recurse,
            shallow: submodule_entry.shallow.map_or(false, |s| s),
            no_init: submodule_entry.no_init.map_or(false, |f| f),
        }
    }

    /// Convert an AddOptions to a SubmoduleEntries tuple
    pub fn into_entries_tuple(self) -> (SubmoduleName, SubmoduleEntry) {
        (self.name.to_owned(), self.clone().into_submodule_entry())
    }
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

#[allow(dead_code)]
impl SubmoduleUpdateOptions {
    /// Create a new instance with defaults
    pub fn new(strategy: SerializableUpdate, recursive: bool, force: bool) -> Self {
        Self {
            strategy,
            recursive,
            force,
        }
    }

    /// Create a new instance with default values
    pub fn default() -> Self {
        Self {
            strategy: SerializableUpdate::default(),
            recursive: false, // Default to not recursive
            force: false,     // Default to not force
        }
    }

    /// Get a new instance with the recursive flag set
    pub fn forced(&self) -> Self {
        Self {
            strategy: self.strategy.clone(),
            recursive: self.recursive,
            force: true, // Set force to true
        }
    }

    /// Convert from SubmoduleGitOptions to SubmoduleUpdateOptions
    pub fn from_options(options: SubmoduleGitOptions) -> Self {
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

/// Settings for a submodule that are not git-specific
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

    /// Whether to skip initialization after adding the submodule
    #[serde(default = "default_false")]
    pub no_init: bool,
}

#[allow(dead_code)]
impl OtherSubmoduleSettings {
    /// Create a new instance with default values
    fn default() -> Self {
        Self {
            url: None,      // Default to None, which makes it easier to identify missing values
            path: None,     // Default to None
            name: None,     // Default to None
            active: true,   // Default to active
            shallow: false, // Default to not shallow
            no_init: false, // Default to not skipping initialization
        }
    }

    fn new(
        url: Option<String>,
        path: Option<String>,
        name: Option<String>,
        active: Option<bool>,
        shallow: Option<bool>,
        no_init: Option<bool>,
    ) -> Self {
        Self {
            url: url.clone().or_else(|| Some(".".to_string())),
            path: path.clone().or_else(|| {
                if let Some(ref u) = url {
                    Some(Self::name_from_url(u))
                } else {
                    None
                }
            }),
            name: name.or_else(|| {
                if let Some(ref p) = path {
                    Some(p.clone())
                } else if let Some(ref u) = url {
                    Some(Self::name_from_url(u))
                } else {
                    None
                }
            }),
            active: active.unwrap_or(true), // Default to true if not specified
            shallow: shallow.unwrap_or(false), // Default to false if not specified
            no_init: no_init.unwrap_or(false), // Default to false if not specified
        }
    }

    /// Helper to derive a default path from the url (e.g., last path component)
    fn name_from_url(url: &str) -> String {
        let url = url.trim_end_matches('/').trim_end_matches(".git");
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
            entry.no_init,
        )
    }

    /// Get a new instance with an updated name
    pub fn update_with_name(&self, name: SubmoduleName) -> Self {
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
    pub shallow: Option<bool>,
    /// Whether to skip initialization after adding
    #[serde(skip)] // never write, we use this for stateful decisions
    pub no_init: Option<bool>,
    /// Sparse checkout paths for this submodule (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sparse_paths: Option<Vec<String>>,
}

#[allow(dead_code)]
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
        no_init: Option<bool>,
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
            no_init: no_init,
            sparse_paths: None,
        }
    }

    /// Create a new submodule entry with defaults, using the URL and path from OtherSubmoduleSettings
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
            Some(other_settings.no_init),
        )
    }

    /// Create a new submodule entries from a gitmodules entry
    pub fn from_gitmodules(
        name: &String,
        entries: std::collections::HashMap<String, String>,
    ) -> Self {
        let url = entries.get("url").cloned();
        let path = if let Some(path) = entries.get("path").cloned() {
            Some(path)
        } else {
            name.to_string().into()
        };
        let branch =
            SerializableBranch::from_gitmodules(entries.get("branch").map_or("", |b| b.as_str()))
                .ok();
        let ignore = entries
            .get("ignore")
            .and_then(|i| SerializableIgnore::from_gitmodules(i).ok());
        let fetch_recurse = entries
            .get("fetchRecurseSubmodules")
            .or_else(|| entries.get("fetchRecurse"))
            .and_then(|fr| SerializableFetchRecurse::from_gitmodules(fr).ok());
        let update = entries
            .get("update")
            .and_then(|u| SerializableUpdate::from_gitmodules(u).ok());
        let active = entries
            .get("active")
            .and_then(|a| a.parse::<bool>().ok())
            .unwrap_or(true);
        let shallow = entries
            .get("shallow")
            .and_then(|s| s.parse::<bool>().ok())
            .unwrap_or(false);
        let no_init = false;
        Self::new(
            url,
            path,
            branch,
            ignore,
            update,
            fetch_recurse,
            Some(active),
            Some(shallow),
            Some(no_init),
        )
    }

    /// Get a new instance with updated options
    pub fn update_with_options(&self, options: SubmoduleGitOptions) -> Self {
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
    pub fn update_with_settings(&self, other_settings: OtherSubmoduleSettings) -> Self {
        let mut new_self = self.clone();
        if let Some(url) = other_settings.url {
            new_self.url = Some(url);
        }
        if let Some(path) = other_settings.path {
            new_self.path = Some(path);
        }
        new_self.active = Some(other_settings.active);
        new_self.shallow = Some(other_settings.shallow);
        new_self.no_init = Some(other_settings.no_init);
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
        url.starts_with("http://")
            || url.starts_with("https://")
            || url.starts_with("ssh://")
            || url.starts_with("git@")
            || url.starts_with("git://")
    }

    /// Helper to derive a default path from the url (e.g., last path component)
    fn name_from_url(url: &str) -> String {
        let url = url.trim_end_matches('/').trim_end_matches(".git");
        url.rsplit(&['/', ':'][..]).next().unwrap_or("").to_string()
    }

    /// Returns the git-specific options for this submodule entry.
    pub fn git_options(&self) -> SubmoduleGitOptions {
        SubmoduleGitOptions {
            ignore: self.ignore.clone(),
            fetch_recurse: self.fetch_recurse.clone(),
            branch: self.branch.clone(),
            update: self.update.clone(),
        }
    }

    /// Returns the non-git settings for this submodule entry (path, url, active state, etc.).
    pub fn settings(&self) -> OtherSubmoduleSettings {
        OtherSubmoduleSettings {
            name: None, // We don't have a name in this struct, so we leave it as None
            url: self.url.clone(),
            path: self.path.clone(),
            active: self.active.unwrap_or(true),
            shallow: self.shallow.unwrap_or(false),
            no_init: self.no_init.unwrap_or(false),
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
            no_init: Some(other.no_init),
            sparse_paths: None,
        }
    }
}

/// A collection of submodule entries, including sparse checkouts
///
/// Revamped to better reflect git's structure so we can use the SubmoduleEntry types directly with gix/git2
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SubmoduleEntries {
    submodules: Option<HashMap<SubmoduleName, SubmoduleEntry>>,
    sparse_checkouts: Option<HashMap<SubmoduleName, Vec<String>>>,
}

impl<'de> Deserialize<'de> for SubmoduleEntries {
    /// Deserialize from the flat TOML format where each top-level key is a submodule name.
    /// Accepts a map where each key maps to a [`SubmoduleEntry`], building both the
    /// `submodules` map and the `sparse_checkouts` map from each entry's `sparse_paths`.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map: HashMap<SubmoduleName, SubmoduleEntry> = HashMap::deserialize(deserializer)?;
        let mut sparse_checkouts: HashMap<SubmoduleName, Vec<String>> = HashMap::new();
        for (name, entry) in &map {
            if let Some(paths) = &entry.sparse_paths {
                if !paths.is_empty() {
                    sparse_checkouts.insert(name.clone(), paths.clone());
                }
            }
        }
        Ok(SubmoduleEntries {
            submodules: Some(map),
            sparse_checkouts: Some(sparse_checkouts),
        })
    }
}

impl Serialize for SubmoduleEntries {
    /// Serialize as a flat map of submodule name → entry, so the round-trip
    /// through `Deserialize` (which also expects a flat map) is consistent.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let submodules = self.submodules.as_ref();
        let len = submodules.map_or(0, HashMap::len);
        let mut map = serializer.serialize_map(Some(len))?;
        if let Some(subs) = submodules {
            for (name, entry) in subs {
                map.serialize_entry(name, entry)?;
            }
        }
        map.end()
    }
}

#[allow(dead_code)]
impl SubmoduleEntries {
    /// Create a new empty SubmoduleEntries
    pub fn new(
        submodules: Option<HashMap<SubmoduleName, SubmoduleEntry>>,
        sparse_checkouts: Option<HashMap<SubmoduleName, Vec<String>>>,
    ) -> Self {
        Self {
            submodules: submodules.or_else(|| Some(HashMap::new())),
            sparse_checkouts: sparse_checkouts.or_else(|| Some(HashMap::new())),
        }
    }

    /// Create a new empty `SubmoduleEntries` with default (empty) collections.
    pub fn default() -> Self {
        Self {
            submodules: Some(HashMap::new()),
            sparse_checkouts: Some(HashMap::new()),
        }
    }

    /// Add a submodule entry
    pub fn add_submodule(self, name: SubmoduleName, entry: SubmoduleEntry) -> Self {
        if self.submodules().is_some() {
            let mut submodules = self.submodules.unwrap().clone();
            submodules.insert(name.clone(), entry);
            Self {
                submodules: Some(submodules),
                sparse_checkouts: self.sparse_checkouts,
            }
        } else {
            let mut submodules = HashMap::new();
            submodules.insert(name.clone(), entry);
            Self {
                submodules: Some(submodules),
                sparse_checkouts: self.sparse_checkouts,
            }
        }
    }

    /// Remove a submodule entry
    pub fn remove_submodule(&mut self, name: &str) -> Self {
        if let Some(submodules) = &mut self.submodules {
            submodules.remove(name);
        }
        self.clone()
    }

    /// Returns a list of all submodule names, or `None` if no submodules are configured.
    pub fn submodule_names(&self) -> Option<Vec<String>> {
        self.submodules
            .as_ref()
            .map(|s| s.keys().cloned().collect())
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
    pub fn add_checkout(&mut self, name: SubmoduleName, checkout: &[String], replace: bool) {
        if let Some(sparse_checkouts) = &mut self.sparse_checkouts {
            if let Some(existing_checkout) = sparse_checkouts.get(&name) {
                match replace {
                    true => {
                        // Replace the existing checkout with the new one
                        sparse_checkouts.insert(name, checkout.to_vec());
                    }
                    false => {
                        // Append to the existing checkout
                        let mut new_checkout = existing_checkout.clone();
                        new_checkout.extend_from_slice(checkout);
                        sparse_checkouts.insert(name, new_checkout);
                    }
                }
            } else {
                // No existing checkout, just insert the new one
                sparse_checkouts.insert(name, checkout.to_vec());
            }
        } else {
            self.sparse_checkouts = Some(HashMap::from([(name, checkout.to_vec())]));
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

    /// Returns `true` if a submodule with the given name exists.
    pub fn contains_key(&self, name: &str) -> bool {
        self.submodules
            .as_ref()
            .map_or(false, |s| s.contains_key(name))
    }

    /// Get an iterator over all submodule entries
    pub fn submodule_iter(&self) -> impl Iterator<Item = (&SubmoduleName, &SubmoduleEntry)> {
        self.submodules.as_ref().into_iter().flat_map(|s| s.iter())
    }

    /// Get an iterator over all sparse checkouts
    pub fn sparse_iter(&self) -> impl Iterator<Item = (&SubmoduleName, &Vec<String>)> {
        self.sparse_checkouts
            .as_ref()
            .into_iter()
            .flat_map(|s| s.iter())
    }

    /// Get an iterator that returns a tuple of submodule and sparse checkout
    pub fn iter(&self) -> impl Iterator<Item = (&SubmoduleName, (&SubmoduleEntry, Vec<String>))> {
        self.submodule_iter().map(move |(name, entry)| {
            let sparse = self
                .sparse_checkouts
                .as_ref()
                .and_then(|s| s.get(name))
                .cloned()
                .unwrap_or_else(Vec::new);
            (name, (entry, sparse))
        })
    }

    /// Create a new SubmoduleEntries from a HashMap of submodule entries
    pub fn from_gitmodules(
        entries: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    ) -> Self {
        let mut submodules = HashMap::new();
        for (name, entry) in entries {
            let submodule_entry = SubmoduleEntry::from_gitmodules(&name, entry);
            submodules.insert(name, submodule_entry);
        }
        Self {
            submodules: Some(submodules),
            sparse_checkouts: Some(HashMap::new()),
        }
    }
    /// Insert or replace a submodule entry by name.
    pub fn update_entry(&mut self, name: SubmoduleName, entry: SubmoduleEntry) {
        // Ensure the submodules map exists and update/insert the entry.
        let submodules = self.submodules.get_or_insert_with(HashMap::new);
        submodules.insert(name.clone(), entry.clone());

        // Keep sparse_checkouts in sync with the entry's sparse paths.
        match entry.sparse_paths {
            Some(ref paths) if !paths.is_empty() => {
                let sparse_map = self.sparse_checkouts.get_or_insert_with(HashMap::new);
                sparse_map.insert(name, paths.clone());
            }
            _ => {
                if let Some(sparse_map) = self.sparse_checkouts.as_mut() {
                    sparse_map.remove(&name);
                }
            }
        }
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

#[allow(dead_code)]
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
        self.submodules = self.submodules.clone().add_submodule(name, submodule);
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
    pub fn entries(
        &self,
    ) -> impl Iterator<Item = (&SubmoduleName, (&SubmoduleEntry, Vec<String>))> {
        self.submodules.iter()
    }

    /// Get a submodule configuration by name
    /// Returns None if the submodule does not exist
    pub fn get_submodule(&self, name: &str) -> Option<&SubmoduleEntry> {
        self.submodules.get(name)
    }

    /// Ensure submod.toml and .gitmodules stay in sync
    pub fn sync_with_git_config(&mut self, git_ops: &mut dyn GitOperations) -> Result<()> {
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

    /// Load configuration from a file, merging with CLI options
    pub fn load(&self, path: impl AsRef<Path>, cli_options: Config) -> anyhow::Result<Self> {
        let fig = Figment::from(Self::default()) // 1) start from Rust-side defaults
            .merge(Toml::file(path)) // 2) file-based overrides
            .merge(cli_options); // 3) CLI overrides file

        // 4) extract into Config, then post-process submodules
        let cfg: Config = fig.extract()?;
        Ok(cfg.apply_defaults())
    }

    /// load configuration from a file without CLI options
    pub fn load_from_file(&self, path: Option<impl AsRef<Path>>) -> anyhow::Result<Self> {
        let p: &dyn AsRef<Path> = match path {
            Some(ref p) => p,
            None => &".",
        };
        let fig = Figment::from(Self::default()).merge(Toml::file(p));
        // Extract the configuration from Figment
        let cfg: Config = fig.extract()?;
        Ok(cfg.apply_defaults())
    }

    /// Load configuration from config and merge with existing gitmodules options
    pub fn load_with_git_sync(
        &self,
        path: impl AsRef<Path>,
        git_ops: &mut dyn GitOperations,
        cli_options: Config,
    ) -> anyhow::Result<Self> {
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
        Metadata::named("CLI arguments").source("cli")
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
                "dictionary".into(),
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

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    // ================================================================
    // SubmoduleDefaults::merge_from
    // ================================================================

    #[test]
    fn test_defaults_merge_from_both_set() {
        let base = SubmoduleDefaults {
            ignore: Some(SerializableIgnore::All),
            fetch_recurse: Some(SerializableFetchRecurse::Always),
            update: Some(SerializableUpdate::Rebase),
        };
        let other = SubmoduleDefaults {
            ignore: Some(SerializableIgnore::Dirty),
            fetch_recurse: None,
            update: Some(SerializableUpdate::Merge),
        };
        let merged = base.merge_from(other);
        // other.ignore overrides
        assert_eq!(merged.ignore, Some(SerializableIgnore::Dirty));
        // other.fetch_recurse is None → base preserved
        assert_eq!(merged.fetch_recurse, Some(SerializableFetchRecurse::Always));
        // other.update overrides
        assert_eq!(merged.update, Some(SerializableUpdate::Merge));
    }

    #[test]
    fn test_defaults_merge_from_empty_other() {
        let base = SubmoduleDefaults {
            ignore: Some(SerializableIgnore::All),
            fetch_recurse: Some(SerializableFetchRecurse::Never),
            update: Some(SerializableUpdate::Checkout),
        };
        let other = SubmoduleDefaults::default();
        let merged = base.merge_from(other);
        // Base values should be preserved
        assert_eq!(merged.ignore, Some(SerializableIgnore::All));
        assert_eq!(merged.fetch_recurse, Some(SerializableFetchRecurse::Never));
        assert_eq!(merged.update, Some(SerializableUpdate::Checkout));
    }

    #[test]
    fn test_defaults_merge_from_empty_base() {
        let base = SubmoduleDefaults::default();
        let other = SubmoduleDefaults {
            ignore: Some(SerializableIgnore::Dirty),
            fetch_recurse: Some(SerializableFetchRecurse::Always),
            update: Some(SerializableUpdate::Merge),
        };
        let merged = base.merge_from(other);
        assert_eq!(merged.ignore, Some(SerializableIgnore::Dirty));
        assert_eq!(merged.fetch_recurse, Some(SerializableFetchRecurse::Always));
        assert_eq!(merged.update, Some(SerializableUpdate::Merge));
    }

    #[test]
    fn test_defaults_merge_from_both_empty_gets_defaults() {
        let base = SubmoduleDefaults::default();
        let other = SubmoduleDefaults::default();
        let merged = base.merge_from(other);
        // Should fill in defaults via .or_else
        assert_eq!(merged.ignore, Some(SerializableIgnore::default()));
        assert_eq!(
            merged.fetch_recurse,
            Some(SerializableFetchRecurse::default())
        );
        assert_eq!(merged.update, Some(SerializableUpdate::default()));
    }

    // ================================================================
    // SubmoduleEntry::is_local / is_remote
    // ================================================================

    #[test]
    fn test_entry_is_local() {
        let mut entry = SubmoduleEntry::new(
            Some("./local-repo".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(entry.is_local());

        entry.url = Some("../sibling".to_string());
        assert!(entry.is_local());

        entry.url = Some("/absolute/path".to_string());
        assert!(entry.is_local());

        // Not local
        entry.url = Some("https://github.com/repo".to_string());
        assert!(!entry.is_local());

        // None url
        entry.url = None;
        assert!(!entry.is_local());
    }

    #[test]
    fn test_entry_is_remote() {
        let mut entry = SubmoduleEntry::new(
            Some("https://github.com/user/repo".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(entry.is_remote());

        entry.url = Some("http://example.com/repo".to_string());
        assert!(entry.is_remote());

        entry.url = Some("ssh://git@github.com/repo".to_string());
        assert!(entry.is_remote());

        entry.url = Some("git@github.com:user/repo.git".to_string());
        assert!(entry.is_remote());

        entry.url = Some("git://example.com/repo".to_string());
        assert!(entry.is_remote());

        // Not remote
        entry.url = Some("./local".to_string());
        assert!(!entry.is_remote());

        entry.url = None;
        assert!(!entry.is_remote());

        // Protocol not at start
        entry.url = Some("/path/to/https://repo".to_string());
        assert!(!entry.is_remote());

        // Minimal protocol
        entry.url = Some("https://".to_string());
        assert!(entry.is_remote());
    }

    #[test]
    fn test_entry_neither_local_nor_remote() {
        // A bare name isn't classified as either
        let entry = SubmoduleEntry::new(
            Some("just-a-name".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(!entry.is_local());
        assert!(!entry.is_remote());
    }

    // ================================================================
    // SubmoduleEntry::from_gitmodules
    // ================================================================

    #[test]
    fn test_entry_from_gitmodules_full() {
        let mut map = HashMap::new();
        map.insert(
            "url".to_string(),
            "https://github.com/user/repo.git".to_string(),
        );
        map.insert("path".to_string(), "libs/repo".to_string());
        map.insert("branch".to_string(), "main".to_string());
        map.insert("ignore".to_string(), "dirty".to_string());
        map.insert("update".to_string(), "rebase".to_string());
        map.insert("fetchRecurseSubmodules".to_string(), "true".to_string());
        map.insert("active".to_string(), "true".to_string());
        map.insert("shallow".to_string(), "true".to_string());

        let entry = SubmoduleEntry::from_gitmodules(&"repo".to_string(), map);
        assert_eq!(
            entry.url,
            Some("https://github.com/user/repo.git".to_string())
        );
        assert_eq!(entry.path, Some("libs/repo".to_string()));
        assert_eq!(
            entry.branch,
            Some(SerializableBranch::Name("main".to_string()))
        );
        assert_eq!(entry.ignore, Some(SerializableIgnore::Dirty));
        assert_eq!(entry.update, Some(SerializableUpdate::Rebase));
        assert_eq!(entry.fetch_recurse, Some(SerializableFetchRecurse::Always));
        assert_eq!(entry.active, Some(true));
        assert_eq!(entry.shallow, Some(true));
    }

    #[test]
    fn test_entry_from_gitmodules_minimal() {
        let mut map = HashMap::new();
        map.insert(
            "url".to_string(),
            "https://example.com/repo.git".to_string(),
        );

        let entry = SubmoduleEntry::from_gitmodules(&"mymod".to_string(), map);
        assert_eq!(entry.url, Some("https://example.com/repo.git".to_string()));
        // path defaults to name when not in the map
        assert!(entry.path.is_some());
        // active defaults to true, shallow to false
        assert_eq!(entry.active, Some(true));
        assert_eq!(entry.shallow, Some(false));
    }

    #[test]
    fn test_entry_from_gitmodules_invalid_values_silently_ignored() {
        let mut map = HashMap::new();
        map.insert("url".to_string(), "https://example.com/repo".to_string());
        map.insert("ignore".to_string(), "INVALID".to_string());
        map.insert("update".to_string(), "BOGUS".to_string());
        map.insert("active".to_string(), "not-a-bool".to_string());

        let entry = SubmoduleEntry::from_gitmodules(&"mod".to_string(), map);
        // Invalid values should result in None (parsed with .ok())
        assert_eq!(entry.ignore, None);
        assert_eq!(entry.update, None);
        // Invalid bool parses to None, defaults to true
        assert_eq!(entry.active, Some(true));
    }

    #[test]
    fn test_entry_from_gitmodules_branch_dot_alias() {
        let mut map = HashMap::new();
        map.insert("branch".to_string(), ".".to_string());
        map.insert("url".to_string(), "https://example.com/repo".to_string());

        let entry = SubmoduleEntry::from_gitmodules(&"mod".to_string(), map);
        assert_eq!(
            entry.branch,
            Some(SerializableBranch::CurrentInSuperproject)
        );
    }

    // ================================================================
    // SubmoduleEntry::update_with_options
    // ================================================================

    #[test]
    fn test_entry_update_with_options() {
        let entry = SubmoduleEntry::new(
            Some("https://example.com".to_string()),
            Some("path".to_string()),
            Some(SerializableBranch::Name("main".to_string())),
            Some(SerializableIgnore::None),
            Some(SerializableUpdate::Checkout),
            Some(SerializableFetchRecurse::OnDemand),
            Some(true),
            Some(false),
            None,
        );

        let opts = SubmoduleGitOptions {
            ignore: Some(SerializableIgnore::All),
            fetch_recurse: None,
            branch: Some(SerializableBranch::Name("develop".to_string())),
            update: None,
        };

        let updated = entry.update_with_options(opts);
        assert_eq!(updated.ignore, Some(SerializableIgnore::All));
        assert_eq!(
            updated.fetch_recurse,
            Some(SerializableFetchRecurse::OnDemand)
        ); // unchanged
        assert_eq!(
            updated.branch,
            Some(SerializableBranch::Name("develop".to_string()))
        );
        assert_eq!(updated.update, Some(SerializableUpdate::Checkout)); // unchanged
    }

    #[test]
    fn test_entry_update_with_empty_options_preserves() {
        let entry = SubmoduleEntry::new(
            Some("url".to_string()),
            Some("path".to_string()),
            Some(SerializableBranch::Name("main".to_string())),
            Some(SerializableIgnore::Dirty),
            Some(SerializableUpdate::Rebase),
            Some(SerializableFetchRecurse::Always),
            Some(true),
            None,
            None,
        );
        // Default-derived SubmoduleGitOptions has all None fields
        let opts = SubmoduleGitOptions {
            ignore: None,
            fetch_recurse: None,
            branch: None,
            update: None,
        };
        let updated = entry.update_with_options(opts);
        // None options don't override existing values
        assert_eq!(updated.ignore, Some(SerializableIgnore::Dirty));
        assert_eq!(updated.update, Some(SerializableUpdate::Rebase));
        assert_eq!(
            updated.branch,
            Some(SerializableBranch::Name("main".to_string()))
        );
        assert_eq!(
            updated.fetch_recurse,
            Some(SerializableFetchRecurse::Always)
        );
    }

    // ================================================================
    // SubmoduleEntries: sparse checkout operations
    // ================================================================

    #[test]
    fn test_entries_add_checkout_replace() {
        let mut entries = SubmoduleEntries::default();
        entries.add_checkout("mod1".to_string(), &["src/".to_string()], false);
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["src/".to_string()]
        );

        // Append
        entries.add_checkout("mod1".to_string(), &["docs/".to_string()], false);
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["src/".to_string(), "docs/".to_string()]
        );

        // Replace
        entries.add_checkout("mod1".to_string(), &["lib/".to_string()], true);
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["lib/".to_string()]
        );
    }

    #[test]
    fn test_entries_add_checkout_when_none() {
        let mut entries = SubmoduleEntries {
            submodules: Some(HashMap::new()),
            sparse_checkouts: None,
        };
        entries.add_checkout("mod1".to_string(), &["src/".to_string()], false);
        assert!(entries.sparse_checkouts().is_some());
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["src/".to_string()]
        );
    }

    #[test]
    fn test_entries_remove_sparse_path() {
        let mut entries = SubmoduleEntries::default();
        entries.add_checkout(
            "mod1".to_string(),
            &["src/".to_string(), "docs/".to_string()],
            false,
        );

        entries.remove_sparse_path("mod1".to_string(), "src/".to_string());
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["docs/".to_string()]
        );

        // Remove last path → entry is cleaned up
        entries.remove_sparse_path("mod1".to_string(), "docs/".to_string());
        assert!(!entries.sparse_checkouts().unwrap().contains_key("mod1"));
    }

    #[test]
    fn test_entries_add_sparse_path() {
        let mut entries = SubmoduleEntries::default();
        entries.add_sparse_path("mod1".to_string(), "src/".to_string());
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["src/".to_string()]
        );
        entries.add_sparse_path("mod1".to_string(), "docs/".to_string());
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["src/".to_string(), "docs/".to_string()]
        );
    }

    #[test]
    fn test_entries_add_sparse_path_when_none() {
        let mut entries = SubmoduleEntries {
            submodules: Some(HashMap::new()),
            sparse_checkouts: None,
        };
        entries.add_sparse_path("mod1".to_string(), "src/".to_string());
        assert!(entries.sparse_checkouts().is_some());
    }

    #[test]
    fn test_entries_delete_checkout() {
        let mut entries = SubmoduleEntries::default();
        entries.add_checkout("mod1".to_string(), &["src/".to_string()], false);
        entries.delete_checkout("mod1".to_string());
        assert!(!entries.sparse_checkouts().unwrap().contains_key("mod1"));
    }

    // ================================================================
    // SubmoduleEntries: update_entry
    // ================================================================

    #[test]
    fn test_entries_update_entry_with_sparse() {
        let mut entries = SubmoduleEntries::default();
        let entry = SubmoduleEntry {
            url: Some("https://example.com/repo".to_string()),
            path: Some("libs/repo".to_string()),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            active: Some(true),
            shallow: None,
            no_init: None,
            sparse_paths: Some(vec!["src/".to_string()]),
        };
        entries.update_entry("repo".to_string(), entry);

        assert!(entries.submodules().unwrap().contains_key("repo"));
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("repo").unwrap(),
            &vec!["src/".to_string()]
        );
    }

    #[test]
    fn test_entries_update_entry_removes_sparse_when_empty() {
        let mut entries = SubmoduleEntries::default();
        // First add with sparse
        let entry_with_sparse = SubmoduleEntry {
            url: Some("url".to_string()),
            path: Some("path".to_string()),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            active: Some(true),
            shallow: None,
            no_init: None,
            sparse_paths: Some(vec!["src/".to_string()]),
        };
        entries.update_entry("repo".to_string(), entry_with_sparse);
        assert!(entries.sparse_checkouts().unwrap().contains_key("repo"));

        // Update without sparse → should remove from sparse map
        let entry_no_sparse = SubmoduleEntry {
            url: Some("url".to_string()),
            path: Some("path".to_string()),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            active: Some(true),
            shallow: None,
            no_init: None,
            sparse_paths: None,
        };
        entries.update_entry("repo".to_string(), entry_no_sparse);
        assert!(!entries.sparse_checkouts().unwrap().contains_key("repo"));
    }

    // ================================================================
    // SubmoduleEntries: iteration and queries
    // ================================================================

    #[test]
    fn test_entries_contains_key() {
        let mut entries = SubmoduleEntries::default();
        assert!(!entries.contains_key("mod1"));

        let entry = SubmoduleEntry::new(
            Some("url".to_string()),
            Some("path".to_string()),
            None,
            None,
            None,
            None,
            Some(true),
            None,
            None,
        );
        entries = entries.add_submodule("mod1".to_string(), entry);
        assert!(entries.contains_key("mod1"));
        assert!(!entries.contains_key("mod2"));
    }

    #[test]
    fn test_entries_submodule_names() {
        let mut entries = SubmoduleEntries::default();
        let entry = SubmoduleEntry::new(
            Some("url".to_string()),
            Some("path".to_string()),
            None,
            None,
            None,
            None,
            Some(true),
            None,
            None,
        );
        entries = entries.add_submodule("alpha".to_string(), entry.clone());
        entries = entries.add_submodule("beta".to_string(), entry);
        let names = entries.submodule_names().unwrap();
        assert!(names.contains(&"alpha".to_string()));
        assert!(names.contains(&"beta".to_string()));
        assert_eq!(names.len(), 2);
    }

    #[test]
    fn test_entries_remove_submodule() {
        let entry = SubmoduleEntry::new(
            Some("url".to_string()),
            Some("path".to_string()),
            None,
            None,
            None,
            None,
            Some(true),
            None,
            None,
        );
        let mut entries = SubmoduleEntries::default()
            .add_submodule("mod1".to_string(), entry.clone())
            .add_submodule("mod2".to_string(), entry);
        assert!(entries.contains_key("mod1"));
        entries.remove_submodule("mod1");
        assert!(!entries.contains_key("mod1"));
        assert!(entries.contains_key("mod2"));
    }

    #[test]
    fn test_entries_iter_joins_sparse() {
        let mut entries = SubmoduleEntries::default();
        let entry = SubmoduleEntry::new(
            Some("url".to_string()),
            Some("path".to_string()),
            None,
            None,
            None,
            None,
            Some(true),
            None,
            None,
        );
        entries = entries.add_submodule("mod1".to_string(), entry);
        entries.add_checkout("mod1".to_string(), &["src/".to_string()], false);

        let items: Vec<_> = entries.iter().collect();
        assert_eq!(items.len(), 1);
        let (name, (_, sparse)) = &items[0];
        assert_eq!(*name, "mod1");
        assert_eq!(*sparse, vec!["src/".to_string()]);
    }

    #[test]
    fn test_entries_iter_no_sparse() {
        let mut entries = SubmoduleEntries::default();
        let entry = SubmoduleEntry::new(
            Some("url".to_string()),
            Some("path".to_string()),
            None,
            None,
            None,
            None,
            Some(true),
            None,
            None,
        );
        entries = entries.add_submodule("mod1".to_string(), entry);

        let items: Vec<_> = entries.iter().collect();
        let (_, (_, sparse)) = &items[0];
        assert!(sparse.is_empty());
    }

    // ================================================================
    // Config::apply_option_default
    // ================================================================

    #[test]
    fn test_apply_option_default_none_gets_default() {
        let mut val: Option<SerializableIgnore> = None;
        let default = Some(SerializableIgnore::Dirty);
        Config::apply_option_default(&mut val, &default, SerializableIgnore::Unspecified);
        assert_eq!(val, Some(SerializableIgnore::Dirty));
    }

    #[test]
    fn test_apply_option_default_unspecified_gets_default() {
        let mut val = Some(SerializableIgnore::Unspecified);
        let default = Some(SerializableIgnore::All);
        Config::apply_option_default(&mut val, &default, SerializableIgnore::Unspecified);
        assert_eq!(val, Some(SerializableIgnore::All));
    }

    #[test]
    fn test_apply_option_default_real_value_preserved() {
        let mut val = Some(SerializableIgnore::Dirty);
        let default = Some(SerializableIgnore::All);
        Config::apply_option_default(&mut val, &default, SerializableIgnore::Unspecified);
        assert_eq!(val, Some(SerializableIgnore::Dirty));
    }

    #[test]
    fn test_apply_option_default_none_value_none_default() {
        let mut val: Option<SerializableIgnore> = None;
        let default: Option<SerializableIgnore> = None;
        Config::apply_option_default(&mut val, &default, SerializableIgnore::Unspecified);
        // Falls back to the sentinel
        assert_eq!(val, Some(SerializableIgnore::Unspecified));
    }

    // ================================================================
    // Config::apply_defaults
    // ================================================================

    #[test]
    fn test_config_apply_defaults() {
        let defaults = SubmoduleDefaults {
            ignore: Some(SerializableIgnore::Dirty),
            fetch_recurse: Some(SerializableFetchRecurse::Always),
            update: Some(SerializableUpdate::Rebase),
        };
        let entry = SubmoduleEntry::new(
            Some("url".to_string()),
            Some("path".to_string()),
            None,
            None,
            None,
            None,
            Some(true),
            None,
            None,
        );
        let entries = SubmoduleEntries::default().add_submodule("mod1".to_string(), entry);
        let config = Config::new(defaults, entries);

        let applied = config.apply_defaults();
        let sub = applied.submodules.get("mod1").unwrap();
        // Submodule had None → should get defaults
        assert_eq!(sub.ignore, Some(SerializableIgnore::Dirty));
        assert_eq!(sub.fetch_recurse, Some(SerializableFetchRecurse::Always));
        assert_eq!(sub.update, Some(SerializableUpdate::Rebase));
    }

    #[test]
    fn test_config_apply_defaults_entry_overrides() {
        let defaults = SubmoduleDefaults {
            ignore: Some(SerializableIgnore::Dirty),
            fetch_recurse: Some(SerializableFetchRecurse::Always),
            update: Some(SerializableUpdate::Rebase),
        };
        let entry = SubmoduleEntry::new(
            Some("url".to_string()),
            Some("path".to_string()),
            None,
            Some(SerializableIgnore::All), // explicit override
            None,
            None,
            Some(true),
            None,
            None,
        );
        let entries = SubmoduleEntries::default().add_submodule("mod1".to_string(), entry);
        let config = Config::new(defaults, entries);

        let applied = config.apply_defaults();
        let sub = applied.submodules.get("mod1").unwrap();
        // Explicit override preserved
        assert_eq!(sub.ignore, Some(SerializableIgnore::All));
        // Others get defaults
        assert_eq!(sub.fetch_recurse, Some(SerializableFetchRecurse::Always));
        assert_eq!(sub.update, Some(SerializableUpdate::Rebase));
    }

    #[test]
    fn test_config_apply_defaults_no_submodules() {
        let config = Config::default();
        let applied = config.apply_defaults();
        // Should not panic
        assert!(applied.submodules.submodules().unwrap().is_empty());
    }

    // ================================================================
    // SubmoduleAddOptions conversions
    // ================================================================

    #[test]
    fn test_add_options_into_submodule_entry() {
        let opts = SubmoduleAddOptions {
            name: "mymod".to_string(),
            path: PathBuf::from("libs/mymod"),
            url: "https://example.com/repo.git".to_string(),
            branch: Some(SerializableBranch::Name("main".to_string())),
            ignore: Some(SerializableIgnore::Dirty),
            update: None,
            fetch_recurse: None,
            shallow: true,
            no_init: false,
        };
        let entry = opts.into_submodule_entry();
        assert_eq!(entry.url, Some("https://example.com/repo.git".to_string()));
        assert_eq!(entry.path, Some("libs/mymod".to_string()));
        assert_eq!(
            entry.branch,
            Some(SerializableBranch::Name("main".to_string()))
        );
        assert_eq!(entry.shallow, Some(true));
        assert_eq!(entry.active, Some(true)); // no_init=false → active=true
    }

    #[test]
    fn test_add_options_into_entry_no_init() {
        let opts = SubmoduleAddOptions {
            name: "mymod".to_string(),
            path: PathBuf::from("libs/mymod"),
            url: "https://example.com/repo.git".to_string(),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            shallow: false,
            no_init: true,
        };
        let entry = opts.into_submodule_entry();
        assert_eq!(entry.active, Some(false)); // no_init=true → active=false
        assert_eq!(entry.no_init, Some(true));
    }

    #[test]
    fn test_add_options_from_tuple_fallbacks() {
        // When url is None, falls back to path, then name
        let entry = SubmoduleEntry {
            url: None,
            path: Some("libs/mymod".to_string()),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            active: None,
            shallow: None,
            no_init: None,
            sparse_paths: None,
        };
        let opts = SubmoduleAddOptions::from_submodule_entries_tuple(("mymod".to_string(), entry));
        // url fallback: path
        assert_eq!(opts.url, "libs/mymod");
        assert_eq!(opts.path, PathBuf::from("libs/mymod"));
    }

    #[test]
    fn test_add_options_from_tuple_no_url_no_path() {
        let entry = SubmoduleEntry {
            url: None,
            path: None,
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            active: None,
            shallow: None,
            no_init: None,
            sparse_paths: None,
        };
        let opts = SubmoduleAddOptions::from_submodule_entries_tuple(("mymod".to_string(), entry));
        // Falls back to name for both url and path
        assert_eq!(opts.url, "mymod");
        assert_eq!(opts.path, PathBuf::from("mymod"));
    }

    // ================================================================
    // SubmoduleEntry::git_options
    // ================================================================

    #[test]
    fn test_entry_git_options() {
        let entry = SubmoduleEntry::new(
            Some("url".to_string()),
            Some("path".to_string()),
            Some(SerializableBranch::Name("dev".to_string())),
            Some(SerializableIgnore::All),
            Some(SerializableUpdate::Merge),
            Some(SerializableFetchRecurse::Never),
            Some(true),
            None,
            None,
        );
        let opts = entry.git_options();
        assert_eq!(
            opts.branch,
            Some(SerializableBranch::Name("dev".to_string()))
        );
        assert_eq!(opts.ignore, Some(SerializableIgnore::All));
        assert_eq!(opts.update, Some(SerializableUpdate::Merge));
        assert_eq!(opts.fetch_recurse, Some(SerializableFetchRecurse::Never));
    }

    // ================================================================
    // SubmoduleEntries: serialization roundtrip
    // ================================================================

    #[test]
    fn test_entries_serde_roundtrip() {
        let mut entries = SubmoduleEntries::default();
        let entry = SubmoduleEntry {
            url: Some("https://example.com/repo".to_string()),
            path: Some("libs/repo".to_string()),
            branch: Some(SerializableBranch::Name("main".to_string())),
            ignore: Some(SerializableIgnore::Dirty),
            update: Some(SerializableUpdate::Rebase),
            fetch_recurse: Some(SerializableFetchRecurse::Always),
            active: Some(true),
            shallow: Some(false),
            no_init: None,
            sparse_paths: Some(vec!["src/".to_string()]),
        };
        entries = entries.add_submodule("mymod".to_string(), entry);

        let serialized = toml::to_string(&entries).unwrap();
        let deserialized: SubmoduleEntries = toml::from_str(&serialized).unwrap();

        assert!(deserialized.submodules().unwrap().contains_key("mymod"));
        let de_entry = deserialized.submodules().unwrap().get("mymod").unwrap();
        assert_eq!(de_entry.url, Some("https://example.com/repo".to_string()));
        assert_eq!(
            de_entry.branch,
            Some(SerializableBranch::Name("main".to_string()))
        );
        assert_eq!(de_entry.ignore, Some(SerializableIgnore::Dirty));
        // Sparse paths should be in the sparse_checkouts map
        assert!(
            deserialized
                .sparse_checkouts()
                .unwrap()
                .contains_key("mymod")
        );
    }

    // ================================================================
    // OtherSubmoduleSettings::name_from_url
    // ================================================================

    #[test]
    fn test_other_settings_name_from_url() {
        assert_eq!(
            OtherSubmoduleSettings::name_from_url("https://github.com/user/repo.git"),
            "repo"
        );
        assert_eq!(
            OtherSubmoduleSettings::name_from_url("git@github.com:user/lib.git"),
            "lib"
        );
        assert_eq!(
            OtherSubmoduleSettings::name_from_url("https://github.com/user/repo/"),
            "repo"
        );
        assert_eq!(OtherSubmoduleSettings::name_from_url("simple"), "simple");
    }

    // ================================================================
    // Config full TOML roundtrip
    // ================================================================

    #[test]
    fn test_config_toml_roundtrip() {
        let toml_str = r#"
[defaults]
ignore = "dirty"
update = "rebase"

[mymod]
path = "libs/mymod"
url = "https://example.com/repo.git"
branch = "main"
active = true
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.defaults.ignore, Some(SerializableIgnore::Dirty));
        assert_eq!(config.defaults.update, Some(SerializableUpdate::Rebase));
        assert!(config.submodules.contains_key("mymod"));
        let entry = config.submodules.get("mymod").unwrap();
        assert_eq!(entry.url, Some("https://example.com/repo.git".to_string()));
        assert_eq!(
            entry.branch,
            Some(SerializableBranch::Name("main".to_string()))
        );
    }

    #[test]
    fn test_config_toml_branch_aliases() {
        let toml_str = r#"
[mymod]
path = "libs/mymod"
url = "https://example.com/repo.git"
branch = "."
active = true
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let entry = config.submodules.get("mymod").unwrap();
        assert_eq!(
            entry.branch,
            Some(SerializableBranch::CurrentInSuperproject)
        );

        let toml_str2 = r#"
[mymod]
path = "libs/mymod"
url = "https://example.com/repo.git"
branch = "super"
active = true
"#;
        let config2: Config = toml::from_str(toml_str2).unwrap();
        let entry2 = config2.submodules.get("mymod").unwrap();
        assert_eq!(
            entry2.branch,
            Some(SerializableBranch::CurrentInSuperproject)
        );
    }

    // ================================================================
    // SubmoduleGitOptions
    // ================================================================

    #[test]
    fn test_git_options_new() {
        let opts = SubmoduleGitOptions::new(
            Some(SerializableIgnore::All),
            Some(SerializableFetchRecurse::Always),
            Some(SerializableBranch::Name("main".to_string())),
            Some(SerializableUpdate::Merge),
        );
        assert_eq!(opts.ignore, Some(SerializableIgnore::All));
        assert_eq!(opts.fetch_recurse, Some(SerializableFetchRecurse::Always));
    }

    // ================================================================
    // SubmoduleEntries::from_gitmodules
    // ================================================================

    #[test]
    fn test_entries_from_gitmodules() {
        let mut outer = HashMap::new();
        let mut inner = HashMap::new();
        inner.insert(
            "url".to_string(),
            "https://example.com/repo.git".to_string(),
        );
        inner.insert("path".to_string(), "libs/repo".to_string());
        outer.insert("repo".to_string(), inner);

        let entries = SubmoduleEntries::from_gitmodules(outer);
        assert!(entries.contains_key("repo"));
        let entry = entries.get("repo").unwrap();
        assert_eq!(entry.url, Some("https://example.com/repo.git".to_string()));
    }

    // ================================================================
    // Provider trait
    // ================================================================

    #[test]
    fn test_config_provider_metadata() {
        let config = Config::default();
        let meta = config.metadata();
        assert_eq!(meta.name, "CLI arguments");
    }

    #[test]
    fn test_config_provider_profile() {
        let config = Config::default();
        assert!(config.profile().is_some());
    }

    #[test]
    fn test_config_provider_data() {
        let config = Config::default();
        let data = config.data();
        assert!(data.is_ok());
    }

    #[test]
    fn test_config_submodule_remote_check() {
        let mut config = Config::default();
        let entry = SubmoduleEntry::new(
            Some("https://github.com/user/repo".to_string()),
            Some("libs/repo".to_string()),
            None,
            None,
            None,
            None,
            Some(true),
            None,
            None,
        );
        config.add_submodule("repo".to_string(), entry);

        let retrieved = config.get_submodule("repo").expect("submodule should exist");
        assert!(retrieved.is_remote());
    }
}
