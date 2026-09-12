// SPDX-FileCopyrightText: 2026 Adam Poulemanos and contributors
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Reset/batch regression tests

mod common;
use common::TestHarness;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn bytes(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, at: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(at).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                walk(root, &entry.path(), result);
            } else {
                result.insert(
                    entry.path().strip_prefix(root).unwrap().to_owned(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    walk(root, root, &mut result);
    result
}

fn assert_tree_eq(
    actual: &BTreeMap<PathBuf, Vec<u8>>,
    expected: &BTreeMap<PathBuf, Vec<u8>>,
    context: &str,
) {
    let changed: std::collections::BTreeSet<_> = actual
        .keys()
        .chain(expected.keys())
        .filter(|path| actual.get(*path) != expected.get(*path))
        .collect();
    assert!(changed.is_empty(), "{context}; changed paths: {changed:?}");
}

fn fixture(initialized: bool) -> TestHarness {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("batch").unwrap();
    let mut config = String::new();
    // Deliberately declare out of order: application order must be sorted.
    for name in ["gamma", "beta", "alpha"] {
        use std::fmt::Write as _;
        let _ = writeln!(
            config,
            "[{name}]\npath = {name:?}\nurl = {:?}",
            remote.to_str().unwrap()
        );
        if initialized {
            h.git_stdout(&[
                "submodule",
                "add",
                "--name",
                name,
                remote.to_str().unwrap(),
                name,
            ]);
        }
    }
    h.create_config(&config).unwrap();
    h.git_stdout(&["add", "."]);
    h.git_stdout(&["commit", "-m", "batch declarations and pins"]);
    h
}

fn command(h: &TestHarness, args: &[&str]) -> Command {
    let mut cmd = Command::new(&h.submod_bin);
    for (key, value) in h.git_cmd().get_envs() {
        if let Some(value) = value {
            cmd.env(key, value);
        } else {
            cmd.env_remove(key);
        }
    }
    cmd.current_dir(&h.work_dir).args(args);
    cmd
}

fn text(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn dirty(h: &TestHarness, name: &str) {
    let child = h.work_dir.join(name);
    fs::write(child.join("LICENSE"), b"staged\0\xff").unwrap();
    h.git_at(&child, &["add", "LICENSE"]);
    fs::write(child.join("LICENSE"), b"unstaged\0\xff").unwrap();
    fs::write(child.join("untracked"), b"untracked\0\xff").unwrap();
}

fn execute_advertised_recovery(h: &TestHarness, child: &Path, output: &Output, stash: &str) {
    let printed = text(output);
    // The product prints the canonical checkout directory, which can differ
    // textually from the fixture spelling (verbatim UNC and 8.3 short names
    // on Windows); compare canonical forms on every platform.
    let displayed = std::fs::canonicalize(child).unwrap();
    assert!(
        printed.contains(displayed.to_str().unwrap()),
        "display the recovery directory: {printed}"
    );
    let line = printed
        .lines()
        .find(|line| line.contains("git stash "))
        .expect("advertised recovery command");
    let start = line.find("git stash ").unwrap();
    let args: Vec<&str> = line[start..].split_whitespace().collect();
    assert_eq!(
        args.len(),
        5,
        "command must be path-free and require no shell quoting: {line}"
    );
    assert_eq!(args[0], "git");
    assert_eq!(args[1], "stash");
    assert_eq!(args[4], stash, "recover the exact created stash object");
    match args[2] {
        "branch" => {
            let branch = args[3];
            assert!(
                branch
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_alphanumeric)
                    && branch
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
                "generated recovery branch must be shell-safe: {branch:?}"
            );
            h.git_at(child, &["check-ref-format", "--branch", branch]);
        }
        "apply" => assert_eq!(args[3], "--index", "recovery must restore staged changes"),
        other => panic!("unrecognized advertised recovery command: {other}"),
    }
    // Execute only the actual advertised argument tokens, directly in the pinned
    // child. No fixture checkout/worktree or hidden base restoration is allowed.
    h.git_at(child, &args[1..]);
    if args[2] == "branch" {
        assert_eq!(
            h.git_at(child, &["symbolic-ref", "--short", "HEAD"]),
            args[3]
        );
    }
    let staged = h
        .git_cmd()
        .args(["show", ":LICENSE"])
        .current_dir(child)
        .output()
        .unwrap();
    assert!(staged.status.success(), "{}", text(&staged));
    assert_eq!(staged.stdout, b"staged\0\xff");
}

#[test]
fn reset_reports_exact_stash_oid_and_usable_recovery_command() {
    let h = fixture(true);
    let child = h.work_dir.join("alpha");
    let pin = h.git_stdout(&["rev-parse", "HEAD:alpha"]);
    fs::write(child.join("local-only"), b"local history").unwrap();
    fs::write(child.join("LICENSE"), b"different committed base\n").unwrap();
    h.git_at(&child, &["add", "local-only", "LICENSE"]);
    h.git_at(&child, &["commit", "-m", "ahead of parent pin"]);
    let original_head = h.git_at(&child, &["rev-parse", "HEAD"]);
    assert_ne!(original_head, pin);
    dirty(&h, "alpha");
    let original_index = h.git_at(&child, &["ls-files", "--stage"]);
    let parent = h.preservation_snapshot();
    let output = h.run_submod(&["reset", "alpha"]).unwrap();
    assert!(output.status.success(), "{}", text(&output));
    let stash = h.git_at(&child, &["rev-parse", "refs/stash"]);
    assert_eq!(h.git_at(&child, &["cat-file", "-t", &stash]), "commit");
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), pin);
    assert_eq!(h.preservation_snapshot(), parent);
    assert_eq!(
        h.git_at(&child, &["rev-parse", &format!("{stash}^1")]),
        original_head
    );
    assert_eq!(
        h.git_at(&child, &["rev-parse", &format!("{original_head}^1")]),
        pin
    );
    execute_advertised_recovery(&h, &child, &output, &stash);
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), original_head);
    assert_eq!(h.git_at(&child, &["ls-files", "--stage"]), original_index);
    assert_eq!(fs::read(child.join("LICENSE")).unwrap(), b"unstaged\0\xff");
    assert_eq!(
        fs::read(child.join("untracked")).unwrap(),
        b"untracked\0\xff"
    );
    assert_eq!(
        fs::read(child.join("local-only")).unwrap(),
        b"local history"
    );
    assert_eq!(h.git_at(&child, &["cat-file", "-t", &stash]), "commit");
    assert_eq!(h.preservation_snapshot(), parent);
}

