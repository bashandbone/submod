// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
//! Read-only inspection, config discovery, and dry-run CLI contracts.
mod common;

use common::TestHarness;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn text(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}
fn success(output: &Output) -> String {
    assert!(output.status.success(), "{}", text(output));
    text(output)
}
fn snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn visit(root: &Path, dir: &Path, entries: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut paths: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                entries.push((
                    path.strip_prefix(root).unwrap().join(".directory-snapshot"),
                    vec![],
                ));
                visit(root, &path, entries);
            } else {
                entries.push((
                    path.strip_prefix(root).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                ));
            }
        }
    }
    let mut entries = vec![];
    visit(root, root, &mut entries);
    entries
}
fn unchanged(before: Vec<(PathBuf, Vec<u8>)>, root: &Path) {
    let after = snapshot(root);
    assert_eq!(before.len(), after.len(), "filesystem entries changed");
    for ((path, bytes), (next, new_bytes)) in before.into_iter().zip(after) {
        assert_eq!(path, next);
        assert!(bytes == new_bytes, "bytes changed: {}", path.display());
    }
}
fn fixture() -> TestHarness {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h
}
fn managed() -> TestHarness {
    let h = fixture();
    let remote = h.create_test_remote("library-remote").unwrap();
    success(
        &h.run_submod(&["add", remote.to_str().unwrap(), "--name", "library"])
            .unwrap(),
    );
    success(&h.run_submod(&["sync"]).unwrap());
    h
}
fn locks(h: &TestHarness) -> [PathBuf; 2] {
    let common = h.git_stdout(&["rev-parse", "--git-common-dir"]);
    let paths = [
        h.work_dir.join(common).join("submod.lock"),
        h.work_dir.join("submod.toml.submod.lock"),
    ];
    for path in &paths {
        fs::write(path, "owned by another process\n").unwrap();
    }
    paths
}
fn traced(h: &TestHarness, cwd: &Path, args: &[&str]) -> Output {
    let trace = h.temp_dir.path().join("inspection-trace.jsonl");
    fs::write(&trace, "").unwrap();
    let output = Command::new(&h.submod_bin)
        .args(args)
        .current_dir(cwd)
        .env("GIT_CONFIG_GLOBAL", h.temp_dir.path().join("gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_TRACE2_EVENT", &trace)
        .output()
        .unwrap();
    let events = fs::read_to_string(trace).unwrap();
    for verb in [
        "fetch",
        "clone",
        "checkout",
        "reset",
        "update-index",
        "read-tree",
        "write-tree",
        "stash",
        "commit",
        "add",
    ] {
        assert!(
            !events.contains(&format!("\"{verb}\"")),
            "inspection ran mutating Git command {verb}: {events}"
        );
    }
    output
}

#[test]
fn r17_phase6_missing_implicit_config_explains_import_but_add_can_create_it() {
    let h = fixture();
    for command in ["check", "list", "init"] {
        let before = snapshot(&h.work_dir);
        let output = h.run_submod(&[command]).unwrap();
        unchanged(before, &h.work_dir);
        assert!(
            !output.status.success(),
            "{command} silently accepted missing config"
        );
        let message = text(&output).to_lowercase();
        assert!(
            message.contains("submod.toml")
                && (message.contains("generate-config") || message.contains("import")),
            "{message}"
        );
    }
    let remote = h.create_test_remote("new-default").unwrap();
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
    assert!(h.config_path().is_file());
    assert!(!h.work_dir.join("library/.git").exists());
}

#[test]
fn r17_phase6_explicit_missing_default_config_is_not_implicit_creation() {
    let h = fixture();
    let remote = h.create_test_remote("explicit-missing").unwrap();
    let before = snapshot(&h.work_dir);
    let output = h
        .run_submod(&[
            "--config",
            "submod.toml",
            "add",
            remote.to_str().unwrap(),
            "--name",
            "library",
            "--no-init",
        ])
        .unwrap();
    unchanged(before, &h.work_dir);
    assert!(!output.status.success());
    assert!(text(&output).contains("submod.toml"));
}

#[test]
fn r17_r25_phase6_root_nested_custom_and_linked_inspection_preserve_all_state() {
    let h = managed();
    h.git_stdout(&["add", ".gitmodules", "library", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "Record managed fixture"]);
    let nested = h.work_dir.join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("custom.toml"), h.read_config().unwrap()).unwrap();
    let linked = h.temp_dir.path().join("linked");
    h.git_stdout(&[
        "worktree",
        "add",
        "-b",
        "linked-inspection",
        linked.to_str().unwrap(),
    ]);
    let held = locks(&h);
    for (cwd, args) in [
        (h.work_dir.as_path(), vec!["list"]),
        (nested.as_path(), vec!["check"]),
        (nested.as_path(), vec!["--config", "custom.toml", "list"]),
        (linked.as_path(), vec!["list"]),
    ] {
        let before = snapshot(&h.work_dir);
        let linked_before = snapshot(&linked);
        let output = traced(&h, cwd, &args);
        unchanged(before, &h.work_dir);
        unchanged(linked_before, &linked);
        let message = success(&output);
        assert!(
            message.contains("library"),
            "inspection omitted managed module: {message}"
        );
    }
    for path in held {
        fs::remove_file(path).unwrap();
    }
}

