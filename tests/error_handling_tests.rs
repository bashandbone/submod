// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Integration tests focused on error handling and edge cases
//!
//! These tests verify that the tool handles various error conditions gracefully
//! and provides meaningful error messages to users.

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

mod common;
use common::TestHarness;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_in_git_repository() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        // Don't initialize git repo

        let output = harness
            .run_submod(&["check"])
            .expect("Failed to run submod");
        assert!(!output.status.success());
        assert_eq!(output.status.code(), Some(1));

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Cannot prepare check"), "{stderr}");
        assert!(
            stderr.contains("Failed to discover") && stderr.contains("not a git repository"),
            "{stderr}"
        );
        assert!(!harness.config_path().exists());
        assert!(!harness.work_dir.join(".git").exists());
    }

    #[test]
    fn test_invalid_git_url() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Try various invalid URLs
        let missing = harness.temp_dir.path().join("missing-remote.git");
        let invalid_urls = [missing.to_str().expect("local path")];

        for invalid_url in invalid_urls {
            let before = harness.preservation_snapshot();
            let output = harness
                .run_submod(&[
                    "add",
                    invalid_url,
                    "--name",
                    "invalid-test",
                    "--path",
                    "lib/invalid",
                ])
                .expect("Failed to run submod");

            assert!(!output.status.success());
            assert_eq!(output.status.code(), Some(1));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("Add failed") && stderr.contains("missing-remote.git"),
                "{stderr}"
            );
            assert!(
                stderr.contains("does not exist")
                    || stderr.contains("does not appear to be a git repository"),
                "{stderr}"
            );
            assert_eq!(harness.preservation_snapshot(), before);
            assert!(!harness.work_dir.join("lib/invalid").exists());
        }
    }

    #[test]
    fn test_invalid_config_file_path() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let before = harness.preservation_snapshot();
        let missing = harness
            .run_submod(&["--config", "missing-config.toml", "check"])
            .expect("Failed to run submod");
        let stderr = String::from_utf8_lossy(&missing.stderr);
        assert!(
            !missing.status.success(),
            "explicit missing config must fail"
        );
        assert!(
            stderr.contains("missing-config.toml"),
            "missing filename context: {stderr}"
        );
        assert!(
            stderr.to_lowercase().contains("not found")
                || stderr.to_lowercase().contains("no such file"),
            "missing-file diagnostic: {stderr}"
        );
        assert_eq!(harness.preservation_snapshot(), before);

        // A config file that exists but is malformed must NOT be swallowed: it
        // fails with a specific parse diagnostic, not a generic catch-all.
        let bad_config = harness.work_dir.join("bad.toml");
        fs::write(&bad_config, "this is = = not toml [[[\n").expect("write bad config");
        let malformed = harness
            .run_submod(&["--config", "bad.toml", "check"])
            .expect("Failed to run submod");
        assert!(
            !malformed.status.success(),
            "a malformed config must cause a non-zero exit"
        );
        let stderr = String::from_utf8_lossy(&malformed.stderr);
        assert!(
            stderr.contains("TOML parse error") || stderr.contains("parse error"),
            "malformed config error must name the parse failure, got: {stderr}"
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_permission_denied_scenarios() {
        let is_root = std::process::Command::new("id")
            .arg("-u")
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).trim() == "0");

        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("perm_test")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        if is_root {
            // For root, create a file where a directory is expected to trigger an IO error
            let path_as_file = harness.work_dir.join("readonly");
            fs::write(&path_as_file, "not a directory").expect("Failed to create file");

            let before = harness.preservation_snapshot();
            let output = harness
                .run_submod(&[
                    "add",
                    &remote_url,
                    "--name",
                    "perm-test",
                    "--path",
                    "readonly/submodule",
                ])
                .expect("Failed to run submod");

            assert!(!output.status.success());
            assert_eq!(output.status.code(), Some(1));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("Not a directory") || stderr.contains("not a directory"),
                "Expected directory collision/IO failure message, got: {stderr}"
            );
            assert_eq!(harness.preservation_snapshot(), before);
            assert_eq!(fs::read_to_string(path_as_file).unwrap(), "not a directory");
        } else {
            // Create a directory we can't write to
            let readonly_dir = harness.work_dir.join("readonly");
            fs::create_dir_all(&readonly_dir).expect("Failed to create readonly dir");

            let before = harness.preservation_snapshot();
            // Make directory read-only
            let original_perms = fs::metadata(&readonly_dir).unwrap().permissions();
            let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&readonly_dir, perms).expect("Failed to set permissions");

            // Try to add submodule to read-only directory
            let output = harness
                .run_submod(&[
                    "add",
                    &remote_url,
                    "--name",
                    "perm-test",
                    "--path",
                    "readonly/submodule",
                ])
                .expect("Failed to run submod");

            fs::set_permissions(&readonly_dir, original_perms)
                .expect("Failed to restore permissions");
            assert_eq!(output.status.code(), Some(1), "{output:?}");
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("Permission denied"), "{stderr}");
            assert_eq!(harness.preservation_snapshot(), before);
            assert!(!readonly_dir.join("submodule").exists());
        }
    }

    #[test]
    fn test_corrupted_config_file() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Create a corrupted TOML config
        let corrupted_configs = vec![
            "[[[[invalid toml",
            "[section\nkey = value\n",
            "key = \"unclosed string",
            "key = value\n[section\nkey2 = \"unclosed",
        ];

        for corrupted_config in corrupted_configs {
            harness
                .create_config(corrupted_config)
                .expect("Failed to create corrupted config");

            let before = harness.preservation_snapshot();
            let output = harness
                .run_submod(&["check"])
                .expect("Failed to run submod");
            assert!(!output.status.success());
            assert_eq!(output.status.code(), Some(2));

            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("Cannot prepare check"), "{stderr}");
            assert!(
                stderr.contains("submod.toml") && stderr.contains("TOML parse error"),
                "{stderr}"
            );
            assert_eq!(harness.preservation_snapshot(), before);
        }
    }

    #[test]
    fn test_missing_submodule_for_operations() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        harness.create_config("").unwrap();
        let before = harness.preservation_snapshot();

        // Try to operate on non-existent submodule
        let operations = vec![
            vec!["reset", "nonexistent-submodule"],
            vec!["update"], // This should succeed but show no submodules
        ];

        for operation in operations {
            let output = harness
                .run_submod(&operation)
                .expect("Failed to run submod");

            match operation[0] {
                "reset" => {
                    assert!(!output.status.success());
                    assert_eq!(output.status.code(), Some(2));
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    assert!(
                        stderr.contains("nonexistent-submodule") && stderr.contains("not found"),
                        "{stderr}"
                    );
                }
                "update" => {
                    // Update should succeed but do nothing
                    assert!(output.status.success());
                }
                _ => {}
            }
            assert_eq!(harness.preservation_snapshot(), before);
        }
    }

    #[test]
    fn test_invalid_sparse_checkout_patterns() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("sparse_invalid")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // A path-traversal sentinel: if any pattern lets the tool write outside the
        // superproject working tree, this file/dir would appear one level up.
        let escape_sentinel = harness
            .work_dir
            .parent()
            .expect("work_dir has a parent")
            .join("escape");
        assert!(
            !escape_sentinel.exists(),
            "sentinel must not pre-exist before the test"
        );

        // Traversal / malformed patterns. The security-relevant property is
        // CONTAINMENT: regardless of success or failure, nothing is written
        // outside the superproject working tree.
        let traversal_patterns = vec!["//invalid//path", "..", "../../../escape", "../escape"];

        for pattern in traversal_patterns {
            let _output = harness
                .run_submod(&[
                    "add",
                    &remote_url,
                    "--name",
                    "sparse-test",
                    "--path",
                    "lib/sparse-test",
                    "--sparse-paths",
                    pattern,
                ])
                .expect("Failed to run submod");

            // Containment: the traversal pattern must never materialize a path
            // outside the working tree, nor at an absolute system location.
            assert!(
                !escape_sentinel.exists(),
                "path traversal escaped the working tree for pattern {pattern:?}: {} was created",
                escape_sentinel.display()
            );
            assert!(
                !std::path::Path::new("/escape").exists(),
                "path traversal reached an absolute location for pattern {pattern:?}"
            );

            // Clean up for next iteration
            let _ = fs::remove_dir_all(harness.work_dir.join("lib/sparse-test"));
        }

        // A NUL byte in an argument cannot be handed to a process at all: std's
        // Command rejects it before spawn, so run_submod returns an Err. Assert
        // that real boundary rejection (previously faked by the harness).
        let nul_result = harness.run_submod(&[
            "add",
            &remote_url,
            "--name",
            "sparse-test",
            "--path",
            "lib/sparse-test",
            "--sparse-paths",
            "path/with/\0/null",
        ]);
        let err =
            nul_result.expect_err("a NUL-byte argument must be rejected at the process boundary");
        assert!(
            err.to_string().to_lowercase().contains("nul"),
            "expected a NUL-byte rejection error, got: {err}"
        );
    }

    #[test]
    fn test_external_config_edit_is_observed() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("concurrent")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add a submodule
        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "concurrent-test",
                "--path",
                "lib/concurrent",
            ])
            .expect("Failed to add submodule");

        // A later invocation observes an external configuration edit.
        let config_content = format!(
            r#"[concurrent-test]
path = "lib/concurrent"
url = "{remote_url}"
active = true

[external-addition]
path = "lib/external"
url = "https://github.com/example/external.git"
active = true
"#
        );
        harness
            .create_config(&config_content)
            .expect("Failed to modify config");

        // Run check (verbose) to see if it handles the externally modified config
        let before = harness.preservation_snapshot();
        let child_head = harness.git_stdout(&["-C", "lib/concurrent", "rev-parse", "HEAD"]);
        let child_index = harness.git_stdout(&["-C", "lib/concurrent", "ls-files", "--stage"]);
        let child_file = std::fs::read(harness.work_dir.join("lib/concurrent/LICENSE")).unwrap();
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run check");
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(harness.preservation_snapshot(), before);
        assert_eq!(
            harness.git_stdout(&["-C", "lib/concurrent", "rev-parse", "HEAD"]),
            child_head
        );
        assert_eq!(
            harness.git_stdout(&["-C", "lib/concurrent", "ls-files", "--stage"]),
            child_index
        );
        assert_eq!(
            std::fs::read(harness.work_dir.join("lib/concurrent/LICENSE")).unwrap(),
            child_file
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("concurrent-test"));
        assert!(stdout.contains("external-addition"));
        assert!(stdout.contains("external-addition: drift: checkout is missing"));
        assert!(!stdout.contains("concurrent-test: drift: checkout is missing"));
        assert!(harness.file_exists("lib/concurrent/.git"));
        assert_eq!(
            harness.index_gitlink_mode("lib/concurrent").as_deref(),
            Some("160000")
        );
    }

    #[test]
    fn test_invalid_command_line_arguments() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let invalid_args = vec![
            vec!["--invalid-flag"],
            vec!["add"],                               // Missing required URL argument
            vec!["add", "--name", "x", "--path", "y"], // Missing required URL argument
            vec!["reset"], // Missing submodule name when not using --all
            vec!["nonexistent-command"],
        ];

        for args in invalid_args {
            let output = harness.run_submod(&args).expect("Failed to run submod");
            assert!(!output.status.success());
            let code = output.status.code().unwrap_or(1);
            assert!(code == 1 || code == 2);

            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("error") || stderr.contains("Usage") || stderr.contains("help")
            );
        }
    }

    #[test]
    fn test_unavailable_local_remote() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let missing = harness.temp_dir.path().join("unavailable.git");
        let timeout_url = missing.to_str().unwrap();

        let output = harness
            .run_submod(&[
                "add",
                timeout_url,
                "--name",
                "timeout-test",
                "--path",
                "lib/timeout",
            ])
            .expect("Failed to run submod");

        assert!(!output.status.success());
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Failed to add submodule")
                || stderr.contains("timeout")
                || stderr.contains("clone failed")
                || stderr.contains("could not resolve")
                || stderr.contains("Name or service not known")
        );
    }

    #[test]
    fn test_malformed_git_repository() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Create a fake "remote" that's not actually a git repository
        let fake_remote = harness.temp_dir.path().join("fake_remote");
        fs::create_dir_all(&fake_remote).expect("Failed to create fake remote");
        fs::write(
            fake_remote.join("not_a_git_repo.txt"),
            "This is not a git repository",
        )
        .expect("Failed to create fake file");

        let fake_url = format!("file://{}", fake_remote.display());

        let before = harness.preservation_snapshot();
        let output = harness
            .run_submod(&[
                "add",
                &fake_url,
                "--name",
                "fake-repo",
                "--path",
                "lib/fake",
            ])
            .expect("Failed to run submod");

        assert!(!output.status.success());
        assert_eq!(output.status.code(), Some(1));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Add failed") && stderr.contains("fake_remote"),
            "{stderr}"
        );
        assert!(
            stderr.contains("does not appear to be a git repository"),
            "{stderr}"
        );
        assert_eq!(harness.preservation_snapshot(), before);
        assert!(!harness.work_dir.join("lib/fake").exists());
        assert_eq!(
            fs::read_to_string(fake_remote.join("not_a_git_repo.txt")).unwrap(),
            "This is not a git repository"
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_config_file_locked() {
        let is_root = std::process::Command::new("id")
            .arg("-u")
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).trim() == "0");

        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Create initial config
        harness
            .create_config(
                "[test]\npath = \"test\"\nurl = \"https://example.com/test.git\"\nactive = true\n",
            )
            .expect("Failed to create config");

        let config_path = harness.config_path();

        let remote_repo = harness
            .create_test_remote("locked_config")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        if is_root {
            // For root, make the config file a directory. Writing to it will fail with EISDIR.
            fs::remove_file(&config_path).expect("Failed to remove config file");
            fs::create_dir(&config_path).expect("Failed to create config directory");
            let index_before = harness.git_stdout(&["ls-files", "--stage"]);
            let git_config_before = fs::read(harness.work_dir.join(".git/config")).unwrap();

            let output = harness
                .run_submod(&[
                    "add",
                    &remote_url,
                    "--name",
                    "locked-test",
                    "--path",
                    "lib/locked",
                ])
                .expect("Failed to run submod");

            assert!(!output.status.success());
            assert_eq!(output.status.code(), Some(1));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("Is a directory")
                    || stderr.contains("Failed to save config")
                    || stderr.contains("is a directory"),
                "Expected directory write/config save failure message, got: {stderr}"
            );
            assert!(config_path.is_dir());
            assert_eq!(harness.git_stdout(&["ls-files", "--stage"]), index_before);
            assert_eq!(
                fs::read(harness.work_dir.join(".git/config")).unwrap(),
                git_config_before
            );
            assert!(!harness.file_exists(".gitmodules"));
            assert!(!harness.dir_exists("lib/locked"));

            // Cleanup
            fs::remove_dir(&config_path).expect("Failed to remove config directory");
        } else {
            // Atomic sibling replacement can update a read-only destination.
            let mut perms = fs::metadata(&config_path).unwrap().permissions();
            perms.set_mode(0o444);
            fs::set_permissions(&config_path, perms).expect("Failed to set permissions");

            // Try to add submodule (which requires writing to config)
            let output = harness
                .run_submod(&[
                    "add",
                    &remote_url,
                    "--name",
                    "locked-test",
                    "--path",
                    "lib/locked",
                ])
                .expect("Failed to run submod");

            assert!(
                output.status.success(),
                "Expected atomic replacement to succeed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let config = submod::Config::parse(&harness.read_config().unwrap())
                .expect("Replacement config must be valid");
            assert!(config.get_submodule("test").is_some());
            let added = config.get_submodule("locked-test").unwrap();
            assert_eq!(added.path.as_deref(), Some("lib/locked"));
            assert_eq!(added.url.as_deref(), Some(remote_url.as_str()));
            assert_eq!(
                fs::metadata(&config_path).unwrap().permissions().mode() & 0o777,
                0o444,
                "Atomic replacement must preserve the original read-only mode"
            );

            // Restore permissions for cleanup
            let mut perms = fs::metadata(&config_path).unwrap().permissions();
            perms.set_mode(0o644);
            fs::set_permissions(&config_path, perms).expect("Failed to restore permissions");
        }
    }
}

