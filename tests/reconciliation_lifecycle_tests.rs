// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Reconciliation lifecycle regression tests

mod common;

use common::TestHarness;
use std::{fs, path::Path, process::Output};

fn success(output: &Output) -> String {
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.status.success(), "{text}");
    text
}

fn fixture() -> TestHarness {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h
}

fn register(h: &TestHarness, remote: &Path, name: &str) -> String {
    h.git_at(
        &h.work_dir,
        &[
            "submodule",
            "add",
            "--name",
            name,
            remote.to_str().unwrap(),
            name,
        ],
    );
    h.git_at(&h.work_dir.join(name), &["rev-parse", "HEAD"])
}

fn declaration(name: &str, remote: &Path, extra: &str) -> String {
    format!("[{name}]\nurl = {:?}\n{extra}\n", remote.to_string_lossy())
}

fn fingerprint(h: &TestHarness, root: &Path, module: &str) -> Vec<(String, Option<Vec<u8>>)> {
    fn files(dir: &Path, paths: &mut Vec<String>) {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                files(&path, paths);
            } else {
                paths.push(path.to_string_lossy().into_owned());
            }
        }
    }
    let mut paths = vec![
        ".git/index".to_owned(),
        ".git/config".into(),
        ".git/HEAD".into(),
        ".gitmodules".into(),
        "submod.toml".into(),
    ];
    let child = root.join(module);
    let gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
    for file in ["index", "config", "HEAD", "packed-refs", "FETCH_HEAD"] {
        paths.push(Path::new(&gitdir).join(file).to_string_lossy().into_owned());
    }
    files(&child, &mut paths);
    let mut result: Vec<_> = paths
        .into_iter()
        .map(|p| {
            let bytes = fs::read(root.join(&p)).ok();
            (p, bytes)
        })
        .collect();
    result.push((
        "child refs".into(),
        Some(h.git_at(&child, &["show-ref"]).into_bytes()),
    ));
    result
}

fn sync_without_fetch(h: &TestHarness, cwd: &Path) {
    let trace = h.temp_dir.path().join("second-sync-trace.jsonl");
    success(
        &std::process::Command::new(&h.submod_bin)
            .arg("sync")
            .current_dir(cwd)
            .env("GIT_CONFIG_GLOBAL", h.temp_dir.path().join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_TRACE2_EVENT", &trace)
            .output()
            .unwrap(),
    );
    let events = fs::read_to_string(trace).unwrap();
    assert!(!events.is_empty(), "Git tracing must actually be enabled");
    assert!(
        !events.contains("\"fetch\""),
        "unchanged sync fetched: {events}"
    );
}

fn unchanged(before: Vec<(String, Option<Vec<u8>>)>, after: Vec<(String, Option<Vec<u8>>)>) {
    assert_eq!(before.len(), after.len(), "snapshot file count changed");
    for ((name, old), (new_name, new)) in before.into_iter().zip(after) {
        assert_eq!(name, new_name, "snapshot path changed");
        assert!(old == new, "bytes changed: {name}");
    }
}

#[test]
fn r11_phase4_toml_only_sync_registers_omitted_path_and_is_exact_noop() {
    let h = fixture();
    let remote = h.create_test_remote("toml").unwrap();
    h.create_config(&declaration("library", remote.as_ref(), ""))
        .unwrap();
    success(&h.run_submod(&["sync"]).unwrap());
    assert_eq!(h.index_gitlink_mode("library").as_deref(), Some("160000"));
    assert_eq!(
        h.git_stdout(&["config", "-f", ".gitmodules", "submodule.library.path"]),
        "library"
    );
    assert_eq!(
        h.git_at(&h.work_dir.join("library"), &["rev-parse", "HEAD"]),
        h.git_at(remote.as_ref(), &["rev-parse", "HEAD"])
    );
    assert!(h.work_dir.join("library/src/main.c").is_file());
    let before = fingerprint(&h, &h.work_dir, "library");
    sync_without_fetch(&h, &h.work_dir);
    unchanged(before, fingerprint(&h, &h.work_dir, "library"));
}

#[test]
fn r12_phase4_fresh_clone_sync_materializes_parent_pin() {
    let h = fixture();
    let remote = h.create_test_remote("fresh").unwrap();
    let pin = register(&h, remote.as_ref(), "library");
    h.create_config(&declaration("library", remote.as_ref(), ""))
        .unwrap();
    h.git_stdout(&["add", ".gitmodules", "library", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "Pin library"]);
    assert_ne!(h.advance_test_remote("fresh").unwrap(), pin);
    let fresh = h.temp_dir.path().join("fresh-parent");
    success(
        &h.git_cmd()
            .arg("clone")
            .arg(&h.work_dir)
            .arg(&fresh)
            .output()
            .unwrap(),
    );
    success(&h.run_submod_at(&fresh, &["sync"]).unwrap());
    assert_eq!(
        h.git_at(&fresh.join("library"), &["rev-parse", "HEAD"]),
        pin
    );
    assert_eq!(
        h.git_at(&fresh, &["ls-files", "--stage", "library"]),
        format!("160000 {pin} 0\tlibrary")
    );
    assert!(fresh.join("library/src/main.c").is_file());
    assert!(!fresh.join("library/ADVANCE.txt").exists());
    let before = fingerprint(&h, &fresh, "library");
    sync_without_fetch(&h, &fresh);
    unchanged(before, fingerprint(&h, &fresh, "library"));
}

#[test]
fn r21_phase4_retained_gitdir_reattaches_and_preserves_refs_and_stash() {
    let h = fixture();
    let remote = h.create_test_remote("retained").unwrap();
    let pin = register(&h, remote.as_ref(), "library");
    h.create_config(&declaration("library", remote.as_ref(), ""))
        .unwrap();
    let child = h.work_dir.join("library");
    h.git_at(&child, &["branch", "precious-local"]);
    fs::write(child.join("LICENSE"), "precious edit\n").unwrap();
    h.git_at(&child, &["stash", "push", "-m", "preserve me"]);
    let stash = h.git_at(&child, &["rev-parse", "refs/stash"]);
    h.git_stdout(&["add", ".gitmodules", "library"]);
    h.git_stdout(&["commit", "-m", "Record library"]);
    h.git_stdout(&["submodule", "deinit", "--", "library"]);
    assert!(!child.join(".git").exists());
    success(&h.run_submod(&["sync"]).unwrap());
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), pin);
    assert_eq!(
        h.git_at(&child, &["rev-parse", "refs/heads/precious-local"]),
        pin
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "refs/stash"]), stash);
    assert_eq!(
        h.git_at(&child, &["show", "refs/stash:LICENSE"]),
        "precious edit"
    );
    assert!(child.join("src/main.c").is_file());
}

