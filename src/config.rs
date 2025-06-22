#![doc = r#"
Configuration types and utilities for submod.

Defines serializable wrappers for git submodule options, project-level defaults, and submodule
configuration management. Supports loading and saving configuration in TOML format.

Main Types:
- SerializableIgnore, SerializableFetchRecurse, SerializableBranch, SerializableUpdate: Wrappers for git submodule config enums, supporting (de)serialization.
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
"#]

use anyhow::{Context, Result};
use bstr::BStr;
use gix_submodule::config::{Branch, FetchRecurse, Ignore, Update};
use serde::{Deserialize, Serialize};
use std::fs;
use std::{collections::HashMap, path::Path};
use toml_edit::{Array, DocumentMut, Item, Table, value};

/// Serializable wrapper for [`Ignore`] config
#[derive(Debug, Default, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SerializableIgnore(pub Ignore);

/// Serializable wrapper for [`FetchRecurse`] config
#[derive(Debug, Default, Clone, Copy, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SerializableFetchRecurse(pub FetchRecurse);

/// Serializable wrapper for [`Branch`] config
#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SerializableBranch(pub Branch);

/// Serializable wrapper for [`Update`] config
#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct SerializableUpdate(pub Update);

/**========================================================================
 **               Implement Serialize/Deserialize for Config
 *========================================================================**/
/// implements Serialize for [`SerializableIgnore`]
impl Serialize for SerializableIgnore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self.0 {
            Ignore::All => "all",
            Ignore::Dirty => "dirty",
            Ignore::Untracked => "untracked",
            Ignore::None => "none",
        };
        serializer.serialize_str(s)
    }
}

/// implements Deserialize for [`SerializableIgnore`]
impl<'de> Deserialize<'de> for SerializableIgnore {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Convert String to BStr for the TryFrom implementation
        let bstr = BStr::new(s.as_bytes());
        match Ignore::try_from(bstr) {
            Ok(ignore) => Ok(Self(ignore)),
            Err(()) => Err(serde::de::Error::custom(format!(
                "Invalid ignore value: {s}"
            ))),
        }
    }
}

/// implements Serialize for [`SerializableFetchRecurse`]
impl Serialize for SerializableFetchRecurse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let s = match self.0 {
            FetchRecurse::OnDemand => "on-demand",
            FetchRecurse::Always => "always",
            FetchRecurse::Never => "never",
        };
        serializer.serialize_str(s)
    }
}

/// implements Deserialize for [`SerializableFetchRecurse`]
impl<'de> Deserialize<'de> for SerializableFetchRecurse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "on-demand" => Ok(Self(FetchRecurse::OnDemand)),
            "always" => Ok(Self(FetchRecurse::Always)),
            "never" => Ok(Self(FetchRecurse::Never)),
            _ => Err(serde::de::Error::custom(format!(
                "Invalid fetch recurse value: {s}"
            ))),
        }
    }
}

/// implements Serialize for [`SerializableBranch`]
impl Serialize for SerializableBranch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.0 {
            Branch::CurrentInSuperproject => serializer.serialize_str("."),
            Branch::Name(name) => serializer.serialize_str(&name.to_string()),
        }
    }
}

/// implements Deserialize for [`SerializableBranch`]
impl<'de> Deserialize<'de> for SerializableBranch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Convert String to BStr for the TryFrom implementation
        let bstr = BStr::new(s.as_bytes());
        match Branch::try_from(bstr) {
            Ok(branch) => Ok(Self(branch)),
            Err(e) => Err(serde::de::Error::custom(format!(
                "Invalid branch value '{s}': {e}"
            ))),
        }
    }
}

/// implements Serialize for [`SerializableUpdate`]
impl Serialize for SerializableUpdate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match &self.0 {
            Update::Checkout => serializer.serialize_str("checkout"),
            Update::Rebase => serializer.serialize_str("rebase"),
            Update::Merge => serializer.serialize_str("merge"),
            Update::None => serializer.serialize_str("none"),
            Update::Command(cmd) => {
                // Convert BString to String with ! prefix
                let cmd_str = format!("!{cmd}");
                serializer.serialize_str(&cmd_str)
            }
        }
    }
}

/// implements Deserialize for [`SerializableUpdate`]
impl<'de> Deserialize<'de> for SerializableUpdate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        // Convert String to BStr for the TryFrom implementation
        let bstr = BStr::new(s.as_bytes());
        match Update::try_from(bstr) {
            Ok(update) => Ok(Self(update)),
            Err(()) => Err(serde::de::Error::custom(format!(
                "Invalid update value: {s}"
            ))),
        }
    }
}

/// Convert from [`Ignore`] to [`SerializableIgnore`]
impl From<Ignore> for SerializableIgnore {
    fn from(value: Ignore) -> Self {
        Self(value)
    }
}

/// Convert from [`SerializableIgnore`] to [`Ignore`]
impl From<SerializableIgnore> for Ignore {
    fn from(value: SerializableIgnore) -> Self {
        value.0
    }
}

/// Convert from [`FetchRecurse`] to [`SerializableFetchRecurse`]
impl From<FetchRecurse> for SerializableFetchRecurse {
    fn from(value: FetchRecurse) -> Self {
        Self(value)
    }
}

