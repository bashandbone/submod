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
}

impl TryFrom<SerializableIgnore> for Git2SubmoduleIgnore {
    type Error = ();

    fn try_from(value: SerializableIgnore) -> Result<Self, Self::Error> {
        Ok(match value {
            SerializableIgnore::All => Git2SubmoduleIgnore::All,
            SerializableIgnore::Dirty => Git2SubmoduleIgnore::Dirty,
            SerializableIgnore::Untracked => Git2SubmoduleIgnore::Untracked,
            SerializableIgnore::None => Git2SubmoduleIgnore::None,
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
            Git2SubmoduleIgnore::None | Git2SubmoduleIgnore::Unspecified => {
                SerializableIgnore::None
            }
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
            SerializableIgnore::None => Ignore::None,
            _ => return Err(()),
        })
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
}

impl SerializableFetchRecurse {
    /// Get the git key for a submodule by the submodule's name (in git config)
    pub fn get_git_key(&self, name: &str) -> String {
        format!("submodule.{name}.{}", self.git_key())
    }

    /// Get the git key for the fetch recurse submodule setting
    pub fn git_key(&self) -> &str {
        "fetchRecurseSubmodules"
    }

    /// Convert to git options string (what you would get from the .gitmodules or .git/config)
    pub fn to_git_options(&self) -> String {
        match self {
            SerializableFetchRecurse::OnDemand => "on-demand".to_string(),
            SerializableFetchRecurse::Always => "true".to_string(),
            SerializableFetchRecurse::Never => "false".to_string(),
        }
    }

    /// Convert from git options string (what you would get from the .gitmodules or .git/config)
    pub fn from_git_options(options: &str) -> Result<Self, ()> {
        match options {
            "" | "none" | "None" | "on-demand" => Ok(SerializableFetchRecurse::OnDemand), // Default is OnDemand
            "true" | "always" => Ok(SerializableFetchRecurse::Always),
            "false" | "never" => Ok(SerializableFetchRecurse::Never),
            _ => Err(()), // Handle unsupported options
        }
    }

    /// Convert from git options bytes (what you would get from the .gitmodules or .git/config)
    pub fn from_git_options_bytes(options: &[u8]) -> Result<Self, ()> {
        let options_str = std::str::from_utf8(options).map_err(|_| ())?;
        Self::from_git_options(options_str)
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
            SerializableFetchRecurse::OnDemand => FetchRecurse::OnDemand,
            SerializableFetchRecurse::Always => FetchRecurse::Always,
            SerializableFetchRecurse::Never => FetchRecurse::Never,
            _ => return Err(()), // Handle unsupported variants
        })
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

impl FromStr for SerializableBranch {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "." || s == "current" || s == "current-in-super-project" || s == "superproject" || s == "super" {
            return Ok(SerializableBranch::CurrentInSuperproject);
        }
        Ok(SerializableBranch::Name(s.to_string()))
    }
}

impl ToString for SerializableBranch {
    fn to_string(&self) -> String {
        match self {
            SerializableBranch::CurrentInSuperproject => ".".to_string(),
            SerializableBranch::Name(name) => name.clone(),
        }
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
}

impl TryFrom<Git2SubmoduleUpdate> for SerializableUpdate {
    type Error = ();
    fn try_from(value: Git2SubmoduleUpdate) -> Result<Self, Self::Error> {
        Ok(match value {
            Git2SubmoduleUpdate::Checkout => SerializableUpdate::Checkout,
            Git2SubmoduleUpdate::Rebase => SerializableUpdate::Rebase,
            Git2SubmoduleUpdate::Merge => SerializableUpdate::Merge,
            Git2SubmoduleUpdate::None => SerializableUpdate::None,
            Git2SubmoduleUpdate::Default => SerializableUpdate::Checkout, // Default is Checkout
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
            Update::Command(_cmd) => SerializableUpdate::None, // Commands are not directly serializable, so we use None
            _ => return Err(()),
        })
    }
}
impl TryFrom<SerializableUpdate> for Update {
    type Error = ();
    fn try_from(value: SerializableUpdate) -> Result<Self, Self::Error> {
        Ok(match value {
            SerializableUpdate::Checkout => Update::Checkout,
            SerializableUpdate::Rebase => Update::Rebase,
            SerializableUpdate::Merge => Update::Merge,
            SerializableUpdate::None => Update::None,
            _ => return Err(()), // Handle unsupported variants
        })
    }
}