#[test]
fn r19_phase4_disabled_and_update_none_missing_modules_skip_unreachable_urls() {
    let h = fixture();
    let unavailable = h.temp_dir.path().join("unavailable.git");
    h.create_config(
        &(declaration("disabled", &unavailable, "active = false")
            + &declaration("none", &unavailable, "update = \"none\"")),
    )
    .unwrap();
    let before = h.read_config().unwrap();
    success(&h.run_submod(&["sync"]).unwrap());
    assert!(!h.work_dir.join("disabled").exists());
    assert!(!h.work_dir.join("none").exists());
    assert_eq!(h.index_gitlink_mode("disabled"), None);
    assert_eq!(h.index_gitlink_mode("none"), None);
    assert_eq!(h.read_config().unwrap(), before);
}

#[test]
fn r13_phase4_update_none_reconciles_safe_metadata_without_materialization() {
    let h = fixture();
    let remote = h.create_test_remote("none").unwrap();
    let pin = register(&h, remote.as_ref(), "library");
    h.git_stdout(&["add", ".gitmodules", "library"]);
    h.git_stdout(&["commit", "-m", "Record library"]);
    h.git_stdout(&["submodule", "deinit", "--", "library"]);
    let unavailable = h.temp_dir.path().join("unavailable.git");
    h.create_config(&declaration(
        "library",
        &unavailable,
        "update = \"none\"\nignore = \"all\"",
    ))
    .unwrap();
    success(&h.run_submod(&["sync"]).unwrap());
    assert!(!h.work_dir.join("library/.git").exists());
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "library"]),
        format!("160000 {pin} 0\tlibrary")
    );
    assert_eq!(
        h.git_stdout(&["config", "-f", ".gitmodules", "submodule.library.update"]),
        "none"
    );
    assert_eq!(
        h.git_stdout(&["config", "-f", ".gitmodules", "submodule.library.ignore"]),
        "all"
    );
}

#[test]
fn r19_phase4_no_init_is_transient_and_later_init_materializes() {
    let h = fixture();
    let remote = h.create_test_remote("deferred").unwrap();
    success(
        &h.run_submod(&[
            "add",
            remote.to_str().unwrap(),
            "--name",
            "library",
            "--no-init",
        ])
        .unwrap(),
    );
    assert!(!h.work_dir.join("library/.git").exists());
    let raw: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
    assert_ne!(
        raw["library"].get("active").and_then(toml::Value::as_bool),
        Some(false)
    );
    success(&h.run_submod(&["init"]).unwrap());
    assert_eq!(h.index_gitlink_mode("library").as_deref(), Some("160000"));
    assert!(h.work_dir.join("library/src/main.c").is_file());
}

#[test]
fn r19_phase4_unmanaged_module_is_preserved_and_reported() {
    let h = fixture();
    let remote = h.create_test_remote("unmanaged").unwrap();
    register(&h, remote.as_ref(), "outsider");
    h.create_config("").unwrap();
    let before = fingerprint(&h, &h.work_dir, "outsider");
    let text = success(&h.run_submod(&["sync"]).unwrap()).to_lowercase();
    assert!(
        text.contains("unmanaged") && text.contains("outsider"),
        "{text}"
    );
    unchanged(before, fingerprint(&h, &h.work_dir, "outsider"));
}

#[test]
fn r13_phase4_explicit_add_none_keeps_initial_checkout_on_sync() {
    let h = fixture();
    let remote = h.create_test_remote("explicit-none").unwrap();
    success(
        &h.run_submod(&[
            "add",
            remote.to_str().unwrap(),
            "--name",
            "library",
            "--update",
            "none",
        ])
        .unwrap(),
    );
    assert_eq!(h.index_gitlink_mode("library").as_deref(), Some("160000"));
    assert!(h.work_dir.join("library/src/main.c").is_file());
    let pin = h.git_at(&h.work_dir.join("library"), &["rev-parse", "HEAD"]);
    success(&h.run_submod(&["sync"]).unwrap());
    assert_eq!(
        h.git_at(&h.work_dir.join("library"), &["rev-parse", "HEAD"]),
        pin
    );
    assert!(h.work_dir.join("library/src/main.c").is_file());
    let before = fingerprint(&h, &h.work_dir, "library");
    sync_without_fetch(&h, &h.work_dir);
    unchanged(before, fingerprint(&h, &h.work_dir, "library"));
}

