// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

#![allow(unreachable_patterns)]
//! Defines serializable wrappers for git submodule configuration enums.
//!
//! These types mirror similar types in `gix_submodule`, and to a lesser extent, `git2`. They represent git's configuration options for submodules.
//!
//! - SerializableIgnore, SerializableFetchRecurse, SerializableBranch, SerializableUpdate
//!
//! Each enum implements conversion traits to and from the corresponding types in `gix_submodule` and `git2` (where applicable).
//!
//! [`SerializableIgnore`],and [`SerializableUpdate`] have direct `git2` counterparts and can convert to and from them. [`SerializableFetchRecurse`] and [`SerializableBranch`] are more specific to `gix_submodule` and do not have direct `git2` counterparts, but they can convert to and from `gix_submodule` types and have methods to convert to their git string and byte equivalents.
use anyhow::Result;
use clap::ValueEnum;
use git2::{SubmoduleIgnore as Git2SubmoduleIgnore, SubmoduleUpdate as Git2SubmoduleUpdate};
use gix_submodule::config::{Branch, FetchRecurse, Ignore, Update};
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

use crate::utilities::{get_current_branch, get_current_repository};

/// Configuration levels for git config operations
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigLevel {
    /// System-wide configuration
    System,
    /// Global user configuration
    Global,
    /// Local repository configuration
    Local,
    /// Worktree-specific configuration
    Worktree,
}

/// Trait for converting between git submodule configuration enums and their gitmodules representation
#[allow(dead_code)]
pub trait GitmodulesConvert {
    /// Get the git key for a submodule by the submodule's name (in git config)
    fn gitmodules_key_path(&self, name: &str) -> String {
        format!("submodule.{name}.{}", self.gitmodules_key()).to_string()
    }

    /// Get the git key for the enum setting
    fn gitmodules_key(&self) -> &str;

    /// Format this option as a `.gitmodules` key=value line.
    fn as_gitmodules_key_value(&self, name: &str) -> String {
        format!(
            "{}={}",
            self.gitmodules_key(),
            self.gitmodules_key_path(&name)
        )
    }

    /// Format this option as a `.gitmodules` key=value line, encoded as bytes.
    fn as_gitmodules_byte_key_value(&self, name: &str) -> Vec<u8> {
        self.as_gitmodules_key_value(name).into_bytes()
    }

    /// Convert to gitmodules string (what you would get from the .gitmodules or .git/config)
    fn to_gitmodules(&self) -> String;

    /// Convert from gitmodules string (what you would get from the .gitmodules or .git/config)
    fn from_gitmodules(options: &str) -> Result<Self, ()>
    where
        Self: Sized;

    /// Convert from gitmodules bytes (what you would get from the .gitmodules or .git/config)
    fn from_gitmodules_bytes(options: &[u8]) -> Result<Self, ()>
    where
        Self: Sized;
}

/// Trait for checking if an enum is unspecified or default
#[allow(dead_code)]
pub trait OptionsChecks {
    /// Check if the enum is unspecified
    fn is_unspecified(&self) -> bool;

    /// Check if the enum is the default value
    fn is_default(&self) -> bool;
}

/// Trait for converting between `git2` and `gix_submodule` types
#[allow(dead_code)]
pub trait GixGit2Convert {
    /// The git2 source type
    type Git2Type;
    /// The gix source type
    type GixType;

    /// Convert from a `git2` type to a `submod` type
    fn from_git2(git2: Self::Git2Type) -> Result<Self, ()>
    where
        Self: Sized;

    /// Convert from a `gix_submodule` type to a `submod` type
    fn from_gix(gix: Self::GixType) -> Result<Self, ()>
    where
        Self: Sized;
}

/// Serializable enum for [`Ignore`] config
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    Serialize,
    Deserialize,
    ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum SerializableIgnore {
    /// Ignore all changes in the submodule, including untracked files. Fastest option.
    All,
    /// Ignore changes to the submodule working tree, only showing committed differences.
    Dirty,
    /// Ignore untracked files in the submodule. All other changes are shown.
    Untracked,
    /// No modifications to the submodule are ignored, showing untracked files and modified files in the worktree. This is the default. It treats the submodule like the rest of the repository.
    #[default]
    None,
    /// Used as a sentinel value internally; do not use in a submod.toml or submod CLI command.
    #[serde(skip)]
    #[value(skip)]
    Unspecified,
}

impl GitmodulesConvert for SerializableIgnore {
    /// Get the git key for the ignore submodule setting
    fn gitmodules_key(&self) -> &str {
        "ignore"
    }

    /// Convert to gitmodules string (what you would get from the .gitmodules or .git/config)
    fn to_gitmodules(&self) -> String {
        match self {
            SerializableIgnore::All => "all".to_string(),
            SerializableIgnore::Dirty => "dirty".to_string(),
            SerializableIgnore::Untracked => "untracked".to_string(),
            SerializableIgnore::None => "none".to_string(),
            SerializableIgnore::Unspecified => "".to_string(), // Unspecified is treated as an empty string
        }
    }

    /// Convert from gitmodules string (what you would get from the .gitmodules or .git/config)
    fn from_gitmodules(options: &str) -> Result<Self, ()> {
        match options {
            "all" => Ok(SerializableIgnore::All),
            "dirty" => Ok(SerializableIgnore::Dirty),
            "untracked" => Ok(SerializableIgnore::Untracked),
            "none" => Ok(SerializableIgnore::None), // Default is None
            "" => Ok(SerializableIgnore::Unspecified), // Empty string is treated as unspecified
            _ => Err(()),                           // Handle unsupported options
        }
    }

    /// Convert from gitmodules bytes (what you would get from the .gitmodules or .git/config)
    fn from_gitmodules_bytes(options: &[u8]) -> Result<Self, ()> {
        let options_str = std::str::from_utf8(options).map_err(|_| ())?;
        Self::from_gitmodules(options_str)
    }
}

impl OptionsChecks for SerializableIgnore {
    /// Check if the enum is unspecified
    fn is_unspecified(&self) -> bool {
        matches!(self, SerializableIgnore::Unspecified)
    }

    /// Check if the enum is the default value
    fn is_default(&self) -> bool {
        matches!(self, SerializableIgnore::None)
    }
}

impl GixGit2Convert for SerializableIgnore {
    type Git2Type = Git2SubmoduleIgnore;
    type GixType = gix_submodule::config::Ignore;
    /// Convert from a `git2` type to a `gix_submodule` type
    fn from_git2(git2: Self::Git2Type) -> Result<Self, ()> {
        Self::try_from(git2).map_err(|_| ()) // Handle unsupported variants
    }