fn nested_fixture() -> TestHarness {
    let h = fixture();
    let leaf = h.create_test_remote("leaf").unwrap();
    let outer = h.create_test_remote("outer").unwrap();
    let work = h.temp_dir.path().join("outer_work");
    h.git_at(
        &work,
        &[
            "submodule",
            "add",
            "--name",
            "nested-logical",
            leaf.to_str().unwrap(),
            "deps/leaf",
        ],
    );
    h.git_at(&work, &["commit", "-am", "Record nested leaf"]);
    h.git_at(&work, &["push", "origin", "main"]);
    h.create_config(&format!("[library]\nurl = {:?}\n", outer.to_str().unwrap()))
        .unwrap();
    success(&h.run_submod(&["init", "--recursive"]).unwrap());
    h
}

#[test]
fn r19_r25_phase6_recursive_list_reports_actual_nested_hierarchy_without_mutation() {
    let h = nested_fixture();
    let held = locks(&h);
    let before = snapshot(&h.work_dir);
    let output = traced(&h, &h.work_dir, &["list", "--recursive"]);
    unchanged(before, &h.work_dir);
    let message = success(&output);
    assert!(
        message.contains("library") && message.contains("library/deps/leaf"),
        "real nested hierarchy missing: {message}"
    );
    assert!(h.work_dir.join("library/deps/leaf/src/main.c").is_file());
    for path in held {
        fs::remove_file(path).unwrap();
    }
}

#[test]
fn r19_phase6_recursive_list_propagates_structural_inspection_errors() {
    let h = nested_fixture();
    fs::write(
        h.work_dir.join("library/.gitmodules"),
        "[broken submodule metadata\n",
    )
    .unwrap();
    let before = snapshot(&h.work_dir);
    let output = traced(&h, &h.work_dir, &["list", "--recursive"]);
    unchanged(before, &h.work_dir);
    assert!(
        !output.status.success(),
        "recursive inspection swallowed malformed metadata: {}",
        text(&output)
    );
    let message = text(&output);
    assert!(
        message.contains("library") && message.contains(".gitmodules"),
        "{message}"
    );
}

#[test]
fn r19_phase6_recursive_list_marks_uninitialized_checkout_without_guessing() {
    let h = managed();
    h.git_stdout(&["submodule", "deinit", "-f", "--", "library"]);
    let before = snapshot(&h.work_dir);
    let output = traced(&h, &h.work_dir, &["list", "--recursive"]);
    unchanged(before, &h.work_dir);
    let message = success(&output);
    assert!(message.contains("library"), "{message}");
    assert!(
        message.contains("not inspected") || message.contains("inspection skipped"),
        "uninitialized checkout was presented as fully inspected: {message}"
    );
}

#[test]
fn r19_phase6_recursive_list_refuses_redirected_child_gitfile_without_mutation() {
    let h = managed();
    let foreign = h.temp_dir.path().join("foreign-list-target");
    fs::create_dir(&foreign).unwrap();
    h.git_at(&foreign, &["init"]);
    h.git_at(
        &foreign,
        &["config", "core.worktree", foreign.to_str().unwrap()],
    );
    fs::write(
        h.work_dir.join("library/.git"),
        format!("gitdir: {}\n", foreign.join(".git").display()),
    )
    .unwrap();
    let before = snapshot(&h.work_dir);
    let foreign_before = snapshot(&foreign);
    let output = traced(&h, &h.work_dir, &["list", "--recursive"]);
    unchanged(before, &h.work_dir);
    unchanged(foreign_before, &foreign);
    assert_eq!(output.status.code(), Some(1), "{}", text(&output));
    let message = text(&output);
    assert!(
        message.contains("library") && (message.contains("belong") || message.contains("checkout")),
        "{message}"
    );
}

