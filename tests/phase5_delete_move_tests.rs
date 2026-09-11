// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Delete/move checkout regression tests

mod common;
use common::TestHarness;
use std::{fs, path::PathBuf};

fn fixture() -> TestHarness {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h
}

fn add_child(h: &TestHarness) -> PathBuf {
    let remote = h.create_test_remote("phase5").unwrap();
    h.run_submod_success(&[
        "add",
        &format!("file://{}", remote.display()),
        "--name",
        "child",
        "--path",
        "lib/child",
    ])
    .unwrap();
    h.git_stdout(&["add", "submod.toml", ".gitmodules", "lib/child"]);
    h.git_stdout(&["commit", "-m", "register child"]);
    PathBuf::from(h.git_stdout(&["-C", "lib/child", "rev-parse", "--absolute-git-dir"]))
}

fn local_history(h: &TestHarness) -> (String, String) {
    h.git_stdout(&[
        "-C",
        "lib/child",
        "commit",
        "--allow-empty",
        "-m",
        "local only",
    ]);
    h.git_stdout(&["-C", "lib/child", "branch", "local-only"]);
    let head = h.git_stdout(&["-C", "lib/child", "rev-parse", "HEAD"]);
    fs::write(h.work_dir.join("lib/child/stash-only"), "retained stash\n").unwrap();
    h.git_stdout(&[
        "-C",
        "lib/child",
        "stash",
        "push",
        "--include-untracked",
        "-m",
        "retain",
    ]);
    let stash = h.git_stdout(&["-C", "lib/child", "rev-parse", "refs/stash"]);
    h.git_stdout(&["add", "lib/child"]);
    h.git_stdout(&["commit", "-m", "record local child"]);
    (head, stash)
}

fn assert_history(h: &TestHarness, storage: &std::path::Path, head: &str, stash: &str) {
    let dir = format!("--git-dir={}", storage.display());
    let worktree = format!("--work-tree={}", h.work_dir.display());
    assert_eq!(
        h.git_stdout(&[&dir, &worktree, "rev-parse", "refs/heads/local-only"]),
        head
    );
    assert_eq!(
        h.git_stdout(&[&dir, &worktree, "rev-parse", "refs/stash"]),
        stash
    );
    assert_eq!(
        h.git_stdout(&[&dir, &worktree, "cat-file", "-t", head]),
        "commit"
    );
    assert_eq!(
        h.git_stdout(&[&dir, &worktree, "show", &format!("{stash}^3:stash-only")]),
        "retained stash"
    );
}

#[test]
fn r22_config_only_delete_preserves_unrelated_state() {
    for occupied in [false, true] {
        let h = fixture();
        h.create_config("[child]\npath = 'lib/child'\nurl = 'https://invalid.invalid/absent.git'\nactive = true\n").unwrap();
        if occupied {
            fs::create_dir_all(h.work_dir.join("lib/child")).unwrap();
            fs::write(h.work_dir.join("lib/child/sentinel"), "unrelated\n").unwrap();
        }
        let before = h.preservation_snapshot();
        let index = fs::read(h.work_dir.join(".git/index")).unwrap();
        h.run_submod_success(&["delete", "child"]).unwrap();
        let config: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
        assert!(config.get("child").is_none());
        let after = h.preservation_snapshot();
        assert_eq!(before.0, after.0);
        assert_eq!(before.1, after.1);
        assert_eq!(before.2[1..], after.2[1..]);
        assert_eq!(fs::read(h.work_dir.join(".git/index")).unwrap(), index);
        if occupied {
            assert_eq!(
                fs::read_to_string(h.work_dir.join("lib/child/sentinel")).unwrap(),
                "unrelated\n"
            );
        } else {
            assert!(!h.work_dir.join("lib/child").exists());
        }
        assert!(!h.work_dir.join(".git/modules").exists());
    }
}

#[test]
fn r22_delete_legacy_embedded_gitdir_retains_objects() {
    let h = fixture();
    let storage = add_child(&h);
    let (head, stash) = local_history(&h);
    fs::remove_file(h.work_dir.join("lib/child/.git")).unwrap();
    fs::rename(&storage, h.work_dir.join("lib/child/.git")).unwrap();
    h.git_stdout(&["-C", "lib/child", "config", "--unset", "core.worktree"]);
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "rev-parse", "HEAD"]),
        head
    );
    assert_eq!(h.index_gitlink_mode("lib/child").as_deref(), Some("160000"));
    h.run_submod_success(&["delete", "child"]).unwrap();
    assert!(!h.work_dir.join("lib/child").exists());
    assert_eq!(h.index_gitlink_mode("lib/child"), None);
    let config: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
    assert!(config.get("child").is_none());
    assert_history(&h, &storage, &head, &stash);
}

#[test]
fn r23_rejected_moves_preserve_exact_state() {
    for case in ["tracked", "untracked", "occupied"] {
        let h = fixture();
        let storage = add_child(&h);
        let (head, stash) = local_history(&h);
        let dirty_path = match case {
            "tracked" => {
                let tracked = h.git_stdout(&["-C", "lib/child", "ls-files"]);
                h.work_dir
                    .join("lib/child")
                    .join(tracked.lines().next().unwrap())
            }
            "untracked" => h.work_dir.join("lib/child/untracked"),
            _ => {
                fs::create_dir_all(h.work_dir.join("lib/destination")).unwrap();
                h.work_dir.join("lib/destination/sentinel")
            }
        };
        fs::write(&dirty_path, "must survive\n").unwrap();
        let before = h.preservation_snapshot();
        let paths = [
            h.work_dir.join(".git/index"),
            h.work_dir.join("lib/child/.git"),
            storage.join("config"),
            storage.join("index"),
            dirty_path.clone(),
        ];
        let bytes: Vec<_> = paths.iter().map(|p| fs::read(p).unwrap()).collect();
        let refs = h.git_stdout(&["-C", "lib/child", "show-ref"]);
        let worktree = h.git_stdout(&["-C", "lib/child", "config", "--get", "core.worktree"]);
        let output = h
            .run_submod(&["change", "child", "--path", "lib/destination"])
            .unwrap();
        assert!(!output.status.success(), "{case}: {output:?}");
        assert_eq!(h.preservation_snapshot(), before, "{case}");
        for (path, expected) in paths.iter().zip(bytes) {
            assert_eq!(fs::read(path).unwrap(), expected, "{case}: {path:?}");
        }
        assert_eq!(
            h.git_stdout(&["-C", "lib/child", "rev-parse", "HEAD"]),
            head
        );
        assert_eq!(h.git_stdout(&["-C", "lib/child", "show-ref"]), refs);
        assert_eq!(
            h.git_stdout(&["-C", "lib/child", "config", "--get", "core.worktree"]),
            worktree
        );
        assert_history(&h, &storage, &head, &stash);
        if case != "occupied" {
            assert!(!h.work_dir.join("lib/destination").exists());
        }
    }
}