#[test]
fn regression_r02_failed_add_preserves_occupied_directory() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h.create_config("[defaults]\n").unwrap();
    std::fs::create_dir(h.work_dir.join("occupied")).unwrap();
    let sentinel = h.work_dir.join("occupied/sentinel");
    std::fs::write(&sentinel, b"irreplaceable local bytes").unwrap();
    let before = h.preservation_snapshot();
    let missing = h.temp_dir.path().join("missing.git");
    let output = h
        .run_submod(&[
            "add",
            missing.to_str().unwrap(),
            "--name",
            "m",
            "--path",
            "occupied",
        ])
        .unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert_eq!(
        std::fs::read(&sentinel).ok().as_deref(),
        Some(b"irreplaceable local bytes".as_slice())
    );
    assert_eq!(h.preservation_snapshot(), before);
}

#[test]
fn regression_r03_disable_preserves_local_commit_and_stash() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("history").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "module",
        remote.to_str().unwrap(),
        "module",
    ]);
    h.create_config(&format!(
        "[m]\npath = \"module\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let child = h.work_dir.join("module");
    std::fs::write(child.join("local-only"), b"local history").unwrap();
    h.git_at(&child, &["add", "local-only"]);
    h.git_at(&child, &["commit", "-m", "local only"]);
    let commit = h.git_at(&child, &["rev-parse", "HEAD"]);
    std::fs::write(child.join("LICENSE"), b"recover me").unwrap();
    h.git_at(&child, &["stash", "push", "-m", "preserved"]);
    let stash = h.git_at(&child, &["rev-parse", "refs/stash"]);
    let gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
    let index = h.git_stdout(&["ls-files", "--stage"]);
    h.run_submod_success(&["disable", "m"]).unwrap();
    assert!(
        std::path::Path::new(&gitdir).join("objects").is_dir(),
        "disable deleted object database {gitdir}"
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), commit);
    assert_eq!(h.git_at(&child, &["rev-parse", "refs/stash"]), stash);
    assert_eq!(h.git_stdout(&["ls-files", "--stage"]), index);
    h.git_at(&child, &["stash", "apply", &stash]);
    assert_eq!(std::fs::read(child.join("LICENSE")).unwrap(), b"recover me");
}