    /// Convert from a `gix_submodule` type to a `submod` type
    fn from_gix(gix: Self::GixType) -> Result<Self, ()> {
        Self::try_from(gix).map_err(|_| ()) // Handle unsupported variants
    }
}

impl TryFrom<SerializableIgnore> for Git2SubmoduleIgnore {
    type Error = ();

    fn try_from(value: SerializableIgnore) -> Result<Self, Self::Error> {
        Ok(match value {
            SerializableIgnore::All => Git2SubmoduleIgnore::All,
            SerializableIgnore::Dirty => Git2SubmoduleIgnore::Dirty,
            SerializableIgnore::Untracked => Git2SubmoduleIgnore::Untracked,
            SerializableIgnore::None => Git2SubmoduleIgnore::None,
            SerializableIgnore::Unspecified => Git2SubmoduleIgnore::Unspecified,
            _ => return Err(()), // Handle unsupported variants
        })
    }
}

impl TryFrom<Git2SubmoduleIgnore> for SerializableIgnore {
    type Error = ();

    fn try_from(value: Git2SubmoduleIgnore) -> Result<Self, Self::Error> {
        Ok(match value {
            Git2SubmoduleIgnore::All => SerializableIgnore::All,
            Git2SubmoduleIgnore::Dirty => SerializableIgnore::Dirty,
            Git2SubmoduleIgnore::Untracked => SerializableIgnore::Untracked,
            Git2SubmoduleIgnore::None => SerializableIgnore::None,
            Git2SubmoduleIgnore::Unspecified => SerializableIgnore::Unspecified,
            _ => return Err(()), // Handle unsupported variants
        })
    }
}

impl TryFrom<Ignore> for SerializableIgnore {
    type Error = ();

    fn try_from(value: Ignore) -> Result<Self, Self::Error> {
        Ok(match value {
            Ignore::All => SerializableIgnore::All,
            Ignore::Dirty => SerializableIgnore::Dirty,
            Ignore::Untracked => SerializableIgnore::Untracked,
            Ignore::None => SerializableIgnore::None,
            _ => return Err(()), // Handle unsupported variants
        })
    }
}
impl TryFrom<SerializableIgnore> for Ignore {
    type Error = ();

    fn try_from(value: SerializableIgnore) -> Result<Self, Self::Error> {
        Ok(match value {
            SerializableIgnore::All => Ignore::All,
            SerializableIgnore::Dirty => Ignore::Dirty,
            SerializableIgnore::Untracked => Ignore::Untracked,
            SerializableIgnore::None | SerializableIgnore::Unspecified => Ignore::None,
            _ => return Err(()),
        })
    }
}

impl std::fmt::Display for SerializableIgnore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_gitmodules())
    }
}

/// Serializable enum for [`FetchRecurse`] config. Sets the fetch behavior for the submodule and its submodules (they said inception was impossible...).
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    Ord,
    PartialOrd,
    Eq,
    PartialEq,
    Hash,
    Serialize,
    Deserialize,
    ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum SerializableFetchRecurse {
    /// Fetch only changed submodules. Default.
    #[default]
    OnDemand,
    /// Fetch all populated submodules, regardless of changes. In some cases, this can be faster because we don't have to check for changes; but more fetches can also mean more data transfer.
    Always,
    /// Submodules are never fetched. This is useful if you want to manage submodules manually or if you don't want to fetch them at all.
    Never,
    /// Used as a sentinel value internally; do not use in a submod.toml or submod CLI command.
    #[serde(skip)]
    #[value(skip)]
    Unspecified,
}

impl GitmodulesConvert for SerializableFetchRecurse {
    /// Get the git key for the fetch recurse submodule setting
    fn gitmodules_key(&self) -> &str {
        "fetchRecurseSubmodules"
    }

    /// Convert to gitmodules string (what you would get from the .gitmodules or .git/config)
    fn to_gitmodules(&self) -> String {
        match self {
            SerializableFetchRecurse::OnDemand | SerializableFetchRecurse::Unspecified => {
                "on-demand".to_string()
            }
            SerializableFetchRecurse::Always => "true".to_string(),
            SerializableFetchRecurse::Never => "false".to_string(),
        }
    }

    /// Convert from gitmodules string (what you would get from the .gitmodules or .git/config)
    fn from_gitmodules(options: &str) -> Result<Self, ()> {
        match options {
            "on-demand" => Ok(SerializableFetchRecurse::OnDemand), // Default is OnDemand
            "true" => Ok(SerializableFetchRecurse::Always),
            "false" => Ok(SerializableFetchRecurse::Never),
            "" => Ok(SerializableFetchRecurse::Unspecified), // Empty string is treated as unspecified
            _ => Err(()),                                    // Handle unsupported options
        }
    }

    /// Convert from gitmodules bytes (what you would get from the .gitmodules or .git/config)
    fn from_gitmodules_bytes(options: &[u8]) -> Result<Self, ()> {
        let options_str = std::str::from_utf8(options).map_err(|_| ())?;
        Self::from_gitmodules(options_str)
    }
}

impl OptionsChecks for SerializableFetchRecurse {
    /// Check if the enum is unspecified
    fn is_unspecified(&self) -> bool {
        matches!(self, SerializableFetchRecurse::Unspecified)
    }

    /// Check if the enum is the default value
    fn is_default(&self) -> bool {
        matches!(self, SerializableFetchRecurse::OnDemand)
    }
}

impl GixGit2Convert for SerializableFetchRecurse {
    type Git2Type = String; // git2 does not have a direct type for FetchRecurse, so we use str
    type GixType = gix_submodule::config::FetchRecurse;
    /// Convert from a `git2` type to a `gix_submodule` type
    fn from_git2(git2: Self::Git2Type) -> Result<Self, ()> {
        Self::from_gitmodules(git2.as_str()).map_err(|_| ()) // Handle unsupported variants
    }

    /// Convert from a `gix_submodule` type to a `submod` type
    fn from_gix(gix: Self::GixType) -> Result<Self, ()> {
        Self::try_from(gix).map_err(|_| ()) // Handle unsupported variants
    }
}

impl TryFrom<FetchRecurse> for SerializableFetchRecurse {
    type Error = ();

    fn try_from(value: FetchRecurse) -> Result<Self, Self::Error> {
        Ok(match value {
            FetchRecurse::OnDemand => SerializableFetchRecurse::OnDemand,
            FetchRecurse::Always => SerializableFetchRecurse::Always,
            FetchRecurse::Never => SerializableFetchRecurse::Never,
            _ => return Err(()), // Handle unsupported variants
        })
    }
}

impl TryFrom<SerializableFetchRecurse> for FetchRecurse {
    type Error = ();