#[test]
fn r23_inactive_path_change_does_not_materialize() {
    let h = fixture();
    h.create_config(
        "[child]\npath = 'lib/child'\nurl = 'https://invalid.invalid/absent.git'\nactive = false\n",
    )
    .unwrap();
    let before = h.preservation_snapshot();
    let index = fs::read(h.work_dir.join(".git/index")).unwrap();
    let trace = h.temp_dir.path().join("inactive-git-trace");
    let output = std::process::Command::new(&h.submod_bin)
        .args(["change", "child", "--path", "lib/destination"])
        .current_dir(&h.work_dir)
        .env("GIT_CONFIG_GLOBAL", h.temp_dir.path().join("gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_TRACE", &trace)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let trace = fs::read_to_string(trace).unwrap();
    assert!(
        trace.contains("built-in: git"),
        "missing Git trace evidence"
    );
    for forbidden in [
        "git clone",
        "git fetch",
        "git-remote-",
        "git remote-http",
        "git submodule add",
    ] {
        assert!(
            !trace.contains(forbidden),
            "unexpected network-capable command: {trace}"
        );
    }
    let config: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
    assert_eq!(config["child"]["path"].as_str(), Some("lib/destination"));
    assert_eq!(config["child"]["active"].as_bool(), Some(false));
    assert_eq!(
        config["child"]["url"].as_str(),
        Some("https://invalid.invalid/absent.git")
    );
    assert!(!h.work_dir.join("lib/child").exists());
    assert!(!h.work_dir.join("lib/destination").exists());
    assert!(!h.work_dir.join(".git/modules").exists());
    assert_eq!(fs::read(h.work_dir.join(".git/index")).unwrap(), index);
    let after = h.preservation_snapshot();
    assert_eq!(before.0, after.0);
    assert_eq!(before.1, after.1);
    assert_eq!(before.2[1..], after.2[1..]);
}

#[cfg(unix)]
#[test]
fn r22_failed_nuke_rebuild_retains_history_and_retry_converges() {
    use std::os::unix::fs::PermissionsExt;
    let h = fixture();
    let storage = add_child(&h);
    let (head, stash) = local_history(&h);
    for scope in [["--file", ".gitmodules"], ["--blob", ":.gitmodules"]] {
        assert_eq!(
            h.git_stdout(&[
                "config",
                scope[0],
                scope[1],
                "--get",
                "submodule.child.shallow"
            ]),
            "false"
        );
    }
    let tracked = h.git_stdout(&["-C", "lib/child", "ls-files", "-z"]);
    let contents: Vec<_> = tracked
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| {
            (
                path.to_owned(),
                fs::read(h.work_dir.join("lib/child").join(path)).unwrap(),
            )
        })
        .collect();
    assert!(!contents.is_empty());
    fs::write(h.work_dir.join("sibling-staged"), "keep staged\n").unwrap();
    h.git_stdout(&["add", "sibling-staged"]);
    let sibling = h.git_stdout(&["ls-files", "--stage", "--", "sibling-staged"]);
    let gitlink = h.git_stdout(&["ls-files", "--stage", "--", "lib/child"]);
    let mut declaration: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
    let child = declaration.as_table_mut().unwrap().remove("child").unwrap();
    declaration
        .as_table_mut()
        .unwrap()
        .insert("nickname".to_owned(), child);
    h.create_config(&toml::to_string(&declaration).unwrap())
        .unwrap();
    let config = h.read_config().unwrap();
    let gitmodules = fs::read(h.work_dir.join(".gitmodules")).unwrap();
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get",
            "submodule.child.path"
        ]),
        "lib/child"
    );
    let wrappers = h.temp_dir.path().join("wrappers");
    fs::create_dir(&wrappers).unwrap();
    let real_git = std::process::Command::new("sh")
        .args(["-c", "command -v git"])
        .output()
        .unwrap();
    assert!(real_git.status.success());
    let real_git = String::from_utf8(real_git.stdout).unwrap();
    fs::write(
        wrappers.join("git"),
        r#"#!/bin/sh
case " $* " in
  *" submodule deinit "*)
    "$PHASE5_REAL_GIT" "$@"
    status=$?
    if [ "$status" -eq 0 ]; then printf deinitialized > "$PHASE5_DEINIT"; fi
    exit "$status"
    ;;
  *" submodule update "*)
    if [ -f "$PHASE5_DEINIT" ]; then
      printf held > "$PHASE5_STORAGE/index.lock"
      printf injected > "$PHASE5_MARKER"
    fi
    ;;
