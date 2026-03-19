// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
//! Utility functions for working with `Gitoxide` APIs commonly used across the codebase.
#![allow(dead_code)]

use anyhow::Result;
use git2::Repository as Git2Repository;
use gix::open::Options;
use std::path::PathBuf;

/// Get the current repository using git2, with an optional provided repository. If no repository is provided, it will attempt to discover one in the current directory.
pub(crate) fn get_current_git2_repository(
    repo: Option<Git2Repository>,
) -> Result<Git2Repository, anyhow::Error> {
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

/*=========================================================================
 *                           Gix Utilities
 *========================================================================*/

/// Get a repository from a given path. The returned repository is isolated (has very limited access to the working tree and environment).
pub(crate) fn repo_from_path(path: &PathBuf) -> Result<gix::Repository, anyhow::Error> {
    let options = Options::isolated();
    gix::ThreadSafeRepository::open_opts(path, options)
        .map(|repo| repo.to_thread_local())
        .map_err(|e| anyhow::anyhow!("Failed to open repository at {:?}: {}", path, e))
}

/// Get the current repository. The returned repository is isolated (has very limited access to the working tree and environment).
pub(crate) fn get_current_repository() -> Result<gix::Repository, anyhow::Error> {
    let options = Options::isolated();
    Ok(gix::ThreadSafeRepository::open_opts(".", options)?.to_thread_local())
}

/// Gets the current working directory
pub(crate) fn get_cwd() -> Result<PathBuf, anyhow::Error> {
    std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get current working directory: {}", e))
}

/// Get a thread-local repository from the given repository.
pub(crate) fn get_thread_local_repo(
    repo: &gix::Repository,
) -> Result<gix::Repository, anyhow::Error> {
    // Get a full access repository from the given repository
    let safe_repo = repo.to_owned().into_sync().to_thread_local();
    Ok(safe_repo)
}

/// Get the main, or superproject, repository.
pub(crate) fn get_main_repo(
    repo: Option<&gix::Repository>,
) -> Result<gix::Repository, anyhow::Error> {
    let repo = match repo {
        Some(r) => r.to_owned(),
        None => get_current_repository()?,
    };
    let super_repo = repo
        .main_repo()
        .map_err(|e| anyhow::anyhow!("Failed to get main repository: {}", e))?;
    Ok(super_repo)
}

/// Get the main repository's root directory.
pub(crate) fn get_main_root(repo: Option<&gix::Repository>) -> Result<PathBuf, anyhow::Error> {
    let repo = get_main_repo(repo)?;
    let path = repo.path().to_path_buf();
    if path.is_dir() {
        Ok(path)
    } else {
        Err(anyhow::anyhow!(
            "Failed to get main repository root: {}",
            path.display()
        ))
    }
}

/// Get the current branch name from the repository.
pub(crate) fn get_current_branch(repo: Option<&gix::Repository>) -> Result<String, anyhow::Error> {
    fn branch_from_repo(repo: &gix::Repository) -> Result<String, anyhow::Error> {
        let head = repo.head()?;
        if let Some(reference) = head.referent_name() {
            return Ok(reference.as_bstr().to_string());
        }
        Err(anyhow::anyhow!("Failed to get current branch name"))
    }
    match repo {
        Some(r) => branch_from_repo(r),
        None => {
            let owned = get_current_repository()?;
            branch_from_repo(&owned)
        }
    }
}

/*=========================================================================
 *                           General Utilities
 *========================================================================*/

/// Get the current working directory.
pub(crate) fn get_current_working_directory() -> Result<PathBuf, anyhow::Error> {
    std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get current working directory: {}", e))
}

/// Convert a `Path` to a `String`, returning an error if the path is not valid UTF-8
pub(crate) fn path_to_string(path: &std::path::Path) -> Result<String, anyhow::Error> {
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("Path is not valid UTF-8"))
}