    fn try_from(value: SerializableFetchRecurse) -> Result<Self, Self::Error> {
        Ok(match value {
            SerializableFetchRecurse::OnDemand | SerializableFetchRecurse::Unspecified => {
                FetchRecurse::OnDemand
            }
            SerializableFetchRecurse::Always => FetchRecurse::Always,
            SerializableFetchRecurse::Never => FetchRecurse::Never,
            _ => return Err(()), // Handle unsupported variants
        })
    }
}

impl std::fmt::Display for SerializableFetchRecurse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_gitmodules())
    }
}

/// Serializable enum for [`Branch`] config
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum SerializableBranch {
    /// Use the same name for remote's branch name as the name of the currently activate branch in the superproject.
    /// This is a special value in git's settings. In a .git/config or .gitmodules it's represented by a period: `.`.
    CurrentInSuperproject,
    /// Track a specific branch by name. (Usually what you want.). The default value is the remote branch's default branch if we can resolve it, else `main`.
    Name(String),
}

impl Serialize for SerializableBranch {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_gitmodules())
    }
}

impl<'de> Deserialize<'de> for SerializableBranch {
    /// Deserialize from a plain string, delegating to [`from_gitmodules`](GitmodulesConvert::from_gitmodules).
    /// Accepts `"."`, `"current"`, `"current-in-super-project"`, `"current-in-superproject"`, `"superproject"`, or `"super"`
    /// as [`CurrentInSuperproject`](SerializableBranch::CurrentInSuperproject); all other
    /// non-empty, non-whitespace strings become [`Name`](SerializableBranch::Name).
    /// Empty or whitespace-only strings are rejected with a deserialization error.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        // Backward compatibility: accept the previously-documented alias
        // "current-in-superproject" in addition to the spellings handled
        // by `from_gitmodules`.
        if s == "current-in-superproject" {
            return Ok(SerializableBranch::CurrentInSuperproject);
        }
        SerializableBranch::from_gitmodules(&s).map_err(|_| {
            serde::de::Error::custom(format!(
                "invalid branch value: {s:?}; expected \".\", \"current\", \"current-in-super-project\", \"superproject\", \"super\", or a non-empty, non-whitespace branch name"
            ))
        })
    }
}

impl SerializableBranch {
    /// Get the current branch name from the superproject repository.
    pub fn current_in_superproject() -> Result<String, anyhow::Error> {
        get_current_repository()
            .map(|repo| {
                get_current_branch(Some(&repo))
                    .unwrap_or_else(|_| "current-in-super-project".to_string())
            })
            .map_err(|_| anyhow::anyhow!("Failed to get current branch in superproject"))
    }
}

impl GitmodulesConvert for SerializableBranch {
    /// Get the git key for the branch submodule setting
    fn gitmodules_key(&self) -> &str {
        "branch"
    }

    /// Convert to gitmodules string (what you would get from the .gitmodules or .git/config)
    fn to_gitmodules(&self) -> String {
        match self {
            SerializableBranch::CurrentInSuperproject => ".".to_string(),
            SerializableBranch::Name(name) => name.to_string(),
        }
    }

    /// Convert from gitmodules string (what you would get from the .gitmodules or .git/config)
    fn from_gitmodules(options: &str) -> Result<Self, ()> {
        if options == "."
            || options == "current"
            || options == "current-in-super-project"
            || options == "superproject"
            || options == "super"
        {
            return Ok(SerializableBranch::CurrentInSuperproject);
        }
        let trimmed = options.trim();
        if trimmed.is_empty() {
            return Err(());
        }
        Ok(SerializableBranch::Name(trimmed.to_string()))
    }

    /// Convert from gitmodules bytes (what you would get from the .gitmodules or .git/config)
    fn from_gitmodules_bytes(options: &[u8]) -> Result<Self, ()> {
        let options_str = std::str::from_utf8(options).map_err(|_| ())?;
        Self::from_gitmodules(options_str)
    }
}

impl TryFrom<Branch> for SerializableBranch {
    type Error = ();

    fn try_from(value: Branch) -> Result<Self, Self::Error> {
        Ok(match value {
            Branch::CurrentInSuperproject => SerializableBranch::CurrentInSuperproject,
            Branch::Name(name) => SerializableBranch::Name(name.to_string()),
            _ => return Err(()), // Handle unsupported variants
        })
    }
}

impl TryFrom<SerializableBranch> for Branch {
    type Error = ();

    fn try_from(value: SerializableBranch) -> Result<Self, Self::Error> {
        Ok(match value {
            SerializableBranch::CurrentInSuperproject => Branch::CurrentInSuperproject,
            SerializableBranch::Name(name) => Branch::Name(name.to_string().as_bytes().into()),
            _ => return Err(()), // Handle unsupported variants
        })
    }
}

impl std::fmt::Display for SerializableBranch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerializableBranch::CurrentInSuperproject => write!(f, "."),
            SerializableBranch::Name(name) => write!(f, "{}", name),
        }
    }
}

impl FromStr for SerializableBranch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "."
            || s == "current"
            || s == "current-in-super-project"
            || s == "superproject"
            || s == "super"
        {
            return Ok(SerializableBranch::CurrentInSuperproject);
        }
        Ok(SerializableBranch::Name(s.to_string()))
    }
}

impl Default for SerializableBranch {
    fn default() -> Self {
        let default_branch = gix_submodule::config::Branch::default();
        SerializableBranch::try_from(default_branch)
            .unwrap_or_else(|_| SerializableBranch::Name("main".to_string()))
    }
}

#[allow(dead_code)]
impl SerializableBranch {
    /// Parse an optional branch string into a `SerializableBranch`, defaulting to the repo's current branch.
    pub fn set_branch(branch: Option<String>) -> Result<Self, anyhow::Error> {
        let branch = if let Some(b) = branch {
            if !b.is_empty() {
                Some(
                    SerializableBranch::from_str(b.trim())
                        .map_err(|_| anyhow::anyhow!("Invalid branch string")),
                )
            } else {
                Some(Ok(SerializableBranch::default()))
            }
        } else {
            Some(Ok(SerializableBranch::default()))
        };
        branch.unwrap_or_else(|| Ok(SerializableBranch::default()))
    }
}

impl GixGit2Convert for SerializableBranch {
    type Git2Type = String; // git2 does not have a direct type for Branch, so we use str
    type GixType = gix_submodule::config::Branch;
    /// Convert from a `git2` type to a `gix_submodule` type
    fn from_git2(git2: Self::Git2Type) -> Result<Self, ()> {
        Self::from_gitmodules(git2.as_str()).map_err(|_| ()) // Handle unsupported variants
    }