#[test]
fn r24_phase4_batch_preflight_conflict_prevents_earlier_mutation() {
    let h = fixture();
    let remote = h.create_test_remote("batch").unwrap();
    h.create_config(
        &(declaration("a_good", remote.as_ref(), "")
            + &declaration("b_conflict", remote.as_ref(), "")
            + &declaration("c_pending", remote.as_ref(), "")),
    )
    .unwrap();
    fs::create_dir(h.work_dir.join("b_conflict")).unwrap();
    fs::write(h.work_dir.join("b_conflict/precious"), b"keep me").unwrap();
    let before = h.preservation_snapshot();
    let output = h.run_submod(&["sync"]).unwrap();
    assert!(!output.status.success());
    assert_eq!(before, h.preservation_snapshot());
    assert!(!h.work_dir.join("a_good").exists());
    assert!(!h.work_dir.join("c_pending").exists());
    assert_eq!(
        fs::read(h.work_dir.join("b_conflict/precious")).unwrap(),
        b"keep me"
    );
}

#[test]
fn r24_phase4_runtime_failure_reports_pending_and_retry_preserves_completed() {
    let h = fixture();
    let remote = h.create_test_remote("batch-retry").unwrap();
    let unavailable = h.temp_dir.path().join("later.git");
    h.create_config(
        &(declaration("a_good", remote.as_ref(), "")
            + &declaration("b_failed", &unavailable, "")
            + &declaration("c_pending", remote.as_ref(), "")),
    )
    .unwrap();
    let declaration_before = h.read_config().unwrap();
    let output = h.run_submod(&["sync"]).unwrap();
    assert_eq!(output.status.code(), Some(1));
    let text = String::from_utf8_lossy(&output.stderr);
    for row in [
        "a_good at a_good: changed:",
        "b_failed at b_failed: failed:",
        "c_pending at c_pending: pending:",
    ] {
        assert!(text.contains(row), "{text}");
    }
    assert!(
        text.contains(
            "Sync incomplete summary: 1 changed, 0 unchanged, 0 skipped, 1 failed, 1 pending."
        ),
        "{text}"
    );
    assert!(!String::from_utf8_lossy(&output.stdout).contains("summary:"));
    for stream in [&output.stdout, &output.stderr] {
        let normalized = String::from_utf8_lossy(stream).to_lowercase();
        assert!(!normalized.contains("sync complete"), "{normalized}");
        assert!(!normalized.contains("sync summary:"), "{normalized}");
    }
    assert_eq!(h.index_gitlink_mode("a_good").as_deref(), Some("160000"));
    assert!(!h.work_dir.join("c_pending").exists());
    assert_eq!(h.read_config().unwrap(), declaration_before);
    let pin = h.git_at(&h.work_dir.join("a_good"), &["rev-parse", "HEAD"]);
    assert!(text.contains(&format!("(target {pin})")), "{text}");
    h.git_at(
        &h.work_dir.join("a_good"),
        &["branch", "preserved-after-failure"],
    );
    success(
        &h.git_cmd()
            .args(["clone", "--bare"])
            .arg(remote.as_ref())
            .arg(&unavailable)
            .output()
            .unwrap(),
    );
    success(&h.run_submod(&["sync"]).unwrap());
    for name in ["a_good", "b_failed", "c_pending"] {
        assert_eq!(h.index_gitlink_mode(name).as_deref(), Some("160000"));
        assert!(h.work_dir.join(name).join("src/main.c").is_file());
    }
    assert_eq!(
        h.git_at(
            &h.work_dir.join("a_good"),
            &["rev-parse", "refs/heads/preserved-after-failure"]
        ),
        pin
    );
}

fn divergent_strategy(strategy: &str) {
    let h = fixture();
    let remote = h.create_test_remote(strategy).unwrap();
    let old = register(&h, remote.as_ref(), "library");
    let pin = h.advance_test_remote(strategy).unwrap();
    let child = h.work_dir.join("library");
    h.git_at(&child, &["fetch", "origin"]);
    h.git_at(&child, &["checkout", "--detach", &pin]);
    h.git_stdout(&["add", "library"]);
    h.git_at(&child, &["checkout", "-b", "local-work", &old]);
    fs::write(child.join("LOCAL.txt"), "local history\n").unwrap();
    h.git_at(&child, &["add", "LOCAL.txt"]);
    h.git_at(&child, &["commit", "-m", "Local divergent work"]);
    let local = h.git_at(&child, &["rev-parse", "HEAD"]);
    h.git_at(&child, &["branch", "saved-local", &local]);
    h.create_config(&declaration(
        "library",
        remote.as_ref(),
        &format!("update = {strategy:?}"),
    ))
    .unwrap();
    success(&h.run_submod(&["sync"]).unwrap());
    let result = h.git_at(&child, &["rev-parse", "HEAD"]);
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "library"]),
        format!("160000 {pin} 0\tlibrary")
    );
    assert!(child.join("ADVANCE.txt").is_file());
    assert_eq!(h.git_at(&child, &["rev-parse", "saved-local"]), local);
    if strategy == "checkout" {
        assert_eq!(result, pin);
        assert!(!child.join("LOCAL.txt").exists());
    } else {
        assert_ne!(result, pin);
        assert_eq!(h.git_at(&child, &["merge-base", &pin, "HEAD"]), pin);
        assert_eq!(
            fs::read_to_string(child.join("LOCAL.txt")).unwrap(),
            "local history\n"
        );
        if strategy == "merge" {
            assert_eq!(h.git_at(&child, &["merge-base", &local, "HEAD"]), local);
        }
    }
    success(&h.run_submod(&["check"]).unwrap());
    let before = fingerprint(&h, &h.work_dir, "library");
    sync_without_fetch(&h, &h.work_dir);
    unchanged(before, fingerprint(&h, &h.work_dir, "library"));
}