esac
exec "$PHASE5_REAL_GIT" "$@"
"#,
    )
    .unwrap();
    fs::set_permissions(wrappers.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
    let marker = h.temp_dir.path().join("injected");
    let deinit_marker = h.temp_dir.path().join("deinitialized");
    let output = std::process::Command::new(&h.submod_bin)
        .args(["nuke-it-from-orbit", "nickname"])
        .current_dir(&h.work_dir)
        .env(
            "PATH",
            format!("{}:{}", wrappers.display(), std::env::var("PATH").unwrap()),
        )
        .env("PHASE5_REAL_GIT", real_git.trim())
        .env("PHASE5_STORAGE", &storage)
        .env("PHASE5_MARKER", &marker)
        .env("PHASE5_DEINIT", &deinit_marker)
        .env("GIT_CONFIG_GLOBAL", h.temp_dir.path().join("gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .unwrap();
    assert!(
        marker.exists(),
        "rebuild injection never reached: {output:?}"
    );
    assert_eq!(fs::read_to_string(&deinit_marker).unwrap(), "deinitialized");
    assert_eq!(fs::read_to_string(&marker).unwrap(), "injected");
    assert_eq!(
        fs::read_to_string(storage.join("index.lock")).unwrap(),
        "held"
    );
    assert!(!output.status.success(), "{output:?}");
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(diagnostic.contains("partial"), "{diagnostic}");
    assert_eq!(h.read_config().unwrap(), config);
    assert_eq!(
        fs::read(h.work_dir.join(".gitmodules")).unwrap(),
        gitmodules
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "lib/child"]),
        gitlink
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "sibling-staged"]),
        sibling
    );
    assert_history(&h, &storage, &head, &stash);
    fs::remove_file(storage.join("index.lock")).unwrap();
    let partial_metadata = h.preservation_snapshot();
    let partial_paths = [
        h.work_dir.join(".git/index"),
        h.work_dir.join("lib/child/.git"),
        storage.join("config"),
        storage.join("index"),
        storage.join("HEAD"),
    ];
    let read_partial = || {
        partial_paths
            .iter()
            .map(|path| match fs::read(path) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => panic!("{path:?}: {error}"),
            })
            .collect::<Vec<_>>()
    };
    let partial_bytes = read_partial();
    let partial_status = h.git_stdout(&[
        "-C",
        "lib/child",
        "status",
        "--porcelain=v1",
        "--untracked-files=all",
    ]);
    assert!(!partial_status.is_empty());
    let partial_refs = h.git_stdout(&["-C", "lib/child", "show-ref"]);
    let retry = h.run_submod(&["nuke-it-from-orbit", "nickname"]).unwrap();
    assert!(
        !retry.status.success(),
        "ordinary retry discarded local deletion state: {retry:?}"
    );
    let guidance = format!(
        "{}{}",
        String::from_utf8_lossy(&retry.stdout),
        String::from_utf8_lossy(&retry.stderr)
    );
    assert!(guidance.contains("--force"), "{guidance}");
    assert_eq!(h.preservation_snapshot(), partial_metadata);
    assert_eq!(read_partial(), partial_bytes);
    assert_eq!(
        h.git_stdout(&[
            "-C",
            "lib/child",
            "status",
            "--porcelain=v1",
            "--untracked-files=all"
        ]),
        partial_status
    );
    assert_eq!(h.git_stdout(&["-C", "lib/child", "show-ref"]), partial_refs);
    for (path, _) in &contents {
        assert!(
            !h.work_dir.join("lib/child").join(path).exists(),
            "ordinary retry restored {path}"
        );
    }
    assert_history(&h, &storage, &head, &stash);
    h.run_submod_success(&["nuke-it-from-orbit", "nickname", "--force"])
        .unwrap();
    for (path, expected) in contents {
        assert_eq!(
            fs::read(h.work_dir.join("lib/child").join(&path)).unwrap(),
            expected,
            "{path}"
        );
    }
    assert!(
        h.git_stdout(&[
            "-C",
            "lib/child",
            "status",
            "--porcelain=v1",
            "--untracked-files=all"
        ])
        .is_empty()
    );
    h.run_submod_success(&["check"]).unwrap();
    assert_eq!(h.read_config().unwrap(), config);
    assert_eq!(
        fs::read(h.work_dir.join(".gitmodules")).unwrap(),
        gitmodules
    );
    assert_eq!(
        PathBuf::from(h.git_stdout(&["-C", "lib/child", "rev-parse", "--absolute-git-dir"])),
        storage
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "lib/child"]),
        gitlink
    );

    assert_history(&h, &storage, &head, &stash);
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "sibling-staged"]),
        sibling
    );
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "rev-parse", "HEAD"]),
        head
    );
}

#[test]
fn r22_force_delete_only_discards_selected_checkout_changes() {
    for case in ["tracked", "untracked", "ignored"] {
        let h = fixture();
        let storage = add_child(&h);
        let (head, stash) = local_history(&h);
        fs::create_dir_all(h.work_dir.join("lib/child-sibling")).unwrap();
        let sibling_path = h.work_dir.join("lib/child-sibling/sentinel");
        fs::write(&sibling_path, "prefix sibling\n").unwrap();
        h.git_stdout(&["add", "lib/child-sibling/sentinel"]);
        h.git_stdout(&["config", "phase5.sibling", "preserve-me"]);
        let outside_path = h.temp_dir.path().join("outside-sentinel");
        fs::write(&outside_path, "outside checkout\n").unwrap();
        let sibling_index = h.git_stdout(&["ls-files", "--stage", "--", "lib/child-sibling"]);
        let parent_refs = h.git_stdout(&["show-ref"]);
        let child_refs = h.git_stdout(&["-C", "lib/child", "show-ref"]);
        let dirty_path = match case {
            "tracked" => {
                let tracked = h.git_stdout(&["-C", "lib/child", "ls-files"]);
                h.work_dir
                    .join("lib/child")
                    .join(tracked.lines().next().unwrap())
            }
            "ignored" => {
                fs::write(storage.join("info/exclude"), "/ignored-local\n").unwrap();
                h.work_dir.join("lib/child/ignored-local")
            }
            _ => h.work_dir.join("lib/child/untracked-local"),
        };
        fs::write(&dirty_path, "selected local changes\n").unwrap();
        if case == "ignored" {
            assert_eq!(
                h.git_stdout(&["-C", "lib/child", "check-ignore", "ignored-local"]),
                "ignored-local"
            );
        }
        let before = h.preservation_snapshot();
        let paths = [
            h.work_dir.join(".git/index"),
            h.work_dir.join("lib/child/.git"),
            storage.join("config"),
            storage.join("index"),
            dirty_path.clone(),
        ];
        let bytes: Vec<_> = paths.iter().map(|path| fs::read(path).unwrap()).collect();
        let output = h.run_submod(&["delete", "child"]).unwrap();
        assert!(
            !output.status.success(),
            "default deletion accepted {case} changes: {output:?}"
        );
        assert_eq!(h.preservation_snapshot(), before, "{case}");
        for (path, expected) in paths.iter().zip(bytes) {
            assert_eq!(fs::read(path).unwrap(), expected, "{case}: {path:?}");
        }
        assert_eq!(h.git_stdout(&["-C", "lib/child", "show-ref"]), child_refs);
        assert_eq!(
            h.git_stdout(&["-C", "lib/child", "rev-parse", "HEAD"]),
            head
        );
        assert_history(&h, &storage, &head, &stash);

        h.run_submod_success(&["delete", "child", "--force"])
            .unwrap();
        assert!(!h.work_dir.join("lib/child").exists());
        assert_eq!(h.index_gitlink_mode("lib/child"), None);
        let config: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
        assert!(config.get("child").is_none());
        for args in [
            vec![
                "config",
                "--file",
                ".gitmodules",
                "--get-regexp",
                "^submodule\\.child\\.",
            ],
            vec!["config", "--local", "--get-regexp", "^submodule\\.child\\."],
        ] {
            let result = h
                .git_cmd()
                .args(args)
                .current_dir(&h.work_dir)
                .output()
                .unwrap();
            assert_eq!(result.status.code(), Some(1), "{result:?}");
            assert!(result.stdout.is_empty());
            assert!(result.stderr.is_empty());
        }
        assert_history(&h, &storage, &head, &stash);
        assert_eq!(
            h.git_stdout(&["ls-files", "--stage", "--", "lib/child-sibling"]),
            sibling_index
        );
        assert_eq!(h.git_stdout(&["show-ref"]), parent_refs);
        assert_eq!(
            h.git_stdout(&["config", "--get", "phase5.sibling"]),
            "preserve-me"
        );
        assert_eq!(
            fs::read_to_string(sibling_path).unwrap(),
            "prefix sibling\n"
        );
        assert_eq!(
            fs::read_to_string(outside_path).unwrap(),
            "outside checkout\n"
        );
    }
}