fn reset_batch_refuses_later(kind: &str) {
    let h = fixture(true);
    dirty(&h, "alpha");
    let later = h.work_dir.join("beta");
    let gitdir = PathBuf::from(h.git_at(&later, &["rev-parse", "--absolute-git-dir"]));
    if kind == "lock" {
        fs::write(gitdir.join("refs/stash.lock"), b"held fixture lock").unwrap();
    } else {
        let old = h.git_at(&later, &["rev-parse", "HEAD"]);
        fs::write(later.join("collision"), b"target tracked file").unwrap();
        h.git_at(&later, &["add", "collision"]);
        h.git_at(&later, &["commit", "-m", "new parent target"]);
        let target = h.git_at(&later, &["rev-parse", "HEAD"]);
        h.git_stdout(&["add", "beta"]);
        h.git_stdout(&["commit", "-m", "record later pin"]);
        assert_eq!(h.git_stdout(&["rev-parse", "HEAD:beta"]), target);
        h.git_at(&later, &["checkout", "--detach", &old]);
        fs::write(gitdir.join("info/exclude"), "/collision\n").unwrap();
        if kind == "ignored" {
            fs::write(later.join("collision"), b"priceless\0\xff").unwrap();
        } else {
            let nested = later.join("collision");
            fs::create_dir(&nested).unwrap();
            h.git_at(&nested, &["init", "-b", "main"]);
            fs::write(nested.join("sentinel"), b"nested history\0\xff").unwrap();
            h.git_at(&nested, &["add", "."]);
            h.git_at(&nested, &["commit", "-m", "nested history"]);
        }
    }
    let before = bytes(&h.work_dir);
    let index = h.git_stdout(&["ls-files", "--stage"]);
    let output = h.run_submod(&["reset", "alpha", "beta", "gamma"]).unwrap();
    assert!(!output.status.success(), "{}", text(&output));
    assert_tree_eq(
        &bytes(&h.work_dir),
        &before,
        &format!(
            "later {kind} must prevent first stash/reset: {}",
            text(&output)
        ),
    );
    assert_eq!(h.git_stdout(&["ls-files", "--stage"]), index);
}
#[test]
fn reset_batch_preflights_later_ignored_collision() {
    reset_batch_refuses_later("ignored");
}
#[test]
fn reset_batch_preflights_later_nested_repository_collision() {
    reset_batch_refuses_later("nested");
}
#[test]
fn reset_batch_preflights_later_stash_lock() {
    reset_batch_refuses_later("lock");
}