#[test]
fn r13_phase4_checkout_converges_divergent_head_to_parent_pin() {
    divergent_strategy("checkout");
}

#[test]
fn r13_phase4_merge_preserves_descendant_and_second_sync_is_noop() {
    divergent_strategy("merge");
}

#[test]
fn r13_phase4_rebase_preserves_descendant_and_second_sync_is_noop() {
    divergent_strategy("rebase");
}

#[test]
fn r13_phase4_metadata_change_never_moves_divergent_head() {
    let h = fixture();
    let remote = h.create_test_remote("metadata-divergence").unwrap();
    let pin = register(&h, remote.as_ref(), "library");
    let child = h.work_dir.join("library");
    h.git_at(&child, &["checkout", "-b", "local-work", "HEAD~1"]);
    fs::write(child.join("LOCAL.txt"), "retain local commit\n").unwrap();
    h.git_at(&child, &["add", "LOCAL.txt"]);
    h.git_at(&child, &["commit", "-m", "Local work"]);
    let local = h.git_at(&child, &["rev-parse", "HEAD"]);
    assert_ne!(local, pin);
    h.create_config(&declaration("library", remote.as_ref(), ""))
        .unwrap();
    success(
        &h.run_submod(&[
            "change", "library", "--branch", "feature", "--ignore", "all",
        ])
        .unwrap(),
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), local);
    assert_eq!(
        h.git_at(&child, &["symbolic-ref", "HEAD"]),
        "refs/heads/local-work"
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "library"]),
        format!("160000 {pin} 0\tlibrary")
    );
    assert_eq!(
        fs::read_to_string(child.join("LOCAL.txt")).unwrap(),
        "retain local commit\n"
    );
    assert_eq!(
        h.git_stdout(&["config", "-f", ".gitmodules", "submodule.library.branch"]),
        "feature"
    );
}

#[test]
fn r13_phase4_dot_branch_materializes_parent_symbolic_branch() {
    let h = fixture();
    let remote = h.create_test_remote("dot-branch").unwrap();
    h.git_stdout(&["checkout", "-b", "feature"]);
    h.create_config(&declaration("library", remote.as_ref(), "branch = \".\""))
        .unwrap();
    let expected = h.git_at(remote.as_ref(), &["rev-parse", "refs/heads/feature"]);
    success(&h.run_submod(&["sync"]).unwrap());
    assert_eq!(
        h.git_at(&h.work_dir.join("library"), &["rev-parse", "HEAD"]),
        expected
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "library"]),
        format!("160000 {expected} 0\tlibrary")
    );
    assert_eq!(
        h.git_stdout(&["config", "-f", ".gitmodules", "submodule.library.branch"]),
        "."
    );
}

#[test]
fn r13_phase4_dot_branch_detached_parent_refuses_before_batch_mutation() {
    let h = fixture();
    let remote = h.create_test_remote("detached-dot").unwrap();
    h.git_stdout(&["checkout", "--detach"]);
    h.create_config(
        &(declaration("a_good", remote.as_ref(), "")
            + &declaration("b_dot", remote.as_ref(), "branch = \".\"")),
    )
    .unwrap();
    let before = h.preservation_snapshot();
    let output = h.run_submod(&["sync"]).unwrap();
    assert!(!output.status.success());
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
    .to_lowercase();
    assert!(
        text.contains("detach") && (text.contains("branch") || text.contains("head")),
        "{text}"
    );
    assert_eq!(before, h.preservation_snapshot());
    assert!(!h.work_dir.join("a_good").exists());
    assert!(!h.work_dir.join("b_dot").exists());
}

#[test]
fn r21_phase4_missing_gitmodules_reconstructs_exact_managed_pin() {
    let h = fixture();
    let remote = h.create_test_remote("missing-registration").unwrap();
    let pin = register(&h, remote.as_ref(), "library");
    h.create_config(&declaration("library", remote.as_ref(), ""))
        .unwrap();
    h.git_stdout(&["add", ".gitmodules", "library", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "Record pin"]);
    h.git_stdout(&["submodule", "deinit", "--", "library"]);
    h.git_stdout(&["rm", ".gitmodules"]);
    assert_ne!(h.advance_test_remote("missing-registration").unwrap(), pin);
    success(&h.run_submod(&["sync"]).unwrap());
    assert_eq!(
        h.git_stdout(&[
            "config",
            "-f",
            ".gitmodules",
            "--get-regexp",
            "^submodule\\..*\\.path$"
        ]),
        "submodule.library.path library"
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "library"]),
        format!("160000 {pin} 0\tlibrary")
    );
    assert_eq!(
        h.git_at(&h.work_dir.join("library"), &["rev-parse", "HEAD"]),
        pin
    );
    assert!(h.work_dir.join("library/src/main.c").is_file());
    assert!(!h.work_dir.join("library/ADVANCE.txt").exists());
}