#[test]
fn regression_r05_failed_stash_ref_lock_preserves_work() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("stash").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "m",
        remote.to_str().unwrap(),
        "module",
    ]);
    h.create_config(&format!(
        "[m]\npath = \"module\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let child = h.work_dir.join("module");
    std::fs::write(child.join("LICENSE"), b"dirty tracked").unwrap();
    std::fs::write(child.join("untracked"), b"untracked bytes").unwrap();
    let gitdir = std::path::PathBuf::from(h.git_at(&child, &["rev-parse", "--absolute-git-dir"]));
    std::fs::write(gitdir.join("info/exclude"), "ignored\n").unwrap();
    std::fs::write(child.join("ignored"), b"ignored bytes").unwrap();
    std::fs::write(gitdir.join("refs/stash.lock"), "held by test\n").unwrap();
    let before = h.preservation_snapshot();
    let head = h.git_at(&child, &["rev-parse", "HEAD"]);
    let output = h.run_submod(&["reset", "m"]).unwrap();
    for (file, bytes) in [
        ("LICENSE", b"dirty tracked".as_slice()),
        ("untracked", b"untracked bytes".as_slice()),
        ("ignored", b"ignored bytes".as_slice()),
    ] {
        assert_eq!(
            std::fs::read(child.join(file)).ok().as_deref(),
            Some(bytes),
            "reset lost {file}; {output:?}"
        );
    }
    assert!(
        !output.status.success(),
        "stash lock must prevent reset: {output:?}"
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), head);
    assert_eq!(h.preservation_snapshot(), before);
}

