// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
//! Utility functions for working with `Gitoxide` APIs commonly used across the codebase.
#![allow(dead_code)]

use anyhow::{Context as _, Result};
use git2::Repository as Git2Repository;
use gix::open::Options;
use std::path::{Component, Path, PathBuf};

/// Render untrusted human text without terminal controls or URL credentials.
///
/// This is a display boundary only: never use it on Git records, stored values,
/// command arguments, or generated completion scripts. Compose intentional line
/// separators after sanitizing fields; embedded newlines and tabs are escaped.
#[must_use]
pub fn safe_human_text(input: &str) -> String {
    fn looks_like_host_port(authority: &str) -> bool {
        let Some((host, port)) = authority.rsplit_once(':') else {
            return false;
        };
        !host.is_empty()
            && (host == "localhost"
                || host.contains('.')
                || (host.starts_with('[') && host.ends_with(']'))
                || host.parse::<std::net::IpAddr>().is_ok())
            && port.parse::<u16>().is_ok_and(|port| port != 0)
    }

    /// Locate the next URL authority: `scheme://`, or a scheme-less `//`
    /// reference such as the one Git prints when it echoes a file URL
    /// without its scheme. A `//` continuing a scheme (`://`) belongs to
    /// that scheme and is skipped here.
    fn next_authority_marker(text: &str) -> Option<(usize, bool)> {
        let scheme = text.find("://");
        let mut bare_search = 0;
        loop {
            let Some(bare) = text[bare_search..]
                .find("//")
                .map(|index| bare_search + index)
            else {
                return scheme.map(|marker| (marker, true));
            };
            if bare == 0 || text.as_bytes()[bare - 1] != b':' {
                return match scheme {
                    Some(marker) if marker < bare => Some((marker, true)),
                    _ => Some((bare, false)),
                };
            }
            bare_search = bare + 2;
        }
    }

    let mut redacted = String::with_capacity(input.len());
    let mut remaining = input;
    while let Some((marker, scheme_present)) = next_authority_marker(remaining) {
        let authority_start = marker + if scheme_present { 3 } else { 2 };
        redacted.push_str(&remaining[..authority_start]);
        remaining = &remaining[authority_start..];
        let mut authority_end = remaining
            .find(|c: char| c.is_whitespace() || matches!(c, '/' | '?' | '#' | '"' | '<' | '>'))
            .unwrap_or(remaining.len());
        // Adjacent URLs in diagnostic prose may have no separating whitespace.
        if let Some(next_marker) = remaining.find("://") {
            let scheme_start = remaining[..next_marker]
                .char_indices()
                .rev()
                .find(|(_, c)| !c.is_ascii_alphanumeric() && !matches!(c, '+' | '-' | '.'))
                .map_or(0, |(index, c)| index + c.len_utf8());
            authority_end = authority_end.min(scheme_start);
        }
        let ordinary_authority_end = authority_end;
        let ordinary_authority = &remaining[..ordinary_authority_end];
        let malformed_authority_delimiter = remaining[authority_end..]
            .chars()
            .next()
            .is_some_and(|c| c.is_control() || matches!(c, '"' | '<' | '>'));
        // A malformed but accepted display value can put a control, quote, or angle
        // bracket inside password text. Look past that delimiter only when the
        // ordinary authority already contains the user/password separator. This
        // avoids treating a later email address in diagnostic prose as URL userinfo.
        if !ordinary_authority.contains('@')
            && ordinary_authority.contains(':')
            && (!looks_like_host_port(ordinary_authority) || malformed_authority_delimiter)
        {
            let extended_end = remaining
                .find(['/', '?', '#', ',', ')', ']'])
                .unwrap_or(remaining.len());
            if remaining[..extended_end].contains('@') {
                authority_end = extended_end;
            }
        }
        let authority = &remaining[..authority_end];
        if let Some(at) = authority.rfind('@') {
            redacted.push_str("[redacted]@");
            redacted.push_str(&authority[at + 1..]);
        } else {
            redacted.push_str(&remaining[..authority_end]);
        }
        remaining = &remaining[authority_end..];
    }
    redacted.push_str(remaining);
    let mut safe = String::with_capacity(redacted.len());
    for c in redacted.chars() {
        if c.is_control() {
            safe.extend(c.escape_default());
        } else {
            safe.push(c);
        }
    }
    safe
}

