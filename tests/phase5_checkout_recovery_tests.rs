// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
//! Checkout and recovery checks against local native Git fixtures.
mod common;

use common::TestHarness;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

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

fn snapshot(root: &Path) -> Vec<(String, Vec<u8>)> {
    fn visit(root: &Path, dir: &Path, result: &mut Vec<(String, Vec<u8>)>) {
        let mut entries: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned();
            if path.is_dir() {
                result.push((format!("{name}/"), Vec::new()));
                visit(root, &path, result);
            } else {
                result.push((name, fs::read(path).unwrap()));
            }
        }
    }
    let mut result = Vec::new();
    visit(root, root, &mut result);
    result
}

fn unchanged(before: Vec<(String, Vec<u8>)>, root: &Path) {
    let after = snapshot(root);
    assert_eq!(before.len(), after.len(), "filesystem entry count changed");
    for ((path, bytes), (new_path, new_bytes)) in before.into_iter().zip(after) {
        assert_eq!(path, new_path);
        assert!(bytes == new_bytes, "bytes changed: {path}");
    }
}

fn clone_parent(h: &TestHarness, source: &Path, destination: &Path) {
    success(
        &h.git_cmd()
            .arg("clone")
            .arg(source)
            .arg(destination)
            .output()
            .unwrap(),
    );
}

fn traced_sync(h: &TestHarness, cwd: &Path) {
    let trace = h.temp_dir.path().join("repeat-trace.jsonl");
    success(
        &Command::new(&h.submod_bin)
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
    assert!(!events.is_empty(), "Git trace must be enabled");
    assert!(
        !events.contains("\"fetch\""),
        "unchanged sync fetched: {events}"
    );
}

#[test]
fn r20_phase5_relative_url_fresh_clone_preserves_pin_and_native_url_resolution() {
    let h = fixture();
    let remote = h.create_test_remote("relative-child").unwrap();
    let parent_remote = h.temp_dir.path().join("parent.git");
    h.git_stdout(&["remote", "add", "origin", parent_remote.to_str().unwrap()]);
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "logical",
        "../relative-child.git",
        "library",
    ]);
    let pin = h.git_at(&h.work_dir.join("library"), &["rev-parse", "HEAD"]);
    h.create_config("[library]\nurl = \"../relative-child.git\"\n")
        .unwrap();
    h.git_stdout(&["add", ".gitmodules", "library", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "Record relative submodule pin"]);
    success(
        &h.git_cmd()
            .args(["clone", "--bare"])
            .arg(&h.work_dir)
            .arg(&parent_remote)
            .output()
            .unwrap(),
    );
    assert_ne!(h.advance_test_remote("relative-child").unwrap(), pin);
    let fresh = h.temp_dir.path().join("fresh");
    let oracle = h.temp_dir.path().join("native-oracle");
    clone_parent(&h, &parent_remote, &fresh);
    clone_parent(&h, &parent_remote, &oracle);
    h.git_at(&oracle, &["submodule", "update", "--init", "--", "library"]);
    let native_parent_url = h.git_at(&oracle, &["config", "--local", "submodule.logical.url"]);
    let native_child_url = h.git_at(&oracle.join("library"), &["remote", "get-url", "origin"]);
    assert_eq!(
        Path::new(&native_parent_url).canonicalize().unwrap(),
        remote.canonicalize().unwrap()
    );
    let portable_before = fs::read(fresh.join(".gitmodules")).unwrap();
    let gitlinks_before = h.git_at(&fresh, &["ls-files", "--stage", "library"]);
    success(&h.run_submod_at(&fresh, &["sync"]).unwrap());
    assert_eq!(
        h.git_at(&fresh.join("library"), &["rev-parse", "HEAD"]),
        pin
    );
    assert!(fresh.join("library/src/main.c").is_file());
    assert!(!fresh.join("library/ADVANCE.txt").exists());
    assert_eq!(
        h.git_at(&fresh, &["config", "--local", "submodule.logical.url"]),
        native_parent_url
    );
    assert_eq!(
        h.git_at(&fresh.join("library"), &["remote", "get-url", "origin"]),
        native_child_url
    );
    assert_eq!(
        fs::read(fresh.join(".gitmodules")).unwrap(),
        portable_before
    );
    assert_eq!(
        h.git_at(&fresh, &["ls-files", "--stage", "library"]),
        gitlinks_before
    );
    let before = snapshot(&fresh);
    traced_sync(&h, &fresh);
    unchanged(before, &fresh);
}