fn declaration_without_gitlink(h: &TestHarness, remote: &Path) {
    h.create_config(&declaration("library", remote, ""))
        .unwrap();
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.logical.path",
        "library",
    ]);
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.logical.url",
        remote.to_str().unwrap(),
    ]);
    h.git_stdout(&["add", ".gitmodules"]);
    h.git_stdout(&["commit", "-m", "Incomplete registration"]);
    assert_eq!(h.index_gitlink_mode("library"), None);
}

#[test]
fn r21_phase4_registration_without_gitlink_completes_empty_destination() {
    let h = fixture();
    let remote = h.create_test_remote("missing-gitlink").unwrap();
    declaration_without_gitlink(&h, remote.as_ref());
    h.create_config(&declaration("library", remote.as_ref(), "shallow = false"))
        .unwrap();
    fs::create_dir(h.work_dir.join("library")).unwrap();
    success(&h.run_submod(&["sync"]).unwrap());
    let pin = h.git_at(remote.as_ref(), &["rev-parse", "HEAD"]);
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "library"]),
        format!("160000 {pin} 0\tlibrary")
    );
    assert_eq!(
        h.git_at(&h.work_dir.join("library"), &["rev-parse", "HEAD"]),
        pin
    );
    assert_eq!(
        h.git_stdout(&[
            "config",
            "-f",
            ".gitmodules",
            "--get-regexp",
            "^submodule\\..*\\.path$"
        ]),
        "submodule.logical.path library"
    );
    assert!(h.work_dir.join("library/src/main.c").is_file());
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--blob",
            ":.gitmodules",
            "--get",
            "submodule.logical.shallow",
        ]),
        "false"
    );
    assert_eq!(
        h.git_stdout(&[
            "config",
            "-f",
            ".gitmodules",
            "--get",
            "submodule.logical.shallow",
        ]),
        "false"
    );
    assert!(
        h.git_at(&h.work_dir, &["diff", "--quiet", "--", ".gitmodules"])
            .is_empty()
    );
}

#[test]
fn r21_phase4_registration_without_gitlink_refuses_ambiguous_content_unchanged() {
    let h = fixture();
    let remote = h.create_test_remote("ambiguous-gitlink").unwrap();
    declaration_without_gitlink(&h, remote.as_ref());
    fs::create_dir(h.work_dir.join("library")).unwrap();
    fs::write(
        h.work_dir.join("library/precious.txt"),
        "independent content\n",
    )
    .unwrap();
    let before = h.preservation_snapshot();
    let output = h.run_submod(&["sync"]).unwrap();
    assert!(!output.status.success());
    assert_eq!(before, h.preservation_snapshot());
    assert_eq!(
        fs::read_to_string(h.work_dir.join("library/precious.txt")).unwrap(),
        "independent content\n"
    );
    assert!(!h.work_dir.join("library/.git").exists());
}

#[test]
fn r21_phase4_manual_toml_path_edit_refuses_init_and_sync_without_mutation() {
    for command in ["init", "sync"] {
        let h = fixture();
        let remote = h.create_test_remote("manual-path-edit").unwrap();
        let pin = register(&h, remote.as_ref(), "library");
        h.create_config(&declaration(
            "library",
            remote.as_ref(),
            "path = \"library\"",
        ))
        .unwrap();
        h.git_stdout(&["add", ".gitmodules", "library", "submod.toml"]);
        h.git_stdout(&["commit", "-m", "Record original module path"]);
        h.create_config(&declaration(
            "library",
            remote.as_ref(),
            "path = \"moved-library\"",
        ))
        .unwrap();

        let child = h.work_dir.join("library");
        let gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
        let core_worktree = h.git_at(&child, &["config", "--local", "core.worktree"]);
        let before = fingerprint(&h, &h.work_dir, "library");
        let output = h.run_submod(&[command]).unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !output.status.success(),
            "{command} accepted an unrequested move: {text}"
        );
        assert!(
            text.contains("library") && text.contains("moved-library"),
            "{command} must identify both conflicting paths: {text}"
        );
        unchanged(before, fingerprint(&h, &h.work_dir, "library"));
        assert!(
            !h.work_dir.join("moved-library").exists(),
            "{command} created the new path"
        );
        assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), pin);
        assert_eq!(
            h.git_at(&child, &["rev-parse", "--absolute-git-dir"]),
            gitdir
        );
        assert_eq!(
            h.git_at(&child, &["config", "--local", "core.worktree"]),
            core_worktree
        );
        assert_eq!(
            h.git_stdout(&["ls-files", "--stage", "library"]),
            format!("160000 {pin} 0\tlibrary")
        );
        assert_eq!(h.index_gitlink_mode("moved-library"), None);
    }
}