#[test]
fn r25_phase6_structural_dry_run_preserves_stale_index_stat_cache() {
    let h = managed();
    let modules = h.work_dir.join(".gitmodules");
    let modules_bytes = fs::read(&modules).unwrap();
    let original_metadata = fs::metadata(&modules).unwrap();
    let replacement = h.work_dir.join(".gitmodules-replacement");
    fs::write(&replacement, &modules_bytes).unwrap();
    fs::rename(&replacement, &modules).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_ne!(
            original_metadata.ino(),
            fs::metadata(&modules).unwrap().ino()
        );
    }
    #[cfg(not(unix))]
    let _ = original_metadata;

    let stage_before = h.git_stdout(&["ls-files", "--stage", "--", ".gitmodules"]);
    assert!(!stage_before.is_empty());
    let index = h.work_dir.join(".git/index");
    let index_bytes = fs::read(&index).unwrap();
    let index_before = fs::metadata(&index).unwrap();
    let before = snapshot(&h.work_dir);

    success(&traced(
        &h,
        &h.work_dir,
        &["delete", "library", "--dry-run"],
    ));

    assert_eq!(index_bytes, fs::read(&index).unwrap());
    let index_after = fs::metadata(&index).unwrap();
    assert_eq!(index_before.len(), index_after.len());
    assert_eq!(
        index_before.modified().unwrap(),
        index_after.modified().unwrap()
    );
    assert_eq!(index_before.permissions(), index_after.permissions());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(index_before.ino(), index_after.ino());
        assert_eq!(index_before.ctime(), index_after.ctime());
        assert_eq!(index_before.ctime_nsec(), index_after.ctime_nsec());
    }
    assert_eq!(modules_bytes, fs::read(&modules).unwrap());
    unchanged(before, &h.work_dir);
    assert_eq!(
        stage_before,
        h.git_stdout(&["ls-files", "--stage", "--", ".gitmodules"])
    );
}

fn dry_run(case: &str) {
    let h = managed();
    let remote = h.create_test_remote("new-remote").unwrap();
    let child = h.work_dir.join("library");
    let pin = h.git_at(&child, &["rev-parse", "HEAD"]);
    if case == "reset" {
        fs::write(child.join("LICENSE"), "dirty to preserve\n").unwrap();
    }
    let args: Vec<&str> = match case {
        "add" => vec!["add", remote.to_str().unwrap(), "--name", "new_module"],
        "no-init" => vec![
            "add",
            remote.to_str().unwrap(),
            "--name",
            "new_module",
            "--no-init",
        ],
        "change" => vec!["change", "library", "--ignore", "all"],
        "change-global" => vec!["change-global", "--branch", "feature"],
        "disable" => vec!["disable", "library"],
        "delete" => vec!["delete", "library"],
        "reset" => vec!["reset", "library"],
        "nuke" => vec!["nuke-it-from-orbit", "library"],
        "init" => vec!["init"],
        "update" => vec!["update"],
        "sync" => vec!["sync"],
        "generate-config" => vec![
            "generate-config",
            "--from-setup",
            "--output",
            "generated.toml",
        ],
        _ => unreachable!(),
    };
    let held = locks(&h);
    let before = snapshot(&h.work_dir);
    let mut preview_args = args.clone();
    preview_args.push("--dry-run");
    let output = traced(&h, &h.work_dir, &preview_args);
    unchanged(before, &h.work_dir);
    let preview = success(&output).to_lowercase();
    let target = match case {
        "add" | "no-init" => "new_module",
        "generate-config" => "generated.toml",
        "change-global" => "feature",
        _ => "library",
    };
    assert!(
        preview.contains(target),
        "preview omits affected target {target}: {preview}"
    );
    for path in held {
        fs::remove_file(path).unwrap();
    }
    let execution_before = snapshot(&h.work_dir);
    let execution = success(&h.run_submod(&args).unwrap()).to_lowercase();
    let raw: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
    match case {
        "add" => {
            assert_eq!(
                h.index_gitlink_mode("new_module").as_deref(),
                Some("160000")
            );
            assert!(h.work_dir.join("new_module/src/main.c").is_file());
        }
        "no-init" => {
            assert!(raw.get("new_module").is_some());
            assert!(!h.work_dir.join("new_module/.git").exists());
        }
        "change" => assert_eq!(raw["library"]["ignore"].as_str(), Some("all")),
        "change-global" => {
            assert_eq!(raw["defaults"]["branch"].as_str(), Some("feature"));
            assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), pin);
        }
        "disable" => {
            assert_eq!(raw["library"]["active"].as_bool(), Some(false));
            assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), pin);
        }
        "delete" => {
            assert!(raw.get("library").is_none());
            assert!(!child.exists());
            assert_eq!(h.index_gitlink_mode("library"), None);
        }
        "reset" => {
            assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), pin);
            assert_eq!(
                h.git_at(&child, &["show", "refs/stash:LICENSE"]),
                "dirty to preserve"
            );
        }
        "nuke" => {
            assert!(raw.get("library").is_some());
            assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), pin);
            assert!(child.join("src/main.c").is_file());
        }
        "generate-config" => {
            let generated: toml::Value =
                toml::from_str(&fs::read_to_string(h.work_dir.join("generated.toml")).unwrap())
                    .unwrap();
            assert!(generated.get("library").is_some());
        }
        "init" | "update" | "sync" => {
            unchanged(execution_before, &h.work_dir);
            assert!(preview.contains("unchanged"), "{case} preview: {preview}");
            assert!(
                execution.contains("unchanged"),
                "{case} execution: {execution}"
            );
            assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), pin);
        }
        _ => unreachable!(),
    }
}
macro_rules! dry_run_case {
    ($name:ident, $case:literal) => {
        #[test]
        fn $name() {
            dry_run($case);
        }
    };
}
dry_run_case!(r25_phase6_add_dry_run_matches_execution, "add");
dry_run_case!(r25_phase6_no_init_dry_run_matches_execution, "no-init");
dry_run_case!(r25_phase6_change_dry_run_matches_execution, "change");
dry_run_case!(
    r25_phase6_change_global_dry_run_matches_execution,
    "change-global"
);
dry_run_case!(r25_phase6_disable_dry_run_matches_execution, "disable");
dry_run_case!(r25_phase6_delete_dry_run_matches_execution, "delete");
dry_run_case!(r25_phase6_reset_dry_run_matches_execution, "reset");
dry_run_case!(r25_phase6_nuke_dry_run_matches_execution, "nuke");
dry_run_case!(r25_phase6_init_dry_run_matches_execution, "init");
dry_run_case!(r25_phase6_update_dry_run_matches_execution, "update");
dry_run_case!(r25_phase6_sync_dry_run_matches_execution, "sync");
dry_run_case!(
    r25_phase6_generate_config_dry_run_matches_execution,
    "generate-config"
);