#[test]
fn phase2_r02_failed_add_preserves_independent_repository() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h.create_config("[defaults]\n").unwrap();
    let occupied = h.work_dir.join("occupied");
    std::fs::create_dir(&occupied).unwrap();
    h.git_at(&occupied, &["init", "-b", "main"]);
    std::fs::write(occupied.join("sentinel"), b"committed local\0\xff").unwrap();
    h.git_at(&occupied, &["add", "sentinel"]);
    h.git_at(&occupied, &["commit", "-m", "independent history"]);
    let refs = h.git_at(&occupied, &["show-ref"]);
    let index = h.git_at(&occupied, &["ls-files", "--stage"]);
    let config = std::fs::read(occupied.join(".git/config")).unwrap();
    std::fs::write(occupied.join("sentinel"), b"uncommitted local\0\xff").unwrap();
    let before = h.preservation_snapshot();
    let missing = h.temp_dir.path().join("missing.git");
    let output = h
        .run_submod(&[
            "add",
            missing.to_str().unwrap(),
            "--name",
            "m",
            "--path",
            "occupied",
        ])
        .unwrap();
    assert_eq!(
        std::fs::read(occupied.join("sentinel")).ok().as_deref(),
        Some(b"uncommitted local\0\xff".as_slice()),
        "{output:?}"
    );
    assert_eq!(h.git_at(&occupied, &["show-ref"]), refs);
    assert_eq!(h.git_at(&occupied, &["ls-files", "--stage"]), index);
    assert_eq!(std::fs::read(occupied.join(".git/config")).unwrap(), config);
    assert_eq!(h.preservation_snapshot(), before);
    assert!(!output.status.success(), "{output:?}");
}

#[test]
fn phase2_r02_failed_add_preserves_dirty_registered_module() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("occupied-module").unwrap();
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
    std::fs::write(child.join("LICENSE"), b"staged change\0\xff").unwrap();
    h.git_at(&child, &["add", "LICENSE"]);
    std::fs::write(child.join("LICENSE"), b"unstaged change\0\xff").unwrap();
    std::fs::write(child.join("untracked"), b"untracked\0\xff").unwrap();
    let refs = h.git_at(&child, &["show-ref"]);
    let index = h.git_at(&child, &["ls-files", "--stage"]);
    let config = h.git_at(&child, &["config", "--local", "--list"]);
    let before = h.preservation_snapshot();
    let missing = h.temp_dir.path().join("missing.git");
    let output = h
        .run_submod(&[
            "add",
            missing.to_str().unwrap(),
            "--name",
            "alias",
            "--path",
            "child",
        ])
        .unwrap();
    assert_eq!(
        std::fs::read(child.join("LICENSE")).ok().as_deref(),
        Some(b"unstaged change\0\xff".as_slice()),
        "{output:?}"
    );
    assert_eq!(
        std::fs::read(child.join("untracked")).unwrap(),
        b"untracked\0\xff"
    );
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(h.git_at(&child, &["ls-files", "--stage"]), index);
    assert_eq!(h.git_at(&child, &["config", "--local", "--list"]), config);
    assert_eq!(h.preservation_snapshot(), before);
    assert!(!output.status.success(), "{output:?}");
}

#[test]
fn phase2_r02_failed_add_preserves_occupied_file() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h.create_config("[defaults]\n").unwrap();
    std::fs::write(h.work_dir.join("occupied"), b"valuable file\0\xff").unwrap();
    let before = h.preservation_snapshot();
    let missing = h.temp_dir.path().join("missing.git");
    let output = h
        .run_submod(&[
            "add",
            missing.to_str().unwrap(),
            "--name",
            "m",
            "--path",
            "occupied",
        ])
        .unwrap();
    assert_eq!(
        std::fs::read(h.work_dir.join("occupied")).ok().as_deref(),
        Some(b"valuable file\0\xff".as_slice()),
        "{output:?}"
    );
    assert_eq!(h.preservation_snapshot(), before);
    assert!(!output.status.success(), "{output:?}");
}

