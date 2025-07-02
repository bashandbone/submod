// SPDX-LicenseIdentifier: MIT OR Apache-2.0
//
// SPDX-FileCopyrightText: 2018-2025 Sebastian Thiel and [contributors](https://github.com/byron/gitoxide/contributors)
// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>

//! A series of functions that mirror gix cli functionality. Sometimes it's just easier to copy what's already there.
//!
//! This module is adapted and simplified from the `gix` CLI (https://github.com/GitoxideLabs/gitoxide/tree/main/src/) and its supporting `gitoxide-core` crate.

use bstr::BString;
use anyhow::Result;
use gix::{features::progress, progress::prodash};
use std::io::{stdout, stderr};
use gitoxide_core::repository::{clean::{Options as CleanOptions}, clone::{PROGRESS_RANGE as CloneProgress, Options as CloneOptions}, fetch::{Options as FetchOptions, PROGRESS_RANGE as FetchProgressRange}, submodule::{list, }, status::{Options as StatusOptions, Submodules}};
use prodash::render::line;

/// A standard range for line renderer.
pub fn setup_line_renderer_range(
    progress: &std::sync::Arc<prodash::tree::Root>,
    levels: std::ops::RangeInclusive<prodash::progress::key::Level>,
) -> line::JoinHandle {
    prodash::render::line(
        std::io::stderr(),
        std::sync::Arc::downgrade(progress),
        prodash::render::line::Options {
            level_filter: Some(levels),
            frames_per_second: 6.0,
            initial_delay: Some(std::time::Duration::from_millis(1000)),
            timestamp: true,
            throughput: true,
            hide_cursor: true,
            ..prodash::render::line::Options::default()
        }
        .auto_configure(prodash::render::line::StreamKind::Stderr),
    )
}

/// Get a progress tree for use with prodash.
pub fn progress_tree(trace: bool) -> std::sync::Arc<prodash::tree::Root> {
    prodash::tree::root::Options {
        message_buffer_capacity: if trace { 10_000 } else { 200 },
        ..Default::default()
    }
    .into()
}

/// Run a function with progress tracking, capturing output to stdout and stderr.
pub fn get_progress(func_name: &str, range: Option<std::ops::RangeInclusive<u8>>, run: impl FnOnce(
    progress::DoOrDiscard<prodash::tree::Item>,
    &mut dyn std::io::Write,
    &mut dyn std::io::Write,
)) -> Result<()> {
    let standard_range = 2..=2;
    let range = range.unwrap_or_else(|| standard_range.clone());
    let progress = progress_tree(false);
    let sub_progress = progress.add_child(func_name);

    let handle = setup_line_renderer_range(&progress, range);

    let mut out = Vec::<u8>::new();
    let mut err = Vec::<u8>::new();

    let _res = gix::trace::coarse!("run")
        .into_scope(|| run(progress::DoOrDiscard::from(Some(sub_progress)), &mut out, &mut err));

    handle.shutdown_and_wait();
    std::io::Write::write_all(&mut stdout(), &out)?;
    std::io::Write::write_all(&mut stderr(), &err)?;
    Ok(())
}

/// Set options for the `clean` command.
///
/// Since we use this as part of our intentionally destructive commands, we can be more aggressive with defaults.
fn clean_options() -> CleanOptions {
    CleanOptions {
        debug: false,
        format: gitoxide_core::OutputFormat::Human,
        execute: true,
        ignored: false,
        pathspec_matches_result: false,
        precious: true,
        directories: true,
        repositories: true,
        skip_hidden_repositories: None,
        find_untracked_repositories: gitoxide_core::repository::clean::FindRepository::All,
    }
}

pub fn harsh_clean(repo: gix::Repository, patterns: Vec<BString>) -> Result<()> {
    gitoxide_core::repository::clean(repo, &mut stdout().lock(), &mut stderr().lock(), patterns, clean_options())
}