#[test]
fn r22_nuke_refuses_deleted_worktree_without_force() {
    let h = fixture();
    let storage = add_child(&h);
    let (head, stash) = local_history(&h);
    let tracked = h.git_stdout(&["-C", "lib/child", "ls-files", "-z"]);
    assert!(!tracked.is_empty());
    for path in tracked.split('\0').filter(|path| !path.is_empty()) {
        let file = h.work_dir.join("lib/child").join(path);
        fs::remove_file(&file).unwrap();
        let mut parent = file.parent().unwrap();
        while parent != h.work_dir.join("lib/child") {
            if fs::read_dir(parent).unwrap().next().is_some() {
                break;
            }
            fs::remove_dir(parent).unwrap();
            parent = parent.parent().unwrap();
        }
    }
    let mut remaining: Vec<_> = fs::read_dir(h.work_dir.join("lib/child"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    remaining.sort();
    assert_eq!(remaining, vec![std::ffi::OsString::from(".git")]);
    let read_status = || {
        let output = h
            .git_cmd()
            .args(["-C", "lib/child", "status", "--porcelain=v1", "-z"])
            .current_dir(&h.work_dir)
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        String::from_utf8(output.stdout).unwrap()
    };
    let status = read_status();
    assert!(!status.is_empty());
    assert!(
        status
            .split('\0')
            .filter(|line| !line.is_empty())
            .all(|line| line.starts_with(" D "))
    );
    let before = h.preservation_snapshot();
    let paths = [
        h.work_dir.join(".git/index"),
        h.work_dir.join("lib/child/.git"),
        storage.join("config"),
        storage.join("index"),
    ];
    let bytes: Vec<_> = paths.iter().map(|path| fs::read(path).unwrap()).collect();
    let refs = h.git_stdout(&["-C", "lib/child", "show-ref"]);
    let output = h.run_submod(&["nuke-it-from-orbit", "child"]).unwrap();
    assert!(
        !output.status.success(),
        "unmarked user deletions were discarded: {output:?}"
    );
    assert_eq!(h.preservation_snapshot(), before);
    for (path, expected) in paths.iter().zip(bytes) {
        assert_eq!(fs::read(path).unwrap(), expected, "{path:?}");
    }
    assert_eq!(read_status(), status);
    assert_eq!(h.git_stdout(&["-C", "lib/child", "show-ref"]), refs);
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "rev-parse", "HEAD"]),
        head
    );
    assert_history(&h, &storage, &head, &stash);
    let after: Vec<_> = fs::read_dir(h.work_dir.join("lib/child"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(after, remaining);
}

#[test]
fn r22_nuke_managed_inactive_update_none_preserves_ordered_unmanaged_settings() {
    let h = fixture();
    let storage = add_child(&h);
    let (head, stash) = local_history(&h);
    let content = fs::read(h.work_dir.join("lib/child/src/main.c")).unwrap();
    let url = h.git_stdout(&[
        "config",
        "--file",
        ".gitmodules",
        "--get",
        "submodule.child.url",
    ]);
    h.create_config(&format!("[nickname]\npath = 'lib/child'\nurl = '{url}'\nactive = false\nupdate = 'none'\nignore = 'dirty'\nbranch = 'main'\nfetchRecurse = 'always'\n")).unwrap();
    for (key, value) in [
        ("update", "none"),
        ("ignore", "dirty"),
        ("branch", "main"),
        ("fetchRecurseSubmodules", "true"),
    ] {
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            &format!("submodule.child.{key}"),
            value,
        ]);
    }
    h.git_stdout(&["add", ".gitmodules", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "inactive desired settings"]);
    h.git_stdout(&["config", "extensions.worktreeConfig", "true"]);
    h.git_stdout(&["config", "--local", "submodule.child.active", "false"]);
    h.git_stdout(&["config", "--local", "submodule.child.update", "none"]);
    h.git_stdout(&["config", "--local", "submodule.child.ignore", "all"]);
    h.git_stdout(&["config", "--local", "submodule.child.branch", "stale"]);
    h.git_stdout(&[
        "config",
        "--local",
        "submodule.child.fetchRecurseSubmodules",
        "false",
    ]);
    for scope in ["--local", "--worktree"] {
        for key in ["submodule.child.custom", "submodule.unrelated.custom"] {
            for value in ["first", "second", "first"] {
                h.git_stdout(&["config", scope, "--add", key, value]);
            }
        }
    }
    let before = h.preservation_snapshot();
    let index = fs::read(h.work_dir.join(".git/index")).unwrap();
    let staged_gitmodules = h.git_stdout(&["show", ":.gitmodules"]);
    let expected_gitmodules_path = h.temp_dir.path().join("expected.gitmodules");
    fs::write(&expected_gitmodules_path, before.2[1].as_ref().unwrap()).unwrap();
    h.git_stdout(&[
        "config",
        "--file",
        expected_gitmodules_path.to_str().unwrap(),
        "--unset-all",
        "submodule.child.shallow",
    ]);
    let expected_gitmodules = fs::read(expected_gitmodules_path).unwrap();
    h.run_submod_success(&["nuke-it-from-orbit", "nickname"])
        .unwrap();
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "rev-parse", "HEAD"]),
        head
    );
    assert_eq!(
        fs::read(h.work_dir.join("lib/child/src/main.c")).unwrap(),
        content
    );
    assert!(
        h.git_stdout(&["-C", "lib/child", "status", "--porcelain=v1"])
            .is_empty()
    );
    assert_eq!(fs::read(h.work_dir.join(".git/index")).unwrap(), index);
    let after = h.preservation_snapshot();
    assert_eq!(before.0, after.0);
    assert_eq!(before.1, after.1);
    assert_eq!(before.2[0], after.2[0]);
    assert_eq!(after.2[1].as_ref().unwrap(), &expected_gitmodules);
    assert_eq!(h.git_stdout(&["show", ":.gitmodules"]), staged_gitmodules);
    for scope in [
        vec!["--file", ".gitmodules"],
        vec!["--local"],
        vec!["--worktree"],
    ] {
        let mut args = vec!["config"];
        args.extend(scope);
        args.extend(["--get-all", "submodule.child.shallow"]);
        let output = h
            .git_cmd()
            .args(args)
            .current_dir(&h.work_dir)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
    assert_eq!(
        PathBuf::from(h.git_stdout(&["-C", "lib/child", "rev-parse", "--absolute-git-dir"])),
        storage
    );
    assert_history(&h, &storage, &head, &stash);
    for (key, value) in [
        ("active", "false"),
        ("update", "none"),
        ("ignore", "dirty"),
        ("branch", "main"),
        ("fetchRecurseSubmodules", "true"),
    ] {
        assert_eq!(
            h.git_stdout(&[
                "config",
                "--local",
                "--get-all",
                &format!("submodule.child.{key}")
            ]),
            value
        );
    }
    for scope in ["--local", "--worktree"] {
        for key in ["submodule.child.custom", "submodule.unrelated.custom"] {
            assert_eq!(
                h.git_stdout(&["config", scope, "--get-all", key]),
                "first\nsecond\nfirst",
                "{scope} {key}"
            );
        }
    }
}