fn shallow_pin(unavailable: bool) {
    let h = fixture();
    let remote = h.create_test_remote("shallow-child").unwrap();
    let url = format!("file://{}", remote.display());
    h.git_stdout(&["submodule", "add", "--name", "library", &url, "library"]);
    let pin = if unavailable {
        "1111111111111111111111111111111111111111".to_owned()
    } else {
        h.git_at(&h.work_dir.join("library"), &["rev-parse", "HEAD~1"])
    };
    if unavailable {
        let missing = h
            .git_cmd()
            .args(["cat-file", "-e", &pin])
            .current_dir(remote.as_ref())
            .output()
            .unwrap();
        assert!(!missing.status.success(), "fixture pin unexpectedly exists");
    }
    h.git_stdout(&[
        "update-index",
        "--cacheinfo",
        &format!("160000,{pin},library"),
    ]);
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.library.shallow",
        "true",
    ]);
    h.create_config(&format!("[library]\nurl = {url:?}\nshallow = true\n"))
        .unwrap();
    h.git_stdout(&["add", ".gitmodules", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "Record shallow checkout pin"]);
    let tip = h.advance_test_remote("shallow-child").unwrap();
    assert_ne!(tip, pin);
    let fresh = h.temp_dir.path().join("fresh-shallow");
    clone_parent(&h, &h.work_dir, &fresh);
    fs::write(fresh.join("KEEP.txt"), "pre-existing user bytes\n").unwrap();
    let preserved_paths = ["submod.toml", ".gitmodules", ".git/index", "KEEP.txt"];
    let preserved: Vec<_> = preserved_paths
        .iter()
        .map(|path| fs::read(fresh.join(path)).unwrap())
        .collect();
    let parent_refs = h.git_at(&fresh, &["show-ref"]);
    let index_before = h.git_at(&fresh, &["ls-files", "--stage", "library"]);
    assert_eq!(index_before, format!("160000 {pin} 0\tlibrary"));
    let output = h.run_submod_at(&fresh, &["init"]).unwrap();
    if !unavailable {
        assert!(
            output.status.success(),
            "available old pin must initialize on the first attempt: {}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    if output.status.success() {
        assert!(!unavailable, "init accepted a nonexistent recorded commit");
        assert_eq!(
            h.git_at(&fresh.join("library"), &["rev-parse", "HEAD"]),
            pin
        );
        assert_eq!(
            h.git_at(
                &fresh.join("library"),
                &["rev-parse", "--is-shallow-repository"]
            ),
            "true"
        );
        assert!(fresh.join("library/src/main.c").is_file());
        assert!(!fresh.join("library/ADVANCE.txt").exists());
        assert_eq!(
            h.git_at(&fresh, &["ls-files", "--stage", "library"]),
            index_before
        );
    } else {
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            text.contains(&pin),
            "refusal must identify missing recorded commit {pin}: {text}"
        );
        assert!(
            text.to_lowercase().contains("commit") || text.to_lowercase().contains("reference"),
            "refusal must explain commit failure: {text}"
        );
        for (path, before) in preserved_paths.iter().zip(&preserved) {
            assert_eq!(
                &fs::read(fresh.join(path)).unwrap(),
                before,
                "changed prior state: {path}"
            );
        }
        assert_eq!(h.git_at(&fresh, &["show-ref"]), parent_refs);
        assert!(!text.to_lowercase().contains("initialized successfully"));
        assert!(
            !fresh.join("library/ADVANCE.txt").exists(),
            "remote-tip content must not stand in for the missing pin"
        );
        let gitdir = fresh.join(".git/modules/library");
        if gitdir.exists() {
            assert!(
                gitdir
                    .canonicalize()
                    .unwrap()
                    .starts_with(fresh.join(".git").canonicalize().unwrap())
            );
            h.git_at(&gitdir, &["rev-parse", "--git-dir"]);
            let missing = h
                .git_cmd()
                .args(["cat-file", "-e", &pin])
                .current_dir(&gitdir)
                .output()
                .unwrap();
            assert!(
                !missing.status.success(),
                "missing target unexpectedly became available"
            );
            let worktree = h.git_at(&gitdir, &["config", "--local", "core.worktree"]);
            assert_eq!(
                gitdir.join(worktree).canonicalize().unwrap(),
                fresh.join("library").canonicalize().unwrap()
            );
        }
        // Correcting the unavailable declaration's gitlink must allow the same
        // retained repository to finish initialization without manual cleanup.
        let repair_pin = h.git_at(remote.as_ref(), &["rev-parse", "HEAD~2"]);
        h.git_at(
            &fresh,
            &[
                "update-index",
                "--cacheinfo",
                &format!("160000,{repair_pin},library"),
            ],
        );
        success(&h.run_submod_at(&fresh, &["init"]).unwrap());
        assert_eq!(
            h.git_at(&fresh.join("library"), &["rev-parse", "HEAD"]),
            repair_pin
        );
        assert!(fresh.join("library/src/main.c").is_file());
        assert!(!fresh.join("library/ADVANCE.txt").exists());
        assert_eq!(h.git_at(&fresh, &["show-ref"]), parent_refs);
        assert_eq!(
            fs::read_to_string(fresh.join("KEEP.txt")).unwrap(),
            "pre-existing user bytes\n"
        );
    }
}

#[test]
fn r20_phase5_shallow_older_recorded_pin_never_substitutes_remote_tip() {
    shallow_pin(false);
}

#[test]
fn r20_phase5_shallow_unavailable_pin_refuses_with_exact_oid_and_preserves_state() {
    shallow_pin(true);
}

fn broken_gitfile(contents: &str) {
    let h = fixture();
    let remote = h.create_test_remote("broken-pointer").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "library",
        remote.to_str().unwrap(),
        "library",
    ]);
    h.create_config(&format!(
        "[library]\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    h.git_stdout(&["add", ".gitmodules", "library", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "Record valid module"]);
    let child = h.work_dir.join("library");
    h.git_at(&child, &["branch", "precious-local"]);
    fs::write(child.join("LOCAL.txt"), "preserve user content\n").unwrap();
    fs::write(child.join(".git"), contents).unwrap();
    let before = snapshot(&h.work_dir);
    for command in ["init", "update", "sync"] {
        let output = h.run_submod(&[command]).unwrap();
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .to_lowercase();
        assert!(
            !output.status.success(),
            "{command} accepted broken gitfile: {text}"
        );
        unchanged(before.clone(), &h.work_dir);
        assert!(
            text.contains("library")
                && (text.contains("gitfile")
                    || text.contains("gitdir")
                    || text.contains("pointer")
                    || text.contains(".git")),
            "{command} lacks targeted pointer diagnosis: {text}"
        );
    }
}

#[test]
fn r21_phase5_malformed_child_gitfile_refuses_without_mutation() {
    broken_gitfile("this is not a Git pointer\n");
}

#[test]
fn r21_phase5_nonexistent_child_gitfile_target_refuses_without_mutation() {
    broken_gitfile("gitdir: ../.git/modules/nonexistent\n");
}
