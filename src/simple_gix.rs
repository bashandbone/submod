//! A series of functions that mirror gix cli functionality. Sometimes it's just easier to copy what's already there.

use std::ops::Sub;

use gitoxide_core::repository::{clean::{self, FindRepository, Options as CleanOptions}, clone, clone::{PROGRESS_RANGE as CloneProgress, Options as CloneOptions}, fetch::{self, Options as FetchOptions, PROGRESS_RANGE as FetchProgressRange}, status::{self, show, Options as StatusOptions, Submodules}, submodule::list};
use gix::{config::overrides, pathspec, shallow};

fn get_out_and_err() -> (impl std::io::Write, impl std::io::Write) {
    let stdout = stdout();
    let stderr = stderr();
    let mut stdout_lock = stdout.lock();
    let mut stderr_lock = stderr.lock();
    (&mut stdout_lock, &mut stderr_lock)
}

/// Set options for the `clean` command.
///
/// Since we use this as part of our intentionally destructive commands, we can be more aggressive with defaults.
fn clean_options() {
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
        find_untracked_repositories: false,
    }
}

pub fn harsh_clean(repo: gix::Repository, patterns: Vec<BString>) -> gix::Result<()> {
    let (out, err) = get_out_and_err();
    clean(repo, out, err, patterns, clean_options())
}

fn status_options() -> StatusOptions {
    StatusOptions {
        ignored: None,
        format: None,
        output_format: gitoxide_core::OutputFormat::Human,
        submodules: Submodules::All,
        thread_limit: None,
        statistics: false,
        allow_write: false,
        index_worktree_renames: None,
    }
}

pub fn get_status(
    repo: gix::Repository,
    patterns: Vec<BString>,
) -> gix::Result<status::Output> {
    let &mut (out, err) = get_out_and_err();
    show(repo, patterns, &mut out, &mut err, None, status_options())
}


fn fetch_options(remote: Option<String>, shallow: bool) -> FetchOptions {
    let shallow = if shallow {
        gix::remote::fetch::Shallow::DepthAtRemote(1)
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

fn fetch_repo(
    repo: gix::Repository,
    remote: Option<String>,
    shallow: bool,
) -> gix::Result<fetch::Outcome> {
    let &mut (out, err) = get_out_and_err();
    fetch(repo, FetchProgressRange, out, err, fetch_options(remote, shallow))
}

pub fn list_submodules(
    repo: gix::Repository
) -> gix::Result<()> {
    let &mut (out, _) = get_out_and_err();
    list(repo, out, gitoxide_core::OutputFormat::Human, None)
}

pub fn list_submodules_with_status(
    repo: gix::Repository,
    dirty_suffix: Option<String>,
) -> gix::Result<()> {
    let &mut (out, _) = get_out_and_err();
    let submodules = repo.submodules()?;
    for sm in submodules {
        if let Some(sub_repo) = sm.open() {
            get_status(sub_repo, vec![])?;
        } else {
            eprintln!("Submodule {} is not a valid repository", sm.name());
        }
    }
}

fn clone_options(shallow: bool) {
    let shallow = if shallow {
        gix::remote::fetch::Shallow::DepthAtRemote(1)
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

pub fn clone_repo(
    url: &str,
    path: Option<&str>,
    shallow: bool,
) {
    let &mut (out, err) = get_out_and_err();
    let url: gix::Url = url.try_into().expect("Invalid URL format");
    let path = path.map_or_else(
        || gix::path::from_bstr(url.path.as_ref()).to_path_buf(),
        |p| p.into(),
    );
    clone(url, Some(path), vec![], CloneProgress, out, err, clone_options(shallow))
}
