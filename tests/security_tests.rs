// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Security tests to ensure robustness against various attack vectors.

use std::fs;

mod common;
use common::TestHarness;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_with_hyphen_injection() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("hyphen-remote")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // A path with a component starting with a hyphen that could be a git flag.
        // Note: Git itself has issues with paths starting with hyphen in the CWD
        // (even with --), so we use a sub-path.
        let malicious_path = "sub/-c";

        // This should not fail with "unknown option" or similar error from git -C
        // It might still fail for other reasons if the path is invalid for a submodule,
        // but it shouldn't be interpreted as a flag to the 'git' command itself.

        // Note: Using add_submodule via harness.
        // We need to make sure the directory doesn't exist or is handled.

        let result = harness.run_submod(&[
            "add",
            &remote_url,
            "--name",
            "hyphen-sub",
            "--path",
            malicious_path,
        ]);

        // The operation might fail because "sub/-c" is a weird path, but it shouldn't be a Command Injection.
        // If it was interpreted as `git -C sub/-c`, it would fail with "unknown option" or similar
        // if our fix wasn't working.

        match result {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(
                    !stderr.contains("unknown option: -c"),
                    "Potential command injection detected: git interpreted path as a flag"
                );
            }
            Err(e) => {
                let err_msg = e.to_string();
                assert!(
                    !err_msg.contains("unknown option: -c"),
                    "Potential command injection detected: git interpreted path as a flag"
                );
            }
        }
    }

    #[test]
    fn test_sparse_checkout_with_hyphen_path() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("sparse-hyphen")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Path with a component starting with hyphen.
        // Note: Git itself has issues with paths starting with hyphen in the CWD
        // (even with --), so we use a sub-path.
        let path = "sub/-sparse";

        // Ensure the directory exists to trigger the CLI fallback in apply_sparse_checkout if needed,
        // although apply_sparse_checkout is usually called after gix/git2 which might fail or be bypassed.

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "sparse-hyphen",
                "--path",
                path,
                "--sparse-paths",
                "src",
            ])
            .expect("Failed to add submodule with hyphenated path");

        // Verify it worked
        assert!(harness.dir_exists("sub/-sparse/src"));
    }

    /// CVE-2018-17456-class option injection: a submodule name or URL that mimics
    /// a `git clone`/`git submodule` flag (e.g. `--upload-pack=<cmd>`) must be
    /// treated as inert data, never as an option that triggers command execution.
    ///
    /// submod drives git via gix/git2 (no shell) and uses `--` before the URL/path
    /// on the CLI last-resort path, so the payload should never run. This test is
    /// non-vacuous: if any code path ever shelled the name/URL out unsafely, the
    /// sentinel file would be created.
    #[test]
    fn test_flag_like_name_and_url_do_not_inject_commands() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("inject-remote")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Sentinels: `touch <relative>` would land in cwd (the working tree);
        // `touch <absolute>` would land at a fixed path. Neither must appear.
        let rel_sentinel = harness.work_dir.join("INJECTED_BY_NAME");
        let abs_sentinel = harness.work_dir.join("INJECTED_BY_URL");
        assert!(!rel_sentinel.exists() && !abs_sentinel.exists());

        // Flag-like NAME (passed via `--name=` so clap accepts the leading dashes).
        let _ = harness
            .run_submod(&[
                "add",
                &remote_url,
                "--name=--upload-pack=touch INJECTED_BY_NAME",
                "--path",
                "lib/inj-name",
            ])
            .expect("Failed to run submod");
        assert!(
            !rel_sentinel.exists(),
            "a flag-like submodule name must not inject a command"
        );

        // Malicious transport URL (ext:: would run a shell command if honored).
        let evil_url = format!("ext::sh -c \"touch {}\"", abs_sentinel.display());
        let _ = harness
            .run_submod(&[
                "add",
                &evil_url,
                "--name",
                "inj-url",
                "--path",
                "lib/inj-url",
            ])
            .expect("Failed to run submod");
        assert!(
            !abs_sentinel.exists(),
            "a malicious transport URL must not execute a command"
        );
    }

    /// A hostile `.gitmodules` fed to `generate-config --from-setup` must be parsed
    /// as data only: its url/branch values are serialized verbatim into the output
    /// config, never executed. Non-vacuous: the generated file must actually contain
    /// the hostile values (proving the parse path ran) while no sentinel is created.
    #[test]
    fn test_generate_config_from_malicious_gitmodules_does_not_execute() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let sentinel = harness.work_dir.join("GC_INJECTED");
        let sentinel_str = sentinel.to_string_lossy().replace('\\', "/");
        let gitmodules = format!(
            "[submodule \"evil\"]\n\tpath = lib/evil\n\turl = ext::sh -c \"touch {sentinel_str}\"\n\tbranch = --upload-pack=touch {sentinel_str}\n"
        );
        fs::write(harness.work_dir.join(".gitmodules"), gitmodules)
            .expect("Failed to write .gitmodules");

        let output = harness
            .run_submod(&[
                "generate-config",
                "--from-setup",
                "--output",
                "out.toml",
                "--force",
            ])
            .expect("Failed to run submod");
        assert!(
            output.status.success(),
            "generate-config failed!\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        assert!(
            !sentinel.exists(),
            "generate-config must not execute values read from .gitmodules"
        );

        // Non-vacuity: the hostile entry was actually parsed and captured as data.
        let generated = fs::read_to_string(harness.work_dir.join("out.toml"))
            .expect("generate-config should have written the output config");
        assert!(
            generated.contains("ext::sh -c"),
            "expected the hostile url to be captured as inert data, got:\n{generated}"
        );
    }

    #[test]
    fn test_symlink_escape_containment() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("symlink-remote")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // 1. Check direct path traversal escape
        let result_traversal = harness
            .run_submod(&[
                "add",
                &remote_url,
                "--name",
                "escaped-sub",
                "--path",
                "../escaped-path",
            ])
            .expect("Failed to run submod");

        assert!(!result_traversal.status.success());
        let stderr = String::from_utf8_lossy(&result_traversal.stderr);
        assert!(
            stderr.contains("escapes repository root") || stderr.contains("Invalid path"),
            "Expected error message indicating path traversal escape, got: {stderr}"
        );

        // 2. Check symlink escape
        // Create a directory outside the repository root to point to
        let outside_dir = harness.temp_dir.path().join("outside_target");
        fs::create_dir_all(&outside_dir).expect("Failed to create outside dir");

        // Create a symlink in the repository pointing to the outside directory
        let symlink_path = harness.work_dir.join("lib_escaped");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside_dir, &symlink_path)
                .expect("Failed to create unix symlink");
        }
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_dir(&outside_dir, &symlink_path)
                .expect("Failed to create windows symlink");
        }

        // Try to add submodule that goes through this symlink
        let result_symlink = harness
            .run_submod(&[
                "add",
                &remote_url,
                "--name",
                "symlink-sub",
                "--path",
                "lib_escaped/submodule",
            ])
            .expect("Failed to run submod");

        assert!(!result_symlink.status.success());
        let stderr = String::from_utf8_lossy(&result_symlink.stderr);
        assert!(
            stderr.contains("escapes repository root") || stderr.contains("Invalid path"),
            "Expected error message indicating symlink path escape, got: {stderr}"
        );
    }
}