#[test]
fn r17_phase6_selection_conflicts_and_duplicates_fail_before_mutation() {
    let h = managed();
    for args in [
        vec!["reset", "--all", "library"],
        vec!["nuke-it-from-orbit", "--all", "library"],
        vec!["reset", "library,library"],
        vec!["nuke-it-from-orbit", "library,library"],
    ] {
        let before = snapshot(&h.work_dir);
        let output = h.run_submod(&args).unwrap();
        unchanged(before, &h.work_dir);
        assert!(
            !output.status.success(),
            "invalid selection accepted: {args:?}"
        );
        if args.contains(&"--all") {
            assert_eq!(output.status.code(), Some(2));
        }
    }
}

#[test]
fn r17_phase6_check_and_sync_report_initialized_update_none_drift() {
    let h = managed();
    let mut config = h.read_config().unwrap();
    config.push_str("update = 'none'\nignore = 'all'\n");
    h.create_config(&config).unwrap();
    let child = h.work_dir.join("library");
    fs::write(child.join("LICENSE"), "policy-skipped dirty bytes\n").unwrap();

    let before = snapshot(&h.work_dir);
    let check = h.run_submod(&["check"]).unwrap();
    unchanged(before, &h.work_dir);
    assert_eq!(check.status.code(), Some(1), "{}", text(&check));
    let check_text = text(&check).to_lowercase();
    assert!(check_text.contains("managed git metadata"), "{check_text}");
    assert!(check_text.contains("working tree"), "{check_text}");

    let sync = h.run_submod(&["sync"]).unwrap();
    assert_eq!(sync.status.code(), Some(1), "{}", text(&sync));
    let sync_text = text(&sync).to_lowercase();
    assert!(sync_text.contains("skipped-policy"), "{sync_text}");
    assert!(sync_text.contains("unresolved"), "{sync_text}");
    assert_eq!(
        fs::read_to_string(child.join("LICENSE")).unwrap(),
        "policy-skipped dirty bytes\n"
    );
    assert_eq!(
        h.git_stdout(&["config", "--get", "submodule.library.ignore"]),
        "all"
    );
}

#[test]
fn r17_phase6_check_reports_unmanaged_git_submodule_without_removing_it() {
    let h = managed();
    let remote = h.create_test_remote("unmanaged-native").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "unmanaged-native",
        remote.to_str().unwrap(),
        "vendor/unmanaged",
    ]);
    let pin = h.git_at(&h.work_dir.join("vendor/unmanaged"), &["rev-parse", "HEAD"]);
    let before = snapshot(&h.work_dir);
    let output = h.run_submod(&["check"]).unwrap();
    unchanged(before, &h.work_dir);
    assert!(output.status.success(), "{}", text(&output));
    let message = text(&output);
    assert!(
        message.contains("Unmanaged Git submodule preserved")
            && message.contains("vendor/unmanaged"),
        "{message}"
    );
    assert_eq!(
        h.git_at(&h.work_dir.join("vendor/unmanaged"), &["rev-parse", "HEAD"]),
        pin
    );
}
