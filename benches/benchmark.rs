//! Benchmarks comparing two implementations of `.gitmodules` line key parsing.
//!
//! The benchmarks measure the performance difference between:
//!
//! - **`line_key_old`**: Allocates a formatted string for each key comparison.
//! - **`line_key_new`**: Performs a zero-allocation prefix check followed by a boundary
//!   character test.
//!
//! Run with:
//! ```sh
//! cargo bench
//! ```

use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

/// Returns the first key from `known_keys` that matches the start of `line`, or `None`.
///
/// Matching uses `format!("{key} =")` and `format!("{key}=")` to build comparison strings,
/// which allocates once per key per line. This is the baseline ("old") implementation
/// used to establish a performance reference point.
///
/// Empty lines and lines beginning with `#` are skipped immediately.
fn line_key_old<'a>(line: &str, known_keys: &[&'a str]) -> Option<&'a str> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    for key in known_keys {
        if trimmed.starts_with(&format!("{key} =")) || trimmed.starts_with(&format!("{key}=")) {
            return Some(key);
        }
    }
    None
}

/// Returns the first key from `known_keys` that matches the start of `line`, or `None`.
///
/// Matching first checks that `line` starts with the key as a prefix, then verifies that
/// the very next character is `=` or ` =` — avoiding any heap allocation. This is the
/// optimized ("new") implementation being benchmarked against [`line_key_old`].
///
/// Empty lines and lines beginning with `#` are skipped immediately.
fn line_key_new<'a>(line: &str, known_keys: &[&'a str]) -> Option<&'a str> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    for key in known_keys {
        if trimmed.starts_with(key) {
            let rest = &trimmed[key.len()..];
            if rest.starts_with('=') || rest.starts_with(" =") {
                return Some(key);
            }
        }
    }
    None
}

/// Registers the `line_key_old` and `line_key_new` benchmarks with Criterion.
///
/// Both functions are exercised over an identical set of representative input lines,
/// covering all supported key forms (`key = value`, `key=value`, leading whitespace,
/// unknown keys, comments, and blank lines) so the measurements are directly comparable.
pub fn criterion_benchmark(c: &mut Criterion) {
    let keys = vec![
        "path",
        "url",
        "branch",
        "ignore",
        "fetch",
        "update",
        "active",
        "shallow",
        "sparse_paths",
    ];
    let lines = vec![
        "path = foo",
        "url=bar",
        "branch = baz",
        "ignore=qux",
        "fetch = quux",
        "update=corge",
        "active = grault",
        "shallow=garply",
        "sparse_paths = waldo",
        "unknown = fred",
        "# comment",
        "",
        "  path = spaced  ",
    ];

    c.bench_function("line_key_old", |b| {
        b.iter(|| {
            for line in &lines {
                black_box(line_key_old(black_box(line), black_box(&keys)));
            }
        })
    });

    c.bench_function("line_key_new", |b| {
        b.iter(|| {
            for line in &lines {
                black_box(line_key_new(black_box(line), black_box(&keys)));
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
