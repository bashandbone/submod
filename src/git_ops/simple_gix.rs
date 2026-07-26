// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
//
// SPDX-FileCopyrightText: 2018-2025 Sebastian Thiel and [contributors](https://github.com/byron/gitoxide/contributors)
// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>

//! A series of functions that mirror gix cli functionality. Sometimes it's just easier to copy what's already there.
//!
//! This module is adapted and simplified from the `gix` CLI (<https://github.com/GitoxideLabs/gitoxide/tree/main/src>/) and its supporting `gitoxide-core` crate.

use anyhow::Result;
use gitoxide_core::repository::fetch::{
    Options as FetchOptions, PROGRESS_RANGE as FetchProgressRange,
};
use gix::{features::progress, progress::prodash};
use prodash::render::line;
use std::io::stderr;

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
            initial_delay: Some(std::time::Duration::from_secs(1)),
            timestamp: true,
            throughput: true,
            hide_cursor: true,
            ..prodash::render::line::Options::default()
        }
        .auto_configure(prodash::render::line::StreamKind::Stderr),
    )
}

/// Get a progress tree for use with prodash.
#[must_use]
pub fn progress_tree(trace: bool) -> std::sync::Arc<prodash::tree::Root> {
    prodash::tree::root::Options {
        message_buffer_capacity: if trace { 10_000 } else { 200 },
        ..Default::default()
    }
    .into()
}

/// Run a function with progress tracking, returning its result alongside the
/// bytes it wrote to the `out` and `err` handles it was given.
///
/// `gitoxide-core` writes CLI-shaped reports to those handles — for `fetch`,
/// the refspec mapping and a per-ref status line. That is narration for someone
/// running `gix` directly, not part of `submod`'s output contract, so it is
/// captured and handed back rather than written to this process's own streams.
/// The caller decides where, and whether, it is shown.
pub fn get_progress<T>(
    func_name: &str,
    range: Option<std::ops::RangeInclusive<u8>>,
    run: impl FnOnce(
        progress::DoOrDiscard<prodash::tree::Item>,
        &mut dyn std::io::Write,
        &mut dyn std::io::Write,
    ) -> T,
) -> (T, Vec<u8>, Vec<u8>) {
    let standard_range = 2..=2;
    let range = range.unwrap_or_else(|| standard_range.clone());
    let progress = progress_tree(false);
    let sub_progress = progress.add_child(func_name);

    let handle = setup_line_renderer_range(&progress, range);

    let mut out = Vec::<u8>::new();
    let mut err = Vec::<u8>::new();

    let result = gix::trace::coarse!("run").into_scope(|| {
        run(
            progress::DoOrDiscard::from(Some(sub_progress)),
            &mut out,
            &mut err,
        )
    });

    handle.shutdown_and_wait();
    (result, out, err)
}

/// Fetch options for the `fetch` command, with an option for shallow fetching.
const fn fetch_options(remote: Option<String>, shallow: bool) -> FetchOptions {
    let shallow = if shallow {
        gix::remote::fetch::Shallow::DepthAtRemote(std::num::NonZeroU32::new(1).unwrap())
    } else {
        gix::remote::fetch::Shallow::NoChange
    };
    FetchOptions {
        format: gitoxide_core::OutputFormat::Human,
        dry_run: false,
        remote,
        ref_specs: Vec::new(),
        shallow,
        handshake_info: false,
        negotiation_info: false,
        open_negotiation_graph: None,
    }
}

/// Fetch updates from a remote repository.
///
/// The fetch report `gitoxide-core` produces never reaches stdout: stdout
/// carries `submod`'s own output, and callers parse it. The report is written to
/// stderr when the caller asked for verbose output, or when the fetch failed and
/// it is the only detail available to explain why.
pub fn fetch_repo(
    repo: gix::Repository,
    remote: Option<String>,
    shallow: bool,
    verbose: bool,
) -> Result<()> {
    let (inner_result, out, err) =
        get_progress("fetch", Some(FetchProgressRange), |progress, out, err| {
            gitoxide_core::repository::fetch(
                repo,
                progress,
                out,
                err,
                fetch_options(remote, shallow),
            )
        });

    if verbose || inner_result.is_err() {
        let mut sink = stderr();
        std::io::Write::write_all(&mut sink, &out)?;
        std::io::Write::write_all(&mut sink, &err)?;
    }

    inner_result.map_err(|e| anyhow::anyhow!("Fetch failed: {e}"))
}