#[test]
fn regression_r01_config_root_target_rejected_before_writes() {
    for target in ["", ".", ".git", ".."] {
        let h = common::TestHarness::new().unwrap();
        h.init_git_repo().unwrap();
        let remote = h.create_test_remote("unsafe").unwrap();
        h.create_config(&format!(
            "[unsafe]\npath = {target:?}\nurl = {:?}\n",
            remote.to_str().unwrap()
        ))
        .unwrap();
        let before = h.preservation_snapshot();
        let sentinel = std::fs::read(h.work_dir.join("README.md")).unwrap();
        let output = h.run_submod(&["init"]).unwrap();
        assert_eq!(
            std::fs::read(h.work_dir.join("README.md")).ok(),
            Some(sentinel),
            "target {target:?}: {output:?}"
        );
        assert_eq!(
            h.preservation_snapshot(),
            before,
            "target {target:?}: {output:?}"
        );
        assert!(
            !output.status.success(),
            "unsafe target {target:?} accepted: {output:?}"
        );
    }
}

fn phase2_unsafe_target_case(target: &str, command: &[&str]) {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("boundary").unwrap();
    let outside = h.temp_dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside\0\xff").unwrap();
    let target = if target == "ABSOLUTE" {
        outside.to_str().unwrap()
    } else {
        target
    };
    h.create_config(&format!(
        "[unsafe]\npath = {target:?}\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let before = h.preservation_snapshot();
    let root = std::fs::read(h.work_dir.join("README.md")).unwrap();
    let output = h.run_submod(command).unwrap();
    assert_eq!(
        std::fs::read(outside.join("sentinel")).unwrap(),
        b"outside\0\xff",
        "{output:?}"
    );
    assert_eq!(
        std::fs::read(h.work_dir.join("README.md")).ok(),
        Some(root),
        "{output:?}"
    );
    assert_eq!(h.preservation_snapshot(), before, "{output:?}");
    assert!(
        !output.status.success(),
        "unsafe {target:?} accepted: {output:?}"
    );
}

macro_rules! phase2_unsafe_target {
    ($name:ident, $target:expr, $args:expr) => {
        #[test]
        fn $name() {
            phase2_unsafe_target_case($target, $args);
        }
    };
}
phase2_unsafe_target!(phase2_r01_dot_init, ".", &["init"]);
phase2_unsafe_target!(phase2_r01_git_init, ".git", &["init"]);
phase2_unsafe_target!(phase2_r01_parent_init, "../outside", &["init"]);
phase2_unsafe_target!(phase2_r01_absolute_init, "ABSOLUTE", &["init"]);
phase2_unsafe_target!(phase2_r01_dot_delete, ".", &["delete", "unsafe"]);
phase2_unsafe_target!(phase2_r01_git_delete, ".git", &["delete", "unsafe"]);
phase2_unsafe_target!(phase2_r01_absolute_reset, "ABSOLUTE", &["reset", "unsafe"]);

#[test]
fn phase2_r01_cli_administrative_name_preserves_metadata() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h.create_config("[defaults]\n").unwrap();
    let remote = h.create_test_remote("admin-name").unwrap();
    let before = h.preservation_snapshot();
    let output = h
        .run_submod(&[
            "add",
            remote.to_str().unwrap(),
            "--name",
            "../../config",
            "--path",
            "safe",
        ])
        .unwrap();
    assert_eq!(h.preservation_snapshot(), before, "{output:?}");
    assert!(!h.work_dir.join("safe").exists());
    assert!(!output.status.success(), "{output:?}");
}

#[cfg(unix)]
#[test]
fn phase2_r01_config_symlink_ancestor_preserves_outside() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("symlink-config").unwrap();
    let outside = h.temp_dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside\0\xff").unwrap();
    std::os::unix::fs::symlink(&outside, h.work_dir.join("escape")).unwrap();
    h.create_config(&format!(
        "[unsafe]\npath = \"escape/child\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let before = h.preservation_snapshot();
    let output = h.run_submod(&["init"]).unwrap();
    assert_eq!(
        std::fs::read(outside.join("sentinel")).unwrap(),
        b"outside\0\xff"
    );
    assert!(!outside.join("child").exists(), "{output:?}");
    assert_eq!(h.preservation_snapshot(), before, "{output:?}");
    assert!(!output.status.success(), "{output:?}");
}

#[test]
fn phase2_r01_redirected_child_gitfile_preserves_unrelated_repository() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let other = common::TestHarness::new().unwrap();
    other.init_git_repo().unwrap();
    let remote = h.create_test_remote("redirect").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "logical",
        remote.to_str().unwrap(),
        "child",
    ]);
    h.create_config(&format!(
        "[alias]\npath = \"child\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    other.git_stdout(&["config", "core.worktree", other.work_dir.to_str().unwrap()]);
    let gitdir = other.git_stdout(&["rev-parse", "--absolute-git-dir"]);
    let pointer = format!("gitdir: {gitdir}\n");
    std::fs::write(h.work_dir.join("child/.git"), &pointer).unwrap();
    std::fs::write(h.work_dir.join("child/LICENSE"), b"intended child\0\xff").unwrap();
    std::fs::write(other.work_dir.join("README.md"), b"unrelated dirty\0\xff").unwrap();
    let before = h.preservation_snapshot();
    let other_before = other.preservation_snapshot();
    let output = h.run_submod(&["reset", "alias"]).unwrap();
    assert_eq!(
        std::fs::read(other.work_dir.join("README.md")).unwrap(),
        b"unrelated dirty\0\xff",
        "{output:?}"
    );
    assert_eq!(
        std::fs::read(h.work_dir.join("child/LICENSE")).unwrap(),
        b"intended child\0\xff"
    );
    assert_eq!(
        std::fs::read_to_string(h.work_dir.join("child/.git")).unwrap(),
        pointer
    );
    assert_eq!(other.preservation_snapshot(), other_before, "{output:?}");
    assert_eq!(h.preservation_snapshot(), before, "{output:?}");
    assert!(
        !output.status.success(),
        "redirected child accepted: {output:?}"
    );
}

#[test]
fn phase2_r01_change_path_rejects_parent_escape_before_writes() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("move-boundary").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "logical",
        remote.to_str().unwrap(),
        "child",
    ]);
    h.create_config(&format!(
        "[alias]\npath = \"child\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let child = h.work_dir.join("child");
    let refs = h.git_at(&child, &["show-ref"]);
    let gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
    let before = h.preservation_snapshot();
    let outside = h.temp_dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside\0\xff").unwrap();
    let output = h
        .run_submod(&["change", "alias", "--path", "../outside/moved"])
        .unwrap();
    assert_eq!(
        std::fs::read(outside.join("sentinel")).unwrap(),
        b"outside\0\xff"
    );
    assert!(!outside.join("moved").exists(), "{output:?}");
    assert_eq!(h.preservation_snapshot(), before, "{output:?}");
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(
        h.git_at(&child, &["rev-parse", "--absolute-git-dir"]),
        gitdir
    );
    assert!(!output.status.success(), "{output:?}");
}

#[test]
fn phase2_r01_cli_root_and_admin_paths_rejected() {
    for target in ["", ".", "./", ".git", ".git/objects", "../outside"] {
        let h = common::TestHarness::new().unwrap();
        h.init_git_repo().unwrap();
        h.create_config("[defaults]\n").unwrap();
        let remote = h.create_test_remote("cli-boundary").unwrap();
        let before = h.preservation_snapshot();
        let root = std::fs::read(h.work_dir.join("README.md")).unwrap();
        let output = h
            .run_submod(&[
                "add",
                remote.to_str().unwrap(),
                "--name",
                "alias",
                "--path",
                target,
            ])
            .unwrap();
        assert_eq!(
            std::fs::read(h.work_dir.join("README.md")).ok(),
            Some(root),
            "target {target:?}: {output:?}"
        );
        assert_eq!(
            h.preservation_snapshot(),
            before,
            "target {target:?}: {output:?}"
        );
        assert!(!h.temp_dir.path().join("outside").exists(), "{output:?}");
        assert!(!output.status.success(), "target {target:?}: {output:?}");
    }
}

#[cfg(unix)]
#[test]
fn phase2_r01_add_refuses_symlinked_storage_ancestor() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h.create_config("[defaults]\n").unwrap();
    let remote = h.create_test_remote("storage-ancestor").unwrap();
    let outside = h.temp_dir.path().join("outside-storage");
    std::fs::create_dir(&outside).unwrap();
    let gitdir = outside.join("retained");
    let worktree = outside.join("separate");
    h.git_stdout(&[
        "clone",
        "--separate-git-dir",
        gitdir.to_str().unwrap(),
        remote.to_str().unwrap(),
        worktree.to_str().unwrap(),
    ]);
    h.git_at(
        &worktree,
        &["config", "core.worktree", worktree.to_str().unwrap()],
    );
    h.git_at(&worktree, &["branch", "outside-history"]);
    std::fs::write(worktree.join("sentinel"), b"outside sentinel\0\xff").unwrap();
    assert_eq!(
        h.git_at(&worktree, &["remote", "get-url", "origin"]),
        remote.to_str().unwrap()
    );
    std::fs::create_dir_all(h.work_dir.join(".git/modules")).unwrap();
    std::os::unix::fs::symlink(&outside, h.work_dir.join(".git/modules/nested")).unwrap();
    let before = h.preservation_snapshot();
    let parent_index = std::fs::read(h.work_dir.join(".git/index")).unwrap();
    let head = h.git_at(&worktree, &["rev-parse", "HEAD"]);
    let refs = h.git_at(&worktree, &["show-ref"]);
    let index = h.git_at(&worktree, &["ls-files", "--stage"]);
    let admin_bytes: Vec<_> = ["HEAD", "config", "index", "refs/heads/outside-history"]
        .into_iter()
        .map(|path| (path, std::fs::read(gitdir.join(path)).unwrap()))
        .collect();
    let paths = h.git_at(&worktree, &["ls-files", "-z"]);
    let files: Vec<_> = paths
        .split('\0')
        .filter(|path| !path.is_empty())
        .chain([".git", "sentinel"])
        .map(|path| (path.to_owned(), std::fs::read(worktree.join(path)).unwrap()))
        .collect();

    let output = h
        .run_submod(&[
            "add",
            remote.to_str().unwrap(),
            "--name",
            "nested/retained",
            "--path",
            "child",
        ])
        .unwrap();

    assert!(
        !output.status.success(),
        "storage escape accepted: {output:?}"
    );
    assert_eq!(h.preservation_snapshot(), before, "{output:?}");
    assert_eq!(
        std::fs::read(h.work_dir.join(".git/index")).unwrap(),
        parent_index
    );
    assert!(!h.work_dir.join("child").exists());
    assert_eq!(h.git_at(&worktree, &["rev-parse", "HEAD"]), head);
    assert_eq!(h.git_at(&worktree, &["show-ref"]), refs);
    assert_eq!(h.git_at(&worktree, &["ls-files", "--stage"]), index);
    for (path, bytes) in admin_bytes {
        assert_eq!(
            std::fs::read(gitdir.join(path)).unwrap(),
            bytes,
            "outside gitdir changed: {path}"
        );
    }
    for (path, bytes) in files {
        assert_eq!(
            std::fs::read(worktree.join(&path)).unwrap(),
            bytes,
            "outside checkout changed: {path}"
        );
    }
}