#[test]
fn nuke_all_preflights_later_dirty_target_before_any_removal() {
    let h = fixture(true);
    dirty(&h, "gamma");
    let before = bytes(&h.work_dir);
    let parent = h.preservation_snapshot();
    let output = h.run_submod(&["nuke-it-from-orbit", "--all"]).unwrap();
    assert!(!output.status.success(), "{}", text(&output));
    assert_tree_eq(
        &bytes(&h.work_dir),
        &before,
        &format!(
            "nuke preflight must preserve every target: {}",
            text(&output)
        ),
    );
    assert_eq!(h.preservation_snapshot(), parent);
}

#[cfg(unix)]
fn wrapper(h: &TestHarness, mode: &str, args: &[&str]) -> Command {
    use std::os::unix::fs::PermissionsExt;
    let found = Command::new("which").arg("git").output().unwrap();
    assert!(found.status.success());
    let real = String::from_utf8(found.stdout).unwrap().trim().to_owned();
    let bin = h.temp_dir.path().join("wrapper");
    fs::create_dir_all(&bin).unwrap();
    fs::write(
        bin.join("git"),
        r"#!/usr/bin/env python3
import os,sys,subprocess,time
args=sys.argv[1:]
root=os.environ['FIXTURE_CONTROL']
mode=os.environ['FIXTURE_MODE']
is_add='submodule' in args and 'add' in args
is_beta=any(x in ('beta', ':(literal)beta') for x in args)
is_deinit='submodule' in args and 'deinit' in args
is_update='submodule' in args and 'update' in args
if mode=='fail' and is_deinit:
    with open(root+'/deinitialized','a') as log: log.write(args[-1]+'\n')
if mode=='fail' and is_update and is_beta:
    sys.stderr.write('fixture local rebuild failure\n'); sys.exit(71)
result=subprocess.run([os.environ['FIXTURE_REAL_GIT']]+args)
if mode=='pause' and is_add and is_beta and result.returncode==0:
    parent=os.getppid()
    with open(root+'/paused','w') as marker: marker.write(str(os.getpid()))
    deadline=time.monotonic()+30
    while not os.path.exists(root+'/release') and time.monotonic()<deadline: time.sleep(.02)
    with open(root+'/wrapper-finished','w') as marker: marker.write('done')
    if os.getppid()!=parent: sys.exit(143)
sys.exit(result.returncode)
",
    )
    .unwrap();
    fs::set_permissions(bin.join("git"), fs::Permissions::from_mode(0o755)).unwrap();
    let mut paths = vec![bin];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap()));
    let mut cmd = command(h, args);
    cmd.env("PATH", std::env::join_paths(paths).unwrap())
        .env("FIXTURE_REAL_GIT", real)
        .env("FIXTURE_CONTROL", h.temp_dir.path())
        .env("FIXTURE_MODE", mode);
    cmd
}

