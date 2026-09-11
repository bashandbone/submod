// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
//! R15: real Git sparse policy reconciliation and preservation.
mod common;
use common::TestHarness;
use std::{fs, path::PathBuf, process::Output};

fn git(h: &TestHarness, args: &[&str]) -> String {
    let out = h
        .git_cmd()
        .current_dir(h.work_dir.join("m"))
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}
fn success(out: Output) {
    assert!(
        out.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
fn policy(h: &TestHarness, patterns: Option<&[&str]>, native: bool) {
    let url = git(h, &["remote", "get-url", "origin"]);
    let mut doc =
        format!("[m]\nurl = {url:?}\npath = \"m\"\nuse_git_default_sparse_checkout = {native}\n");
    if let Some(patterns) = patterns {
        doc.push_str(&format!("sparse_paths = {patterns:?}\n"));
    }
    fs::write(h.config_path(), doc).unwrap();
}
fn fixture() -> TestHarness {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_complex_remote("r15").unwrap();
    h.run_submod_success(&[
        "add",
        &format!("file://{}", remote.display()),
        "--name",
        "m",
        "--path",
        "m",
    ])
    .unwrap();
    let out = h
        .git_cmd()
        .current_dir(&h.work_dir)
        .args(["commit", "-am", "record module"])
        .output()
        .unwrap();
    success(out);
    policy(&h, Some(&["/src/", "/docs/"]), false);
    git(
        &h,
        &[
            "sparse-checkout",
            "set",
            "--no-cone",
            "!/*",
            "/src/",
            "/docs/",
        ],
    );
    h
}
fn exact(h: &TestHarness, patterns: &str) {
    assert_eq!(git(h, &["config", "--bool", "core.sparseCheckout"]), "true");
    assert_eq!(
        git(h, &["config", "--bool", "core.sparseCheckoutCone"]),
        "false"
    );
    assert_eq!(
        fs::read_to_string(h.get_sparse_checkout_file_path("m")).unwrap(),
        patterns
    );
}
fn snapshot(h: &TestHarness) -> Vec<(PathBuf, Option<Vec<u8>>)> {
    let dir = PathBuf::from(git(h, &["rev-parse", "--absolute-git-dir"]));
    let paths = [
        dir.join("config"),
        dir.join("config.worktree"),
        dir.join("index"),
        dir.join("info/sparse-checkout"),
        h.config_path(),
        h.work_dir.join(".git/index"),
        h.work_dir.join("m/src/lib.rs"),
        h.work_dir.join("m/docs/API.md"),
    ];
    paths
        .into_iter()
        .map(|p| {
            let bytes = fs::read(&p).ok();
            (p, bytes)
        })
        .collect()
}

#[test]
fn r15_ordered_globs_and_negation_materialize_exact_files() {
    let h = fixture();
    policy(
        &h,
        Some(&[
            "/*",
            "!/docs/*",
            "!/tests/*",
            "!/examples/*",
            "!/src/*.rs",
            "/src/lib.rs",
        ]),
        false,
    );
    success(h.run_submod(&["sync"]).unwrap());
    exact(
        &h,
        "!/*\n/*\n!/docs/*\n!/tests/*\n!/examples/*\n!/src/*.rs\n/src/lib.rs\n",
    );
    assert!(h.work_dir.join("m/src/lib.rs").is_file());
    assert!(h.work_dir.join("m/README.md").is_file());
    for path in ["docs/API.md", "tests/test.rs", "examples/basic.rs"] {
        assert!(!h.work_dir.join("m").join(path).exists(), "{path}");
    }
}
fn drift(kind: &str) {
    let h = fixture();
    match kind {
        "extra" => {
            fs::write(
                h.get_sparse_checkout_file_path("m"),
                "!/*\n/src/\n/docs/\n/tests/\n",
            )
            .unwrap();
        }
        "order" => {
            fs::write(h.get_sparse_checkout_file_path("m"), "!/*\n/docs/\n/src/\n").unwrap();
        }
        "enabled" => {
            git(&h, &["sparse-checkout", "disable"]);
        }
        "cone" => {
            git(&h, &["sparse-checkout", "set", "--cone", "src", "docs"]);
        }
        _ => unreachable!(),
    }
    let before = snapshot(&h);
    assert_eq!(
        h.run_submod(&["check"]).unwrap().status.code(),
        Some(1),
        "{kind} drift must fail check"
    );
    assert_eq!(snapshot(&h), before, "check mutated {kind} drift");
    success(h.run_submod(&["sync"]).unwrap());
    exact(&h, "!/*\n/src/\n/docs/\n");
    assert!(h.work_dir.join("m/src/lib.rs").is_file());
    assert!(!h.work_dir.join("m/tests/test.rs").exists());
}
#[test]
fn r15_extra_patterns_drift() {
    drift("extra");
}
#[test]
fn r15_reordered_patterns_drift() {
    drift("order");
}
#[test]
fn r15_disabled_sparse_drift() {
    drift("enabled");
}
#[test]
fn r15_cone_mode_drift() {
    drift("cone");
}
#[test]
fn r15_app_default_mode_change_removes_automatic_prefix() {
    let h = fixture();
    policy(&h, Some(&["/src/", "/docs/"]), true);
    success(h.run_submod(&["sync"]).unwrap());
    exact(&h, "/src/\n/docs/\n");
}
fn disable(empty: bool) {
    let h = fixture();
    policy(&h, if empty { Some(&[]) } else { None }, false);
    success(h.run_submod(&["sync"]).unwrap());
    assert_eq!(
        git(&h, &["config", "--bool", "core.sparseCheckout"]),
        "false"
    );
    for path in [
        "src/lib.rs",
        "docs/API.md",
        "tests/test.rs",
        "examples/basic.rs",
        "README.md",
        "Cargo.toml",
    ] {
        let expected = git(&h, &["show", &format!("HEAD:{path}")]);
        assert_eq!(
            fs::read_to_string(h.work_dir.join("m").join(path))
                .unwrap()
                .trim(),
            expected
        );
    }
}
#[test]
fn r15_removed_patterns_restore_full_checkout() {
    disable(false);
}
#[test]
fn r15_empty_patterns_restore_full_checkout() {
    disable(true);
}
#[test]
fn r15_unchanged_sync_preserves_bytes_and_skips_sparse_commands() {
    let h = fixture();
    // Settle unrelated metadata once; the second sync is the no-op under test.
    success(h.run_submod(&["sync"]).unwrap());
    let before = snapshot(&h);
    let trace = h.temp_dir.path().join("trace.json");
    let out = std::process::Command::new(&h.submod_bin)
        .arg("sync")
        .current_dir(&h.work_dir)
        .env("GIT_CONFIG_GLOBAL", h.temp_dir.path().join("gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TRACE", &trace)
        .output()
        .unwrap();
    success(out);
    assert_eq!(snapshot(&h), before);
    let trace = fs::read_to_string(trace).unwrap();
    for line in trace.lines() {
        assert!(!line.contains(" read-tree "), "checkout reapplied: {line}");
        for operation in ["set", "reapply", "init", "disable"] {
            assert!(
                !line.contains(&format!(" sparse-checkout {operation}")),
                "sparse reapplied: {line}"
            );
        }
    }
}
fn dirty(excluded: bool) {
    let h = fixture();
    let path = if excluded {
        "tests/test.rs"
    } else {
        "src/lib.rs"
    };
    let file = h.work_dir.join("m").join(path);
    fs::create_dir_all(file.parent().unwrap()).unwrap();
    fs::write(&file, b"local edits must survive\n").unwrap();
    policy(&h, if excluded { None } else { Some(&["/docs/"]) }, false);
    let before = snapshot(&h);
    let out = h.run_submod(&["sync"]).unwrap();
    assert_eq!(out.status.code(), Some(1));
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
    .to_lowercase();
    assert!(
        diagnostic.contains("dirty")
            || diagnostic.contains("local changes")
            || diagnostic.contains("overwrite"),
        "wrong refusal: {diagnostic}"
    );
    assert_eq!(fs::read(file).unwrap(), b"local edits must survive\n");
    assert_eq!(
        snapshot(&h),
        before,
        "refused sparse edit changed repository state"
    );
}
#[test]
fn r15_dirty_excluded_path_refuses_disable_without_overwrite() {
    dirty(true);
}
#[test]
fn r15_dirty_included_path_refuses_exclusion_without_overwrite() {
    dirty(false);
}