#[test]
fn phase2_r01_init_accepts_valid_external_child_gitdir() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("valid-external-gitdir").unwrap();
    let gitdir = h.temp_dir.path().join("external-child.git");
    let child = h.work_dir.join("child");
    h.git_stdout(&[
        "init",
        "--separate-git-dir",
        gitdir.to_str().unwrap(),
        child.to_str().unwrap(),
    ]);
    h.git_at(
        &child,
        &["config", "core.worktree", child.to_str().unwrap()],
    );
    h.git_at(
        &child,
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    h.git_at(&child, &["fetch", "origin"]);
    h.git_at(&child, &["checkout", "--detach", "origin/main"]);
    h.git_at(&child, &["branch", "retained-history"]);
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.logical.path",
        "child",
    ]);
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.logical.url",
        remote.to_str().unwrap(),
    ]);
    h.git_stdout(&["config", "submodule.logical.url", remote.to_str().unwrap()]);
    h.git_stdout(&["config", "submodule.logical.active", "true"]);
    h.create_config(&format!(
        "[logical]\npath = \"child\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    h.git_stdout(&["add", ".gitmodules", "submod.toml", "child"]);
    h.git_stdout(&["commit", "-m", "Record valid external child gitdir"]);
    let before = h.preservation_snapshot();
    let parent_index = std::fs::read(h.work_dir.join(".git/index")).unwrap();
    let head = h.git_at(&child, &["rev-parse", "HEAD"]);
    let refs = h.git_at(&child, &["show-ref"]);
    let actual_gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
    assert_eq!(
        std::fs::canonicalize(&actual_gitdir).unwrap(),
        std::fs::canonicalize(&gitdir).unwrap()
    );
    let config = std::fs::read(gitdir.join("config")).unwrap();
    let paths = h.git_at(&child, &["ls-files", "-z"]);
    let files: Vec<_> = paths
        .split('\0')
        .filter(|path| !path.is_empty())
        .chain(std::iter::once(".git"))
        .map(|path| (path.to_owned(), std::fs::read(child.join(path)).unwrap()))
        .collect();

    h.run_submod_success(&["init"]).unwrap();

    assert_eq!(h.preservation_snapshot(), before);
    assert_eq!(
        std::fs::read(h.work_dir.join(".git/index")).unwrap(),
        parent_index
    );
    assert_eq!(
        h.git_at(&child, &["rev-parse", "--absolute-git-dir"]),
        actual_gitdir
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), head);
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(std::fs::read(gitdir.join("config")).unwrap(), config);
    for (path, bytes) in files {
        assert_eq!(
            std::fs::read(child.join(&path)).unwrap(),
            bytes,
            "intended checkout changed: {path}"
        );
    }
}

