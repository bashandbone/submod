// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

#![doc = r"
Configuration types and utilities for submod.

Defines project-level defaults, and submodule
configuration management. Supports loading and saving configuration in TOML format.

Main Types:
- `SubmoduleGitOptions`: Git-specific options for a submodule.
- `SubmoduleDefaults`: Project-level default submodule options.
- `SubmoduleConfig`: Configuration for a single submodule.
- Config: Main configuration structure, containing defaults and all submodules.

Features:
- Load and save configuration from/to TOML files.
- Serialize/deserialize submodule options for config files.
- Manage submodule entries and defaults programmatically.
"]

use crate::options::SerializableBranch;
use crate::options::{
    GitmodulesConvert, SerializableFetchRecurse, SerializableIgnore, SerializableUpdate,
};
use anyhow::{Context, Result};
use serde::de::Deserializer;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use std::path::PathBuf;
use std::{collections::HashMap, path::Path};
// TODO: Implement figment::Profile for modular configs
use figment::{
    Metadata, Provider, Result as FigmentResult,
    value::{Dict, Map, Value},
};

/// Returns true. Used as a serde default for boolean fields.
const fn default_true() -> bool {
    true
}

/// Returns false. Used as a serde default for boolean fields.
const fn default_false() -> bool {
    false
}

/// Serialization skipping filter for `shallow` -- only serialize if the value is true,
/// So the function inverts falsey values to true.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn shallow_filter(shallow: &bool) -> bool {
    // We skip if false
    !shallow
}

// Just a type wrapper around str to make it clear what we're working with
/// A type alias for submodule names used throughout the configuration.
pub type SubmoduleName = String;

/// Git options for a submodule
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SubmoduleGitOptions {
    /// How to handle dirty files when updating submodules
    #[serde(default)]
    pub ignore: Option<SerializableIgnore>,
    /// Whether to fetch submodules recursively
    // Serialized as the documented `fetchRecurse` TOML key (see README). The rename keeps
    // the serialized and deserialized spellings identical so figment's layered merge does
    // not see two different keys for the same field.
    #[serde(default, rename = "fetchRecurse")]
    pub fetch_recurse: Option<SerializableFetchRecurse>,
    /// Branch to track for the submodule
    #[serde(default)]
    pub branch: Option<SerializableBranch>,
    /// Update strategy for the submodule
    #[serde(default)]
    pub update: Option<SerializableUpdate>,
}

impl Default for SubmoduleGitOptions {
    fn default() -> Self {
        Self {
            ignore: Some(SerializableIgnore::default()),
            fetch_recurse: Some(SerializableFetchRecurse::default()),
            branch: None,
            update: Some(SerializableUpdate::default()),
        }
    }
}

#[allow(dead_code)]
impl SubmoduleGitOptions {
    /// Create a new instance with defaults
    #[must_use]
    pub const fn new(
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
    #[must_use]
    pub const fn new(
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
            Some(i) => git2::SubmoduleIgnore::try_from(i).map_err(|()| {
                "Failed to convert SerializableIgnore to git2::SubmoduleIgnore".to_string()
            })?,
            None => git2::SubmoduleIgnore::Unspecified,
        };
        let update = match options.update {
            Some(u) => git2::SubmoduleUpdate::try_from(u).map_err(|()| {
                "Failed to convert SerializableUpdate to git2::SubmoduleUpdate".to_string()
            })?,
            None => git2::SubmoduleUpdate::Default,
        };
        let branch = options.branch.map(|b| b.to_gitmodules());
        let fetch_recurse = options.fetch_recurse.map(|fr| fr.to_gitmodules());
        Ok(Self::new(ignore, update, branch, fetch_recurse))
    }
}

/// Project-level defaults for git submodule options (for all submodules)
/// Can be used to set global defaults for submodule behavior in the repository
/// And overridden by submodule-specific configurations
#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SubmoduleDefaults {
    /// Branch inherited by modules without an explicit branch declaration.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<SerializableBranch>,
    /// [`Ignore`][SerializableIgnore] setting for submodules
    pub ignore: Option<SerializableIgnore>,
    /// [`Update`][SerializableUpdate] setting for submodules
    #[serde(rename = "fetchRecurse")]
    pub fetch_recurse: Option<SerializableFetchRecurse>,
    /// [`Update`][SerializableUpdate] setting for submodules
    pub update: Option<SerializableUpdate>,
    /// When `true`, use git's built-in sparse-checkout behavior (no `!/*` prefix is
    /// prepended). Defaults to `false`, which uses submod's deny-all-by-default model.
    /// Individual submodules can override this per-entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_git_default_sparse_checkout: Option<bool>,
}

impl Iterator for SubmoduleDefaults {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        Some(self.clone())
    }
}

#[allow(dead_code)]
impl SubmoduleDefaults {
    /// Returns a vector of `SubmoduleDefaults` with the current values (for comparison)
    #[must_use]
    pub fn get_values(&self) -> Vec<Self> {
        vec![self.clone()].into_iter().flatten().collect()
    }

    /// Merge another `SubmoduleDefaults` into this one. Only updates fields that are set in the other. Returns a new instance with the merged values.
    #[must_use]
    pub fn merge_from(&self, other: Self) -> Self {
        let mut mut_self = self.clone();
        if other.branch.is_some() {
            mut_self.branch = other.branch;
        }
        if other.ignore.is_some() {
            mut_self.ignore = other.ignore;
        }
        if other.fetch_recurse.is_some() {
            mut_self.fetch_recurse = other.fetch_recurse;
        }
        if other.update.is_some() {
            mut_self.update = other.update;
        }
        if other.use_git_default_sparse_checkout.is_some() {
            mut_self.use_git_default_sparse_checkout = other.use_git_default_sparse_checkout;
        }
        {
            let ignore = mut_self.ignore;
            let update = mut_self.update;
            Self {
                branch: mut_self.branch,
                ignore: ignore.or_else(|| Some(SerializableIgnore::default())),
                fetch_recurse: mut_self
                    .fetch_recurse
                    .or_else(|| Some(SerializableFetchRecurse::default())),
                update: update.or_else(|| Some(SerializableUpdate::default())),
                use_git_default_sparse_checkout: mut_self.use_git_default_sparse_checkout,
            }
        }
    }
}