#[cfg(unix)]
fn nuke_runtime_failure(args: &[&str]) {
    let h = fixture(true);
    let declarations = fs::read(h.config_path()).unwrap();
    let gamma = bytes(&h.work_dir.join("gamma"));
    let gamma_index = h.git_stdout(&["ls-files", "--stage", "--", "gamma"]);
    let alpha_pin = h.git_at(&h.work_dir.join("alpha"), &["rev-parse", "HEAD"]);
    let beta_pin = h.git_at(&h.work_dir.join("beta"), &["rev-parse", "HEAD"]);
    let beta_refs = h.git_at(&h.work_dir.join("beta"), &["show-ref"]);
    let parent_index = h.git_stdout(&["ls-files", "--stage"]);
    let gitmodules = fs::read(h.work_dir.join(".gitmodules")).unwrap();
    let alpha_bytes = bytes(&h.work_dir.join("alpha"));
    let output = wrapper(&h, "fail", args).output().unwrap();
    assert!(!output.status.success(), "{}", text(&output));
    let deinitialized = fs::read_to_string(h.temp_dir.path().join("deinitialized")).unwrap();
    assert_eq!(
        deinitialized
            .lines()
            .map(|line| line.trim_start_matches(":(literal)"))
            .collect::<Vec<_>>(),
        ["alpha", "beta"],
        "sorted deterministic order"
    );
    assert_eq!(fs::read(h.config_path()).unwrap(), declarations);
    assert_eq!(
        h.git_at(&h.work_dir.join("alpha"), &["rev-parse", "HEAD"]),
        alpha_pin
    );
    assert_eq!(bytes(&h.work_dir.join("gamma")), gamma);
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "gamma"]),
        gamma_index
    );
    assert_tree_eq(
        &bytes(&h.work_dir.join("alpha")),
        &alpha_bytes,
        "completed alpha must be rebuilt at its original pin",
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage"]),
        parent_index,
        "rebuild must retain original gitlinks"
    );
    assert_eq!(
        fs::read(h.work_dir.join(".gitmodules")).unwrap(),
        gitmodules
    );
    assert!(
        !h.work_dir.join("beta/LICENSE").exists(),
        "failure occurs after beta deinit, before its update"
    );
    let retained = h.git_stdout(&[
        "rev-parse",
        "--path-format=absolute",
        "--git-path",
        "modules/beta",
    ]);
    assert_eq!(
        h.git_stdout(&[
            "--git-dir",
            &retained,
            "--work-tree",
            h.work_dir.to_str().unwrap(),
            "cat-file",
            "-t",
            &beta_pin
        ]),
        "commit"
    );
    assert_eq!(
        h.git_stdout(&[
            "--git-dir",
            &retained,
            "--work-tree",
            h.work_dir.to_str().unwrap(),
            "show-ref"
        ]),
        beta_refs
    );
    let printed = text(&output);
    for expected in ["completed: alpha", "failed: beta", "pending: gamma"] {
        assert!(printed.contains(expected), "missing {expected}: {printed}");
    }
}

#[cfg(unix)]
#[test]
fn nuke_all_sorted_order_reports_completed_failed_pending() {
    nuke_runtime_failure(&["nuke-it-from-orbit", "--all"]);
}

#[cfg(unix)]
#[test]
fn nuke_runtime_failure_preserves_state_and_reports_outcome() {
    nuke_runtime_failure(&["nuke-it-from-orbit", "alpha", "beta", "gamma"]);
}