#[test]
fn r22_nuke_changed_relative_url_fetches_missing_pin_from_selected_remote() {
    let h = fixture();
    let storage = add_child(&h);
    let (old_head, stash) = local_history(&h);
    let new_remote = h.create_test_remote("reachable").unwrap();
    let new_work = h.temp_dir.path().join("reachable_work");
    fs::write(
        new_work.join("required-only-new.txt"),
        "required parent pin from new remote\n",
    )
    .unwrap();
    h.git_at(&new_work, &["add", "required-only-new.txt"]);
    h.git_at(&new_work, &["commit", "-m", "pin only in new remote"]);
    h.git_at(&new_work, &["push", "origin", "main"]);
    let pin = h.git_at(&new_work, &["rev-parse", "HEAD"]);
    let missing = h
        .git_cmd()
        .args(["-C", "lib/child", "cat-file", "-e", &pin])
        .current_dir(&h.work_dir)
        .output()
        .unwrap();
    assert_eq!(
        missing.status.code(),
        Some(1),
        "pin unexpectedly present or Git inspection failed: {missing:?}"
    );
    let expected_url = format!("file://{}", new_remote.display());
    let parent_url = format!("file://{}", h.temp_dir.path().join("super.git").display());
    h.git_stdout(&["remote", "add", "upstream", &parent_url]);
    h.git_stdout(&["config", "branch.main.remote", "upstream"]);
    h.git_stdout(&["config", "branch.main.merge", "refs/heads/main"]);
    h.git_stdout(&["-C", "lib/child", "remote", "rename", "origin", "selected"]);
    h.git_stdout(&[
        "-C",
        "lib/child",
        "config",
        "branch.main.remote",
        "selected",
    ]);
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "symbolic-ref", "--short", "HEAD"]),
        "main"
    );
    h.create_config("[nickname]\npath = 'lib/child'\nurl = '../reachable.git'\nactive = true\nupdate = 'checkout'\n").unwrap();
    let stale_url = h.git_stdout(&[
        "config",
        "--file",
        ".gitmodules",
        "--get",
        "submodule.child.url",
    ]);
    assert_ne!(stale_url, expected_url);
    assert_ne!(stale_url, "../reachable.git");
    assert_eq!(
        h.git_stdout(&["config", "--local", "--get", "submodule.child.url"]),
        stale_url
    );
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "config", "--get", "remote.selected.url"]),
        stale_url
    );
    h.git_stdout(&[
        "config",
        "--file",
        ".gitmodules",
        "submodule.child.update",
        "checkout",
    ]);
    h.git_stdout(&[
        "update-index",
        "--cacheinfo",
        &format!("160000,{pin},lib/child"),
    ]);
    h.git_stdout(&["add", ".gitmodules", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "record pin and TOML-only URL change"]);
    let selected_fetch = h.git_stdout(&[
        "-C",
        "lib/child",
        "config",
        "--get-all",
        "remote.selected.fetch",
    ]);
    let before = h.preservation_snapshot();
    let index = fs::read(h.work_dir.join(".git/index")).unwrap();
    let staged_gitmodules = h.git_stdout(&["show", ":.gitmodules"]);
    let expected_gitmodules_path = h.temp_dir.path().join("expected.gitmodules");
    fs::write(&expected_gitmodules_path, before.2[1].as_ref().unwrap()).unwrap();
    h.git_stdout(&[
        "config",
        "--file",
        expected_gitmodules_path.to_str().unwrap(),
        "submodule.child.url",
        "../reachable.git",
    ]);
    h.git_stdout(&[
        "config",
        "--file",
        expected_gitmodules_path.to_str().unwrap(),
        "--unset-all",
        "submodule.child.shallow",
    ]);
    let expected_gitmodules = fs::read(expected_gitmodules_path).unwrap();
    h.run_submod_success(&["nuke-it-from-orbit", "nickname", "--force"])
        .unwrap();
    assert_eq!(h.git_stdout(&["-C", "lib/child", "rev-parse", "HEAD"]), pin);
    assert_eq!(
        fs::read_to_string(h.work_dir.join("lib/child/required-only-new.txt")).unwrap(),
        "required parent pin from new remote\n"
    );
    assert!(
        h.git_stdout(&["-C", "lib/child", "status", "--porcelain=v1"])
            .is_empty()
    );
    assert_eq!(
        h.git_stdout(&["config", "--local", "--get", "submodule.child.url"]),
        expected_url
    );
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "config", "--get", "remote.selected.url"]),
        expected_url
    );
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "config", "--get", "branch.main.remote"]),
        "selected"
    );
    assert_eq!(
        h.git_stdout(&[
            "-C",
            "lib/child",
            "config",
            "--get-all",
            "remote.selected.fetch"
        ]),
        selected_fetch
    );
    assert_eq!(
        PathBuf::from(h.git_stdout(&["-C", "lib/child", "rev-parse", "--absolute-git-dir"])),
        storage
    );
    assert_eq!(fs::read(h.work_dir.join(".git/index")).unwrap(), index);
    let after = h.preservation_snapshot();
    assert_eq!(before.0, after.0);
    assert_eq!(before.1, after.1);
    assert_eq!(before.2[0], after.2[0]);
    assert_eq!(after.2[1].as_ref().unwrap(), &expected_gitmodules);
    assert_eq!(h.git_stdout(&["show", ":.gitmodules"]), staged_gitmodules);
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get",
            "submodule.child.url"
        ]),
        "../reachable.git"
    );
    assert_history(&h, &storage, &old_head, &stash);
}