    /// Convert from a `gix_submodule` type to a `submod` type
    fn from_gix(gix: Self::GixType) -> Result<Self, ()> {
        Self::try_from(gix).map_err(|_| ()) // Handle unsupported variants
    }
}

/// Serializable enum for [`Update`] config
#[derive(
    Debug, Clone, Default, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, ValueEnum,
)]
#[serde(rename_all = "kebab-case")]
pub enum SerializableUpdate {
    /// Update the submodule by checking out the commit specified in the superproject.
    #[default]
    Checkout,
    /// Update the submodule by rebasing the current branch onto the commit specified in the superproject. Default behavior. This keeps the submodule's history linear and avoids merge commits.
    Rebase,
    /// Update the submodule by merging the commit specified in the superproject into the current branch. This is useful if you want to keep the submodule's history intact and allow for merge commits.
    Merge,
    /// Do not update the submodule at all. This is useful if you want to manage submodules manually or if you don't want to update them at all.
    None,
    /// Used as a sentinel value internally; do not use in a submod.toml or submod CLI command.
    #[serde(skip)]
    #[value(skip)]
    Unspecified,
}

impl OptionsChecks for SerializableUpdate {
    /// Check if the enum is unspecified
    fn is_unspecified(&self) -> bool {
        matches!(self, SerializableUpdate::Unspecified)
    }

    /// Check if the enum is the default value
    fn is_default(&self) -> bool {
        matches!(self, SerializableUpdate::Checkout)
    }
}

impl GitmodulesConvert for SerializableUpdate {
    /// Get the git key for the update submodule setting
    fn gitmodules_key(&self) -> &str {
        "update"
    }

    /// Convert to gitmodules string (what you would get from the .gitmodules or .git/config)
    fn to_gitmodules(&self) -> String {
        match self {
            SerializableUpdate::Checkout => "checkout".to_string(),
            SerializableUpdate::Rebase => "rebase".to_string(),
            SerializableUpdate::Merge => "merge".to_string(),
            SerializableUpdate::None => "none".to_string(),
            SerializableUpdate::Unspecified => "".to_string(), // Unspecified is treated as an empty string
        }
    }

    /// Convert from gitmodules string (what you would get from the .gitmodules or .git/config)
    fn from_gitmodules(options: &str) -> Result<Self, ()> {
        match options {
            "checkout" => Ok(SerializableUpdate::Checkout),
            "rebase" => Ok(SerializableUpdate::Rebase),
            "merge" => Ok(SerializableUpdate::Merge),
            "none" => Ok(SerializableUpdate::None), // Default is None
            "" => Ok(SerializableUpdate::Unspecified), // Empty string is treated as unspecified
            _ => Err(()),                           // Handle unsupported options
        }
    }

    /// Convert from gitmodules bytes (what you would get from the .gitmodules or .git/config)
    fn from_gitmodules_bytes(options: &[u8]) -> Result<Self, ()> {
        let options_str = std::str::from_utf8(options).map_err(|_| ())?;
        Self::from_gitmodules(options_str)
    }
}

impl TryFrom<Git2SubmoduleUpdate> for SerializableUpdate {
    type Error = ();
    fn try_from(value: Git2SubmoduleUpdate) -> Result<Self, Self::Error> {
        Ok(match value {
            Git2SubmoduleUpdate::Checkout => SerializableUpdate::Checkout,
            Git2SubmoduleUpdate::Rebase => SerializableUpdate::Rebase,
            Git2SubmoduleUpdate::Merge => SerializableUpdate::Merge,
            Git2SubmoduleUpdate::None => SerializableUpdate::None,
            Git2SubmoduleUpdate::Default => SerializableUpdate::Unspecified,
            _ => return Err(()),
        })
    }
}

impl TryFrom<SerializableUpdate> for Git2SubmoduleUpdate {
    type Error = ();
    fn try_from(value: SerializableUpdate) -> Result<Self, Self::Error> {
        Ok(match value {
            SerializableUpdate::Checkout => Git2SubmoduleUpdate::Checkout,
            SerializableUpdate::Rebase => Git2SubmoduleUpdate::Rebase,
            SerializableUpdate::Merge => Git2SubmoduleUpdate::Merge,
            SerializableUpdate::None => Git2SubmoduleUpdate::None,
            SerializableUpdate::Unspecified => Git2SubmoduleUpdate::Default,
            _ => return Err(()), // Handle unsupported variants
        })
    }
}

impl TryFrom<Update> for SerializableUpdate {
    type Error = ();
    fn try_from(value: Update) -> Result<Self, Self::Error> {
        Ok(match value {
            Update::Checkout => SerializableUpdate::Checkout,
            Update::Rebase => SerializableUpdate::Rebase,
            Update::Merge => SerializableUpdate::Merge,
            Update::None => SerializableUpdate::None,
            // Commands are not directly serializable, and can't be defined in .gitmodules, so we use unspecified. `gix` has it as a variant because it can be provided by library call.
            Update::Command(_cmd) => SerializableUpdate::Unspecified,
            _ => return Err(()),
        })
    }
}
impl TryFrom<SerializableUpdate> for Update {
    type Error = ();
    fn try_from(value: SerializableUpdate) -> Result<Self, Self::Error> {
        Ok(match value {
            SerializableUpdate::Checkout | SerializableUpdate::Unspecified => Update::Checkout,
            SerializableUpdate::Rebase => Update::Rebase,
            SerializableUpdate::Merge => Update::Merge,
            SerializableUpdate::None => Update::None,

            _ => return Err(()), // Handle unsupported variants
        })
    }
}

impl std::fmt::Display for SerializableUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_gitmodules())
    }
}

impl GixGit2Convert for SerializableUpdate {
    type Git2Type = Git2SubmoduleUpdate;
    type GixType = gix_submodule::config::Update;
    /// Convert from a `git2` type to a `gix_submodule` type
    fn from_git2(git2: Self::Git2Type) -> Result<Self, ()> {
        Self::try_from(git2).map_err(|_| ()) // Handle unsupported variants
    }