/// Repository locations resolved by native Git, including linked worktrees.
#[derive(Debug, Clone)]
pub struct RepositoryContext {
    /// Canonical directory from which the command was invoked.
    pub invocation_dir: PathBuf,
    /// Canonical worktree root used for all module checkout paths.
    pub worktree_root: PathBuf,
    /// Worktree-specific Git metadata directory.
    pub git_dir: PathBuf,
    /// Shared Git metadata directory, also shared by linked worktrees.
    pub common_dir: PathBuf,
    /// Default root config or explicit invocation-relative config.
    pub config_path: PathBuf,
}

impl RepositoryContext {
    /// Discover a worktree and resolve its config; an explicit config must exist.
    pub fn discover(invocation_dir: &Path, explicit_config: Option<&Path>) -> Result<Self> {
        let invocation_dir = invocation_dir.canonicalize()?;
        let worktree_root = git_path(&invocation_dir, &["--show-toplevel"]).map_err(|error| {
            anyhow::anyhow!("A non-bare repository worktree is required: {error}")
        })?;
        // Git spells the reported top-level per platform (forward slashes on
        // Windows), so canonicalize it exactly like the invocation directory:
        // every checkout comparison below assumes a canonical root.
        let worktree_root = worktree_root.canonicalize().with_context(|| {
            format!(
                "Discovered worktree root is missing: {}",
                worktree_root.display()
            )
        })?;
        // Same platform spelling as the worktree root above: canonicalize
        // both Git metadata directories so identity comparisons hold.
        let git_dir = git_path(&invocation_dir, &["--absolute-git-dir"])?;
        let git_dir = git_dir.canonicalize().with_context(|| {
            format!("Discovered Git directory is missing: {}", git_dir.display())
        })?;
        let common_dir = git_path(
            &invocation_dir,
            &["--path-format=absolute", "--git-common-dir"],
        )?;
        let common_dir = common_dir.canonicalize().with_context(|| {
            format!(
                "Discovered common Git directory is missing: {}",
                common_dir.display()
            )
        })?;
        let config_path = match explicit_config {
            Some(path) => {
                let path = invocation_dir.join(path);
                match std::fs::symlink_metadata(&path) {
                    Ok(metadata) if !metadata.is_file() && !metadata.file_type().is_symlink() => {
                        anyhow::bail!("Explicit config path is not a file: {}", path.display());
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        anyhow::bail!("Explicit config file not found: {}", path.display())
                    }
                    Err(error) => return Err(error.into()),
                }
                path
            }
            None => worktree_root.join("submod.toml"),
        };
        Ok(Self {
            invocation_dir,
            worktree_root,
            git_dir,
            common_dir,
            config_path,
        })
    }
}

