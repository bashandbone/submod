// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: MIT
// Licensed under the [Plain MIT License][../LICENSE.md]
//! A collection of utility functions for submod.

use std::{path::PathBuf, vec};
use std::result::Result;
use anyhow::Ok;
use git2::Repository as Git2Repository;
use gix::{open::Options};
// Get the current repository using git2.
pub fn get_current_git2_repository(repo: Option<Git2Repository>) -> Result<Git2Repository, anyhow::Error> {
    match repo {
        Some(r) => Ok(r),
        None => {
            let rep = Git2Repository::discover(".")
                .map_err(|e| anyhow::anyhow!("Failed to discover repository: {}", e))?;
            if rep.is_bare() {
                return Err(anyhow::anyhow!("Bare repositories are not supported"));
            }
            Ok(rep)
        }
    }
}

/// Get the current repository. The returned repository is isolated (has very limited access to the working tree and environment).
pub fn get_current_repository() -> Result<gix::Repository, anyhow::Error> {
    let options = Options::isolated();
    Ok(gix::ThreadSafeRepository::open_opts(".", options)?.to_thread_local())
}

/// Gets the current working directory
pub fn get_cwd() -> Result<PathBuf, anyhow::Error> {
    std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get current working directory: {}", e))
}

pub fn get_thread_local_repo(repo: &gix::Repository) -> Result<gix::Repository, anyhow::Error> {
    // Get a full access repository from the given repository
    let safe_repo = repo.to_owned().into_sync().to_thread_local();
    Ok(safe_repo)
}

/// Get the main, or superproject, repository.
pub fn get_main_repo(
    repo: Option<&gix::Repository>,
) -> Result<gix::Repository, anyhow::Error> {
    let repo = match repo {
        Some(r) => r.to_owned(),
        None => get_current_repository()?,
    };
    let super_repo = repo.main_repo().map_err(|e| {
        anyhow::anyhow!("Failed to get main repository: {}", e)
    })?;
    Ok(super_repo)
}

pub fn path_to_string(path: &std::path::Path) -> Result<String, anyhow::Error> {
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Path is not valid UTF-8"))
}

/// Get the main repository's root directory.
pub fn get_main_root(
    repo: Option<&gix::Repository>,
) -> Result<PathBuf, anyhow::Error> {
    let repo = get_main_repo(repo)?;
    let path = repo.path().to_path_buf();
    if path.is_dir() {
        Ok(path)
    } else {
        Err(anyhow::anyhow!("Failed to get main repository root: {}", path.display()))
    }
}

/// Get the current working directory.
pub fn get_current_working_directory() -> Result<PathBuf, anyhow::Error> {
    std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get current working directory: {}"  , e))
}
/// Get the current branch name from the repository.
pub fn get_current_branch(repo: Option<&gix::Repository>) -> Result<String, anyhow::Error> {
    let repo = match repo {
        Some(r) => r,
        None => &get_current_repository()?,
    };
    let head = repo.head()?;
    if let Some(reference) = head.referent_name() {
        let ref_bstr = reference.as_bstr();
        return Ok(ref_bstr.to_string());
    }
    Err(anyhow::anyhow!("Failed to get current branch name"))
}

/// Get the main repository's [submodules][gix::submodule::Submodule] from the repository.
pub fn get_main_repo_submodules(
    repo: Option<&gix::Repository>,
) -> Result<Vec<impl Iterator<Item = gix::Submodule>>, anyhow::Error> {
    let main_repo = get_main_repo(repo)?;
    let submodules = main_repo.submodules()
        .map_err(|e| anyhow::anyhow!("Failed to get submodules: {}", e))?;

    let mut submodule_list = Vec::new();
    for submodule in submodules {
        submodule_list.push(submodule.collect());
    }
    Ok(submodule_list)
}

/// Get the [modules file][gix::submodule::ModulesSnapshot] from the repository.
pub fn get_modules_file(repo: Option<&gix::Repository>) -> Result<gix::submodule::ModulesSnapshot, anyhow::Error> {
    let repo = match repo {
        Some(r) => r,
        None => {
            let current_repo = get_current_repository()?;
            return current_repo.modules()
                .map_err(|e| anyhow::anyhow!("Failed to get modules file: {}", e))
                .and_then(|modules| {
                    if modules.is_empty() {
                        Err(anyhow::anyhow!("No modules found"))
                    } else {
                        Ok(modules)
                    }
                });
        }
    };
    repo.modules()
        .map_err(|e| anyhow::anyhow!("Failed to get modules file: {}", e))
        .and_then(|modules| {
            if modules.is_empty() {
                Err(anyhow::anyhow!("No modules found"))
            } else {
                Ok(modules)
            }
        })
}