    /// Convert from a `gix_submodule` type to a `submod` type
    fn from_gix(gix: Self::GixType) -> Result<Self, ()> {
        Self::try_from(gix).map_err(|_| ()) // Handle unsupported variants
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn test_branch_deserialize_from_toml_rejects_empty_and_whitespace() {
        // Empty string should be rejected
        let res_empty: Result<SerializableBranch, toml::de::Error> =
            toml::from_str("branch = \"\"");
        assert!(res_empty.is_err(), "expected error for empty branch value");
        let err_empty = res_empty.unwrap_err().to_string();
        assert!(
            err_empty.contains("invalid branch value"),
            "error for empty branch value should contain context, got: {err_empty}"
        );

        // Whitespace-only string should be rejected
        let res_ws: Result<SerializableBranch, toml::de::Error> =
            toml::from_str("branch = \"   \"");
        assert!(res_ws.is_err(), "expected error for whitespace-only branch value");
        let err_ws = res_ws.unwrap_err().to_string();
        assert!(
            err_ws.contains("invalid branch value"),
            "error for whitespace-only branch value should contain context, got: {err_ws}"
        );
    }

    #[test]
    fn test_serializable_ignore_gitmodules_key() {
        assert_eq!(SerializableIgnore::All.gitmodules_key(), "ignore");
        assert_eq!(SerializableIgnore::Dirty.gitmodules_key(), "ignore");
        assert_eq!(SerializableIgnore::Untracked.gitmodules_key(), "ignore");
        assert_eq!(SerializableIgnore::None.gitmodules_key(), "ignore");
        assert_eq!(SerializableIgnore::Unspecified.gitmodules_key(), "ignore");
    }

    #[test]
    fn test_serializable_ignore_to_gitmodules() {
        assert_eq!(SerializableIgnore::All.to_gitmodules(), "all");
        assert_eq!(SerializableIgnore::Dirty.to_gitmodules(), "dirty");
        assert_eq!(SerializableIgnore::Untracked.to_gitmodules(), "untracked");
        assert_eq!(SerializableIgnore::None.to_gitmodules(), "none");
        assert_eq!(SerializableIgnore::Unspecified.to_gitmodules(), "");
    }

    #[test]
    fn test_serializable_ignore_from_gitmodules() {
        assert_eq!(
            SerializableIgnore::from_gitmodules("all").unwrap(),
            SerializableIgnore::All
        );
        assert_eq!(
            SerializableIgnore::from_gitmodules("dirty").unwrap(),
            SerializableIgnore::Dirty
        );
        assert_eq!(
            SerializableIgnore::from_gitmodules("untracked").unwrap(),
            SerializableIgnore::Untracked
        );
        assert_eq!(
            SerializableIgnore::from_gitmodules("none").unwrap(),
            SerializableIgnore::None
        );
        assert_eq!(
            SerializableIgnore::from_gitmodules("").unwrap(),
            SerializableIgnore::Unspecified
        );

        assert!(SerializableIgnore::from_gitmodules("invalid").is_err());
        assert!(SerializableIgnore::from_gitmodules("ALL").is_err());
    }

    #[test]
    fn test_serializable_ignore_from_gitmodules_bytes() {
        assert_eq!(
            SerializableIgnore::from_gitmodules_bytes(b"all").unwrap(),
            SerializableIgnore::All
        );
        assert_eq!(
            SerializableIgnore::from_gitmodules_bytes(b"dirty").unwrap(),
            SerializableIgnore::Dirty
        );
        assert_eq!(
            SerializableIgnore::from_gitmodules_bytes(b"untracked").unwrap(),
            SerializableIgnore::Untracked
        );
        assert_eq!(
            SerializableIgnore::from_gitmodules_bytes(b"none").unwrap(),
            SerializableIgnore::None
        );
        assert_eq!(
            SerializableIgnore::from_gitmodules_bytes(b"").unwrap(),
            SerializableIgnore::Unspecified
        );

        assert!(SerializableIgnore::from_gitmodules_bytes(b"invalid").is_err());

        // Invalid UTF-8
        assert!(SerializableIgnore::from_gitmodules_bytes(&[0xFF, 0xFE, 0xFD]).is_err());
    }

    // ================================================================
    // SerializableIgnore: Display, OptionsChecks, TryFrom conversions
    // ================================================================

    #[test]
    fn test_ignore_display() {
        assert_eq!(format!("{}", SerializableIgnore::All), "all");
        assert_eq!(format!("{}", SerializableIgnore::Dirty), "dirty");
        assert_eq!(format!("{}", SerializableIgnore::Untracked), "untracked");
        assert_eq!(format!("{}", SerializableIgnore::None), "none");
        assert_eq!(format!("{}", SerializableIgnore::Unspecified), "");
    }

    #[test]
    fn test_ignore_options_checks() {
        assert!(SerializableIgnore::Unspecified.is_unspecified());
        assert!(!SerializableIgnore::All.is_unspecified());
        assert!(!SerializableIgnore::None.is_unspecified());

        assert!(SerializableIgnore::None.is_default());
        assert!(!SerializableIgnore::All.is_default());
        assert!(!SerializableIgnore::Unspecified.is_default());
    }

    #[test]
    fn test_ignore_gitmodules_key_path() {
        assert_eq!(
            SerializableIgnore::All.gitmodules_key_path("mymod"),
            "submodule.mymod.ignore"
        );
    }

    #[test]
    fn test_ignore_tryfrom_git2_roundtrip() {
        use git2::SubmoduleIgnore as G2;
        // Our type → git2
        let git2_all: G2 = SerializableIgnore::All.try_into().unwrap();
        assert_eq!(git2_all, G2::All);
        let git2_dirty: G2 = SerializableIgnore::Dirty.try_into().unwrap();
        assert_eq!(git2_dirty, G2::Dirty);
        let git2_untracked: G2 = SerializableIgnore::Untracked.try_into().unwrap();
        assert_eq!(git2_untracked, G2::Untracked);
        let git2_none: G2 = SerializableIgnore::None.try_into().unwrap();
        assert_eq!(git2_none, G2::None);
        let git2_unspec: G2 = SerializableIgnore::Unspecified.try_into().unwrap();
        assert_eq!(git2_unspec, G2::Unspecified);

        // git2 → our type
        let ours: SerializableIgnore = G2::All.try_into().unwrap();
        assert_eq!(ours, SerializableIgnore::All);
        let ours: SerializableIgnore = G2::Dirty.try_into().unwrap();
        assert_eq!(ours, SerializableIgnore::Dirty);
        let ours: SerializableIgnore = G2::None.try_into().unwrap();
        assert_eq!(ours, SerializableIgnore::None);
        let ours: SerializableIgnore = G2::Unspecified.try_into().unwrap();
        assert_eq!(ours, SerializableIgnore::Unspecified);
    }

    #[test]
    fn test_ignore_tryfrom_gix_roundtrip() {
        // Our type → gix
        let gix_all: Ignore = SerializableIgnore::All.try_into().unwrap();
        assert_eq!(gix_all, Ignore::All);
        let gix_dirty: Ignore = SerializableIgnore::Dirty.try_into().unwrap();
        assert_eq!(gix_dirty, Ignore::Dirty);
        // Unspecified maps to None in gix
        let gix_unspec: Ignore = SerializableIgnore::Unspecified.try_into().unwrap();
        assert_eq!(gix_unspec, Ignore::None);

        // gix → our type
        let ours: SerializableIgnore = Ignore::All.try_into().unwrap();
        assert_eq!(ours, SerializableIgnore::All);
        let ours: SerializableIgnore = Ignore::None.try_into().unwrap();
        assert_eq!(ours, SerializableIgnore::None);
    }

    #[test]
    fn test_ignore_gixgit2convert() {
        let from_git2 = SerializableIgnore::from_git2(git2::SubmoduleIgnore::All).unwrap();
        assert_eq!(from_git2, SerializableIgnore::All);
        let from_gix = SerializableIgnore::from_gix(Ignore::Dirty).unwrap();
        assert_eq!(from_gix, SerializableIgnore::Dirty);
    }

    // ================================================================
    // SerializableFetchRecurse: full coverage
    // ================================================================

    #[test]
    fn test_fetch_recurse_gitmodules_key() {
        assert_eq!(
            SerializableFetchRecurse::OnDemand.gitmodules_key(),
            "fetchRecurseSubmodules"
        );
    }

    #[test]
    fn test_fetch_recurse_to_gitmodules() {
        assert_eq!(
            SerializableFetchRecurse::OnDemand.to_gitmodules(),
            "on-demand"
        );
        assert_eq!(SerializableFetchRecurse::Always.to_gitmodules(), "true");
        assert_eq!(SerializableFetchRecurse::Never.to_gitmodules(), "false");
        assert_eq!(
            SerializableFetchRecurse::Unspecified.to_gitmodules(),
            "on-demand"
        );
    }

    #[test]
    fn test_fetch_recurse_from_gitmodules() {
        assert_eq!(
            SerializableFetchRecurse::from_gitmodules("on-demand").unwrap(),
            SerializableFetchRecurse::OnDemand
        );
        assert_eq!(
            SerializableFetchRecurse::from_gitmodules("true").unwrap(),
            SerializableFetchRecurse::Always
        );
        assert_eq!(
            SerializableFetchRecurse::from_gitmodules("false").unwrap(),
            SerializableFetchRecurse::Never
        );
        assert_eq!(
            SerializableFetchRecurse::from_gitmodules("").unwrap(),
            SerializableFetchRecurse::Unspecified
        );
        assert!(SerializableFetchRecurse::from_gitmodules("invalid").is_err());
        assert!(SerializableFetchRecurse::from_gitmodules("ON-DEMAND").is_err());
    }

    #[test]
    fn test_fetch_recurse_from_gitmodules_bytes() {
        assert_eq!(
            SerializableFetchRecurse::from_gitmodules_bytes(b"on-demand").unwrap(),
            SerializableFetchRecurse::OnDemand
        );
        assert_eq!(
            SerializableFetchRecurse::from_gitmodules_bytes(b"true").unwrap(),
            SerializableFetchRecurse::Always
        );
        assert!(SerializableFetchRecurse::from_gitmodules_bytes(&[0xFF]).is_err());
    }

    #[test]
    fn test_fetch_recurse_options_checks() {
        assert!(SerializableFetchRecurse::Unspecified.is_unspecified());
        assert!(!SerializableFetchRecurse::OnDemand.is_unspecified());

        assert!(SerializableFetchRecurse::OnDemand.is_default());
        assert!(!SerializableFetchRecurse::Always.is_default());
    }

    #[test]
    fn test_fetch_recurse_display() {
        assert_eq!(
            format!("{}", SerializableFetchRecurse::OnDemand),
            "on-demand"
        );
        assert_eq!(format!("{}", SerializableFetchRecurse::Always), "true");
        assert_eq!(format!("{}", SerializableFetchRecurse::Never), "false");
    }

    #[test]
    fn test_fetch_recurse_tryfrom_gix_roundtrip() {
        let gix_od: FetchRecurse = SerializableFetchRecurse::OnDemand.try_into().unwrap();
        assert_eq!(gix_od, FetchRecurse::OnDemand);
        let gix_always: FetchRecurse = SerializableFetchRecurse::Always.try_into().unwrap();
        assert_eq!(gix_always, FetchRecurse::Always);
        let gix_never: FetchRecurse = SerializableFetchRecurse::Never.try_into().unwrap();
        assert_eq!(gix_never, FetchRecurse::Never);
        // Unspecified maps to OnDemand in gix
        let gix_unspec: FetchRecurse = SerializableFetchRecurse::Unspecified.try_into().unwrap();
        assert_eq!(gix_unspec, FetchRecurse::OnDemand);

        let ours: SerializableFetchRecurse = FetchRecurse::OnDemand.try_into().unwrap();
        assert_eq!(ours, SerializableFetchRecurse::OnDemand);
        let ours: SerializableFetchRecurse = FetchRecurse::Always.try_into().unwrap();
        assert_eq!(ours, SerializableFetchRecurse::Always);
        let ours: SerializableFetchRecurse = FetchRecurse::Never.try_into().unwrap();
        assert_eq!(ours, SerializableFetchRecurse::Never);
    }

    #[test]
    fn test_fetch_recurse_gixgit2convert() {
        let from_git2 = SerializableFetchRecurse::from_git2("on-demand".to_string()).unwrap();
        assert_eq!(from_git2, SerializableFetchRecurse::OnDemand);
        let from_git2 = SerializableFetchRecurse::from_git2("true".to_string()).unwrap();
        assert_eq!(from_git2, SerializableFetchRecurse::Always);
        assert!(SerializableFetchRecurse::from_git2("bogus".to_string()).is_err());

        let from_gix = SerializableFetchRecurse::from_gix(FetchRecurse::Never).unwrap();
        assert_eq!(from_gix, SerializableFetchRecurse::Never);
    }

    #[test]
    fn test_fetch_recurse_gitmodules_key_path() {
        assert_eq!(
            SerializableFetchRecurse::Always.gitmodules_key_path("sub1"),
            "submodule.sub1.fetchRecurseSubmodules"
        );
    }

    // ================================================================
    // SerializableBranch: comprehensive coverage
    // ================================================================

    #[test]
    fn test_branch_from_str_aliases() {
        assert_eq!(
            SerializableBranch::from_str(".").unwrap(),
            SerializableBranch::CurrentInSuperproject
        );
        assert_eq!(
            SerializableBranch::from_str("current").unwrap(),
            SerializableBranch::CurrentInSuperproject
        );
        assert_eq!(
            SerializableBranch::from_str("current-in-super-project").unwrap(),
            SerializableBranch::CurrentInSuperproject
        );
        assert_eq!(
            SerializableBranch::from_str("superproject").unwrap(),
            SerializableBranch::CurrentInSuperproject
        );
        assert_eq!(
            SerializableBranch::from_str("super").unwrap(),
            SerializableBranch::CurrentInSuperproject
        );
    }

    #[test]
    fn test_branch_from_str_named() {
        assert_eq!(
            SerializableBranch::from_str("main").unwrap(),
            SerializableBranch::Name("main".to_string())
        );
        assert_eq!(
            SerializableBranch::from_str("develop").unwrap(),
            SerializableBranch::Name("develop".to_string())
        );
        assert_eq!(
            SerializableBranch::from_str("feature/my-feature").unwrap(),
            SerializableBranch::Name("feature/my-feature".to_string())
        );
    }

    #[test]
    fn test_branch_display() {
        assert_eq!(
            format!("{}", SerializableBranch::CurrentInSuperproject),
            "."
        );
        assert_eq!(
            format!("{}", SerializableBranch::Name("main".to_string())),
            "main"
        );
    }

    #[test]
    fn test_branch_default() {
        let default = SerializableBranch::default();
        // Default should be a Name variant (either from gix default or "main" fallback)
        match &default {
            SerializableBranch::Name(_) => {} // expected
            SerializableBranch::CurrentInSuperproject => {
                // also acceptable if gix default is CurrentInSuperproject
            }
        }
    }

    #[test]
    fn test_branch_to_gitmodules() {
        assert_eq!(
            SerializableBranch::CurrentInSuperproject.to_gitmodules(),
            "."
        );
        assert_eq!(
            SerializableBranch::Name("develop".to_string()).to_gitmodules(),
            "develop"
        );
    }

    #[test]
    fn test_branch_gitmodules_key() {
        assert_eq!(
            SerializableBranch::CurrentInSuperproject.gitmodules_key(),
            "branch"
        );
    }

    #[test]
    fn test_branch_from_gitmodules_aliases() {
        assert_eq!(
            SerializableBranch::from_gitmodules(".").unwrap(),
            SerializableBranch::CurrentInSuperproject
        );
        assert_eq!(
            SerializableBranch::from_gitmodules("current").unwrap(),
            SerializableBranch::CurrentInSuperproject
        );
        assert_eq!(
            SerializableBranch::from_gitmodules("superproject").unwrap(),
            SerializableBranch::CurrentInSuperproject
        );
        assert_eq!(
            SerializableBranch::from_gitmodules("super").unwrap(),
            SerializableBranch::CurrentInSuperproject
        );
    }

    #[test]
    fn test_branch_from_gitmodules_named() {
        assert_eq!(
            SerializableBranch::from_gitmodules("main").unwrap(),
            SerializableBranch::Name("main".to_string())
        );
    }

    #[test]
    fn test_branch_from_gitmodules_empty_is_err() {
        assert!(SerializableBranch::from_gitmodules("").is_err());
        assert!(SerializableBranch::from_gitmodules("   ").is_err());
    }

    #[test]
    fn test_branch_from_gitmodules_bytes() {
        assert_eq!(
            SerializableBranch::from_gitmodules_bytes(b".").unwrap(),
            SerializableBranch::CurrentInSuperproject
        );
        assert_eq!(
            SerializableBranch::from_gitmodules_bytes(b"main").unwrap(),
            SerializableBranch::Name("main".to_string())
        );
        assert!(SerializableBranch::from_gitmodules_bytes(&[0xFF]).is_err());
    }

    #[test]
    fn test_branch_tryfrom_gix_roundtrip() {
        let gix_cur: Branch = SerializableBranch::CurrentInSuperproject
            .try_into()
            .unwrap();
        assert_eq!(gix_cur, Branch::CurrentInSuperproject);

        let gix_name: Branch = SerializableBranch::Name("main".to_string())
            .try_into()
            .unwrap();
        match gix_name {
            Branch::Name(n) => assert_eq!(n.to_string(), "main"),
            _ => panic!("Expected Branch::Name"),
        }

        let ours: SerializableBranch = Branch::CurrentInSuperproject.try_into().unwrap();
        assert_eq!(ours, SerializableBranch::CurrentInSuperproject);
    }

    #[test]
    fn test_branch_gixgit2convert() {
        let from_git2 = SerializableBranch::from_git2("main".to_string()).unwrap();
        assert_eq!(from_git2, SerializableBranch::Name("main".to_string()));

        let from_git2 = SerializableBranch::from_git2(".".to_string()).unwrap();
        assert_eq!(from_git2, SerializableBranch::CurrentInSuperproject);

        let from_gix = SerializableBranch::from_gix(Branch::CurrentInSuperproject).unwrap();
        assert_eq!(from_gix, SerializableBranch::CurrentInSuperproject);
    }

    #[test]
    fn test_branch_set_branch_with_name() {
        let result = SerializableBranch::set_branch(Some("develop".to_string())).unwrap();
        assert_eq!(result, SerializableBranch::Name("develop".to_string()));
    }

    #[test]
    fn test_branch_set_branch_with_alias() {
        let result = SerializableBranch::set_branch(Some(".".to_string())).unwrap();
        assert_eq!(result, SerializableBranch::CurrentInSuperproject);

        let result = SerializableBranch::set_branch(Some("super".to_string())).unwrap();
        assert_eq!(result, SerializableBranch::CurrentInSuperproject);
    }

    #[test]
    fn test_branch_set_branch_empty_returns_default() {
        let result = SerializableBranch::set_branch(Some("".to_string())).unwrap();
        // Empty string → default
        assert_eq!(result, SerializableBranch::default());
    }

    #[test]
    fn test_branch_set_branch_none_returns_default() {
        let result = SerializableBranch::set_branch(None).unwrap();
        assert_eq!(result, SerializableBranch::default());
    }

    #[test]
    fn test_branch_set_branch_trims_whitespace() {
        let result = SerializableBranch::set_branch(Some("  main  ".to_string())).unwrap();
        assert_eq!(result, SerializableBranch::Name("main".to_string()));
    }

    #[test]
    fn test_branch_deserialize_from_toml() {
        #[derive(Deserialize)]
        struct TestConfig {
            branch: SerializableBranch,
        }
        let toml_str = r#"branch = "main""#;
        let config: TestConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.branch, SerializableBranch::Name("main".to_string()));

        let toml_str = r#"branch = ".""#;
        let config: TestConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.branch, SerializableBranch::CurrentInSuperproject);