#[test]
fn r22_nuke_absolute_url_fetches_missing_pin_from_selected_remote() {
    let h = fixture();
    let storage = add_child(&h);
    let (old_head, stash) = local_history(&h);
    let new_remote = h.create_test_remote("reachable").unwrap();
    let new_work = h.temp_dir.path().join("reachable_work");
    fs::write(
        new_work.join("required-only-new.txt"),
        "required parent pin from new remote\n",
    )
    .unwrap();
    h.git_at(&new_work, &["add", "required-only-new.txt"]);
    h.git_at(&new_work, &["commit", "-m", "pin only in new remote"]);
    h.git_at(&new_work, &["push", "origin", "main"]);
    let pin = h.git_at(&new_work, &["rev-parse", "HEAD"]);
    let missing = h
        .git_cmd()
        .args(["-C", "lib/child", "cat-file", "-e", &pin])
        .current_dir(&h.work_dir)
        .output()
        .unwrap();
    assert_eq!(
        missing.status.code(),
        Some(1),
        "pin unexpectedly present or Git inspection failed: {missing:?}"
    );
    let expected_url = format!("file://{}", new_remote.display());
    let parent_url = format!("file://{}", h.temp_dir.path().join("super.git").display());
    h.git_stdout(&["remote", "add", "upstream", &parent_url]);
    h.git_stdout(&["config", "branch.main.remote", "upstream"]);
    h.git_stdout(&["config", "branch.main.merge", "refs/heads/main"]);
    h.git_stdout(&["-C", "lib/child", "remote", "rename", "origin", "selected"]);
    h.git_stdout(&[
        "-C",
        "lib/child",
        "config",
        "branch.main.remote",
        "selected",
    ]);
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "symbolic-ref", "--short", "HEAD"]),
        "main"
    );
    h.create_config(&format!(
        "[nickname]\npath = 'lib/child'\nurl = '{expected_url}'\nactive = true\nupdate = 'checkout'\n"
    ))
    .unwrap();
    let stale_url = h.git_stdout(&[
        "config",
        "--file",
        ".gitmodules",
        "--get",
        "submodule.child.url",
    ]);
    assert_ne!(stale_url, expected_url);
    assert_eq!(
        h.git_stdout(&["config", "--local", "--get", "submodule.child.url"]),
        stale_url
    );
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "config", "--get", "remote.selected.url"]),
        stale_url
    );
    h.git_stdout(&[
        "config",
        "--file",
        ".gitmodules",
        "submodule.child.update",
        "checkout",
    ]);
    h.git_stdout(&[
        "update-index",
        "--cacheinfo",
        &format!("160000,{pin},lib/child"),
    ]);
    h.git_stdout(&["add", ".gitmodules", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "record pin and TOML-only URL change"]);
    let selected_fetch = h.git_stdout(&[
        "-C",
        "lib/child",
        "config",
        "--get-all",
        "remote.selected.fetch",
    ]);
    let before = h.preservation_snapshot();
    let index = fs::read(h.work_dir.join(".git/index")).unwrap();
    let staged_gitmodules = h.git_stdout(&["show", ":.gitmodules"]);
    let expected_gitmodules_path = h.temp_dir.path().join("expected.gitmodules");
    fs::write(&expected_gitmodules_path, before.2[1].as_ref().unwrap()).unwrap();
    h.git_stdout(&[
        "config",
        "--file",
        expected_gitmodules_path.to_str().unwrap(),
        "submodule.child.url",
        &expected_url,
    ]);
    h.git_stdout(&[
        "config",
        "--file",
        expected_gitmodules_path.to_str().unwrap(),
        "--unset-all",
        "submodule.child.shallow",
    ]);
    let expected_gitmodules = fs::read(expected_gitmodules_path).unwrap();
    h.run_submod_success(&["nuke-it-from-orbit", "nickname", "--force"])
        .unwrap();
    assert_eq!(h.git_stdout(&["-C", "lib/child", "rev-parse", "HEAD"]), pin);
    assert_eq!(
        fs::read_to_string(h.work_dir.join("lib/child/required-only-new.txt")).unwrap(),
        "required parent pin from new remote\n"
    );
    assert!(
        h.git_stdout(&["-C", "lib/child", "status", "--porcelain=v1"])
            .is_empty()
    );
    assert_eq!(
        h.git_stdout(&["config", "--local", "--get", "submodule.child.url"]),
        expected_url
    );
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "config", "--get", "remote.selected.url"]),
        expected_url
    );
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "config", "--get", "branch.main.remote"]),
        "selected"
    );
    assert_eq!(
        h.git_stdout(&[
            "-C",
            "lib/child",
            "config",
            "--get-all",
            "remote.selected.fetch"
        ]),
        selected_fetch
    );
    assert_eq!(
        PathBuf::from(h.git_stdout(&["-C", "lib/child", "rev-parse", "--absolute-git-dir"])),
        storage
    );
    assert_eq!(fs::read(h.work_dir.join(".git/index")).unwrap(), index);
    let after = h.preservation_snapshot();
    assert_eq!(before.0, after.0);
    assert_eq!(before.1, after.1);
    assert_eq!(before.2[0], after.2[0]);
    assert_eq!(after.2[1].as_ref().unwrap(), &expected_gitmodules);
    assert_eq!(h.git_stdout(&["show", ":.gitmodules"]), staged_gitmodules);
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get",
            "submodule.child.url"
        ]),
        expected_url
    );
    assert_history(&h, &storage, &old_head, &stash);
}