fn status_options() -> StatusOptions {
    StatusOptions {
        ignored: None,
        format: gitoxide_core::repository::status::Format::Simplified,
        output_format: gitoxide_core::OutputFormat::Human,
        submodules: Some(Submodules::All),
        thread_limit: None,
        statistics: false,
        allow_write: false,
        index_worktree_renames: None,
    }
}

/// Get the status of the repository, optionally filtering by patterns.
pub fn get_status(
    repo: gix::Repository,
    patterns: Vec<BString>,
) -> Result<()> {
    get_progress("status", None, |progress, out, err| {
        let _ = gitoxide_core::repository::status::show(repo, patterns, out, err, progress, status_options());
    })
}

/// Fetch options for the `fetch` command, with an option for shallow fetching.
fn fetch_options(remote: Option<String>, shallow: bool) -> FetchOptions {
    let shallow = if shallow {
        gix::remote::fetch::Shallow::DepthAtRemote(std::num::NonZeroU32::new(1).unwrap())
    } else {
        gix::remote::fetch::Shallow::NoChange
    };
    FetchOptions {
        format: gitoxide_core::OutputFormat::Human,
        dry_run: false,
        remote: remote,
        ref_specs: Vec::new(),
        shallow: shallow,
        handshake_info: false,
        negotiation_info: false,
        open_negotiation_graph: None,
    }
}

/// Fetch updates from a remote repository.
pub fn fetch_repo(
    repo: gix::Repository,
    remote: Option<String>,
    shallow: bool,
) -> Result<()> {
    get_progress("fetch", Some(FetchProgressRange), |progress, out, err| {
        if let Err(e) = gitoxide_core::repository::fetch(
            repo,
            progress,
            out,
            err,
            fetch_options(remote, shallow),
        ) {
            // Optionally print error to stderr directly
            eprintln!("Fetch failed: {:?}", e);
        }
    })
}

/// List all submodules in the repository.
pub fn list_submodules(
    repo: gix::Repository
) -> Result<()> {
    list(repo, &mut stdout().lock(), gitoxide_core::OutputFormat::Human, None)
}

/// List all submodules in the repository and their status.
pub fn list_submodules_with_status(
    repo: gix::Repository,
) -> Result<()> {
    let submodules = repo.submodules()?;
    if let Some(submodules) = submodules {
        submodules.into_iter().try_for_each(|sm| {
            let sm_repo = sm.open()?;
            if let Some(repo) = sm_repo {
                get_status(repo, vec![])?;
            }
            Ok(())
        })
    } else {
        Ok(())
    }
}

/// Clone options for the `clone` command, with an option for shallow cloning.
fn clone_options(shallow: bool) -> CloneOptions {
    let shallow = if shallow {
        gix::remote::fetch::Shallow::DepthAtRemote(std::num::NonZeroU32::new(1).unwrap())
    } else {
        gix::remote::fetch::Shallow::NoChange
    };
    CloneOptions {
        format: gitoxide_core::OutputFormat::Human,
        bare: false,
        handshake_info: false,
        no_tags: false,
        shallow: shallow,
        ref_name: None,
    }
}

/// Clone a repository from a given URL to a specified path, with an option for shallow cloning.
pub fn clone_repo(
    url: &str,
    path: Option<&str>,
    shallow: bool,
) {
    let path = path.map_or_else(
        || std::path::PathBuf::from(
            std::path::Path::new(url)
                .file_name()
                .unwrap_or_else(|| std::ffi::OsStr::new("repo"))
        ),
        |p| p.into(),
    );
    let osstr_url = std::ffi::OsStr::new(url);
    get_progress("clone", Some(CloneProgress), |progress, out, err| {
        let _ = gitoxide_core::repository::clone(
            osstr_url,
            Some(path),
            Vec::<BString>::new(), // No overrides for now
            progress,
            out,
            err,
            clone_options(shallow),
        );
    });
}