/// Convert a `Path` to a `String`, using lossy conversion for non-UTF-8 characters
pub(crate) fn path_to_string_lossy(path: &std::path::Path) -> String {
    let lossy = path.to_string_lossy();
    eprintln!(
        "Warning: Path contains non-UTF-8 characters, using lossy conversion: {}",
        lossy
    );
    lossy.to_string()
}

/// Convert a `Path` to an `OsString`
pub(crate) fn path_to_os_string(path: &std::path::Path) -> std::ffi::OsString {
    path.as_os_str().to_owned()
}

/// Set the path from an OS string, converting to UTF-8 if possible
/// Used for CLI arguments and other scenarios where the path may not be valid UTF-8
pub(crate) fn set_path(path: std::ffi::OsString) -> Result<String, anyhow::Error> {
    match path.to_str() {
        Some(path) => Ok(path.to_string()),
        None => {
            Ok(path_to_string_lossy(&PathBuf::from(path))) // Use lossy conversion if the path is not valid UTF-8
        }
    }
}

/// Extract the name from a URL, trimming trailing slashes and `.git` suffix
pub(crate) fn name_from_url(url: &str) -> Result<String, anyhow::Error> {
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
pub(crate) fn name_from_osstring(os_string: std::ffi::OsString) -> Result<String, anyhow::Error> {
    osstring_to_string(os_string).and_then(|s| {
        if s.contains('\0') {
            return Err(anyhow::anyhow!("Name cannot contain null bytes"));
        }
        if s.trim().is_empty() {
            return Err(anyhow::anyhow!("Name cannot be empty or whitespace-only"));
        }
        let sep = std::path::MAIN_SEPARATOR.to_string();
        s.trim()
            .split(&sep)
            .last()
            .map(|name| name.to_string())
            .ok_or_else(|| anyhow::anyhow!("Failed to extract name from OsString"))
    })
}

/// Convert an `OsString` to a `String`, returning an error if the conversion fails
pub(crate) fn osstring_to_string(os_string: std::ffi::OsString) -> Result<String, anyhow::Error> {
    os_string
        .into_string()
        .map_err(|_| anyhow::anyhow!("Failed to convert OsString to String"))
}

/// Validate and return the sparse paths, ensuring they do not contain null bytes
pub(crate) fn get_sparse_paths(
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
        }
        None => None,
    };
    Ok(sparse_paths_vec)
}