/// Convert from [`SerializableFetchRecurse`] to [`FetchRecurse`]
impl From<SerializableFetchRecurse> for FetchRecurse {
    fn from(value: SerializableFetchRecurse) -> Self {
        value.0
    }
}

/// Convert from [`Branch`] to [`SerializableBranch`]
impl From<Branch> for SerializableBranch {
    fn from(value: Branch) -> Self {
        Self(value)
    }
}

/// Convert from [`SerializableBranch`] to [`Branch`]
impl From<SerializableBranch> for Branch {
    fn from(value: SerializableBranch) -> Self {
        value.0
    }
}

/// Convert from [`Update`] to [`SerializableUpdate`]
impl From<Update> for SerializableUpdate {
    fn from(value: Update) -> Self {
        Self(value)
    }
}

/// Convert from [`SerializableUpdate`] to [`Update`]
impl From<SerializableUpdate> for Update {
    fn from(value: SerializableUpdate) -> Self {
        value.0
    }
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

// Default implementation for [`SubmoduleGitOptions`]
impl SubmoduleGitOptions {
    /// Create a new instance with default git options
    #[allow(dead_code)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            ignore: Some(SerializableIgnore(Ignore::default())),
            fetch_recurse: Some(SerializableFetchRecurse(FetchRecurse::default())),
            branch: Some(SerializableBranch(Branch::default())),
            update: Some(SerializableUpdate(Update::default())),
        }
    }
}

/// Project-level defaults for git submodule options
#[derive(Debug, Default, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct SubmoduleDefaults(pub SubmoduleGitOptions);
impl SubmoduleDefaults {
    /// Create new default submodule configuration
    #[allow(dead_code)]
    #[must_use]
    pub fn new() -> Self {
        Self(SubmoduleGitOptions::new())
    }
}

/// Configuration for a single submodule
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SubmoduleConfig {
    /// Git-specific options for this submodule
    #[serde(flatten)]
    pub git_options: SubmoduleGitOptions,
    /// Whether this submodule is active
    pub active: bool,
    /// Path where the submodule should be checked out
    pub path: Option<String>,
    /// URL of the submodule repository
    pub url: Option<String>,
    /// Sparse checkout paths for this submodule
    pub sparse_paths: Option<Vec<String>>,
}

impl SubmoduleConfig {
    /// Create a new submodule configuration with defaults
    #[allow(dead_code)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            git_options: SubmoduleGitOptions::new(),
            active: true,
            path: None,
            url: None,
            sparse_paths: None,
        }
    }
    /// Check if our active setting matches what git would report
    /// `git_active_state` should be the result of calling git's active check
    #[allow(dead_code)]
    #[must_use]
    pub const fn active_setting_matches_git(&self, git_active_state: bool) -> bool {
        self.active == git_active_state
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
                "# Submodule configuration for gitoxide-based submodule manager",
                Item::None,
            );
            doc.insert("# Each section [name] defines a submodule", Item::None);
            doc.insert("", Item::None); // Empty line for spacing
            doc
        };

        // Handle defaults section
        if !self.defaults_are_empty() {
            let mut defaults_table = Table::new();

            if let Some(ref ignore) = self.defaults.0.ignore {
                let serialized = serde_json::to_string(ignore).unwrap_or_default();
                let clean_value = serialized.trim_matches('"'); // Remove JSON quotes
                defaults_table["ignore"] = value(clean_value);
            }
            if let Some(ref update) = self.defaults.0.update {
                let serialized = serde_json::to_string(update).unwrap_or_default();
                let clean_value = serialized.trim_matches('"');
                defaults_table["update"] = value(clean_value);
            }
            if let Some(ref branch) = self.defaults.0.branch {
                let serialized = serde_json::to_string(branch).unwrap_or_default();
                let clean_value = serialized.trim_matches('"');
                defaults_table["branch"] = value(clean_value);
            }
            if let Some(ref fetch_recurse) = self.defaults.0.fetch_recurse {
                let serialized = serde_json::to_string(fetch_recurse).unwrap_or_default();
                let clean_value = serialized.trim_matches('"');
                defaults_table["fetchRecurse"] = value(clean_value);
            }

            doc["defaults"] = Item::Table(defaults_table);
        }

        // Remove existing submodule sections but preserve defaults and comments
        let keys_to_remove: Vec<String> = doc
            .iter()
            .filter_map(|(key, _)| {
                if key != "defaults" && self.submodules.contains_key(key) {
                    Some(key.to_string())
                } else {
                    None
                }
            })
            .collect();

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
            if let Some(ref url) = submodule.url {
                submodule_table["url"] = value(url);
            }

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

    const fn defaults_are_empty(&self) -> bool {
        self.defaults.0.ignore.is_none()
            && self.defaults.0.update.is_none()
            && self.defaults.0.branch.is_none()
            && self.defaults.0.fetch_recurse.is_none()
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
            "branch" => submodule
                .git_options
                .branch
                .as_ref()
                .or(self.defaults.0.branch.as_ref())
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