pub(crate) fn git_path(directory: &Path, args: &[&str]) -> Result<PathBuf> {
    let output = std::process::Command::new("git")
        .arg("--no-optional-locks")
        .arg("rev-parse")
        .args(args)
        .current_dir(directory)
        .output()?;
    if !output.status.success() {
        anyhow::bail!(
            "git rev-parse {} failed in {}: {}",
            args.join(" "),
            directory.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    git_path_from_stdout(output.stdout)
}

/// Decode a single path printed by Git without losing Unix path bytes.
pub(crate) fn git_path_from_stdout(mut bytes: Vec<u8>) -> Result<PathBuf> {
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
    }
    #[cfg(unix)]
    let path = {
        use std::os::unix::ffi::OsStringExt;
        PathBuf::from(std::ffi::OsString::from_vec(bytes))
    };
    #[cfg(not(unix))]
    let path = PathBuf::from(String::from_utf8(bytes)?);
    if !path.is_absolute() {
        anyhow::bail!(
            "Git returned a non-absolute repository path: {}",
            path.display()
        );
    }
    Ok(path)
}

/// Get the current repository using git2, with an optional provided repository. If no repository is provided, it will attempt to discover one in the current directory.
pub fn get_current_git2_repository(
    repo: Option<Git2Repository>,
) -> Result<Git2Repository, anyhow::Error> {
    if let Some(r) = repo {
        Ok(r)
    } else {
        let rep = Git2Repository::discover(".")
            .map_err(|e| anyhow::anyhow!("Failed to discover repository: {e}"))?;
        if rep.is_bare() {
            return Err(anyhow::anyhow!("Bare repositories are not supported"));
        }
        Ok(rep)
    }
}

/*=========================================================================
 *                           Gix Utilities
 *========================================================================*/

/// Get a repository from a given path. The returned repository is isolated (has very limited access to the working tree and environment).
pub fn repo_from_path(path: &PathBuf) -> Result<gix::Repository, anyhow::Error> {
    let options = Options::isolated();
    gix::ThreadSafeRepository::open_opts(path, options)
        .map(|repo| repo.to_thread_local())
        .map_err(|e| anyhow::anyhow!("Failed to open repository at {}: {e}", path.display()))
}

/// Get the current repository. The returned repository is isolated (has very limited access to the working tree and environment).
pub fn get_current_repository() -> Result<gix::Repository, anyhow::Error> {
    let options = Options::isolated();
    Ok(gix::ThreadSafeRepository::open_opts(".", options)?.to_thread_local())
}

/// Gets the current working directory
pub fn get_cwd() -> Result<PathBuf, anyhow::Error> {
    std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get current working directory: {e}"))
}

/// Get a thread-local repository from the given repository.
#[must_use]
pub fn get_thread_local_repo(repo: &gix::Repository) -> gix::Repository {
    // Get a full access repository from the given repository
    repo.to_owned().into_sync().to_thread_local()
}

/// Get the main, or superproject, repository.
pub fn get_main_repo(repo: Option<&gix::Repository>) -> Result<gix::Repository, anyhow::Error> {
    let repo = match repo {
        Some(r) => r.to_owned(),
        None => get_current_repository()?,
    };
    let super_repo = repo
        .main_repo()
        .map_err(|e| anyhow::anyhow!("Failed to get main repository: {e}"))?;
    Ok(super_repo)
}

/// Get the main repository's root directory.
pub fn get_main_root(repo: Option<&gix::Repository>) -> Result<PathBuf, anyhow::Error> {
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
pub fn get_current_branch(repo: Option<&gix::Repository>) -> Result<String, anyhow::Error> {
    fn branch_from_repo(repo: &gix::Repository) -> Result<String, anyhow::Error> {
        let head = repo.head()?;
        if let Some(reference) = head.referent_name() {
            return Ok(reference.as_bstr().to_string());
        }
        Err(anyhow::anyhow!("Failed to get current branch name"))
    }
    if let Some(r) = repo {
        branch_from_repo(r)
    } else {
        let owned = get_current_repository()?;
        branch_from_repo(&owned)
    }
}

/*=========================================================================
 *                           General Utilities
 *========================================================================*/

/// Get the current working directory.
pub fn get_current_working_directory() -> Result<PathBuf, anyhow::Error> {
    std::env::current_dir()
        .map_err(|e| anyhow::anyhow!("Failed to get current working directory: {e}"))
}

/// Convert a `Path` to a `String`, returning an error if the path is not valid UTF-8
pub fn path_to_string(path: &std::path::Path) -> Result<String, anyhow::Error> {
    path.to_str()
        .map(std::string::ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("Path is not valid UTF-8"))
}

/// Convert a `Path` to a `String`, using lossy conversion for non-UTF-8 characters
#[must_use]
pub fn path_to_string_lossy(path: &std::path::Path) -> String {
    if let Some(s) = path.to_str() {
        return s.to_string();
    }
    let lossy = path.to_string_lossy();
    eprintln!(
        "Warning: Path contains non-UTF-8 characters, using lossy conversion: {}",
        safe_human_text(&lossy)
    );
    lossy.to_string()
}

/// Convert a `Path` to an `OsString`
#[must_use]
pub fn path_to_os_string(path: &std::path::Path) -> std::ffi::OsString {
    path.as_os_str().to_owned()
}

/// Set the path from an OS string, converting to UTF-8 if possible
/// Used for CLI arguments and other scenarios where the path may not be valid UTF-8
#[must_use]
#[allow(clippy::option_if_let_else)]
pub fn set_path(path: std::ffi::OsString) -> String {
    if let Some(s) = path.to_str() {
        s.to_string()
    } else {
        path_to_string_lossy(&PathBuf::from(path))
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
        .next_back()
        .map(std::string::ToString::to_string)
        .ok_or_else(|| anyhow::anyhow!("Failed to extract name from URL"))
}

/// Convert an `OsString` to a `String`, extracting the name from the path
pub fn name_from_osstring(os_string: std::ffi::OsString) -> Result<String, anyhow::Error> {
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
            .map(std::string::ToString::to_string)
            .ok_or_else(|| anyhow::anyhow!("Failed to extract name from OsString"))
    })
}

/// Convert an `OsString` to a `String`, returning an error if the conversion fails
pub fn osstring_to_string(os_string: std::ffi::OsString) -> Result<String, anyhow::Error> {
    os_string
        .into_string()
        .map_err(|_| anyhow::anyhow!("Failed to convert OsString to String"))
}

/// Validate and return the sparse paths, ensuring they do not contain null bytes
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
        }
        None => None,
    };
    Ok(sparse_paths_vec)
}