#[cfg(unix)]
#[test]
fn phase2_r02_failed_add_preserves_occupied_symlink() {
    let h = common::TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h.create_config("[defaults]\n").unwrap();
    let outside = h.temp_dir.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("sentinel"), b"outside bytes\0\xff").unwrap();
    std::os::unix::fs::symlink(&outside, h.work_dir.join("occupied")).unwrap();
    let before = h.preservation_snapshot();
    let missing = h.temp_dir.path().join("missing.git");
    let output = h
        .run_submod(&[
            "add",
            missing.to_str().unwrap(),
            "--name",
            "m",
            "--path",
            "occupied",
        ])
        .unwrap();
    assert_eq!(
        std::fs::read(outside.join("sentinel")).unwrap(),
        b"outside bytes\0\xff"
    );
    assert_eq!(
        std::fs::read_link(h.work_dir.join("occupied")).unwrap(),
        outside
    );
    assert_eq!(h.preservation_snapshot(), before);
    assert!(!output.status.success(), "{output:?}");
}

// Capture all bytes, including an ignored nested repository's administrative files.
fn acceptance_tree_bytes(
    root: &std::path::Path,
) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    fn visit(
        root: &std::path::Path,
        path: &std::path::Path,
        files: &mut std::collections::BTreeMap<std::path::PathBuf, Vec<u8>>,
    ) {
        for entry in fs::read_dir(path).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), files);
            } else {
                files.insert(
                    entry.path().strip_prefix(root).unwrap().to_owned(),
                    fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut files = std::collections::BTreeMap::new();
    visit(root, root, &mut files);
    files
}

fn acceptance_traced_reset(h: &TestHarness) -> (std::process::Output, Vec<String>) {
    let trace = h.temp_dir.path().join("reset-trace.json");
    let mut command = std::process::Command::new(&h.submod_bin);
    // Reuse exactly the fixture's child environment without process-global changes.
    for (key, value) in h.git_cmd().get_envs() {
        match value {
            Some(value) => {
                command.env(key, value);
            }
            None => {
                command.env_remove(key);
            }
        }
    }
    let output = command
        .args(["reset", "nickname"])
        .current_dir(&h.work_dir)
        .env("GIT_TRACE2_EVENT", &trace)
        .output()
        .unwrap();
    let trace = fs::read_to_string(trace).expect("native Git must produce trace evidence");
    let names: Vec<String> = trace
        .lines()
        .filter(|line| line.contains("\"event\":\"cmd_name\""))
        .map(|line| {
            line.split("\"name\":\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
                .to_owned()
        })
        .collect();
    assert!(
        !names.is_empty(),
        "trace must contain actual native Git commands"
    );
    // Git stash -u itself runs stash/clean after saving the untracked objects.
    // Reject an application clean command, while retaining native stash behavior.
    for line in trace.lines().filter(|line| {
        line.contains("\"event\":\"cmd_name\"") && line.contains("\"name\":\"clean\"")
    }) {
        let hierarchy = line
            .split("\"hierarchy\":\"")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap();
        assert!(
            hierarchy.split('/').any(|command| command == "stash"),
            "reset must not run a standalone git clean: {line}"
        );
    }
    (output, names)
}

fn acceptance_reset_collision(kind: &str) {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("reset-collision").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "logical",
        remote.to_str().unwrap(),
        "child",
    ]);
    h.create_config(&format!(
        "[nickname]\npath = \"child\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let child = h.work_dir.join("child");
    let old_head = h.git_at(&child, &["rev-parse", "HEAD"]);
    fs::write(child.join("collision"), b"tracked parent target\n").unwrap();
    h.git_at(&child, &["add", "collision"]);
    h.git_at(
        &child,
        &["commit", "-m", "target introduces tracked collision"],
    );
    let target = h.git_at(&child, &["rev-parse", "HEAD"]);
    h.git_stdout(&["add", "child", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "record reset target"]);
    assert_eq!(h.git_stdout(&["rev-parse", "HEAD:child"]), target);
    assert!(
        h.git_stdout(&["ls-files", "--stage", "--", "child"])
            .starts_with(&format!("160000 {target} "))
    );
    h.git_at(&child, &["checkout", "--detach", &old_head]);
    assert_ne!(old_head, target);
    fs::write(child.join("LICENSE"), b"prior stash\0\xff").unwrap();
    h.git_at(&child, &["stash", "push", "-m", "prior stash"]);
    let stash = h.git_at(&child, &["rev-parse", "refs/stash"]);
    let gitdir = std::path::PathBuf::from(h.git_at(&child, &["rev-parse", "--absolute-git-dir"]));
    fs::write(gitdir.join("info/exclude"), "/collision\n").unwrap();
    if kind == "file" {
        fs::write(child.join("collision"), b"ignored priceless\0\xff").unwrap();
    } else {
        let collision = child.join("collision");
        fs::create_dir(&collision).unwrap();
        fs::write(
            collision.join("priceless"),
            b"ignored nested priceless\0\xff",
        )
        .unwrap();
        if kind == "repository" {
            h.git_at(&collision, &["init", "-b", "main"]);
            h.git_at(&collision, &["add", "priceless"]);
            h.git_at(&collision, &["commit", "-m", "nested local history"]);
        }
    }
    assert_eq!(
        h.git_at(&child, &["check-ignore", "collision"]),
        "collision"
    );
    fs::write(child.join("LICENSE"), b"staged bytes\0\xff").unwrap();
    h.git_at(&child, &["add", "LICENSE"]);
    fs::write(child.join("LICENSE"), b"unstaged bytes\0\xff").unwrap();
    fs::write(child.join("untracked"), b"untracked bytes\0\xff").unwrap();
    let tree = acceptance_tree_bytes(&child);
    let parent = h.preservation_snapshot();
    let parent_index = fs::read(h.work_dir.join(".git/index")).unwrap();
    let index = fs::read(gitdir.join("index")).unwrap();
    let index_entries = h.git_at(&child, &["ls-files", "--stage"]);
    let config = fs::read(gitdir.join("config")).unwrap();
    let refs = h.git_at(&child, &["show-ref"]);
    let (output, commands) = acceptance_traced_reset(&h);
    assert!(!output.status.success(), "{kind}: {output:?}");
    assert!(
        !commands
            .iter()
            .any(|name| ["stash", "reset", "checkout"].contains(&name.as_str())),
        "collision must refuse before preservation or checkout: {commands:?}"
    );
    assert_eq!(acceptance_tree_bytes(&child), tree, "{kind}: {output:?}");
    assert_eq!(fs::read(gitdir.join("index")).unwrap(), index, "{kind}");
    assert_eq!(h.git_at(&child, &["ls-files", "--stage"]), index_entries);
    assert_eq!(fs::read(gitdir.join("config")).unwrap(), config);
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), old_head);
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(h.git_at(&child, &["rev-parse", "refs/stash"]), stash);
    assert_eq!(h.git_at(&child, &["cat-file", "-t", &stash]), "commit");
    assert_eq!(h.preservation_snapshot(), parent);
    assert_eq!(
        fs::read(h.work_dir.join(".git/index")).unwrap(),
        parent_index
    );
    assert_eq!(h.git_stdout(&["rev-parse", "HEAD:child"]), target);
}