/// Serialize a submodule path for `submod.toml` using Git's spelling.
///
/// Nested paths use forward slashes in `.gitmodules` on every platform, so
/// keep that spelling where the OS separates with backslashes. A Unix
/// backslash is a filename character rather than a separator and stays
/// verbatim, so the fold is Windows-only.
pub(crate) fn stored_submodule_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    #[cfg(windows)]
    {
        text.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        text.into_owned()
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
    /// Create an add options from a `SubmoduleEntry`
    #[must_use]
    pub fn into_submodule_entry(self) -> SubmoduleEntry {
        SubmoduleEntry {
            url: Some(self.url),
            path: Some(stored_submodule_path(&self.path)),
            branch: self.branch,
            ignore: self.ignore,
            update: self.update,
            fetch_recurse: self.fetch_recurse,
            shallow: Some(self.shallow),
            active: Some(!self.no_init), // we're adding so unless we have a 'no_init" flag, we can assume active
            no_init: Some(self.no_init),
            sparse_paths: None,
            use_git_default_sparse_checkout: None,
        }
    }

    /// Create an add options from a entries tuple (name and `SubmoduleEntry`)
    pub fn from_submodule_entries_tuple(entry: (SubmoduleName, SubmoduleEntry)) -> Self {
        let (name, submodule_entry) = entry;
        Self {
            name: name.clone(),
            url: submodule_entry
                .url
                .unwrap_or_else(|| submodule_entry.path.clone().unwrap_or_else(|| name.clone())),
            path: submodule_entry
                .path
                .map_or_else(|| PathBuf::from(name.clone()), PathBuf::from),
            branch: submodule_entry.branch,
            ignore: submodule_entry.ignore,
            update: submodule_entry.update,
            fetch_recurse: submodule_entry.fetch_recurse,
            shallow: submodule_entry.shallow.is_some_and(|s| s),
            no_init: submodule_entry.no_init.is_some_and(|f| f),
        }
    }

    /// Convert an `AddOptions` to a `SubmoduleEntries` tuple
    #[must_use]
    pub fn into_entries_tuple(self) -> (SubmoduleName, SubmoduleEntry) {
        (self.name.clone(), self.into_submodule_entry())
    }
}

/// Options for updating a submodule
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct SubmoduleUpdateOptions {
    /// Update strategy to use
    pub strategy: SerializableUpdate,
    /// Whether to update recursively
    pub recursive: bool,
    /// Whether to force the update
    pub force: bool,
    /// Whether to advance to the configured remote tracking target.
    pub remote: bool,
}

#[allow(dead_code)]
impl SubmoduleUpdateOptions {
    /// Create a new instance with defaults
    #[must_use]
    pub const fn new(strategy: SerializableUpdate, recursive: bool, force: bool) -> Self {
        Self {
            strategy,
            recursive,
            force,
            remote: false,
        }
    }

    /// Get a new instance with the recursive flag set
    #[must_use]
    pub fn forced(&self) -> Self {
        Self {
            strategy: self.strategy.clone(),
            recursive: self.recursive,
            force: true, // Set force to true
            remote: self.remote,
        }
    }