/// Get the name from either a provided name, URL, or path.
pub fn get_name(
    name: Option<String>,
    url: Option<String>,
    path: Option<std::ffi::OsString>,
) -> Result<String, anyhow::Error> {
    if let Some(name) = name {
        let trimmed_name = name.trim().to_string();
        if trimmed_name.is_empty() {
            get_name(None, url, path)
        } else {
            Ok(trimmed_name)
        }
    } else if let Some(path) = path {
        name_from_osstring(path)
    } else if let Some(url) = url {
        name_from_url(&url)
    } else {
        Err(anyhow::anyhow!("No valid name source provided"))
    }
}

/// Normalize harmless dots while rejecting root and administrative destinations.
/// Whether an I/O failure means "nothing is there" for an existence probe.
///
/// `NotFound` is the portable answer. Names the OS refuses to state at all
/// (Windows rejects control characters such as `\n` and `"` with
/// `InvalidFilename`) likewise hold nothing to preserve, so probes treat
/// them as absent and let the later mutating call report the OS truth.
#[must_use]
pub fn is_absent_path(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::NotFound | std::io::ErrorKind::InvalidFilename
    )
}

pub fn normalize_submodule_path(path: &Path) -> Result<PathBuf> {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(name) => {
                let git_alias = name.to_str().is_some_and(|name| {
                    name.trim_end_matches(['.', ' '])
                        .eq_ignore_ascii_case(".git")
                });
                if name.as_encoded_bytes().eq_ignore_ascii_case(b".git") || git_alias {
                    anyhow::bail!("Submodule path cannot contain Git administrative components");
                }
                normalized.push(name);
            }
            Component::CurDir => {}
            Component::ParentDir => anyhow::bail!("Submodule path cannot contain '..' components"),
            Component::RootDir | Component::Prefix(_) => {
                anyhow::bail!("Submodule path cannot contain root or prefix components")
            }
        }
    }
    if normalized.as_os_str().is_empty() {
        anyhow::bail!("Submodule path must be a nonempty strict descendant of the repository root");
    }
    Ok(normalized)
}