#[cfg(unix)]
#[test]
fn phase2_r01_init_accepts_gitfile_target_symlink_to_valid_gitdir() {
    use std::os::unix::fs::symlink;

    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("gitfile-target-link").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "logical",
        remote.to_str().unwrap(),
        "child",
    ]);
    h.create_config(&format!(
        "[logical]\npath = \"child\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let child = h.work_dir.join("child");
    let actual_gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
    let linked_gitdir = h.temp_dir.path().join("linked-child-gitdir");
    symlink(&actual_gitdir, &linked_gitdir).unwrap();
    let pointer = format!("gitdir: {}\n", linked_gitdir.display());
    fs::write(child.join(".git"), pointer.as_bytes()).unwrap();
    assert_eq!(
        fs::canonicalize(h.git_at(&child, &["rev-parse", "--absolute-git-dir"])).unwrap(),
        fs::canonicalize(&actual_gitdir).unwrap()
    );
    let before = h.preservation_snapshot();
    let child_head = h.git_at(&child, &["rev-parse", "HEAD"]);
    let child_refs = h.git_at(&child, &["show-ref"]);

    h.run_submod_success(&["init"]).unwrap();

    assert_eq!(h.preservation_snapshot(), before);
    assert_eq!(fs::read_to_string(child.join(".git")).unwrap(), pointer);
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), child_head);
    assert_eq!(h.git_at(&child, &["show-ref"]), child_refs);
}