fn remote_tracking_update(explicit_branch: bool) {
    {
        let h = fixture();
        let remote = h.create_test_remote("remote-tracking").unwrap();
        if !explicit_branch {
            h.git_at(
                remote.as_ref(),
                &["symbolic-ref", "HEAD", "refs/heads/feature"],
            );
        }
        let pin = register(&h, remote.as_ref(), "library");
        let settings = if explicit_branch {
            "branch = \"feature\""
        } else {
            ""
        };
        h.create_config(&declaration("library", remote.as_ref(), settings))
            .unwrap();
        if explicit_branch {
            h.git_stdout(&[
                "config",
                "-f",
                ".gitmodules",
                "submodule.library.branch",
                "feature",
            ]);
        }
        h.git_stdout(&["add", ".gitmodules", "library", "submod.toml"]);
        h.git_stdout(&["commit", "-m", "Record initial pin"]);

        let remote_work = h.temp_dir.path().join("remote-tracking_work");
        h.git_at(&remote_work, &["checkout", "feature"]);
        fs::write(
            remote_work.join("TRACKING.txt"),
            "non-main tracking update\n",
        )
        .unwrap();
        h.git_at(&remote_work, &["add", "TRACKING.txt"]);
        h.git_at(&remote_work, &["commit", "-m", "Advance tracking branch"]);
        h.git_at(&remote_work, &["push", "origin", "feature"]);
        let target = h.git_at(&remote_work, &["rev-parse", "HEAD"]);
        assert_ne!(pin, target);

        success(&h.run_submod(&["update"]).unwrap());
        let child = h.work_dir.join("library");
        assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), pin);
        assert!(!child.join("TRACKING.txt").exists());
        let index_before = h.git_stdout(&["ls-files", "--stage"]);
        success(&h.run_submod(&["update", "--remote"]).unwrap());
        assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), target);
        assert_eq!(
            fs::read_to_string(child.join("TRACKING.txt")).unwrap(),
            "non-main tracking update\n"
        );
        assert_eq!(h.git_stdout(&["ls-files", "--stage"]), index_before);
        assert_eq!(
            h.git_stdout(&["ls-files", "--stage", "library"]),
            format!("160000 {pin} 0\tlibrary")
        );
        assert_eq!(
            h.git_stdout(&["diff", "--name-only", "--", "library"]),
            "library"
        );
        assert!(
            h.git_stdout(&["diff", "--cached", "--name-only", "--", "library"])
                .is_empty()
        );
    }
}

fn recursive_selection(command: &str) {
    {
        let h = fixture();
        let leaf_remote = h.create_test_remote("nested-leaf").unwrap();
        let outer_remote = h.create_test_remote("nested-outer").unwrap();
        let outer_work = h.temp_dir.path().join("nested-outer_work");
        h.git_at(
            &outer_work,
            &[
                "submodule",
                "add",
                "--name",
                "nested-logical",
                leaf_remote.to_str().unwrap(),
                "deps/leaf",
            ],
        );
        let leaf_pin = h.git_at(&outer_work.join("deps/leaf"), &["rev-parse", "HEAD"]);
        h.git_at(&outer_work, &["add", ".gitmodules", "deps/leaf"]);
        h.git_at(&outer_work, &["commit", "-m", "Pin nested module"]);
        h.git_at(&outer_work, &["push", "origin", "main"]);
        let outer_pin = h.git_at(&outer_work, &["rev-parse", "HEAD"]);
        assert_ne!(h.advance_test_remote("nested-leaf").unwrap(), leaf_pin);
        h.create_config(&declaration(
            "library",
            outer_remote.as_ref(),
            "fetch = \"always\"",
        ))
        .unwrap();

        // Fetch recursion policy alone must not opt into recursive materialization.
        if command == "update" {
            success(&h.run_submod(&["init"]).unwrap());
            success(&h.run_submod(&["update"]).unwrap());
        } else {
            success(&h.run_submod(&[command]).unwrap());
        }
        let outer = h.work_dir.join("library");
        let leaf = outer.join("deps/leaf");
        assert_eq!(h.git_at(&outer, &["rev-parse", "HEAD"]), outer_pin);
        assert_eq!(
            h.git_at(&outer, &["ls-files", "--stage", "deps/leaf"]),
            format!("160000 {leaf_pin} 0\tdeps/leaf")
        );
        assert!(!leaf.join(".git").exists());
        assert!(!leaf.join("src/main.c").exists());
        let parent_index = h.git_stdout(&["ls-files", "--stage"]);
        let outer_index = h.git_at(&outer, &["ls-files", "--stage"]);

        success(&h.run_submod(&[command, "--recursive"]).unwrap());
        assert!(leaf.join(".git").is_file());
        assert_eq!(h.git_at(&leaf, &["rev-parse", "HEAD"]), leaf_pin);
        assert!(leaf.join("src/main.c").is_file());
        assert!(!leaf.join("ADVANCE.txt").exists());
        assert_eq!(h.git_stdout(&["ls-files", "--stage"]), parent_index);
        assert_eq!(h.git_at(&outer, &["ls-files", "--stage"]), outer_index);
        assert_eq!(h.git_at(&outer, &["rev-parse", "HEAD"]), outer_pin);
    }
}

#[test]
fn r13_phase4_remote_update_uses_non_main_default_without_staging_parent_pin() {
    remote_tracking_update(false);
}

#[test]
fn r13_phase4_remote_update_uses_configured_branch_without_staging_parent_pin() {
    remote_tracking_update(true);
}

#[test]
fn r19_r20_phase4_recursive_selection_init_materializes_real_nested_pins() {
    recursive_selection("init");
}

#[test]
fn r19_r20_phase4_recursive_selection_sync_materializes_real_nested_pins() {
    recursive_selection("sync");
}