#[test]
fn r22_sync_relative_parent_url_resolves_child_url_from_submodule() {
    let h = fixture();
    add_child(&h);
    // Relative superproject remote: native `submodule sync` resolves the
    // child URL against this string, then re-bases it below the submodule
    // with one `../` per submodule path component (Git's `get_up_path`).
    // `super.git` never needs to exist; sync only resolves strings.
    h.git_stdout(&["remote", "add", "upstream", "../super.git"]);
    h.git_stdout(&["config", "branch.main.remote", "upstream"]);
    h.git_stdout(&["config", "branch.main.merge", "refs/heads/main"]);
    h.create_config("[nickname]\npath = 'lib/child'\nurl = '../reachable.git'\nactive = true\nupdate = 'checkout'\n").unwrap();
    h.run_submod_success(&["sync"]).unwrap();
    assert_eq!(
        h.git_stdout(&["config", "--local", "--get", "submodule.child.url"]),
        "../reachable.git"
    );
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get",
            "submodule.child.url"
        ]),
        "../reachable.git"
    );
    // One `../` per path component (Git's `get_up_path`: separators plus
    // one): from lib/child, `../../../reachable.git` re-bases the
    // superproject-relative `../reachable.git` below the submodule. Verified
    // against native `git submodule sync`, which writes the same value.
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "config", "--get", "remote.origin.url"]),
        "../../../reachable.git"
    );
}

#[test]
fn r22_nuke_refuses_nested_ignored_content_without_mutation() {
    fn snapshot(
        path: &std::path::Path,
        entries: &mut std::collections::BTreeMap<PathBuf, Option<Vec<u8>>>,
    ) {
        if path.is_dir() {
            entries.insert(path.to_path_buf(), None);
            for entry in fs::read_dir(path).unwrap() {
                snapshot(&entry.unwrap().path(), entries);
            }
        } else {
            entries.insert(path.to_path_buf(), Some(fs::read(path).unwrap()));
        }
    }
    let h = fixture();
    add_child(&h);
    let remote = h.create_test_remote("nested").unwrap();
    h.git_stdout(&[
        "-C",
        "lib/child",
        "submodule",
        "add",
        "--name",
        "inner",
        &format!("file://{}", remote.display()),
        "nested/inner",
    ]);
    h.git_stdout(&["-C", "lib/child", "commit", "-am", "register inner"]);
    h.git_stdout(&["-C", "lib/child", "rm", ".gitmodules"]);
    h.git_stdout(&[
        "-C",
        "lib/child",
        "commit",
        "-m",
        "retain inner gitlink without declarations",
    ]);
    assert!(!h.work_dir.join("lib/child/.gitmodules").exists());
    assert!(
        h.git_stdout(&[
            "-C",
            "lib/child",
            "ls-files",
            "--stage",
            "--",
            "nested/inner"
        ])
        .starts_with("160000 ")
    );
    h.git_stdout(&["add", "lib/child"]);
    h.git_stdout(&["commit", "-m", "record nested checkout"]);
    for path in [".", "lib/child", "lib/child/nested/inner"] {
        assert!(
            h.git_stdout(&["-C", path, "status", "--porcelain=v1"])
                .is_empty(),
            "{path} must start clean"
        );
    }
    // Without .gitmodules there is no name lookup for submodule.inner.ignore.
    h.git_stdout(&[
        "-C",
        "lib/child",
        "config",
        "--local",
        "diff.ignoreSubmodules",
        "all",
    ]);
    h.git_stdout(&[
        "-C",
        "lib/child",
        "config",
        "--local",
        "submodule.inner.ignore",
        "all",
    ]);
    let inner_storage = PathBuf::from(h.git_stdout(&[
        "-C",
        "lib/child/nested/inner",
        "rev-parse",
        "--absolute-git-dir",
    ]));
    fs::write(inner_storage.join("info/exclude"), "/precious-ignored\n").unwrap();
    fs::write(
        h.work_dir.join("lib/child/nested/inner/precious-ignored"),
        "irreplaceable ignored bytes\n",
    )
    .unwrap();
    fs::write(
        h.work_dir.join("lib/child/nested/inner/src/main.c"),
        "precious tracked edits\n",
    )
    .unwrap();
    assert_eq!(
        h.git_stdout(&[
            "-C",
            "lib/child/nested/inner",
            "check-ignore",
            "precious-ignored"
        ]),
        "precious-ignored"
    );
    assert!(
        h.git_stdout(&["-C", "lib/child", "status", "--porcelain=v1"])
            .is_empty(),
        "outer status should hide inner changes"
    );
    assert!(
        !h.git_stdout(&["-C", "lib/child/nested/inner", "status", "--porcelain=v1"])
            .is_empty()
    );

    let mut before = std::collections::BTreeMap::new();
    snapshot(&h.work_dir, &mut before);
    let output = h.run_submod(&["nuke-it-from-orbit", "child"]).unwrap();
    assert!(
        !output.status.success(),
        "nested content was discarded: {output:?}"
    );
    let guidance = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(guidance.contains("--force"), "{guidance}");
    let mut after = std::collections::BTreeMap::new();
    snapshot(&h.work_dir, &mut after);
    assert_eq!(
        after.keys().collect::<Vec<_>>(),
        before.keys().collect::<Vec<_>>(),
        "fixture paths changed"
    );
    for (path, expected) in before {
        assert_eq!(after.get(&path).unwrap(), &expected, "changed {path:?}");
    }
}

