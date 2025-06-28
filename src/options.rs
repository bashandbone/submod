// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: MIT
// Licensed under the [Plain MIT License][../LICENSE.md]
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
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Configuration levels for git config operations
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

pub trait  GitmodulesConvert {
    /// Get the git key for a submodule by the submodule's name (in git config)
    fn gitmodules_key_path(&self, name: &str) -> String;

    /// Get the git key for the enum setting
    fn gitmodules_key(&self) -> &str;

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

pub trait OptionsChecks {
    /// Check if the enum is unspecified
    fn is_unspecified(&self) -> bool;

    /// Check if the enum is the default value
    fn is_default(&self) -> bool;

}

pub trait IsUnspecified {
    /// Check if the enum is unspecified
    fn is_unspecified(&self) -> bool;
}

pub trait IsDefault {
    /// Check if the enum is the default value
    fn is_default(&self) -> bool;
}

pub trait GixGit2Convert {
    /// Convert from a `git2` type to a `gix_submodule` type
    fn from_git2(git2: Git2SubmoduleIgnore) -> Result<Self, ()>
    where
        Self: Sized;

    /// Convert from a `gix_submodule` type to a `submod` type
    fn from_gix(gix: gix_submodule::config::Ignore) -> Result<Self, ()>
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
    Unspecified,
}

impl GitmodulesConvert for SerializableIgnore {
    /// Get the git key for a submodule by the submodule's name (in git config)
    fn gitmodules_key_path(&self, name: &str) -> String {
        format!("submodule.{name}.{}", self.gitmodules_key())
    }

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
            _ => Err(()), // Handle unsupported options
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
    Unspecified,
}

impl GitmodulesConvert for SerializableFetchRecurse {
    /// Get the git key for a submodule by the submodule's name (in git config)
    fn gitmodules_key_path(&self, name: &str) -> String {
        format!("submodule.{name}.{}", self.gitmodules_key())
    }

    /// Get the git key for the fetch recurse submodule setting
    fn gitmodules_key(&self) -> &str {
        "fetchRecurseSubmodules"
    }

    /// Convert to gitmodules string (what you would get from the .gitmodules or .git/config)
    fn to_gitmodules(&self) -> String {
        match self {
            SerializableFetchRecurse::OnDemand | SerializableFetchRecurse::Unspecified => "on-demand".to_string(),
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
            _ => Err(()), // Handle unsupported options
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
            SerializableFetchRecurse::OnDemand | SerializableFetchRecurse::Unspecified => FetchRecurse::OnDemand,
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
#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SerializableBranch {
    /// Use the same name for remote's branch name as the name of the currently activate branch in the superproject.
    /// This is a special value in git's settings. In a .git/config or .gitmodules it's represented by a period: `.`.
    CurrentInSuperproject,
    /// Track a specific branch by name. (Usually what you want.). The default value is the remote branch's default branch if we can resolve it, else `main`.
    Name(String),
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
        if s == "." || s == "current" || s == "current-in-super-project" || s == "superproject" || s == "super" {
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
    Unspecified
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
    /// Get the git key for a submodule by the submodule's name (in git config)
    fn gitmodules_key_path(&self, name: &str) -> String {
        format!("submodule.{name}.{}", self.gitmodules_key())
    }

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
            _ => Err(()), // Handle unsupported options
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