#[cfg(unix)]
#[test]
fn interrupted_init_retains_partial_state_and_owned_lock_recovery_converges() {
    use std::{
        thread,
        time::{Duration, Instant},
    };
    let h = fixture(false);
    let declarations = fs::read(h.config_path()).unwrap();
    let common =
        PathBuf::from(h.git_stdout(&["rev-parse", "--path-format=absolute", "--git-common-dir"]));
    let locks = [
        common.join("submod.lock"),
        h.work_dir.join("submod.toml.submod.lock"),
    ];
    assert!(
        locks.iter().all(|path| !path.exists()),
        "fixture owns no pre-existing locks"
    );
    let log = fs::File::create(h.temp_dir.path().join("interrupted.log")).unwrap();
    let mut cmd = wrapper(&h, "pause", &["init"]);
    cmd.stdout(log.try_clone().unwrap()).stderr(log);
    let mut child = cmd.spawn().unwrap();
    let paused = h.temp_dir.path().join("paused");
    let deadline = Instant::now() + Duration::from_secs(25);
    while !paused.exists() && Instant::now() < deadline {
        if let Some(status) = child.try_wait().unwrap() {
            panic!(
                "CLI exited before deterministic pause: {status}; {}",
                fs::read_to_string(h.temp_dir.path().join("interrupted.log")).unwrap()
            );
        }
        thread::sleep(Duration::from_millis(20));
    }
    if !paused.exists() {
        child.kill().unwrap();
        child.wait().unwrap();
        panic!("wrapper never reached pause");
    }
    child.kill().unwrap();
    assert!(!child.wait().unwrap().success());
    fs::write(
        h.temp_dir.path().join("release"),
        b"release orphan wrapper without further Git",
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !h.temp_dir.path().join("wrapper-finished").exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(h.temp_dir.path().join("wrapper-finished").exists());
    assert_eq!(fs::read(h.config_path()).unwrap(), declarations);
    let alpha = h.git_at(&h.work_dir.join("alpha"), &["rev-parse", "HEAD"]);
    let beta = h.git_at(&h.work_dir.join("beta"), &["rev-parse", "HEAD"]);
    assert_eq!(h.index_gitlink_mode("alpha").as_deref(), Some("160000"));
    assert_eq!(h.index_gitlink_mode("beta").as_deref(), Some("160000"));
    assert!(!h.work_dir.join("gamma/LICENSE").exists());
    let owned: Vec<_> = locks
        .iter()
        .map(|path| {
            (
                path.clone(),
                fs::read(path).expect("interrupted operation retains its lock"),
            )
        })
        .collect();
    let alpha_refs = h.git_at(&h.work_dir.join("alpha"), &["show-ref"]);
    let beta_refs = h.git_at(&h.work_dir.join("beta"), &["show-ref"]);
    let completed_index = h.git_stdout(&["ls-files", "--stage", "--", "alpha", "beta"]);
    let partial = bytes(&h.work_dir);
    let blocked = h.run_submod(&["init"]).unwrap();
    assert!(!blocked.status.success());
    assert_tree_eq(
        &bytes(&h.work_dir),
        &partial,
        "blocked retry must preserve partial state",
    );
    assert!(text(&blocked).to_lowercase().contains("lock"));
    // The only removed files are known locks absent before our now-dead process.
    for (path, expected) in owned {
        assert_eq!(fs::read(&path).unwrap(), expected);
        fs::remove_file(path).unwrap();
    }
    h.run_submod_success(&["init"]).unwrap();
    assert_eq!(fs::read(h.config_path()).unwrap(), declarations);
    assert_eq!(
        h.git_at(&h.work_dir.join("alpha"), &["rev-parse", "HEAD"]),
        alpha
    );
    assert_eq!(
        h.git_at(&h.work_dir.join("beta"), &["rev-parse", "HEAD"]),
        beta
    );
    for name in ["alpha", "beta", "gamma"] {
        assert!(h.work_dir.join(name).join("LICENSE").is_file());
        assert_eq!(h.index_gitlink_mode(name).as_deref(), Some("160000"));
    }
    assert_eq!(
        h.git_at(&h.work_dir.join("alpha"), &["show-ref"]),
        alpha_refs
    );
    assert_eq!(h.git_at(&h.work_dir.join("beta"), &["show-ref"]), beta_refs);
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "alpha", "beta"]),
        completed_index
    );
    let converged = h.preservation_snapshot();
    h.run_submod_success(&["init"]).unwrap();
    assert_eq!(h.preservation_snapshot(), converged);
    assert!(locks.iter().all(|path| !path.exists()));
}