/// Get the name from either a provided name, URL, or path.
pub(crate) fn get_name(
    name: Option<String>,
    url: Option<String>,
    path: Option<std::ffi::OsString>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_name_valid_name() {
        assert_eq!(
            get_name(Some("my-repo".to_string()), None, None).unwrap(),
            "my-repo"
        );
        assert_eq!(
            get_name(Some("  spaced-repo  ".to_string()), None, None).unwrap(),
            "spaced-repo"
        );
    }

    #[test]
    fn test_get_name_empty_name_fallback_url() {
        assert_eq!(
            get_name(
                Some("   ".to_string()),
                Some("https://github.com/user/repo.git".to_string()),
                None
            )
            .unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_get_name_empty_name_fallback_path() {
        assert_eq!(
            get_name(
                Some("".to_string()),
                None,
                Some(PathBuf::from_iter(["path", "to", "my-module"]).into_os_string())
            )
            .unwrap(),
            "my-module"
        );
    }

    #[test]
    fn test_get_name_empty_name_no_fallback() {
        assert!(get_name(Some("  ".to_string()), None, None).is_err());
    }

    #[test]
    fn test_get_name_no_name_valid_path() {
        assert_eq!(
            get_name(None, None, Some(std::ffi::OsString::from("another-path"))).unwrap(),
            "another-path"
        );
    }

    #[test]
    fn test_get_name_no_name_no_path_valid_url() {
        assert_eq!(
            get_name(
                None,
                Some("git@github.com:user/another-repo.git".to_string()),
                None
            )
            .unwrap(),
            "another-repo"
        );
    }

    #[test]
    fn test_get_name_all_none() {
        assert!(get_name(None, None, None).is_err());
    }

    // ================================================================
    // name_from_url edge cases
    // ================================================================

    #[test]
    fn test_name_from_url_standard() {
        assert_eq!(
            name_from_url("https://github.com/user/repo.git").unwrap(),
            "repo"
        );
        assert_eq!(
            name_from_url("https://github.com/user/repo").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_name_from_url_trailing_slashes() {
        assert_eq!(
            name_from_url("https://github.com/user/repo/").unwrap(),
            "repo"
        );
        assert_eq!(
            name_from_url("https://github.com/user/repo///").unwrap(),
            "repo"
        );
    }

    #[test]
    fn test_name_from_url_ssh_format() {
        assert_eq!(
            name_from_url("git@github.com:user/mylib.git").unwrap(),
            "mylib"
        );
    }

    #[test]
    fn test_name_from_url_file_url() {
        assert_eq!(name_from_url("file:///path/to/repo.git").unwrap(), "repo");
    }

    #[test]
    fn test_name_from_url_simple_name() {
        assert_eq!(name_from_url("repo").unwrap(), "repo");
        assert_eq!(name_from_url("my-lib.git").unwrap(), "my-lib");
    }

    #[test]
    fn test_name_from_url_empty() {
        assert!(name_from_url("").is_err());
    }

    // ================================================================
    // name_from_osstring
    // ================================================================

    #[test]
    fn test_name_from_osstring_simple() {
        assert_eq!(
            name_from_osstring(std::ffi::OsString::from("my-repo")).unwrap(),
            "my-repo"
        );
    }

    #[test]
    fn test_name_from_osstring_path() {
        let path = PathBuf::from_iter(["path", "to", "module"]);
        assert_eq!(name_from_osstring(path.into_os_string()).unwrap(), "module");
    }

    #[test]
    fn test_name_from_osstring_empty() {
        assert!(name_from_osstring(std::ffi::OsString::from("")).is_err());
        assert!(name_from_osstring(std::ffi::OsString::from("  ")).is_err());
    }

    #[test]
    fn test_name_from_osstring_null_byte() {
        assert!(name_from_osstring(std::ffi::OsString::from("foo\0bar")).is_err());
    }

    // ================================================================
    // osstring_to_string
    // ================================================================

    #[test]
    fn test_osstring_to_string_valid() {
        assert_eq!(
            osstring_to_string(std::ffi::OsString::from("hello")).unwrap(),
            "hello"
        );
    }

    // ================================================================
    // path_to_string
    // ================================================================

    #[test]
    fn test_path_to_string_valid() {
        let path = std::path::Path::new("/home/user/repo");
        assert_eq!(path_to_string(path).unwrap(), "/home/user/repo");
    }

    // ================================================================
    // path_to_os_string
    // ================================================================

    #[test]
    fn test_path_to_os_string_roundtrip() {
        let path = std::path::Path::new("/some/path");
        let os = path_to_os_string(path);
        assert_eq!(os, std::ffi::OsString::from("/some/path"));
    }

    // ================================================================
    // set_path
    // ================================================================

    #[test]
    fn test_set_path_valid_utf8() {
        let os = std::ffi::OsString::from("/valid/path");
        assert_eq!(set_path(os).unwrap(), "/valid/path");
    }

    // ================================================================
    // get_sparse_paths
    // ================================================================

    #[test]
    fn test_get_sparse_paths_valid() {
        let paths = Some(vec!["src/".to_string(), "docs/".to_string()]);
        let result = get_sparse_paths(paths).unwrap();
        assert_eq!(result, Some(vec!["src/".to_string(), "docs/".to_string()]));
    }

    #[test]
    fn test_get_sparse_paths_none() {
        let result = get_sparse_paths(None).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_sparse_paths_null_byte() {
        let paths = Some(vec!["src/\0bad".to_string()]);
        assert!(get_sparse_paths(paths).is_err());
    }

    #[test]
    fn test_get_sparse_paths_empty_vec() {
        let result = get_sparse_paths(Some(vec![])).unwrap();
        assert_eq!(result, Some(vec![]));
    }
}
