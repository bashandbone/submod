// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

use std::{path::PathBuf};
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

/**========================================================================
 **                          Gix Utilities
 *========================================================================**/

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

/// Get a thread-local repository from the given repository.
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


/**========================================================================
 **                            General Utilities
 *========================================================================**/

/// Get the current working directory.
pub fn get_current_working_directory() -> Result<PathBuf, anyhow::Error> {
    std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get current working directory: {}"  , e))
}

/// Convert a `Path` to a `String`, returning an error if the path is not valid UTF-8
pub fn path_to_string(path: &std::path::Path) -> Result<String, anyhow::Error> {
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Path is not valid UTF-8"))
}

/// Convert a `Path` to a `String`, using lossy conversion for non-UTF-8 characters
pub fn path_to_string_lossy(path: &std::path::Path) -> String {
    let lossy = path.to_string_lossy();
    eprintln!("Warning: Path contains non-UTF-8 characters, using lossy conversion: {}", lossy);
    lossy.to_string()
}

/// Convert a `Path` to an `OsString`
pub fn path_to_os_string(path: &std::path::Path) -> std::ffi::OsString {
    path.as_os_str().to_owned()
}

/// Set the path from an OS string, converting to UTF-8 if possible
/// Used for CLI arguments and other scenarios where the path may not be valid UTF-8
pub fn set_path(path: std::ffi::OsString) -> Result<String, anyhow::Error> {

    match path.to_str() {
        Some(path) => {
            Ok(path.to_string())
        }
        None => { Ok(path_to_string_lossy(&PathBuf::from(path)))  // Use lossy conversion if the path is not valid UTF-8
        }
    }
}

/// Extract the name from a URL, trimming trailing slashes and `.git` suffix
pub fn name_from_url(url: &str) -> Result<String, anyhow::Error> {
    if url.is_empty() {
        return Err(anyhow::anyhow!("URL cannot be empty"));
    }
    let cleaned_url = url.trim_end_matches('/').trim_end_matches(".git");
    cleaned_url
        .split('/')
        .last()
        .map(|name| name.to_string())
        .ok_or_else(|| anyhow::anyhow!("Failed to extract name from URL"))
}

/// Convert an `OsString` to a `String`, extracting the name from the path
pub fn name_from_osstring(os_string: std::ffi::OsString) -> Result<String, anyhow::Error> {
    osstring_to_string(os_string).and_then(|s| {
        if s.is_empty() {
            if s.contains('\0') {
                Err(anyhow::anyhow!("Name cannot contain null bytes"))
            } else {
                Ok(s)
            }
        } else {
            let sep = std::path::MAIN_SEPARATOR.to_string();
            s.trim().split(&sep)
                .last()
                .map(|name| name.to_string())
                .ok_or_else(|| anyhow::anyhow!("Failed to extract name from OsString"))
        }
    })
}

/// Convert an `OsString` to a `String`, returning an error if the conversion fails
pub fn osstring_to_string(os_string: std::ffi::OsString) -> Result<String, anyhow::Error> {
    os_string
        .into_string()
        .map_err(|_| anyhow::anyhow!("Failed to convert OsString to String"))
}

pub fn get_sparse_paths(
    sparse_paths: Option<Vec<String>>,
) -> Result<Option<Vec<String>>, anyhow::Error> {
    let sparse_paths_vec = match sparse_paths {
        Some(paths) => {
            for path in &paths {
                if path.contains('\0') {
                    return Err(anyhow::anyhow!(
                        "Invalid sparse path pattern: contains null byte"
                    ));
                }
            }
            Some(paths)
        },
        None => None,
    };
    Ok(sparse_paths_vec)
}

/// Get the name from either a provided name, URL, or path.
pub fn get_name(
    name: Option<String>, url: Option<String>, path: Option<std::ffi::OsString>,
) -> Result<String, anyhow::Error> {
    if let Some(name) = name {
        let trimmed_name = name.trim().to_string();
        match trimmed_name.is_empty() {
            true => get_name(None, url, path), // recycle to get name from URL or path
            false => Ok(trimmed_name),
        }
    } else if let Some(path) = path {
        name_from_osstring(path)
    } else if let Some(url) = url {
        name_from_url(&url)
    } else {
        Err(anyhow::anyhow!("No valid name source provided"))
    }
}