#[test]
fn phase2_acceptance_r05_ignored_file_collision_refuses_before_stash() {
    acceptance_reset_collision("file");
}
#[test]
fn phase2_acceptance_r05_ignored_directory_collision_refuses_before_stash() {
    acceptance_reset_collision("directory");
}
#[test]
fn phase2_acceptance_r05_ignored_nested_repository_collision_refuses_before_stash() {
    acceptance_reset_collision("repository");
}

#[test]
fn phase2_acceptance_r05_successful_reset_reaches_pin_and_recovers_work() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("reset-success").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "logical",
        remote.to_str().unwrap(),
        "child",
    ]);
    h.create_config(&format!(
        "[nickname]\npath = \"child\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    h.git_stdout(&["add", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "record parent pin"]);
    let target = h.git_stdout(&["rev-parse", "HEAD:child"]);
    let child = h.work_dir.join("child");
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), target);
    fs::write(child.join("local-only"), b"local history\0\xff").unwrap();
    h.git_at(&child, &["add", "local-only"]);
    h.git_at(&child, &["commit", "-m", "local child commit beyond pin"]);
    let old_head = h.git_at(&child, &["rev-parse", "HEAD"]);
    assert_ne!(old_head, target);
    fs::write(child.join("LICENSE"), b"prior stash\0\xff").unwrap();
    h.git_at(&child, &["stash", "push", "-m", "prior stash"]);
    let prior_stash = h.git_at(&child, &["rev-parse", "refs/stash"]);
    fs::write(child.join("LICENSE"), b"staged original\0\xff").unwrap();
    h.git_at(&child, &["add", "LICENSE"]);
    fs::write(child.join("LICENSE"), b"unstaged original\0\xff").unwrap();
    fs::write(child.join("untracked"), b"untracked original\0\xff").unwrap();
    let original_index = h.git_at(&child, &["ls-files", "--stage"]);
    let gitdir = std::path::PathBuf::from(h.git_at(&child, &["rev-parse", "--absolute-git-dir"]));
    fs::write(gitdir.join("info/exclude"), "/ignored-precious\n").unwrap();
    fs::write(child.join("ignored-precious"), b"keep ignored\0\xff").unwrap();
    let parent = h.preservation_snapshot();
    let (output, commands) = acceptance_traced_reset(&h);
    assert!(output.status.success(), "{output:?}");
    assert!(
        commands.iter().any(|name| name == "stash"),
        "successful reset must preserve dirty work: {commands:?}"
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), target);
    assert!(!child.join("local-only").exists());
    assert_eq!(fs::read(child.join("LICENSE")).unwrap(), b"MIT License\n");
    assert!(!child.join("untracked").exists());
    assert_eq!(
        fs::read(child.join("ignored-precious")).unwrap(),
        b"keep ignored\0\xff"
    );
    assert_eq!(h.preservation_snapshot(), parent);
    assert_eq!(h.git_stdout(&["rev-parse", "HEAD:child"]), target);
    assert!(
        h.git_stdout(&["ls-files", "--stage", "--", "child"])
            .starts_with(&format!("160000 {target} "))
    );
    let stash = h.git_at(&child, &["rev-parse", "refs/stash"]);
    assert_ne!(stash, prior_stash);
    assert_eq!(h.git_at(&child, &["cat-file", "-t", &stash]), "commit");
    assert_eq!(
        h.git_at(&child, &["cat-file", "-t", &prior_stash]),
        "commit"
    );
    assert_eq!(
        h.git_at(&child, &["rev-parse", &format!("{stash}^1")]),
        old_head
    );
    let recovery = h.temp_dir.path().join("recovery");
    h.git_at(
        &child,
        &[
            "worktree",
            "add",
            "--detach",
            recovery.to_str().unwrap(),
            &old_head,
        ],
    );
    h.git_at(&recovery, &["stash", "apply", "--index", &stash]);
    assert_eq!(
        h.git_at(&recovery, &["ls-files", "--stage"]),
        original_index
    );
    assert_eq!(
        fs::read(recovery.join("LICENSE")).unwrap(),
        b"unstaged original\0\xff"
    );
    assert_eq!(
        fs::read(recovery.join("untracked")).unwrap(),
        b"untracked original\0\xff"
    );
    assert_eq!(
        fs::read(recovery.join("local-only")).unwrap(),
        b"local history\0\xff"
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), target);
    assert_eq!(h.git_at(&child, &["rev-parse", "refs/stash"]), stash);
}

#[test]
fn phase2_acceptance_custom_portable_update_refused_without_execution() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("portable-custom-update").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "logical",
        remote.to_str().unwrap(),
        "child",
    ]);
    h.create_config(&format!(
        "[nickname]\npath = \"child\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    h.run_submod_success(&["update"]).unwrap();
    h.git_stdout(&[
        "config",
        "--file",
        ".gitmodules",
        "submodule.logical.update",
        "!touch custom-update-executed",
    ]);
    h.git_stdout(&["add", ".gitmodules"]);
    let child = h.work_dir.join("child");
    let gitdir = std::path::PathBuf::from(h.git_at(&child, &["rev-parse", "--absolute-git-dir"]));
    let parent = h.preservation_snapshot();
    let parent_index = fs::read(h.work_dir.join(".git/index")).unwrap();
    let checkout = acceptance_tree_bytes(&child);
    let index = fs::read(gitdir.join("index")).unwrap();
    let config = fs::read(gitdir.join("config")).unwrap();
    let refs = h.git_at(&child, &["show-ref"]);
    let head = h.git_at(&child, &["rev-parse", "HEAD"]);
    let output = h.run_submod(&["update"]).unwrap();
    assert!(!output.status.success(), "{output:?}");
    assert!(!child.join("custom-update-executed").exists(), "{output:?}");
    assert!(
        !h.work_dir.join("custom-update-executed").exists(),
        "{output:?}"
    );
    assert_eq!(acceptance_tree_bytes(&child), checkout);
    assert_eq!(h.preservation_snapshot(), parent);
    assert_eq!(
        fs::read(h.work_dir.join(".git/index")).unwrap(),
        parent_index
    );
    assert_eq!(fs::read(gitdir.join("index")).unwrap(), index);
    assert_eq!(fs::read(gitdir.join("config")).unwrap(), config);
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), head);
}