/// Validate a mutation destination without following any symlink components.
pub fn validate_submodule_path(repo_root: &Path, path: &Path) -> Result<()> {
    let normalized = normalize_submodule_path(path)?;
    let mut current = repo_root
        .canonicalize()
        .with_context(|| format!("Could not inspect repository root {}", repo_root.display()))?;
    for component in normalized.components() {
        current.push(component);
        match std::fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                anyhow::bail!("Submodule path contains a symlink: {}", current.display());
            }
            Ok(_) => {}
            Err(error) if is_absent_path(&error) => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("Could not inspect submodule path {}", current.display())
                });
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn absent_path_covers_missing_and_unstatable_names() {
        use std::io::{Error, ErrorKind};
        assert!(is_absent_path(&Error::new(ErrorKind::NotFound, "gone")));
        assert!(is_absent_path(&Error::new(
            ErrorKind::InvalidFilename,
            "unstatable"
        )));
        assert!(!is_absent_path(&Error::new(
            ErrorKind::PermissionDenied,
            "denied"
        )));
    }

    fn checked_git(directory: &Path, args: &[&str]) {
        let output = std::process::Command::new("git")
            .args(["-c", "commit.gpgsign=false"])
            .args(args)
            .current_dir(directory)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn repository_context_nested_and_linked_worktrees() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        checked_git(&root, &["init", "main"]);
        let main = root.join("main");
        checked_git(
            &main,
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--allow-empty",
                "-m",
                "initial",
            ],
        );
        checked_git(&main, &["worktree", "add", "-b", "linked", "../linked"]);
        for name in ["main", "linked"] {
            let worktree = root.join(name);
            let nested = worktree.join("nested/deeper");
            std::fs::create_dir_all(&nested).unwrap();
            let context = RepositoryContext::discover(&nested, None).unwrap();
            assert_eq!(context.invocation_dir, nested);
            assert_eq!(context.worktree_root, worktree);
            assert_eq!(context.common_dir, main.join(".git"));
            assert_eq!(
                context.git_dir,
                if name == "main" {
                    main.join(".git")
                } else {
                    main.join(".git/worktrees/linked")
                }
            );
            assert_eq!(context.config_path, worktree.join("submod.toml"));
            std::fs::write(nested.join("custom.toml"), "").unwrap();
            let explicit =
                RepositoryContext::discover(&nested, Some(Path::new("custom.toml"))).unwrap();
            assert_eq!(explicit.config_path, nested.join("custom.toml"));
            assert!(RepositoryContext::discover(&nested, Some(Path::new("missing.toml"))).is_err());
        }
        assert!(RepositoryContext::discover(&root, None).is_err());
        checked_git(&root, &["init", "--bare", "bare"]);
        let error = RepositoryContext::discover(&root.join("bare"), None).unwrap_err();
        assert!(error.to_string().contains("non-bare repository worktree"));
    }

    #[test]
    #[cfg(unix)]
    fn repository_context_preserves_newline_paths() {
        use std::os::unix::ffi::OsStringExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp
            .path()
            .canonicalize()
            .unwrap()
            .join(std::ffi::OsString::from_vec(b"repo-\n".to_vec()));
        std::fs::create_dir(&root).unwrap();
        checked_git(&root, &["init"]);
        let context = RepositoryContext::discover(&root, None).unwrap();
        assert_eq!(context.worktree_root, root);
        assert_eq!(context.git_dir, root.join(".git"));
        assert_eq!(context.common_dir, root.join(".git"));
        let child = PathBuf::from(std::ffi::OsString::from_vec(b"child-\xff".to_vec()));
        assert_eq!(normalize_submodule_path(&child).unwrap(), child);
        assert_eq!(
            normalize_submodule_path(Path::new("./vendor/./child")).unwrap(),
            Path::new("vendor/child")
        );
    }

    #[test]
    fn mutation_paths_reject_root_admin_and_parent_components() {
        let root = tempfile::tempdir().unwrap();
        for path in [
            "",
            ".",
            "././",
            ".git",
            "vendor/.GiT/objects",
            "vendor/../child",
            "../child",
        ] {
            assert!(
                validate_submodule_path(root.path(), std::path::Path::new(path)).is_err(),
                "accepted {path:?}"
            );
        }
        assert!(validate_submodule_path(root.path(), &root.path().join("child")).is_err());
        assert!(
            validate_submodule_path(root.path(), std::path::Path::new("./vendor/./new-child"))
                .is_ok()
        );
    }

    #[test]
    #[cfg(unix)]
    fn mutation_paths_reject_all_symlink_components() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("real")).unwrap();
        std::os::unix::fs::symlink("real", root.path().join("alias")).unwrap();
        std::os::unix::fs::symlink("missing", root.path().join("dangling")).unwrap();
        for path in ["alias", "alias/child", "dangling/child"] {
            assert!(
                validate_submodule_path(root.path(), std::path::Path::new(path)).is_err(),
                "accepted {path}"
            );
        }
    }

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
                Some(String::new()),
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
        assert_eq!(set_path(os), "/valid/path");
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

    #[test]
    fn test_path_to_string_lossy_valid() {
        let path = std::path::Path::new("valid_utf8");
        assert_eq!(path_to_string_lossy(path), "valid_utf8");
    }

    #[test]
    #[cfg(unix)]
    fn test_path_to_string_lossy_invalid() {
        use std::os::unix::ffi::OsStringExt;
        let bytes = vec![0x61, 0xFF, 0x62]; // 'a', invalid, 'b'
        let os_string = std::ffi::OsString::from_vec(bytes);
        let path = std::path::Path::new(&os_string);

        let result = path_to_string_lossy(path);
        assert_eq!(result, format!("a{}b", std::char::REPLACEMENT_CHARACTER));
    }
}

