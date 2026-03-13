use criterion::{Criterion, criterion_group, criterion_main};
use std::hint::black_box;

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