#[test]
fn duplicate_reset_names_refuse_before_cli_or_manager_mutation() {
    if std::env::var_os("SUBMOD_DUPLICATE_RESET_CHILD").is_some() {
        let mut manager =
            submod::git_manager::GitManager::new(PathBuf::from("submod.toml")).unwrap();
        let result = manager.reset_submodules(false, vec!["alpha".to_owned(), "alpha".to_owned()]);
        assert!(
            result.is_err(),
            "shared manager boundary must reject duplicate reset names"
        );
        return;
    }
    let h = fixture(true);
    dirty(&h, "alpha");
    let before = bytes(&h.work_dir);
    let output = h.run_submod(&["reset", "alpha", "alpha"]).unwrap();
    assert!(!output.status.success(), "{}", text(&output));
    assert_tree_eq(
        &bytes(&h.work_dir),
        &before,
        "duplicate CLI names must not mutate",
    );

    // Exercise the public manager in a fixture-rooted subprocess, bypassing clap
    // without changing this test process's cwd or environment.
    let mut child = Command::new(std::env::current_exe().unwrap());
    for (key, value) in h.git_cmd().get_envs() {
        if let Some(value) = value {
            child.env(key, value);
        } else {
            child.env_remove(key);
        }
    }
    let output = child
        .current_dir(&h.work_dir)
        .env("SUBMOD_DUPLICATE_RESET_CHILD", "1")
        .args([
            "--exact",
            "duplicate_reset_names_refuse_before_cli_or_manager_mutation",
            "--nocapture",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "manager child assertion failed: {}",
        text(&output)
    );
    assert_tree_eq(
        &bytes(&h.work_dir),
        &before,
        "duplicate manager names must not mutate",
    );
}

#[test]
fn spaced_module_path_reports_executable_path_free_recovery_command() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("spaced-recovery").unwrap();
    let path = "vendor/module with spaces";
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "spaced",
        remote.to_str().unwrap(),
        path,
    ]);
    h.create_config(&format!(
        "[spaced]\npath = {path:?}\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    h.git_stdout(&["add", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "record spaced module pin"]);
    let child = h.work_dir.join(path);
    let pin = h.git_stdout(&["rev-parse", &format!("HEAD:{path}")]);
    fs::write(child.join("local-only"), b"spaced local history").unwrap();
    fs::write(child.join("LICENSE"), b"different spaced committed base\n").unwrap();
    h.git_at(&child, &["add", "local-only", "LICENSE"]);
    h.git_at(&child, &["commit", "-m", "spaced module ahead of pin"]);
    let original_head = h.git_at(&child, &["rev-parse", "HEAD"]);
    assert_ne!(original_head, pin);
    dirty(&h, path);
    let original_index = h.git_at(&child, &["ls-files", "--stage"]);
    let parent = h.preservation_snapshot();
    let output = h.run_submod(&["reset", "spaced"]).unwrap();
    assert!(output.status.success(), "{}", text(&output));
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), pin);
    assert_eq!(h.preservation_snapshot(), parent);
    let stash = h.git_at(&child, &["rev-parse", "refs/stash"]);
    assert_eq!(h.git_at(&child, &["cat-file", "-t", &stash]), "commit");
    assert_eq!(
        h.git_at(&child, &["rev-parse", &format!("{stash}^1")]),
        original_head
    );
    assert_eq!(
        h.git_at(&child, &["rev-parse", &format!("{original_head}^1")]),
        pin
    );
    execute_advertised_recovery(&h, &child, &output, &stash);
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), original_head);
    assert_eq!(h.git_at(&child, &["ls-files", "--stage"]), original_index);
    assert_eq!(fs::read(child.join("LICENSE")).unwrap(), b"unstaged\0\xff");
    assert_eq!(
        fs::read(child.join("untracked")).unwrap(),
        b"untracked\0\xff"
    );
    assert_eq!(
        fs::read(child.join("local-only")).unwrap(),
        b"spaced local history"
    );
    assert_eq!(h.git_at(&child, &["cat-file", "-t", &stash]), "commit");
    assert_eq!(h.preservation_snapshot(), parent);
}