#[test]
fn r24_phase4_dirty_later_transition_preflights_before_earlier_add() {
    let h = fixture();
    let remote = h.create_test_remote("dirty-transition").unwrap();
    let old = register(&h, remote.as_ref(), "b_dirty");
    let remote_work = h.temp_dir.path().join("dirty-transition_work");
    fs::write(remote_work.join("LICENSE"), "upstream license\n").unwrap();
    h.git_at(&remote_work, &["add", "LICENSE"]);
    h.git_at(&remote_work, &["commit", "-m", "Change tracked content"]);
    h.git_at(&remote_work, &["push", "origin", "main"]);
    let target = h.git_at(&remote_work, &["rev-parse", "HEAD"]);
    let child = h.work_dir.join("b_dirty");
    h.git_at(&child, &["fetch", "origin"]);
    h.git_at(&child, &["checkout", "--detach", &target]);
    h.git_stdout(&["add", "b_dirty", ".gitmodules"]);
    h.git_stdout(&["commit", "-m", "Record required transition"]);
    h.git_at(&child, &["checkout", "--detach", &old]);
    fs::write(child.join("LICENSE"), "precious dirty content\n").unwrap();
    h.create_config(
        &(declaration("a_good", remote.as_ref(), "")
            + &declaration("b_dirty", remote.as_ref(), "")),
    )
    .unwrap();
    let before = fingerprint(&h, &h.work_dir, "b_dirty");
    let output = h.run_submod(&["sync"]).unwrap();
    assert!(
        !output.status.success(),
        "dirty transition unexpectedly succeeded"
    );
    unchanged(before, fingerprint(&h, &h.work_dir, "b_dirty"));
    assert!(!h.work_dir.join("a_good").exists());
    assert_eq!(h.index_gitlink_mode("a_good"), None);
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), old);
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "b_dirty"]),
        format!("160000 {target} 0\tb_dirty")
    );
    assert_eq!(
        fs::read_to_string(child.join("LICENSE")).unwrap(),
        "precious dirty content\n"
    );
}

fn init_wrong_pin(strategy: &str) {
    let h = fixture();
    let remote = h.create_test_remote("init-wrong-pin").unwrap();
    let old = register(&h, remote.as_ref(), "library");
    let target = h.advance_test_remote("init-wrong-pin").unwrap();
    let child = h.work_dir.join("library");
    h.git_at(&child, &["fetch", "origin"]);
    h.git_at(&child, &["checkout", "--detach", &target]);
    h.git_stdout(&["add", "library", ".gitmodules"]);
    h.git_stdout(&["commit", "-m", "Record new pin"]);
    h.git_at(&child, &["checkout", "--detach", &old]);
    assert!(!child.join("ADVANCE.txt").exists());
    assert!(h.git_at(&child, &["status", "--porcelain"]).is_empty());
    h.create_config(&declaration(
        "library",
        remote.as_ref(),
        &format!("update = {strategy:?}"),
    ))
    .unwrap();
    let parent_index = h.git_stdout(&["ls-files", "--stage", "library"]);
    success(&h.run_submod(&["init"]).unwrap());
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), target);
    assert_eq!(
        fs::read_to_string(child.join("ADVANCE.txt")).unwrap(),
        "advanced init-wrong-pin\n"
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "library"]),
        parent_index
    );
}

#[test]
fn r12_phase4_init_initialized_wrong_pin_checkout_converges() {
    init_wrong_pin("checkout");
}

#[test]
fn r13_phase4_init_initialized_wrong_pin_merge_converges() {
    init_wrong_pin("merge");
}

#[test]
fn r13_phase4_init_initialized_wrong_pin_rebase_converges() {
    init_wrong_pin("rebase");
}

#[test]
fn r21_phase4_unmerged_gitlink_stages_refuse_all_lifecycle_commands_unchanged() {
    use std::io::Write;
    use std::process::Stdio;

    for command in ["init", "update", "sync"] {
        let h = fixture();
        let remote = h.create_test_remote("unmerged-gitlink").unwrap();
        let ours = register(&h, remote.as_ref(), "library");
        let child = h.work_dir.join("library");
        let base = h.git_at(&child, &["rev-parse", "HEAD~1"]);
        let theirs = h.git_at(&child, &["rev-parse", "refs/remotes/origin/feature"]);
        assert_ne!(base, ours);
        assert_ne!(ours, theirs);
        h.create_config(&declaration("library", remote.as_ref(), ""))
            .unwrap();
        h.git_stdout(&["add", ".gitmodules", "library", "submod.toml"]);
        h.git_stdout(&["commit", "-m", "Record clean initial state"]);
        let mut index = h
            .git_cmd()
            .args(["update-index", "--index-info"])
            .current_dir(&h.work_dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = format!(
            "0 {}\tlibrary\n160000 {base} 1\tlibrary\n160000 {ours} 2\tlibrary\n160000 {theirs} 3\tlibrary\n",
            "0".repeat(40)
        );
        index
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        success(&index.wait_with_output().unwrap());
        let stages = format!(
            "160000 {base} 1\tlibrary\n160000 {ours} 2\tlibrary\n160000 {theirs} 3\tlibrary"
        );
        assert_eq!(h.git_stdout(&["ls-files", "--unmerged", "library"]), stages);
        let before = fingerprint(&h, &h.work_dir, "library");
        let output = h.run_submod(&[command]).unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .to_lowercase();
        assert!(
            !output.status.success(),
            "{command} accepted an unmerged gitlink: {text}"
        );
        assert!(
            text.contains("unmerged") || text.contains("conflict"),
            "{command} lacks conflict diagnosis: {text}"
        );
        assert!(
            !text.contains("updated") && !text.contains("sync complete"),
            "{command} printed misleading success: {text}"
        );
        unchanged(before, fingerprint(&h, &h.work_dir, "library"));
        assert_eq!(h.git_stdout(&["ls-files", "--unmerged", "library"]), stages);
        assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), ours);
        assert!(child.join("src/main.c").is_file());
    }
}