#[cfg(test)]
mod human_output_tests {
    use super::safe_human_text;

    #[test]
    fn redacts_all_url_userinfo_before_escaping_controls() {
        assert_eq!(
            safe_human_text(
                "clone 'https://USER_A:PASS_A@example.invalid/a', then (file://USER_B:P%40SS_B@localhost/b): failed\r\n"
            ),
            "clone 'https://[redacted]@example.invalid/a', then (file://[redacted]@localhost/b): failed\\r\\n"
        );
        assert_eq!(
            safe_human_text("https://u:p@host,https://x:y@other"),
            "https://[redacted]@host,https://[redacted]@other"
        );
        assert_eq!(
            safe_human_text("https://u:p'ass@host/éhttps://x:y@other"),
            "https://[redacted]@host/éhttps://[redacted]@other"
        );
        assert_eq!(
            safe_human_text("ssh://token@host/a https://u:p@ss@host/b"),
            "ssh://[redacted]@host/a https://[redacted]@host/b"
        );
        assert_eq!(
            safe_human_text("https://R26_FAKE_USER:PA\tSS@example.invalid/a"),
            "https://[redacted]@example.invalid/a"
        );
        assert_eq!(
            safe_human_text("https://R26_FAKE_USER:PA\"SS@example.invalid/a"),
            "https://[redacted]@example.invalid/a"
        );
        assert_eq!(
            safe_human_text("https://R26_FAKE_USER:PA<SS@example.invalid/a"),
            "https://[redacted]@example.invalid/a"
        );
        assert_eq!(
            safe_human_text("remote https://example.invalid failed for alice@example.org"),
            "remote https://example.invalid failed for alice@example.org"
        );
        assert_eq!(
            safe_human_text("remote https://example.invalid:443 failed for alice@example.org"),
            "remote https://example.invalid:443 failed for alice@example.org"
        );
        assert_eq!(
            safe_human_text("https://user.example:443\tPRIVATE@example.invalid/repo"),
            "https://[redacted]@example.invalid/repo"
        );
        assert_eq!(
            safe_human_text("bibliothèque/été\t\u{1b}[2J\u{8}\u{7f}\u{85}"),
            "bibliothèque/été\\t\\u{1b}[2J\\u{8}\\u{7f}\\u{85}"
        );
        assert_eq!(
            safe_human_text("https://example.invalid/path@example.invalid?q=yes"),
            "https://example.invalid/path@example.invalid?q=yes"
        );
        // Git echoes some file URLs without their scheme; the credentials
        // must still be redacted while plain UNC shares stay untouched.
        assert_eq!(
            safe_human_text(
                "fatal: '//R26_FAKE_USER:R26_FAKE_PASS@localhostC:/w/missing.git' nope"
            ),
            "fatal: '//[redacted]@localhostC:/w/missing.git' nope"
        );
        assert_eq!(
            safe_human_text("share //server/share/file and C:/win/path stay"),
            "share //server/share/file and C:/win/path stay"
        );
    }
}