#[test]
fn r22_config_only_nuke_materializes_inactive_then_rebuilds_pinned_sibling() {
    let h = fixture();
    let storage = add_child(&h);
    let (existing_pin, stash) = local_history(&h);
    let existing_content = fs::read(h.work_dir.join("lib/child/src/main.c")).unwrap();
    let old_remote_work = h.temp_dir.path().join("phase5_work");
    fs::write(
        old_remote_work.join("remote-tip-only"),
        "must not replace parent pin\n",
    )
    .unwrap();
    h.git_at(&old_remote_work, &["add", "remote-tip-only"]);
    h.git_at(
        &old_remote_work,
        &["commit", "-m", "advance remote past recorded pin"],
    );
    h.git_at(&old_remote_work, &["push", "origin", "main"]);
    assert_ne!(
        h.git_at(&old_remote_work, &["rev-parse", "HEAD"]),
        existing_pin
    );

    let remote = h.create_test_remote("config_only_nuke").unwrap();
    let remote_work = h.temp_dir.path().join("config_only_nuke_work");
    let new_pin = h.git_at(&remote_work, &["rev-parse", "HEAD"]);
    let new_content = fs::read(remote_work.join("src/main.c")).unwrap();
    let second_remote = h.create_test_remote("second_config_only_nuke").unwrap();
    let second_work = h.temp_dir.path().join("second_config_only_nuke_work");
    fs::write(
        second_work.join("src/main.c"),
        "distinct second config-only content\n",
    )
    .unwrap();
    h.git_at(&second_work, &["add", "src/main.c"]);
    h.git_at(&second_work, &["commit", "-m", "distinct second pin"]);
    h.git_at(&second_work, &["push", "origin", "main"]);
    let second_pin = h.git_at(&second_work, &["rev-parse", "HEAD"]);
    assert_ne!(second_pin, new_pin);
    let second_content = fs::read(second_work.join("src/main.c")).unwrap();
    let mut config: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
    config
        .get_mut("child")
        .unwrap()
        .as_table_mut()
        .unwrap()
        .insert("ignore".to_owned(), toml::Value::String("dirty".to_owned()));
    let new_entry: toml::Value = toml::from_str(&format!(
        "path = 'lib/new'\nurl = 'file://{}'\nactive = false\nupdate = 'none'\nshallow = false\n",
        remote.display()
    ))
    .unwrap();
    config
        .as_table_mut()
        .unwrap()
        .insert("new".to_owned(), new_entry);
    let second_entry: toml::Value = toml::from_str(&format!("path = 'lib/second'\nurl = 'file://{}'\nactive = false\nupdate = 'none'\nshallow = false\n", second_remote.display())).unwrap();
    config
        .as_table_mut()
        .unwrap()
        .insert("second".to_owned(), second_entry);
    h.create_config(&toml::to_string(&config).unwrap()).unwrap();
    for (key, value) in [("ignore", "dirty"), ("shallow", "false")] {
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            &format!("submodule.child.{key}"),
            value,
        ]);
    }
    h.git_stdout(&["add", ".gitmodules"]);
    let raw = h.read_config().unwrap();
    assert!(!h.work_dir.join("lib/new").exists());
    assert!(!h.work_dir.join(".git/modules/new").exists());
    assert_eq!(h.index_gitlink_mode("lib/new"), None);
    assert!(!h.work_dir.join("lib/second").exists());
    assert!(!h.work_dir.join(".git/modules/second").exists());
    assert_eq!(h.index_gitlink_mode("lib/second"), None);
    fs::write(h.work_dir.join("unrelated-staged"), "keep staged\n").unwrap();
    h.git_stdout(&["add", "unrelated-staged"]);
    fs::write(
        h.work_dir.join("unrelated-unstaged"),
        "keep outside checkout\n",
    )
    .unwrap();
    h.git_stdout(&["config", "--add", "phase5.unrelated", "first"]);
    h.git_stdout(&["config", "--add", "phase5.unrelated", "second"]);
    let parent_refs = h.git_stdout(&["show-ref"]);
    let before_index = h.git_stdout(&["ls-files", "--stage"]);

    h.run_submod_success(&["nuke-it-from-orbit", "new", "second", "child"])
        .unwrap();
    assert_eq!(h.read_config().unwrap(), raw);
    assert_eq!(
        h.git_stdout(&["-C", "lib/new", "rev-parse", "HEAD"]),
        new_pin
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "lib/new"]),
        format!("160000 {new_pin} 0\tlib/new")
    );
    assert_eq!(
        fs::read(h.work_dir.join("lib/new/src/main.c")).unwrap(),
        new_content
    );
    assert!(
        h.git_stdout(&["-C", "lib/new", "status", "--porcelain=v1"])
            .is_empty()
    );
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get",
            "submodule.new.path"
        ]),
        "lib/new"
    );
    assert_eq!(
        h.git_stdout(&["config", "--local", "--get", "submodule.new.active"]),
        "false"
    );
    for scope in [vec!["--local"], vec!["--file", ".gitmodules"]] {
        let mut args = vec!["config"];
        args.extend(scope);
        args.extend(["--get", "submodule.new.update"]);
        assert_eq!(h.git_stdout(&args), "none");
    }
    assert_eq!(
        h.git_stdout(&["-C", "lib/second", "rev-parse", "HEAD"]),
        second_pin
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "lib/second"]),
        format!("160000 {second_pin} 0\tlib/second")
    );
    assert_eq!(
        fs::read(h.work_dir.join("lib/second/src/main.c")).unwrap(),
        second_content
    );
    assert!(
        h.git_stdout(&["-C", "lib/second", "status", "--porcelain=v1"])
            .is_empty()
    );
    assert_eq!(
        h.git_stdout(&["config", "--local", "--get", "submodule.second.active"]),
        "false"
    );
    assert_eq!(
        h.git_stdout(&["config", "--local", "--get", "submodule.second.update"]),
        "none"
    );
    for (name, path) in [("new", "lib/new"), ("second", "lib/second")] {
        for (key, value) in [("path", path), ("update", "none"), ("shallow", "false")] {
            assert_eq!(
                h.git_stdout(&[
                    "config",
                    "--blob",
                    ":.gitmodules",
                    "--get",
                    &format!("submodule.{name}.{key}")
                ]),
                value
            );
        }
    }
    assert!(h.git_stdout(&["diff", "--", ".gitmodules"]).is_empty());
    assert_eq!(
        h.git_stdout(&["-C", "lib/child", "rev-parse", "HEAD"]),
        existing_pin
    );
    assert_eq!(
        fs::read(h.work_dir.join("lib/child/src/main.c")).unwrap(),
        existing_content
    );
    assert!(!h.work_dir.join("lib/child/remote-tip-only").exists());
    assert_eq!(
        h.git_stdout(&["config", "--local", "--get", "submodule.child.ignore"]),
        "dirty"
    );
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get",
            "submodule.child.ignore"
        ]),
        "dirty"
    );
    assert_history(&h, &storage, &existing_pin, &stash);
    assert_eq!(h.git_stdout(&["show-ref"]), parent_refs);
    let after_index = h.git_stdout(&["ls-files", "--stage"]);
    let untouched_entries = |index: &str| {
        index
            .lines()
            .filter(|line| {
                !line.ends_with("\t.gitmodules")
                    && !line.ends_with("\tlib/new")
                    && !line.ends_with("\tlib/second")
            })
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        untouched_entries(&after_index),
        untouched_entries(&before_index)
    );
    assert_eq!(
        h.git_stdout(&["config", "--get-all", "phase5.unrelated"]),
        "first\nsecond"
    );
    assert_eq!(
        fs::read_to_string(h.work_dir.join("unrelated-staged")).unwrap(),
        "keep staged\n"
    );
    assert_eq!(
        fs::read_to_string(h.work_dir.join("unrelated-unstaged")).unwrap(),
        "keep outside checkout\n"
    );
}