        let toml_str = r#"branch = "super""#;
        let config: TestConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.branch, SerializableBranch::CurrentInSuperproject);
    }

    #[test]
    fn test_branch_gitmodules_key_path() {
        assert_eq!(
            SerializableBranch::Name("main".to_string()).gitmodules_key_path("mymod"),
            "submodule.mymod.branch"
        );
    }

    // ================================================================
    // SerializableUpdate: full coverage
    // ================================================================

    #[test]
    fn test_update_gitmodules_key() {
        assert_eq!(SerializableUpdate::Checkout.gitmodules_key(), "update");
    }

    #[test]
    fn test_update_to_gitmodules() {
        assert_eq!(SerializableUpdate::Checkout.to_gitmodules(), "checkout");
        assert_eq!(SerializableUpdate::Rebase.to_gitmodules(), "rebase");
        assert_eq!(SerializableUpdate::Merge.to_gitmodules(), "merge");
        assert_eq!(SerializableUpdate::None.to_gitmodules(), "none");
        assert_eq!(SerializableUpdate::Unspecified.to_gitmodules(), "");
    }

    #[test]
    fn test_update_from_gitmodules() {
        assert_eq!(
            SerializableUpdate::from_gitmodules("checkout").unwrap(),
            SerializableUpdate::Checkout
        );
        assert_eq!(
            SerializableUpdate::from_gitmodules("rebase").unwrap(),
            SerializableUpdate::Rebase
        );
        assert_eq!(
            SerializableUpdate::from_gitmodules("merge").unwrap(),
            SerializableUpdate::Merge
        );
        assert_eq!(
            SerializableUpdate::from_gitmodules("none").unwrap(),
            SerializableUpdate::None
        );
        assert_eq!(
            SerializableUpdate::from_gitmodules("").unwrap(),
            SerializableUpdate::Unspecified
        );
        assert!(SerializableUpdate::from_gitmodules("invalid").is_err());
    }

    #[test]
    fn test_update_from_gitmodules_bytes() {
        assert_eq!(
            SerializableUpdate::from_gitmodules_bytes(b"checkout").unwrap(),
            SerializableUpdate::Checkout
        );
        assert_eq!(
            SerializableUpdate::from_gitmodules_bytes(b"rebase").unwrap(),
            SerializableUpdate::Rebase
        );
        assert!(SerializableUpdate::from_gitmodules_bytes(&[0xFF]).is_err());
    }

    #[test]
    fn test_update_options_checks() {
        assert!(SerializableUpdate::Unspecified.is_unspecified());
        assert!(!SerializableUpdate::Checkout.is_unspecified());

        assert!(SerializableUpdate::Checkout.is_default());
        assert!(!SerializableUpdate::Rebase.is_default());
    }

    #[test]
    fn test_update_display() {
        assert_eq!(format!("{}", SerializableUpdate::Checkout), "checkout");
        assert_eq!(format!("{}", SerializableUpdate::Rebase), "rebase");
        assert_eq!(format!("{}", SerializableUpdate::Merge), "merge");
        assert_eq!(format!("{}", SerializableUpdate::None), "none");
        assert_eq!(format!("{}", SerializableUpdate::Unspecified), "");
    }

    #[test]
    fn test_update_tryfrom_git2_roundtrip() {
        use git2::SubmoduleUpdate as G2U;
        let git2_co: G2U = SerializableUpdate::Checkout.try_into().unwrap();
        assert_eq!(git2_co, G2U::Checkout);
        let git2_rebase: G2U = SerializableUpdate::Rebase.try_into().unwrap();
        assert_eq!(git2_rebase, G2U::Rebase);
        let git2_merge: G2U = SerializableUpdate::Merge.try_into().unwrap();
        assert_eq!(git2_merge, G2U::Merge);
        let git2_none: G2U = SerializableUpdate::None.try_into().unwrap();
        assert_eq!(git2_none, G2U::None);
        let git2_unspec: G2U = SerializableUpdate::Unspecified.try_into().unwrap();
        assert_eq!(git2_unspec, G2U::Default);

        let ours: SerializableUpdate = G2U::Checkout.try_into().unwrap();
        assert_eq!(ours, SerializableUpdate::Checkout);
        let ours: SerializableUpdate = G2U::Rebase.try_into().unwrap();
        assert_eq!(ours, SerializableUpdate::Rebase);
        let ours: SerializableUpdate = G2U::Default.try_into().unwrap();
        assert_eq!(ours, SerializableUpdate::Unspecified);
    }

    #[test]
    fn test_update_tryfrom_gix_roundtrip() {
        let gix_co: Update = SerializableUpdate::Checkout.try_into().unwrap();
        assert_eq!(gix_co, Update::Checkout);
        let gix_rebase: Update = SerializableUpdate::Rebase.try_into().unwrap();
        assert_eq!(gix_rebase, Update::Rebase);
        let gix_merge: Update = SerializableUpdate::Merge.try_into().unwrap();
        assert_eq!(gix_merge, Update::Merge);
        let gix_none: Update = SerializableUpdate::None.try_into().unwrap();
        assert_eq!(gix_none, Update::None);
        // Unspecified maps to Checkout in gix
        let gix_unspec: Update = SerializableUpdate::Unspecified.try_into().unwrap();
        assert_eq!(gix_unspec, Update::Checkout);

        let ours: SerializableUpdate = Update::Checkout.try_into().unwrap();
        assert_eq!(ours, SerializableUpdate::Checkout);
        let ours: SerializableUpdate = Update::Rebase.try_into().unwrap();
        assert_eq!(ours, SerializableUpdate::Rebase);
        let ours: SerializableUpdate = Update::None.try_into().unwrap();
        assert_eq!(ours, SerializableUpdate::None);
    }

    #[test]
    fn test_update_gix_command_variant_maps_to_unspecified() {
        // Update::Command is not serializable; it should map to Unspecified
        let cmd = Update::Command("!echo hello".into());
        let ours: SerializableUpdate = cmd.try_into().unwrap();
        assert_eq!(ours, SerializableUpdate::Unspecified);
    }

    #[test]
    fn test_update_gixgit2convert() {
        let from_git2 = SerializableUpdate::from_git2(git2::SubmoduleUpdate::Merge).unwrap();
        assert_eq!(from_git2, SerializableUpdate::Merge);

        let from_gix = SerializableUpdate::from_gix(Update::Rebase).unwrap();
        assert_eq!(from_gix, SerializableUpdate::Rebase);
    }

    #[test]
    fn test_update_gitmodules_key_path() {
        assert_eq!(
            SerializableUpdate::Checkout.gitmodules_key_path("sub1"),
            "submodule.sub1.update"
        );
    }
}