#[test]
fn r31_phase4_mixed_metadata_and_add_preserves_gitmodules_layers_and_retries() {
    let h = fixture();
    let remote = h.create_test_remote("mixed-metadata-add").unwrap();
    let existing_pin = register(&h, remote.as_ref(), "a_existing");
    h.git_stdout(&["add", ".gitmodules", "a_existing"]);
    h.git_stdout(&["commit", "-m", "Record existing module"]);
    h.create_config(
        &(declaration("a_existing", remote.as_ref(), "ignore = \"all\"")
            + &declaration("b_new", remote.as_ref(), "")),
    )
    .unwrap();
    let declaration_before = h.read_config().unwrap();
    let readme_index = h.git_stdout(&["ls-files", "--stage", "README.md"]);
    let expected_new_pin = h.git_at(remote.as_ref(), &["rev-parse", "HEAD"]);

    success(&h.run_submod(&["sync"]).unwrap());
    for (name, pin) in [("a_existing", &existing_pin), ("b_new", &expected_new_pin)] {
        assert_eq!(
            h.git_stdout(&["ls-files", "--stage", name]),
            format!("160000 {pin} 0\t{name}")
        );
        assert_eq!(
            h.git_at(&h.work_dir.join(name), &["rev-parse", "HEAD"]),
            *pin
        );
        assert!(h.work_dir.join(name).join("src/main.c").is_file());
        let key = format!("submodule.{name}.path");
        assert_eq!(h.git_stdout(&["config", "-f", ".gitmodules", &key]), name);
        assert_eq!(
            h.git_stdout(&["config", "--blob", ":.gitmodules", "--get", &key]),
            name
        );
        let key = format!("submodule.{name}.url");
        assert_eq!(
            h.git_stdout(&["config", "--blob", ":.gitmodules", "--get", &key]),
            remote.to_str().unwrap()
        );
    }
    // Adding b_new stages its registration, while a_existing's policy edit
    // remains an independent worktree change for review.
    assert_eq!(
        h.git_stdout(&["config", "-f", ".gitmodules", "submodule.a_existing.ignore"]),
        "all"
    );
    let staged_ignore = h
        .git_cmd()
        .args([
            "config",
            "--blob",
            ":.gitmodules",
            "--get",
            "submodule.a_existing.ignore",
        ])
        .current_dir(&h.work_dir)
        .output()
        .unwrap();
    assert_eq!(staged_ignore.status.code(), Some(1));
    assert!(staged_ignore.stdout.is_empty());
    assert!(staged_ignore.stderr.is_empty());
    assert_eq!(
        h.git_stdout(&["config", "--local", "submodule.a_existing.ignore"]),
        "all"
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "README.md"]),
        readme_index
    );
    assert_eq!(h.read_config().unwrap(), declaration_before);
    let existing_before = fingerprint(&h, &h.work_dir, "a_existing");
    let new_before = fingerprint(&h, &h.work_dir, "b_new");
    sync_without_fetch(&h, &h.work_dir);
    unchanged(existing_before, fingerprint(&h, &h.work_dir, "a_existing"));
    unchanged(new_before, fingerprint(&h, &h.work_dir, "b_new"));
}

#[test]
fn r31_phase4_incomplete_registration_unstaged_comment_refuses_without_removing_empty_dir() {
    let h = fixture();
    let remote = h.create_test_remote("completion-layer-conflict").unwrap();
    declaration_without_gitlink(&h, remote.as_ref());
    let gitmodules = h.work_dir.join(".gitmodules");
    let staged_gitmodules = h.git_stdout(&["show", ":.gitmodules"]);
    let mut edited = fs::read_to_string(&gitmodules).unwrap();
    edited.push_str("\n# unrelated user edit\n");
    fs::write(&gitmodules, &edited).unwrap();
    let empty = h.work_dir.join("library");
    fs::create_dir(&empty).unwrap();
    #[cfg(unix)]
    let inode_before = {
        use std::os::unix::fs::MetadataExt;
        fs::metadata(&empty).unwrap().ino()
    };
    let before = h.preservation_snapshot();
    let index_before = fs::read(h.work_dir.join(".git/index")).unwrap();
    let output = h.run_submod(&["sync"]).unwrap();
    assert!(!output.status.success());
    assert_eq!(before, h.preservation_snapshot());
    assert_eq!(
        fs::read(h.work_dir.join(".git/index")).unwrap(),
        index_before
    );
    assert_eq!(fs::read_to_string(&gitmodules).unwrap(), edited);
    assert_eq!(h.git_stdout(&["show", ":.gitmodules"]), staged_gitmodules);
    assert!(empty.is_dir());
    assert!(fs::read_dir(&empty).unwrap().next().is_none());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(fs::metadata(&empty).unwrap().ino(), inode_before);
    }
    assert_eq!(h.index_gitlink_mode("library"), None);
    assert!(!h.work_dir.join(".git/modules/logical").exists());
}

#[test]
fn r19_r20_phase4_recursive_selection_update_materializes_real_nested_pins() {
    recursive_selection("update");
}