#[test]
fn reset_unmerged_gitlink_refuses_before_stashing_dirty_child() {
    use std::io::Write;
    use std::process::Stdio;
    let h = fixture(true);
    let child = h.work_dir.join("alpha");
    fs::write(child.join("LICENSE"), b"existing stash\0\xff").unwrap();
    h.git_at(&child, &["stash", "push", "-m", "preserve existing stash"]);
    let stash = h.git_at(&child, &["rev-parse", "refs/stash"]);
    let base = h.git_at(&child, &["rev-parse", "HEAD~1"]);
    let ours = h.git_at(&child, &["rev-parse", "HEAD"]);
    let theirs = h.git_at(&child, &["rev-parse", "origin/feature"]);
    assert_ne!(ours, theirs);
    dirty(&h, "alpha");
    let records = format!(
        "0 {}\talpha\n160000 {base} 1\talpha\n160000 {ours} 2\talpha\n160000 {theirs} 3\talpha\n",
        "0".repeat(ours.len())
    );
    let mut git = h
        .git_cmd()
        .args(["update-index", "--index-info"])
        .current_dir(&h.work_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    git.stdin
        .take()
        .unwrap()
        .write_all(records.as_bytes())
        .unwrap();
    let setup = git.wait_with_output().unwrap();
    assert!(
        setup.status.success(),
        "index setup failed: {}",
        text(&setup)
    );
    let entries = h.git_stdout(&["ls-files", "--stage", "--", "alpha"]);
    assert_eq!(
        entries
            .lines()
            .map(|line| line.split_whitespace().nth(2).unwrap())
            .collect::<Vec<_>>(),
        ["1", "2", "3"]
    );
    let before = bytes(&h.work_dir);
    let parent = h.preservation_snapshot();
    let child_index = h.git_at(&child, &["ls-files", "--stage"]);
    let refs = h.git_at(&child, &["show-ref"]);
    let trace_path = h.temp_dir.path().join("unmerged-reset-trace.json");
    let output = command(&h, &["reset", "alpha"])
        .env("GIT_TRACE2_EVENT", &trace_path)
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "unmerged gitlink must refuse reset: {}",
        text(&output)
    );
    assert_tree_eq(
        &bytes(&h.work_dir),
        &before,
        "unmerged reset must preserve raw parent/child index, config, files, refs, and stash",
    );
    assert_eq!(h.preservation_snapshot(), parent);
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "alpha"]),
        entries
    );
    assert_eq!(h.git_at(&child, &["ls-files", "--stage"]), child_index);
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), ours);
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(h.git_at(&child, &["rev-parse", "refs/stash"]), stash);
    let trace = fs::read_to_string(trace_path).unwrap();
    assert!(
        trace.contains("\"event\":\"cmd_name\""),
        "require actual Git trace evidence"
    );
    for line in trace
        .lines()
        .filter(|line| line.contains("\"event\":\"cmd_name\""))
    {
        for prohibited in ["stash", "reset", "checkout", "clean"] {
            assert!(
                !line.contains(&format!("\"name\":\"{prohibited}\"")),
                "unmerged gitlink must refuse before mutation: {line}"
            );
        }
    }
}
