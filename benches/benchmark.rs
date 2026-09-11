// SPDX-FileCopyrightText: 2026 Adam Poulemanos
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Production config microbenchmarks. CLI inspection/sync are measured separately
//! by scripts/measure-performance.py, using immutable bench-profile executables.
use criterion::{BatchSize, BenchmarkId, Criterion};
use std::hint::black_box;
use std::time::Instant;
use submod::config::{Config, SubmoduleEntry};

fn fixture(count: usize) -> String {
    use std::fmt::Write as _;
    let mut text = String::from("[defaults]\nignore = \"dirty\"\nupdate = \"checkout\"\n\n");
    for i in 0..count {
        let _ = write!(
            text,
            "[module-{i}]\npath = \"lib/module-{i}\"\nurl = \"file:///local/origin-{i}\"\nactive = true\nsparse_paths = [\"src\", \"docs\"]\n\n"
        );
    }
    text
}

fn insertion() -> SubmoduleEntry {
    SubmoduleEntry {
        path: Some("lib/inserted".into()),
        url: Some("file:///local/inserted".into()),
        active: Some(true),
        branch: None,
        ignore: None,
        update: None,
        fetch_recurse: None,
        shallow: None,
        no_init: None,
        sparse_paths: None,
        use_git_default_sparse_checkout: None,
    }
}

fn validate(config: &Config, count: usize) {
    assert_eq!(config.get_submodules().count(), count);
    let entry = config.effective_entry("module-0").unwrap();
    assert_eq!(entry.path.as_deref(), Some("lib/module-0"));
    assert_eq!(entry.active, Some(true));
    assert!(entry.ignore.is_some());
    assert!(entry.update.is_some());
}

fn main() {
    // One sample per process lets the comparison driver alternate immutable
    // baseline/candidate artifacts. Preparation and validation are not timed.
    if let Ok(workload) = std::env::var("SUBMOD_MEASURE_CONFIG") {
        let count: usize = std::env::var("SUBMOD_MEASURE_COUNT")
            .unwrap()
            .parse()
            .unwrap();
        let path = std::env::var("SUBMOD_MEASURE_FILE").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let config = Config::parse(&text).unwrap();
        validate(&config, count);
        let iterations = 1000;
        let mut results = Vec::with_capacity(iterations);
        let mut prepared: Vec<_> = if workload == "add" {
            (0..iterations)
                .map(|_| (config.clone(), "inserted".to_string(), insertion()))
                .collect()
        } else {
            Vec::new()
        };
        let start = Instant::now();
        match workload.as_str() {
            "parse" => {
                for _ in 0..iterations {
                    results.push(Config::parse(black_box(&text)).unwrap());
                }
            }
            "load" => {
                for _ in 0..iterations {
                    results.push(
                        Config::default()
                            .load_from_file(Some(black_box(&path)))
                            .unwrap(),
                    );
                }
            }
            "add" => {
                for (config, name, entry) in &mut prepared {
                    let name = std::mem::take(name);
                    let entry = std::mem::replace(entry, insertion());
                    config.add_submodule(name, entry);
                }
            }
            _ => panic!("unknown config workload"),
        }
        let nanos = start.elapsed().as_nanos();
        if workload == "add" {
            for (config, _, _) in &prepared {
                assert_eq!(config.get_submodules().count(), count + 1);
                assert_eq!(
                    config.get_submodule("inserted").unwrap().path.as_deref(),
                    Some("lib/inserted")
                );
            }
        } else {
            for config in &results {
                validate(config, count);
            }
        }
        println!("{{\"iterations\":{iterations},\"elapsed_ns\":{nanos},\"verdict\":\"pass\"}}");
        return;
    }
    let mut criterion = Criterion::default().configure_from_args();
    let temp = tempfile::tempdir().unwrap();
    for count in [1, 10, 100] {
        let text = fixture(count);
        let path = temp.path().join(format!("config-{count}.toml"));
        std::fs::write(&path, &text).unwrap();
        let config = Config::parse(&text).unwrap();
        validate(&config, count);
        criterion.bench_with_input(BenchmarkId::new("config_parse", count), &text, |b, text| {
            b.iter(|| Config::parse(black_box(text)).unwrap());
        });
        criterion.bench_with_input(
            BenchmarkId::new("config_load_file", count),
            &path,
            |b, path| {
                b.iter(|| {
                    Config::default()
                        .load_from_file(Some(black_box(path)))
                        .unwrap()
                });
            },
        );
        criterion.bench_with_input(
            BenchmarkId::new("config_add_one", count),
            &config,
            |b, config| {
                b.iter_batched(
                    || (config.clone(), "inserted".to_string(), insertion()),
                    |(mut config, name, entry)| {
                        config.add_submodule(name, entry);
                        black_box(config)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }
    criterion.final_summary();
}