    /// Convert from `SubmoduleGitOptions` to `SubmoduleUpdateOptions`
    #[must_use]
    pub fn from_options(options: SubmoduleGitOptions) -> Self {
        Self {
            strategy: options.update.unwrap_or_default(),
            recursive: false,
            force: false, // Default to not force
            remote: false,
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
    const fn default() -> Self {
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
        let name = name.or_else(|| {
            path.as_ref().map_or_else(
                || url.as_ref().map(|u| Self::name_from_url(u)),
                |p| Some(p.clone()),
            )
        });
        let final_path = path.or_else(|| url.as_ref().map(|u| Self::name_from_url(u)));
        let final_url = url.or_else(|| Some(".".to_string()));
        Self {
            url: final_url,
            path: final_path,
            name,
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

    /// Create a new instance from `SubmoduleEntry`, optionally providing a name
    #[must_use]
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
    #[must_use]
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
    #[serde(rename = "fetchRecurse")]
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
    /// When `true`, use git's built-in sparse-checkout behavior instead of submod's
    /// deny-all-by-default model.  Overrides the global `[defaults]` setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_git_default_sparse_checkout: Option<bool>,
}

#[allow(dead_code)]
impl SubmoduleEntry {
    /// Create a new submodule entry with defaults
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
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
            active,
            shallow,
            no_init,
            sparse_paths: None,
            use_git_default_sparse_checkout: None,
        }
    }

    /// Create a new submodule entry with defaults, using the URL and path from `OtherSubmoduleSettings`
    #[must_use]
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
    #[must_use]
    pub fn from_gitmodules(
        name: &str,
        entries: &std::collections::HashMap<String, String>,
    ) -> Self {
        let url = entries.get("url").cloned();
        let path = entries
            .get("path")
            .cloned()
            .map_or_else(|| Some(name.to_string()), Some);
        let branch =
            SerializableBranch::from_git_branch(entries.get("branch").map_or("", |b| b.as_str()))
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
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn is_local(&self) -> bool {
        let url = self.url.clone().unwrap_or_default();
        url.starts_with("./") || url.starts_with("../") || url.starts_with('/')
    }

    /// Returns true if the url is a remote repository (http, ssh, git, etc)
    #[must_use]
    pub fn is_remote(&self) -> bool {
        let url = self.url.clone().unwrap_or_default();
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
    #[must_use]
    pub fn git_options(&self) -> SubmoduleGitOptions {
        SubmoduleGitOptions {
            ignore: self.ignore,
            fetch_recurse: self.fetch_recurse,
            branch: self.branch.clone(),
            update: self.update.clone(),
        }
    }

    /// Returns the non-git settings for this submodule entry (path, url, active state, etc.).
    #[must_use]
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
        Git2SubmoduleOptions::try_from(self.git_options()).map_err(|e| anyhow::anyhow!(e))
    }

    /// convert path to `PathBuf` for filesystem operations
    pub fn path_as_pathbuf(&self) -> Option<PathBuf> {
        self.path.as_ref().map(PathBuf::from)
    }

    /// Convert the submodule's URL to a string
    #[must_use]
    pub fn url_as_string(&self) -> String {
        self.url.clone().unwrap_or_default()
    }

    /// Get the configuration active setting
    #[must_use]
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
            use_git_default_sparse_checkout: None,
        }
    }
}

/// A collection of submodule entries, including sparse checkouts
///
/// Revamped to better reflect git's structure so we can use the `SubmoduleEntry` types directly with gix/git2
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmoduleEntries {
    submodules: Option<HashMap<SubmoduleName, SubmoduleEntry>>,
}

impl<'de> Deserialize<'de> for SubmoduleEntries {
    /// Deserialize from the flat TOML format where each top-level key is a submodule name.
    /// Each entry owns its sparse patterns.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map: HashMap<SubmoduleName, SubmoduleEntry> = HashMap::deserialize(deserializer)?;
        Ok(Self {
            submodules: Some(map),
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

impl Default for SubmoduleEntries {
    fn default() -> Self {
        Self {
            submodules: Some(HashMap::new()),
        }
    }
}

#[allow(dead_code)]
impl SubmoduleEntries {
    /// Create a new empty `SubmoduleEntries`
    #[must_use]
    pub fn new(
        submodules: Option<HashMap<SubmoduleName, SubmoduleEntry>>,
        sparse_checkouts: Option<HashMap<SubmoduleName, Vec<String>>>,
    ) -> Self {
        let mut entries = Self {
            submodules: Some(submodules.unwrap_or_default()),
        };
        for (name, paths) in sparse_checkouts.unwrap_or_default() {
            entries.add_checkout(&name, &paths, true);
        }
        entries
    }

    /// Add a submodule entry
    #[must_use]
    pub fn add_submodule(mut self, name: SubmoduleName, entry: SubmoduleEntry) -> Self {
        self.update_entry(name, entry);
        self
    }

    /// Remove a submodule entry
    #[must_use]
    pub fn remove_submodule(&mut self, name: &str) -> Option<SubmoduleEntry> {
        self.submodules.as_mut()?.remove(name)
    }

    /// Returns a list of all submodule names, or `None` if no submodules are configured.
    #[must_use]
    pub fn submodule_names(&self) -> Option<Vec<String>> {
        self.submodules
            .as_ref()
            .map(|s| s.keys().cloned().collect())
    }

    /// Get the submodules map
    #[must_use]
    pub const fn submodules(&self) -> Option<&HashMap<SubmoduleName, SubmoduleEntry>> {
        self.submodules.as_ref()
    }

    /// Get the sparse checkouts map
    #[must_use]
    pub fn sparse_checkouts(&self) -> Option<HashMap<SubmoduleName, Vec<String>>> {
        Some(
            self.sparse_iter()
                .map(|(name, paths)| (name.clone(), paths.clone()))
                .collect(),
        )
    }

    /// Add or replace patterns on the authoritative entry.
    pub fn add_checkout(&mut self, name: &str, checkout: &[String], replace: bool) {
        if let Some(entry) = self.submodules.as_mut().and_then(|m| m.get_mut(name)) {
            let paths = entry.sparse_paths.get_or_insert_with(Vec::new);
            if replace {
                paths.clear();
            }
            paths.extend_from_slice(checkout);
        }
    }

    /// Remove all sparse patterns.
    pub fn delete_checkout(&mut self, name: &str) {
        self.set_sparse_paths_for(name, Vec::new());
    }

    /// Remove a sparse pattern.
    pub fn remove_sparse_path(&mut self, name: &str, path: &str) {
        if let Some(entry) = self.submodules.as_mut().and_then(|m| m.get_mut(name))
            && let Some(paths) = &mut entry.sparse_paths
        {
            paths.retain(|p| p != path);
        }
    }

    /// Append a sparse pattern.
    pub fn add_sparse_path(&mut self, name: &str, path: String) {
        self.add_checkout(name, &[path], false);
    }

    /// Get a submodule entry by name
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&SubmoduleEntry> {
        self.submodules.as_ref()?.get(name)
    }

    /// Returns `true` if a submodule with the given name exists.
    #[must_use]
    pub fn contains_key(&self, name: &str) -> bool {
        self.submodules
            .as_ref()
            .is_some_and(|s| s.contains_key(name))
    }

    /// Get an iterator over all submodule entries
    pub fn submodule_iter(&self) -> impl Iterator<Item = (&SubmoduleName, &SubmoduleEntry)> {
        self.submodules.as_ref().into_iter().flat_map(|s| s.iter())
    }

    /// Get an iterator over all sparse checkouts
    pub fn sparse_iter(&self) -> impl Iterator<Item = (&SubmoduleName, &Vec<String>)> {
        self.submodule_iter().filter_map(|(name, entry)| {
            entry
                .sparse_paths
                .as_ref()
                .filter(|p| !p.is_empty())
                .map(|paths| (name, paths))
        })
    }

    /// Get an iterator that returns a tuple of submodule and sparse checkout
    pub fn iter(&self) -> impl Iterator<Item = (&SubmoduleName, (&SubmoduleEntry, &[String]))> {
        self.submodule_iter().map(move |(name, entry)| {
            let sparse = entry.sparse_paths.as_deref().unwrap_or_default();
            (name, (entry, sparse))
        })
    }

    /// Create a new `SubmoduleEntries` from a `HashMap` of submodule entries
    #[must_use]
    pub fn from_gitmodules(
        entries: std::collections::HashMap<String, std::collections::HashMap<String, String>>,
    ) -> Self {
        let mut submodules = HashMap::new();
        for (name, entry) in entries {
            let submodule_entry = SubmoduleEntry::from_gitmodules(&name, &entry);
            submodules.insert(name, submodule_entry);
        }
        Self {
            submodules: Some(submodules),
        }
    }
    /// Insert or replace a submodule entry by name.
    pub fn update_entry(&mut self, name: SubmoduleName, entry: SubmoduleEntry) {
        self.submodules
            .get_or_insert_with(HashMap::new)
            .insert(name, entry);
    }

    /// Set sparse paths for an existing entry.
    pub fn set_sparse_paths_for(&mut self, name: &str, paths: Vec<String>) {
        if let Some(entry) = self.submodules.as_mut().and_then(|m| m.get_mut(name)) {
            entry.sparse_paths = if paths.is_empty() { None } else { Some(paths) };
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
#[derive(Debug, Default, Clone, Serialize)]
pub struct Config {
    /// Global default settings that apply to all submodules
    #[serde(default)]
    pub defaults: SubmoduleDefaults,
    /// Individual submodule configurations, keyed by submodule name
    #[serde(flatten)]
    pub submodules: SubmoduleEntries,
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let table = toml::Table::deserialize(deserializer)?;
        Self::from_table(table, None)
            .map_err(|error| serde::de::Error::custom(format!("{error:#}")))
    }
}

#[allow(dead_code)]
impl Config {
    /// Parse and validate raw declarations, without resolving inherited fields.
    pub fn parse(source: &str) -> Result<Self> {
        let document = toml_edit::Document::parse(source)?;
        Self::from_table(toml::from_str(source)?, Some(&document))
    }

    fn from_table(
        mut table: toml::Table,
        document: Option<&toml_edit::Document<&str>>,
    ) -> Result<Self> {
        let location = |section: &str, field: &str| {
            let label = match (section.is_empty(), field.is_empty()) {
                (true, _) => field.to_string(),
                (_, true) => format!("[{section}]"),
                _ => format!("[{section}].{field}"),
            };
            let Some(document) = document else {
                return label;
            };
            let section_item = if section.is_empty() {
                Some(document.as_item())
            } else {
                document.get(section)
            };
            let span = section_item.and_then(|item| {
                item.as_table_like()
                    .and_then(|table| {
                        table
                            .get_key_value(field)
                            .or_else(|| {
                                (field == "fetchRecurse")
                                    .then(|| {
                                        table
                                            .get_key_value("fetch")
                                            .or_else(|| table.get_key_value("fetch_recurse"))
                                    })
                                    .flatten()
                            })
                            .and_then(|(key, value)| key.span().or_else(|| value.span()))
                    })
                    .or_else(|| item.span())
            });
            span.map_or_else(
                || label.clone(),
                |span| {
                    let line = document.raw()[..span.start]
                        .bytes()
                        .filter(|byte| *byte == b'\n')
                        .count()
                        + 1;
                    format!("{label} (line {line})")
                },
            )
        };
        if let Some(version) = table.remove("schema_version") {
            anyhow::ensure!(
                matches!(version.as_str(), Some("1.0.0" | "1.1.0")),
                "{}: unsupported version {version}; expected 1.0.0 or 1.1.0",
                location("", "schema_version")
            );
        }
        let mut defaults = SubmoduleDefaults::default();
        let mut entries = SubmoduleEntries::default();
        for (name, value) in table {
            let mut fields = value
                .as_table()
                .cloned()
                .with_context(|| format!("{}: expected a table", location(&name, "")))?;
            let global = name == "defaults";
            for key in fields.keys() {
                let common = matches!(
                    key.as_str(),
                    "branch"
                        | "ignore"
                        | "update"
                        | "fetchRecurse"
                        | "fetch"
                        | "fetch_recurse"
                        | "use_git_default_sparse_checkout"
                );
                anyhow::ensure!(
                    common
                        || (!global
                            && matches!(
                                key.as_str(),
                                "url" | "path" | "branch" | "active" | "shallow" | "sparse_paths"
                            )),
                    "{}: unknown field",
                    location(&name, key)
                );
            }
            let spellings: Vec<_> = ["fetchRecurse", "fetch", "fetch_recurse"]
                .into_iter()
                .filter(|key| fields.contains_key(*key))
                .collect();
            anyhow::ensure!(
                spellings.len() <= 1,
                "{}: conflicting aliases {spellings:?}",
                location(&name, "fetchRecurse")
            );
            if let Some(alias) = spellings.first().filter(|alias| **alias != "fetchRecurse") {
                let value = fields.remove(*alias).expect("present alias");
                fields.insert("fetchRecurse".into(), value);
                eprintln!(
                    "warning: [{}].{alias} is legacy; use fetchRecurse",
                    crate::utilities::safe_human_text(&name)
                );
            }
            if matches!(
                fields.get("fetchRecurse").and_then(toml::Value::as_str),
                Some("true" | "false")
            ) {
                eprintln!(
                    "warning: [{}].fetchRecurse uses a legacy value; use always/never",
                    crate::utilities::safe_human_text(&name)
                );
            }
            if fields.get("branch").and_then(toml::Value::as_str) == Some("HEAD") {
                eprintln!(
                    "warning: [{}].branch=HEAD is legacy remote-default tracking and explicitly overrides any global branch",
                    crate::utilities::safe_human_text(&name)
                );
            }
            for (key, value) in &fields {
                let valid = match key.as_str() {
                    "active" | "shallow" | "use_git_default_sparse_checkout" => value.is_bool(),
                    "url" | "path" | "branch" => value.is_str(),
                    "sparse_paths" => value
                        .as_array()
                        .is_some_and(|paths| paths.iter().all(toml::Value::is_str)),
                    "ignore" => value.clone().try_into::<SerializableIgnore>().is_ok(),
                    "update" => value.clone().try_into::<SerializableUpdate>().is_ok(),
                    "fetchRecurse" => value.clone().try_into::<SerializableFetchRecurse>().is_ok(),
                    _ => true,
                };
                anyhow::ensure!(
                    valid,
                    "{}: invalid value or type {value}",
                    location(&name, key)
                );
                if key == "branch" {
                    anyhow::ensure!(
                        value.clone().try_into::<SerializableBranch>().is_ok(),
                        "{}: invalid branch {value}",
                        location(&name, "branch")
                    );
                }
            }
            if global {
                defaults = toml::Value::Table(fields)
                    .try_into()
                    .with_context(|| location("defaults", ""))?;
            } else {
                let entry: SubmoduleEntry = toml::Value::Table(fields)
                    .try_into()
                    .with_context(|| location(&name, ""))?;
                anyhow::ensure!(
                    entry.url.as_deref().is_some_and(
                        |url| !url.trim().is_empty() && !url.contains(['\0', '\n', '\r'])
                    ),
                    "{}: required nonempty URL without NUL/newlines",
                    location(&name, "url")
                );
                let path = entry.path.as_deref().unwrap_or(&name);
                anyhow::ensure!(
                    !path.trim().is_empty() && !path.contains(['\0', '\n', '\r']),
                    "{}: invalid path characters",
                    location(&name, "path")
                );
                crate::utilities::normalize_submodule_path(Path::new(path))
                    .with_context(|| location(&name, "path"))?;
                if let Some(patterns) = &entry.sparse_paths {
                    anyhow::ensure!(
                        patterns.iter().all(|p| !p.contains(['\0', '\n', '\r'])),
                        "{}: patterns cannot contain NUL or newlines",
                        location(&name, "sparse_paths")
                    );
                }
                entries.update_entry(name, entry);
            }
        }
        Ok(Self::new(defaults, entries))
    }

    /// Resolve one entry without changing its raw declaration.
    #[must_use]
    pub fn effective_entry(&self, name: &str) -> Option<SubmoduleEntry> {
        let mut entry = self.submodules.get(name)?.clone();
        entry.path.get_or_insert_with(|| name.to_string());
        entry.branch = entry.branch.or_else(|| self.defaults.branch.clone());
        entry.ignore = entry
            .ignore
            .or(self.defaults.ignore)
            .or_else(|| Some(SerializableIgnore::default()));
        entry.update = entry
            .update
            .or_else(|| self.defaults.update.clone())
            .or_else(|| Some(SerializableUpdate::default()));
        entry.fetch_recurse = entry
            .fetch_recurse
            .or(self.defaults.fetch_recurse)
            .or_else(|| Some(SerializableFetchRecurse::default()));
        entry.use_git_default_sparse_checkout = entry
            .use_git_default_sparse_checkout
            .or(self.defaults.use_git_default_sparse_checkout)
            .or(Some(false));
        entry.active = Some(entry.active.unwrap_or(true));
        entry.shallow = Some(entry.shallow.unwrap_or(false));
        Some(entry)
    }

    /// Create a new configuration with the given defaults and submodules
    #[must_use]
    pub const fn new(defaults: SubmoduleDefaults, submodules: SubmoduleEntries) -> Self {
        Self {
            defaults,
            submodules,
        }
    }

    fn get_submodule_entry(&self, name: &str) -> Option<&SubmoduleEntry> {
        self.submodules.get(name)
    }
    /// Helper to apply a default if the value is None or Unspecified
    fn apply_option_default<T: Clone + PartialEq>(
        value: &mut Option<T>,
        default: Option<&T>,
        unspecified: T,
    ) {
        if value.is_none() || value.as_ref() == Some(&unspecified) {
            *value = default.cloned().or(Some(unspecified));
        } else {
            *value = value.clone().or(Some(unspecified));
        }
    }

    /// Create a new configuration, resolving defaults
    #[must_use]
    pub fn apply_defaults(mut self) -> Self {
        let resolved: Vec<_> = self
            .get_submodules()
            .filter_map(|(name, _)| {
                self.effective_entry(name)
                    .map(|entry| (name.clone(), entry))
            })
            .collect();
        for (name, entry) in resolved {
            self.submodules.update_entry(name, entry);
        }
        self
    }

    /// Add a submodule configuration
    pub fn add_submodule(&mut self, name: String, submodule: SubmoduleEntry) {
        self.submodules.update_entry(name, submodule);
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
    pub fn entries(&self) -> impl Iterator<Item = (&SubmoduleName, (&SubmoduleEntry, &[String]))> {
        self.submodules.iter()
    }

    /// Get a submodule configuration by name
    /// Returns None if the submodule does not exist
    #[must_use]
    pub fn get_submodule(&self, name: &str) -> Option<&SubmoduleEntry> {
        self.submodules.get(name)
    }

    /// Overlay CLI-supplied options onto this configuration.
    ///
    /// Only fields explicitly set on the CLI override the current values: a
    /// `None` `[defaults]` field (or an empty submodule set) leaves the existing
    /// value intact, so an absent CLI flag never erases configuration the file
    /// provided. Merging the whole `cli_options` struct through figment instead
    /// would serialize its unset `[defaults]` fields as nulls and clobber the
    /// file's values (#62 P1).
    fn merge_cli_overrides(&mut self, cli: Self) {
        let cli_defaults = cli.defaults;
        if cli_defaults.branch.is_some() {
            self.defaults.branch = cli_defaults.branch;
        }
        if cli_defaults.ignore.is_some() {
            self.defaults.ignore = cli_defaults.ignore;
        }
        if cli_defaults.fetch_recurse.is_some() {
            self.defaults.fetch_recurse = cli_defaults.fetch_recurse;
        }
        if cli_defaults.update.is_some() {
            self.defaults.update = cli_defaults.update;
        }
        if cli_defaults.use_git_default_sparse_checkout.is_some() {
            self.defaults.use_git_default_sparse_checkout =
                cli_defaults.use_git_default_sparse_checkout;
        }
        // CLI submodule entries override/extend by name (no-op when none given).
        for (name, entry) in cli.submodules {
            self.submodules.update_entry(name, entry);
        }
    }

    /// Load configuration from a file, merging with CLI options
    #[allow(clippy::unused_self)]
    pub fn load(&self, path: impl AsRef<Path>, cli_options: Self) -> anyhow::Result<Self> {
        let path = path.as_ref();
        let mut cfg = Self::parse(&std::fs::read_to_string(path)?).map_err(|error| {
            anyhow::anyhow!("Invalid configuration {}: {error:#}", path.display())
        })?;
        cfg.merge_cli_overrides(cli_options);
        Ok(cfg)
    }

    /// Load raw declarations from a file.
    ///
    /// The `Option<impl AsRef<Path>>` parameter is deliberate: call sites pass
    /// owned paths, borrows, and `None` interchangeably.
    #[allow(clippy::needless_pass_by_value)]
    pub fn load_from_file(&self, path: Option<impl AsRef<Path>>) -> anyhow::Result<Self> {
        self.load(
            path.map_or_else(
                || Path::new("submod.toml").to_path_buf(),
                |path| path.as_ref().to_path_buf(),
            ),
            Self::default(),
        )
    }
}

const REPO: figment::Profile = figment::Profile::const_new("repo");

// TODO: Implement figment::Profile for modular configs
/*
const USER: `figment::Profile` = `figment::Profile::const_new("user`");
const DEVELOPER: `figment::Profile` = `figment::Profile::const_new("developer`");
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
// `figment::Jail::expect_with` fixes the closure error type to the large
// `figment::Error`; mapping it in every test would obscure the assertions.
#[allow(clippy::result_large_err)]
mod tests {
    use super::*;

    // ================================================================
    // SubmoduleDefaults::merge_from
    // ================================================================

    #[test]
    fn semantic_errors_identify_original_source_lines() {
        for (source, field, line) in [
            (
                "# metadata\n\nschema_version = '9.0.0'\n",
                "schema_version",
                3,
            ),
            (
                "# module\n[module]\nurl = 'repo'\n\nunknown = true\n",
                "[module].unknown",
                5,
            ),
            (
                "[module]\nurl = 'repo'\n# policy\nactive = 'false'\n",
                "[module].active",
                4,
            ),
            (
                "[defaults]\n# historical spelling\nfetch = true\n",
                "[defaults].fetchRecurse",
                3,
            ),
            (
                "# quoted names and keys\n['a.b']\nurl = 'repo'\n'active' = 'false'\n",
                "[a.b].active",
                4,
            ),
            (
                "# inline table\nmodule = { url = 'repo', active = 'false' }\n",
                "[module].active",
                2,
            ),
        ] {
            let error = Config::parse(source).unwrap_err().to_string();
            assert!(error.contains(field), "{error}");
            assert!(error.contains(&format!("line {line}")), "{error}");
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("submod.toml");
        std::fs::write(&path, "# metadata\nschema_version='9.0.0'\n").unwrap();
        let error = Config::default()
            .load(&path, Config::default())
            .unwrap_err()
            .to_string();
        assert!(error.contains("schema_version (line 2)"), "{error}");
        assert!(error.contains(&path.display().to_string()), "{error}");
    }

    #[test]
    fn global_branch_is_inherited_without_pinning_or_overriding_head() {
        let mut config = Config::parse("[defaults]\nbranch='main'\n[inherited]\nurl='repo'\n[explicit]\nurl='repo'\nbranch='HEAD'\n").unwrap();
        assert_eq!(
            config.defaults.branch,
            Some(SerializableBranch::Name("main".into()))
        );
        assert_eq!(config.get_submodule("inherited").unwrap().branch, None);
        assert_eq!(
            config.effective_entry("inherited").unwrap().branch,
            config.defaults.branch
        );
        assert_eq!(
            config.effective_entry("explicit").unwrap().branch,
            Some(SerializableBranch::Name("HEAD".into()))
        );
        config.defaults.branch = Some(SerializableBranch::CurrentInSuperproject);
        let saved = toml::to_string(&config).unwrap();
        let reloaded = Config::parse(&saved).unwrap();
        assert_eq!(reloaded.get_submodule("inherited").unwrap().branch, None);
        assert_eq!(
            reloaded.effective_entry("inherited").unwrap().branch,
            Some(SerializableBranch::CurrentInSuperproject)
        );
        assert_eq!(
            reloaded.effective_entry("explicit").unwrap().branch,
            Some(SerializableBranch::Name("HEAD".into()))
        );
        for value in ["''", "'bad..branch'", "true", "1"] {
            assert!(
                Config::parse(&format!("[defaults]\nbranch={value}"))
                    .unwrap_err()
                    .to_string()
                    .contains("[defaults].branch")
            );
        }
        let merged = SubmoduleDefaults::default().merge_from(config.defaults.clone());
        assert_eq!(merged.branch, config.defaults.branch);
        let mut cli = Config::default();
        cli.defaults.branch = Some(SerializableBranch::Name("develop".into()));
        config.merge_cli_overrides(cli);
        assert_eq!(
            config.defaults.branch,
            Some(SerializableBranch::Name("develop".into()))
        );
    }

    #[test]
    fn raw_defaults_survive_resolution_and_serialization() {
        let mut config = Config::parse("[defaults]\nignore='dirty'\nuse_git_default_sparse_checkout=true\n[inherited]\nurl='repo'\n[explicit]\nurl='repo'\nignore='none'\nuse_git_default_sparse_checkout=false\n").unwrap();
        config.defaults.ignore = Some(SerializableIgnore::All);
        assert_eq!(config.get_submodule("inherited").unwrap().ignore, None);
        assert_eq!(
            config.effective_entry("inherited").unwrap().ignore,
            Some(SerializableIgnore::All)
        );
        assert_eq!(
            config.effective_entry("inherited").unwrap().path.as_deref(),
            Some("inherited")
        );
        assert_eq!(config.effective_entry("inherited").unwrap().branch, None);
        assert_eq!(
            config.effective_entry("explicit").unwrap().ignore,
            Some(SerializableIgnore::None)
        );
        assert_eq!(
            config
                .effective_entry("explicit")
                .unwrap()
                .use_git_default_sparse_checkout,
            Some(false)
        );
        let reloaded = Config::parse(&toml::to_string(&config).unwrap()).unwrap();
        assert_eq!(reloaded.get_submodule("inherited").unwrap().ignore, None);
        assert_eq!(
            reloaded.effective_entry("inherited").unwrap().ignore,
            Some(SerializableIgnore::All)
        );
    }

    #[test]
    fn config_schema_aliases_and_legacy_values() {
        for version in ["", "schema_version='1.0.0'\n", "schema_version='1.1.0'\n"] {
            for key in ["fetchRecurse", "fetch", "fetch_recurse"] {
                let source =
                    format!("{version}[module]\nurl='repo'\n{key}='true'\nbranch='HEAD'\n");
                let config: Config = toml::from_str(&source).unwrap();
                assert_eq!(
                    config.get_submodule("module").unwrap().fetch_recurse,
                    Some(SerializableFetchRecurse::Always)
                );
                assert_eq!(
                    config.effective_entry("module").unwrap().branch,
                    Some(SerializableBranch::Name("HEAD".into()))
                );
            }
        }
        Config::parse(include_str!("../sample_config/submod.toml")).unwrap();
    }

    #[test]
    // spellchecker:off
    fn config_rejects_invalid_fields_before_actions() {
        for (source, context) in [
            ("schema_version='9.0.0'", "schema_version"),
            ("schema_version=1", "schema_version"),
            (
                "[module]\nurl='repo'\nfetch='always'\nfetchRecurse='never'",
                "conflicting aliases",
            ),
            ("[defaults]\nignroe='all'", "ignroe"),
            ("[module]\nurl='repo'\nunknown=true", "unknown"),
            ("[module]\npath='child'", "url"),
            ("[module]\nurl='  '", "url"),
            ("[module]\nurl='repo'\npath='../outside'", "path"),
            ("[module]\nurl='repo'\npath='.'", "path"),
            ("[module]\nurl='repo'\npath='.git/config'", "path"),
            ("[module]\nurl='repo'\nactive='false'", "module"),
            ("[module]\nurl='repo'\nupdate='!evil'", "module"),
            (
                "[module]\nurl='repo'\nsparse_paths=[\"a\\nb\"]",
                "sparse_paths",
            ),
            ("[module]\nurl='repo'\nsparse_paths=[true]", "module"),
            ("module=true", "module"),
        ] {
            let error = Config::parse(source).unwrap_err();
            assert!(
                format!("{error:#}").contains(context),
                "{source}: {error:#}"
            );
        }
    }
    // spellchecker:on
    #[test]
    fn sparse_patterns_have_one_authority() {
        let mut config = Config::parse("[module]\nurl='repo'\nsparse_paths=['src/']").unwrap();
        config.submodules.add_sparse_path("module", "docs/".into());
        let mut entry = config.get_submodule("module").unwrap().clone();
        assert_eq!(entry.sparse_paths.as_ref().unwrap().len(), 2);
        entry.sparse_paths = Some(vec!["lib/".into()]);
        config.submodules = config.submodules.add_submodule("module".into(), entry);
        assert_eq!(
            config.get_sparse_checkouts().next().unwrap().1,
            &vec!["lib/".to_string()]
        );
        let _ = config.submodules.remove_submodule("module");
        assert!(config.get_sparse_checkouts().next().is_none());
    }

    #[test]
    fn test_defaults_merge_from_both_set() {
        let base = SubmoduleDefaults {
            branch: None,
            ignore: Some(SerializableIgnore::All),
            fetch_recurse: Some(SerializableFetchRecurse::Always),
            update: Some(SerializableUpdate::Rebase),
            use_git_default_sparse_checkout: None,
        };
        let other = SubmoduleDefaults {
            branch: None,
            ignore: Some(SerializableIgnore::Dirty),
            fetch_recurse: None,
            update: Some(SerializableUpdate::Merge),
            use_git_default_sparse_checkout: None,
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
            branch: None,
            ignore: Some(SerializableIgnore::All),
            fetch_recurse: Some(SerializableFetchRecurse::Never),
            update: Some(SerializableUpdate::Checkout),
            use_git_default_sparse_checkout: None,
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
            branch: None,
            ignore: Some(SerializableIgnore::Dirty),
            fetch_recurse: Some(SerializableFetchRecurse::Always),
            update: Some(SerializableUpdate::Merge),
            use_git_default_sparse_checkout: None,
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

    #[test]
    fn test_defaults_merge_from_carries_other_sparse_default() {
        // Regression (#62 P2): merge_from dropped other.use_git_default_sparse_checkout.
        let base = SubmoduleDefaults {
            branch: None,
            ignore: None,
            fetch_recurse: None,
            update: None,
            use_git_default_sparse_checkout: None,
        };
        let other = SubmoduleDefaults {
            branch: None,
            ignore: None,
            fetch_recurse: None,
            update: None,
            use_git_default_sparse_checkout: Some(true),
        };
        let merged = base.merge_from(other);
        assert_eq!(
            merged.use_git_default_sparse_checkout,
            Some(true),
            "other.use_git_default_sparse_checkout must override an unset base value"
        );
    }

    #[test]
    fn test_defaults_merge_from_other_sparse_default_overrides_base() {
        // The override must win even when base already holds a value.
        let base = SubmoduleDefaults {
            branch: None,
            ignore: None,
            fetch_recurse: None,
            update: None,
            use_git_default_sparse_checkout: Some(true),
        };
        let other = SubmoduleDefaults {
            branch: None,
            ignore: None,
            fetch_recurse: None,
            update: None,
            use_git_default_sparse_checkout: Some(false),
        };
        let merged = base.merge_from(other);
        assert_eq!(
            merged.use_git_default_sparse_checkout,
            Some(false),
            "other's explicit sparse default must override base's"
        );
    }

    #[test]
    fn test_defaults_merge_from_unset_other_sparse_default_preserves_base() {
        // When other leaves it unset, base's value must survive.
        let base = SubmoduleDefaults {
            branch: None,
            ignore: None,
            fetch_recurse: None,
            update: None,
            use_git_default_sparse_checkout: Some(true),
        };
        let other = SubmoduleDefaults::default();
        let merged = base.merge_from(other);
        assert_eq!(
            merged.use_git_default_sparse_checkout,
            Some(true),
            "an unset other must not clobber base's sparse default"
        );
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

        let entry = SubmoduleEntry::from_gitmodules("repo", &map);
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

        let entry = SubmoduleEntry::from_gitmodules("mymod", &map);
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

        let entry = SubmoduleEntry::from_gitmodules("mod", &map);
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

        let entry = SubmoduleEntry::from_gitmodules("mod", &map);
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
        entries.update_entry(
            "mod1".into(),
            SubmoduleEntry::new(
                Some("repo".into()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
        );
        entries.add_checkout("mod1", &["src/".to_string()], false);
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["src/".to_string()]
        );

        // Append
        entries.add_checkout("mod1", &["docs/".to_string()], false);
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["src/".to_string(), "docs/".to_string()]
        );

        // Replace
        entries.add_checkout("mod1", &["lib/".to_string()], true);
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["lib/".to_string()]
        );
    }

    #[test]
    fn test_entries_add_checkout_when_none() {
        let mut entries = SubmoduleEntries {
            submodules: Some(HashMap::new()),
        };
        entries.update_entry(
            "mod1".into(),
            SubmoduleEntry::new(
                Some("repo".into()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
        );
        entries.add_checkout("mod1", &["src/".to_string()], false);
        assert!(entries.sparse_checkouts().is_some());
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["src/".to_string()]
        );
    }

    #[test]
    fn test_entries_remove_sparse_path() {
        let mut entries = SubmoduleEntries::default();
        entries.update_entry(
            "mod1".into(),
            SubmoduleEntry::new(
                Some("repo".into()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
        );
        entries.add_checkout("mod1", &["src/".to_string(), "docs/".to_string()], false);

        entries.remove_sparse_path("mod1", "src/");
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["docs/".to_string()]
        );

        // Remove last path → entry is cleaned up
        entries.remove_sparse_path("mod1", "docs/");
        assert!(!entries.sparse_checkouts().unwrap().contains_key("mod1"));
    }

    #[test]
    fn test_entries_add_sparse_path() {
        let mut entries = SubmoduleEntries::default();
        entries.update_entry(
            "mod1".into(),
            SubmoduleEntry::new(
                Some("repo".into()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
        );
        entries.add_sparse_path("mod1", "src/".to_string());
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["src/".to_string()]
        );
        entries.add_sparse_path("mod1", "docs/".to_string());
        assert_eq!(
            entries.sparse_checkouts().unwrap().get("mod1").unwrap(),
            &vec!["src/".to_string(), "docs/".to_string()]
        );
    }

    #[test]
    fn test_entries_add_sparse_path_when_none() {
        let mut entries = SubmoduleEntries {
            submodules: Some(HashMap::new()),
        };
        entries.update_entry(
            "mod1".into(),
            SubmoduleEntry::new(
                Some("repo".into()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
        );
        entries.add_sparse_path("mod1", "src/".to_string());
        assert!(entries.sparse_checkouts().is_some());
    }

    #[test]
    fn test_entries_delete_checkout() {
        let mut entries = SubmoduleEntries::default();
        entries.update_entry(
            "mod1".into(),
            SubmoduleEntry::new(
                Some("repo".into()),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            ),
        );
        entries.add_checkout("mod1", &["src/".to_string()], false);
        entries.delete_checkout("mod1");
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
            use_git_default_sparse_checkout: None,
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
            use_git_default_sparse_checkout: None,
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
            use_git_default_sparse_checkout: None,
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
        let _ = entries.remove_submodule("mod1");
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
        entries.add_checkout("mod1", &["src/".to_string()], false);

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
        Config::apply_option_default(&mut val, default.as_ref(), SerializableIgnore::Unspecified);
        assert_eq!(val, Some(SerializableIgnore::Dirty));
    }

    #[test]
    fn test_apply_option_default_unspecified_gets_default() {
        let mut val = Some(SerializableIgnore::Unspecified);
        let default = Some(SerializableIgnore::All);
        Config::apply_option_default(&mut val, default.as_ref(), SerializableIgnore::Unspecified);
        assert_eq!(val, Some(SerializableIgnore::All));
    }

    #[test]
    fn test_apply_option_default_real_value_preserved() {
        let mut val = Some(SerializableIgnore::Dirty);
        let default = Some(SerializableIgnore::All);
        Config::apply_option_default(&mut val, default.as_ref(), SerializableIgnore::Unspecified);
        assert_eq!(val, Some(SerializableIgnore::Dirty));
    }

    #[test]
    fn test_apply_option_default_none_value_none_default() {
        let mut val: Option<SerializableIgnore> = None;
        let default: Option<SerializableIgnore> = None;
        Config::apply_option_default(&mut val, default.as_ref(), SerializableIgnore::Unspecified);
        // Falls back to the sentinel
        assert_eq!(val, Some(SerializableIgnore::Unspecified));
    }

    // ================================================================
    // Config::apply_defaults
    // ================================================================

    #[test]
    fn test_config_apply_defaults() {
        let defaults = SubmoduleDefaults {
            branch: None,
            ignore: Some(SerializableIgnore::Dirty),
            fetch_recurse: Some(SerializableFetchRecurse::Always),
            update: Some(SerializableUpdate::Rebase),
            use_git_default_sparse_checkout: None,
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
            branch: None,
            ignore: Some(SerializableIgnore::Dirty),
            fetch_recurse: Some(SerializableFetchRecurse::Always),
            update: Some(SerializableUpdate::Rebase),
            use_git_default_sparse_checkout: None,
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
    fn test_add_options_entry_path_uses_git_separators() {
        // Join with the native separator the way path normalization does, so
        // the stored spelling stays `lib/noinit` even on Windows.
        let opts = SubmoduleAddOptions {
            name: "mymod".to_string(),
            path: PathBuf::from("lib").join("noinit"),
            url: "https://example.com/repo.git".to_string(),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            shallow: false,
            no_init: true,
        };
        let entry = opts.into_submodule_entry();
        assert_eq!(entry.path, Some("lib/noinit".to_string()));
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
            use_git_default_sparse_checkout: None,
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
            use_git_default_sparse_checkout: None,
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
            use_git_default_sparse_checkout: None,
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

    #[test]
    fn test_config_toml_fetch_recurse_camelcase_key_is_parsed() {
        // README documents the key as `fetchRecurse` (camelCase). It must
        // deserialize into the `fetch_recurse` field rather than being
        // silently dropped. Assert the PARSED value, not the file text.
        let toml_str = r#"
[defaults]
fetchRecurse = "always"

[mymod]
path = "libs/mymod"
url = "https://example.com/repo.git"
fetchRecurse = "never"
active = true
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(
            config.defaults.fetch_recurse,
            Some(SerializableFetchRecurse::Always),
            "[defaults] fetchRecurse should parse into fetch_recurse"
        );
        let entry = config.submodules.get("mymod").unwrap();
        assert_eq!(
            entry.fetch_recurse,
            Some(SerializableFetchRecurse::Never),
            "per-submodule fetchRecurse should parse into fetch_recurse"
        );
    }

    // ================================================================
    // Config::load — CLI-over-file precedence
    // ================================================================

    #[test]
    fn test_config_load_cli_overrides_file_but_preserves_unspecified() {
        // `Config::load` layers three sources: Rust-side defaults → submod.toml
        // → cli_options. A value supplied via `cli_options` must override the
        // same key from the file, while a key the CLI leaves unset must keep the
        // file's value — an absent CLI flag must not erase file configuration.
        // This precedence contract was previously untested (#62 P1).
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "submod.toml",
                r#"
[defaults]
ignore = "all"
update = "rebase"
"#,
            )?;

            // The CLI overrides `ignore` and leaves `update` unspecified.
            let mut cli_options = Config::default();
            cli_options.defaults.ignore = Some(SerializableIgnore::Dirty);

            let cfg = Config::default()
                .load("submod.toml", cli_options)
                .expect("load should succeed");

            assert_eq!(
                cfg.defaults.ignore,
                Some(SerializableIgnore::Dirty),
                "CLI-supplied `ignore` must override the file's value"
            );
            assert_eq!(
                cfg.defaults.update,
                Some(SerializableUpdate::Rebase),
                "file's `update` must survive when the CLI leaves it unspecified"
            );

            Ok(())
        });
    }

    #[test]
    fn test_config_load_default_cli_preserves_file_defaults() {
        // The production callers (`GitManager::with_verbose`) load config with
        // `cli_options = Config::default()`, whose `[defaults]` are all `None`.
        // Those unset CLI fields must not erase the file's `[defaults]`; the
        // whole `[defaults]` block was previously clobbered to `None` on every
        // load (#62 P1).
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "submod.toml",
                r#"
[defaults]
ignore = "all"
update = "rebase"
fetchRecurse = "always"
"#,
            )?;

            let cfg = Config::default()
                .load("submod.toml", Config::default())
                .expect("load should succeed");

            assert_eq!(
                cfg.defaults.ignore,
                Some(SerializableIgnore::All),
                "file `[defaults] ignore` must survive a default-CLI load"
            );
            assert_eq!(
                cfg.defaults.update,
                Some(SerializableUpdate::Rebase),
                "file `[defaults] update` must survive a default-CLI load"
            );
            assert_eq!(
                cfg.defaults.fetch_recurse,
                Some(SerializableFetchRecurse::Always),
                "file `[defaults] fetchRecurse` must survive a default-CLI load"
            );

            Ok(())
        });
    }

    #[test]
    fn test_config_load_from_file_preserves_file_defaults() {
        // `load_from_file` shares the figment-base hazard with `load`: an empty
        // default provider layered beneath the file erased the file's
        // `[defaults]`. The file's values must survive (#62 P1).
        figment::Jail::expect_with(|jail| {
            jail.create_file(
                "submod.toml",
                r#"
[defaults]
ignore = "all"
update = "rebase"
"#,
            )?;

            let cfg = Config::default()
                .load_from_file(Some("submod.toml"))
                .expect("load_from_file should succeed");

            assert_eq!(cfg.defaults.ignore, Some(SerializableIgnore::All));
            assert_eq!(cfg.defaults.update, Some(SerializableUpdate::Rebase));

            Ok(())
        });
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

        let retrieved = config
            .get_submodule("repo")
            .expect("submodule should exist");
        assert!(retrieved.is_remote());
    }

    #[test]
    fn test_config_get_submodule() {
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
        config.add_submodule("repo".to_string(), entry.clone());

        let retrieved = config
            .get_submodule("repo")
            .expect("submodule should exist");
        assert_eq!(retrieved, &entry);

        assert!(config.get_submodule("non-existent").is_none());
    }

    // ================================================================
    // git2 option bridges: Git2SubmoduleOptions / SubmoduleUpdateOptions
    // (#62 P1 — these conversions ship into every git2-backed add/update
    // but had no test coverage.)
    // ================================================================

    #[test]
    fn test_git2_options_try_from_maps_every_field() {
        let opts = SubmoduleGitOptions::new(
            Some(SerializableIgnore::All),
            Some(SerializableFetchRecurse::Always),
            Some(SerializableBranch::Name("feature".to_string())),
            Some(SerializableUpdate::Rebase),
        );
        let git2 = Git2SubmoduleOptions::try_from(opts).expect("conversion should succeed");
        assert_eq!(git2.ignore, git2::SubmoduleIgnore::All);
        assert_eq!(git2.update, git2::SubmoduleUpdate::Rebase);
        assert_eq!(git2.branch.as_deref(), Some("feature"));
        // fetch_recurse bridges through `to_gitmodules` → git's `true`/`false`
        // encoding, NOT the `always`/`never` serde spelling.
        assert_eq!(git2.fetch_recurse.as_deref(), Some("true"));
    }

    #[test]
    fn test_git2_options_try_from_none_uses_git2_defaults() {
        let opts = SubmoduleGitOptions::new(None, None, None, None);
        let git2 = Git2SubmoduleOptions::try_from(opts).expect("conversion should succeed");
        // Absent ignore/update become git2's neutral sentinels, not a panic.
        assert_eq!(git2.ignore, git2::SubmoduleIgnore::Unspecified);
        assert_eq!(git2.update, git2::SubmoduleUpdate::Default);
        assert_eq!(git2.branch, None);
        assert_eq!(git2.fetch_recurse, None);
    }

    #[test]
    fn test_git2_options_fetch_recurse_on_demand_encoding() {
        // OnDemand must NOT collapse to the same string as Always — proves the
        // bridge preserves the distinction git cares about.
        let opts =
            SubmoduleGitOptions::new(None, Some(SerializableFetchRecurse::OnDemand), None, None);
        let git2 = Git2SubmoduleOptions::try_from(opts).expect("conversion should succeed");
        assert_eq!(git2.fetch_recurse.as_deref(), Some("on-demand"));
    }

    #[test]
    fn test_update_options_from_options_never_selects_recursive_materialization() {
        // fetchRecurse controls fetching nested history. Recursive checkout is
        // an explicit lifecycle-command selection and remains off here.
        let always = SubmoduleUpdateOptions::from_options(SubmoduleGitOptions::new(
            None,
            Some(SerializableFetchRecurse::Always),
            None,
            Some(SerializableUpdate::Merge),
        ));
        assert!(
            !always.recursive,
            "fetchRecurse=always must not select recursive materialization"
        );
        assert_eq!(always.strategy, SerializableUpdate::Merge);
        assert!(!always.force, "from_options never forces");
        assert!(!always.remote, "from_options never selects remote tracking");

        // Every other fetch_recurse value (including None) also leaves it off.
        for fr in [
            None,
            Some(SerializableFetchRecurse::OnDemand),
            Some(SerializableFetchRecurse::Never),
        ] {
            let opts = SubmoduleUpdateOptions::from_options(SubmoduleGitOptions::new(
                None, fr, None, None,
            ));
            assert!(
                !opts.recursive,
                "fetchRecurse must stay independent of recursive checkout, got {fr:?}"
            );
            // An absent update strategy falls back to the Checkout default.
            assert_eq!(opts.strategy, SerializableUpdate::Checkout);
        }
    }

    #[test]
    fn test_entry_to_git2_options_reflects_entry_git_options() {
        let entry = SubmoduleEntry::new(
            Some("https://example.com/repo.git".to_string()),
            Some("libs/repo".to_string()),
            None,
            Some(SerializableIgnore::Dirty),
            Some(SerializableUpdate::Checkout),
            None,
            Some(true),
            None,
            None,
        );
        let git2 = entry
            .to_git2_options()
            .expect("to_git2_options should succeed");
        assert_eq!(git2.ignore, git2::SubmoduleIgnore::Dirty);
        assert_eq!(git2.update, git2::SubmoduleUpdate::Checkout);
    }
}