#[cfg(test)]
mod phase3_acceptance_persistence {
    use super::*;
    use std::path::Path;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    fn spawn(h: &TestHarness, cwd: &Path, config: &Path, field: &str, value: &str) -> Child {
        Command::new(&h.submod_bin)
            .current_dir(cwd)
            .env("GIT_CONFIG_GLOBAL", h.temp_dir.path().join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .args([
                "--config",
                config.to_str().unwrap(),
                "change-global",
                field,
                value,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    }

    fn finish(mut child: Child) {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if child.try_wait().unwrap().is_some() {
                let output = child.wait_with_output().unwrap();
                assert!(output.status.success(), "writer failed: {output:?}");
                return;
            }
            if Instant::now() > deadline {
                child.kill().unwrap();
                let output = child.wait_with_output().unwrap();
                panic!("writer deadlocked: {output:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn config_lock(config: &Path) -> std::path::PathBuf {
        config
            .parent()
            .unwrap()
            .canonicalize()
            .unwrap()
            .join(format!(
                "{}.submod.lock",
                config.file_name().unwrap().to_str().unwrap()
            ))
    }

    fn common_lock(h: &TestHarness, cwd: &Path) -> std::path::PathBuf {
        let common = h.git_at(
            cwd,
            &["rev-parse", "--path-format=absolute", "--git-common-dir"],
        );
        Path::new(common.trim())
            .canonicalize()
            .unwrap()
            .join("submod.lock")
    }

    fn release_after_arrival(
        children: &mut [Child],
        arrivals: &[std::path::PathBuf],
        gates: &[std::path::PathBuf],
        before_release: impl FnOnce(),
    ) {
        let deadline = Instant::now() + Duration::from_secs(5);
        let problem = loop {
            if children
                .iter_mut()
                .any(|child| child.try_wait().unwrap().is_some())
            {
                break Some("writer exited before lock arrival");
            }
            if arrivals.iter().all(|path| path.is_file()) {
                break None;
            }
            if Instant::now() >= deadline {
                break Some("lock-arrival barrier timed out");
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        if problem.is_none() {
            before_release();
        }
        for gate in gates {
            fs::remove_file(gate).unwrap();
        }
        if let Some(problem) = problem {
            for child in children {
                let _ = child.kill();
                let status = child.wait().unwrap();
                let mut stderr = String::new();
                std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut stderr)
                    .unwrap();
                eprintln!("writer {status}: {stderr}");
            }
            panic!("{problem}; expected lock paths: {arrivals:?}");
        }
    }

    fn writers(layout: &str) {
        let h = TestHarness::new().unwrap();
        h.init_git_repo().unwrap();
        let linked = h.temp_dir.path().join("linked");
        let other = h.temp_dir.path().join("repo-b");
        if layout == "linked" {
            h.git_stdout(&[
                "worktree",
                "add",
                "-b",
                "acceptance-linked",
                linked.to_str().unwrap(),
            ]);
        }
        if layout == "shared" {
            fs::create_dir(&other).unwrap();
            h.git_at(&other, &["init"]);
        }
        let second_cwd = match layout {
            "linked" => linked.as_path(),
            "shared" => other.as_path(),
            _ => h.work_dir.as_path(),
        };
        // Keep output locks lexically after both canonical common-dir locks.
        let configs = h.temp_dir.path().join("z-configs");
        fs::create_dir(&configs).unwrap();
        let first_config = configs.join("a.toml");
        fs::write(&first_config, "# retain\n[defaults]\nignore='dirty'\n").unwrap();
        let second_config = if layout == "different" || layout == "linked" {
            let path = configs.join("b.toml");
            fs::write(&path, "# retain\n[defaults]\nignore='dirty'\n").unwrap();
            path
        } else {
            first_config.clone()
        };
        let first_common = common_lock(&h, &h.work_dir);
        let second_common = common_lock(&h, second_cwd);
        let first_output = config_lock(&first_config);
        let second_output = config_lock(&second_config);
        assert!(first_common < first_output);
        assert!(second_common < second_output);
        if layout == "shared" {
            assert_ne!(first_common, second_common);
            assert_eq!(first_output, second_output);
        } else {
            assert_eq!(first_common, second_common);
        }
        let mut gates = vec![first_output.clone(), second_output.clone()];
        gates.sort();
        gates.dedup();
        for gate in &gates {
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(gate)
                .unwrap();
        }
        let mut children = vec![
            spawn(&h, &h.work_dir, &first_config, "--ignore", "all"),
            spawn(&h, second_cwd, &second_config, "--fetch", "never"),
        ];
        // Distinct repositories must BOTH acquire their common lock before release.
        // Shared-common-dir writers require that common lock plus both live children.
        release_after_arrival(
            &mut children,
            &[first_common.clone(), second_common.clone()],
            &gates,
            || {},
        );
        for child in children {
            finish(child);
        }
        let first_doc: toml::Value =
            toml::from_str(&fs::read_to_string(&first_config).unwrap()).unwrap();
        let second_doc: toml::Value =
            toml::from_str(&fs::read_to_string(&second_config).unwrap()).unwrap();
        assert_eq!(first_doc["defaults"]["ignore"].as_str(), Some("all"));
        assert_eq!(
            second_doc["defaults"]["fetchRecurse"].as_str(),
            Some("never")
        );
        for (cwd, config) in [
            (&h.work_dir as &Path, &first_config),
            (second_cwd, &second_config),
        ] {
            assert!(fs::read_to_string(config).unwrap().contains("# retain"));
            let output = h
                .run_submod_at(cwd, &["--config", config.to_str().unwrap(), "list"])
                .unwrap();
            assert!(
                output.status.success(),
                "fresh process rejected config: {output:?}"
            );
            assert!(!config_lock(config).exists());
        }
        assert!(!first_common.exists());
        assert!(!second_common.exists());
        if layout == "linked" {
            let gitdir = h.git_at(second_cwd, &["rev-parse", "--absolute-git-dir"]);
            assert!(!Path::new(gitdir.trim()).join("submod.lock").exists());
        }
    }

    #[test]
    fn r10_simultaneous_same_config() {
        writers("same");
    }
    #[test]
    fn r10_simultaneous_different_configs() {
        writers("different");
    }
    #[test]
    fn r10_simultaneous_linked_worktrees() {
        writers("linked");
    }
    #[test]
    fn r10_simultaneous_shared_config_across_repositories() {
        writers("shared");
    }

    fn from_setup_relative_output(relative: Option<&str>) {
        let h = TestHarness::new().unwrap();
        h.init_git_repo().unwrap();
        let destination = h.work_dir.join(relative.unwrap_or("submod.toml"));
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        let common = common_lock(&h, &h.work_dir);
        let output_lock = config_lock(&destination);
        let mut args = vec!["generate-config", "--from-setup"];
        if let Some(relative) = relative {
            args.extend(["--output", relative]);
        }
        h.run_submod_success(&args).unwrap();
        let _: toml::Value = toml::from_str(&fs::read_to_string(&destination).unwrap()).unwrap();
        h.run_submod_success(&["--config", destination.to_str().unwrap(), "list"])
            .unwrap();
        assert!(!common.exists(), "common lock leaked");
        assert!(!output_lock.exists(), "output lock leaked");
        if relative.is_some() {
            assert!(
                !h.config_path().exists(),
                "explicit relative output also wrote default output"
            );
        }
    }

    #[test]
    fn r10_from_setup_default_relative_output() {
        from_setup_relative_output(None);
    }

    #[test]
    fn r10_from_setup_relative_subdirectory_output() {
        from_setup_relative_output(Some("nested/generated.toml"));
    }

    #[test]
    fn r10_from_setup_acquires_common_lock_before_output_lock() {
        let h = TestHarness::new().unwrap();
        h.init_git_repo().unwrap();
        let output = h.work_dir.join("generated.toml");
        let common = common_lock(&h, &h.work_dir);
        let gate = config_lock(&output);
        assert!(common < gate, "fixture must hold the later-sorting lock");
        assert!(!common.exists());
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&gate)
            .unwrap();
        let child = Command::new(&h.submod_bin)
            .current_dir(&h.work_dir)
            .env("GIT_CONFIG_GLOBAL", h.temp_dir.path().join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .args(["generate-config", "--from-setup", "--output"])
            .arg(&output)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut children = vec![child];
        release_after_arrival(
            &mut children,
            std::slice::from_ref(&common),
            std::slice::from_ref(&gate),
            || {
                assert!(
                    !output.exists(),
                    "output written before acquiring output lock"
                );
            },
        );
        finish(children.pop().unwrap());
        let _: toml::Value = toml::from_str(&fs::read_to_string(&output).unwrap()).unwrap();
        h.run_submod_success(&["--config", output.to_str().unwrap(), "list"])
            .unwrap();
        assert!(!common.exists());
        assert!(!gate.exists());
    }

    #[test]
    fn r10_reloads_config_changed_after_preload_before_lock() {
        let h = TestHarness::new().unwrap();
        h.init_git_repo().unwrap();
        h.create_config("[defaults]\nignore='dirty'\n").unwrap();
        let config = h.config_path();
        let common = common_lock(&h, &h.work_dir);
        let output = config_lock(&config);
        assert!(common < output);
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
            .unwrap();
        let mut children = vec![spawn(&h, &h.work_dir, &config, "--ignore", "all")];
        // Observing the first acquired lock proves construction/preload finished,
        // while the fixture-owned second lock prevents the locked reload.
        release_after_arrival(
            &mut children,
            std::slice::from_ref(&common),
            std::slice::from_ref(&output),
            || {
                fs::write(&config, "# external edit after preload\n[defaults]\nignore='dirty'\nfetchRecurse='never'\n").unwrap();
            },
        );
        finish(children.pop().unwrap());
        let after = fs::read_to_string(&config).unwrap();
        let raw: toml::Value = toml::from_str(&after).unwrap();
        assert_eq!(raw["defaults"]["ignore"].as_str(), Some("all"));
        assert_eq!(raw["defaults"]["fetchRecurse"].as_str(), Some("never"));
        assert!(after.contains("# external edit after preload"));
        assert!(!common.exists());
        assert!(!output.exists());
        h.run_submod_success(&["list"]).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn r10_unwritable_parent_preserves_bytes_and_releases_locks() {
        let h = TestHarness::new().unwrap();
        h.init_git_repo().unwrap();
        h.create_config("[defaults]\nignore='dirty'\n").unwrap();
        let before = fs::read(h.config_path()).unwrap();
        let mode = fs::metadata(&h.work_dir).unwrap().permissions();
        fs::set_permissions(&h.work_dir, fs::Permissions::from_mode(0o555)).unwrap();
        let output = h.run_submod(&["change-global", "--ignore", "all"]).unwrap();
        fs::set_permissions(&h.work_dir, mode).unwrap();
        assert!(
            !output.status.success(),
            "unwritable parent accepted: {output:?}"
        );
        assert_eq!(fs::read(h.config_path()).unwrap(), before);
        let _: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
        assert!(!h.work_dir.join(".git/submod.lock").exists());
        assert!(!h.work_dir.join("submod.toml.submod.lock").exists());
        h.run_submod_success(&["change-global", "--ignore", "all"])
            .unwrap();
        h.run_submod_success(&["list"]).unwrap();
    }
}
