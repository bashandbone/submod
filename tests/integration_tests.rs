// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Integration tests for the submod CLI tool
//!
//! These tests focus on end-to-end behavior rather than implementation details,
//! testing actual CLI invocations, file system interactions, and git operations.

use std::fs;

mod common;
use common::TestHarness;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_command_with_no_config() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Run check command without config file
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run submod");

        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("submod.toml") && stderr.contains("was not found"));
        assert!(stderr.contains("generate-config --from-setup"));
        assert!(!harness.file_exists("submod.toml"));
        assert!(!harness.file_exists(".gitmodules"));
    }

    #[test]
    fn test_check_command_with_empty_config() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("# Empty config\n")
            .expect("Failed to create config");

        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run submod");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Check complete: all configured submodules match."));
        assert_eq!(harness.read_config().unwrap(), "# Empty config\n");
    }

    #[test]
    fn test_add_submodule_basic() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("test_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add a submodule
        let stdout = harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "test-lib",
                "--path",
                "lib/test",
            ])
            .expect("Failed to add submodule");

        assert!(stdout.contains("Added submodule"));

        // Verify config file was created/updated
        let config = harness.read_config().expect("Failed to read config");
        assert!(config.contains("[test-lib]"));
        assert!(config.contains("path = \"lib/test\""));
        assert!(config.contains(&format!("url = \"{remote_url}\"")));
        let parsed = submod::Config::parse(&config).expect("Failed to parse config");
        assert_eq!(parsed.get_submodule("test-lib").unwrap().active, None);
        assert_eq!(
            parsed.effective_entry("test-lib").unwrap().active,
            Some(true)
        );

        // Verify directory structure was created
        assert!(harness.dir_exists("lib/test"));
        assert!(harness.file_exists("lib/test/.git"));
    }

    #[test]
    fn test_add_submodule_with_sparse_paths() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("sparse_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add submodule with sparse paths
        let stdout = harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "sparse-lib",
                "--path",
                "lib/sparse",
                "--sparse-paths",
                "src,docs",
            ])
            .expect("Failed to add submodule");

        assert!(stdout.contains("Added submodule"));
        assert!(stdout.contains("Configured sparse checkout"));

        // Verify config includes sparse paths
        let config = harness.read_config().expect("Failed to read config");
        assert!(config.contains("sparse_paths = [\"src\", \"docs\"]"));

        // Verify sparse checkout is configured
        let sparse_file = harness.get_sparse_checkout_file_path("lib/sparse");
        assert!(sparse_file.exists());

        let sparse_content = fs::read_to_string(sparse_file).expect("Failed to read sparse file");
        assert!(sparse_content.contains("src"));
        assert!(sparse_content.contains("docs"));
    }

    #[test]
    fn test_init_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("init_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Create config manually
        let config_content = format!(
            r#"[init-lib]
path = "lib/init"
url = "{remote_url}"
active = true
sparse_paths = ["src"]
"#
        );
        harness
            .create_config(&config_content)
            .expect("Failed to create config");

        // Run init command (verbose to verify status messages)
        let stdout = harness
            .run_submod_success(&["init", "--verbose"])
            .expect("Failed to run init");

        assert!(stdout.contains("init-lib at lib/init: changed:"));
        assert!(stdout.contains(
            "Initialization summary: 1 changed, 0 unchanged, 0 skipped, 0 failed, 0 pending."
        ));
        let target = harness.git_at(&remote_repo, &["rev-parse", "HEAD"]);
        assert!(stdout.contains(&format!("(target {target})")));
        assert_eq!(
            harness.git_stdout(&["-C", "lib/init", "rev-parse", "HEAD"]),
            target
        );
        assert!(harness.file_exists("lib/init/src/main.c"));

        // Verify directory was created
        assert!(harness.dir_exists("lib/init"));
        assert!(harness.file_exists("lib/init/.git"));

        // Verify sparse checkout was configured
        let sparse_file = harness.get_sparse_checkout_file_path("lib/init");
        assert!(sparse_file.exists());
    }

    #[test]
    fn test_update_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("update_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add and initialize submodule first
        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "update-lib",
                "--path",
                "lib/update",
            ])
            .expect("Failed to add submodule");

        // Run update command
        let stdout = harness
            .run_submod_success(&["update"])
            .expect("Failed to run update");

        let target = harness.git_at(&remote_repo, &["rev-parse", "HEAD"]);
        assert!(
            stdout.contains("update-lib at lib/update: unchanged:"),
            "{stdout}"
        );
        assert!(
            stdout.contains(
                "Update summary: 0 changed, 1 unchanged, 0 skipped, 0 failed, 0 pending."
            ),
            "{stdout}"
        );
        assert!(stdout.contains(&format!("(target {target})")), "{stdout}");
        assert_eq!(
            harness.git_stdout(&["-C", "lib/update", "rev-parse", "HEAD"]),
            target
        );

        // gitoxide's fetch report is plumbing narration for `gix fetch`, not part
        // of submod's output. Leaking it to stdout is what made the per-submodule
        // line unassertable in the first place.
        assert!(
            !stdout.contains("refs/remotes/origin/"),
            "update must not leak the raw fetch refspec report to stdout; got: {stdout}"
        );
    }

    /// `update` must check the submodule worktree out to the commit recorded as
    /// the superproject's gitlink — the defining job of `git submodule update`.
    /// Regression test for the gix path silently treating checkout as a no-op
    /// (it fetched but never moved the worktree), which left the submodule
    /// stuck behind its recorded commit (#62 P1).
    #[test]
    fn update_checks_out_recorded_gitlink_commit() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("upd_pin")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&["add", &remote_url, "--name", "upd-pin", "--path", "lib/upd"])
            .expect("Failed to add submodule");

        // C1 = the commit the submodule was added at.
        let c1 = harness.git_stdout(&["-C", "lib/upd", "rev-parse", "HEAD"]);

        // Create a second commit C2 inside the submodule, record it as the
        // superproject's gitlink, then move the worktree back to C1 so the
        // recorded gitlink is *ahead* of the checked-out worktree.
        fs::write(harness.work_dir.join("lib/upd/NEW.txt"), "new content\n")
            .expect("Failed to write file in submodule");
        harness.git_stdout(&["-C", "lib/upd", "add", "."]);
        harness.git_stdout(&["-C", "lib/upd", "commit", "-m", "c2"]);
        let c2 = harness.git_stdout(&["-C", "lib/upd", "rev-parse", "HEAD"]);
        assert_ne!(c1, c2, "the two submodule commits must differ");

        harness.git_stdout(&["add", "lib/upd"]); // record gitlink at C2
        harness.git_stdout(&["-C", "lib/upd", "checkout", &c1]); // worktree back to C1

        // Preconditions (guard against a vacuous pass): worktree behind the
        // recorded gitlink, which itself is a staged submodule pointing at C2.
        assert_eq!(
            harness.git_stdout(&["-C", "lib/upd", "rev-parse", "HEAD"]),
            c1,
            "precondition: submodule worktree must start at C1"
        );
        assert_eq!(
            harness.index_gitlink_mode("lib/upd").as_deref(),
            Some("160000"),
            "precondition: lib/upd must be a staged submodule"
        );
        assert!(
            harness
                .git_stdout(&["ls-files", "--stage", "lib/upd"])
                .contains(&c2),
            "precondition: the recorded gitlink must point at C2"
        );

        harness
            .run_submod_success(&["update"])
            .expect("Failed to run update");

        // update must have checked the worktree out to the recorded commit C2.
        assert_eq!(
            harness.git_stdout(&["-C", "lib/upd", "rev-parse", "HEAD"]),
            c2,
            "update must checkout the superproject-recorded gitlink commit (C2)"
        );
        assert!(
            harness.file_exists("lib/upd/NEW.txt"),
            "C2's tree must be materialized in the worktree after update"
        );
    }

    /// Default update retains the parent pin even when the remote advances.
    #[test]
    fn update_against_advanced_remote_preserves_parent_pin() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("upd_adv")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&["add", &remote_url, "--name", "upd-adv", "--path", "lib/adv"])
            .expect("Failed to add submodule");

        // The recorded gitlink and the worktree both start at this commit.
        let recorded = harness.git_stdout(&["-C", "lib/adv", "rev-parse", "HEAD"]);

        // Move the remote forward; the new commit is not yet known locally.
        let advanced = harness
            .advance_test_remote("upd_adv")
            .expect("Failed to advance remote");
        assert_ne!(recorded, advanced, "the remote must have actually moved");
        let unknown = harness
            .git_cmd()
            .env("LC_ALL", "C")
            .args(["-C", "lib/adv", "cat-file", "-t", &advanced])
            .current_dir(&harness.work_dir)
            .output()
            .expect("probe unknown object");
        assert!(
            unknown.status.code() == Some(128)
                && unknown.stdout.is_empty()
                && String::from_utf8_lossy(&unknown.stderr).contains("could not get object info"),
            "precondition: the advanced commit must be unknown before update: {unknown:?}"
        );

        harness
            .run_submod_success(&["update"])
            .expect("Failed to run update");

        assert!(!harness.file_exists("lib/adv/ADVANCE.txt"));
        // Default update materializes the recorded commit.
        assert_eq!(
            harness.git_stdout(&["-C", "lib/adv", "rev-parse", "HEAD"]),
            recorded,
            "HEAD must stay at the recorded gitlink, not jump to the remote tip"
        );
    }

    /// `check` must report a submodule whose worktree has uncommitted changes as
    /// dirty. Regression test for `is_dirty` being a stub that always reported
    /// clean when HEAD resolved (`src/git_manager.rs`), so the status command
    /// could never surface a modified working tree (#62 P1).
    #[test]
    fn check_reports_dirty_submodule_worktree() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("dirty_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "dirty-lib",
                "--path",
                "lib/dirty",
            ])
            .expect("Failed to add submodule");

        // Force checkout with LF line endings using test global config to prevent CRLF dirty status on Windows
        let _ = harness.git_stdout(&["-C", "lib/dirty", "checkout", "--", "."]);

        // Precondition (guards against a vacuous pass): a clean worktree is
        // reported clean. If this ever fails, the dirty assertion below would
        // be meaningless.
        let clean_out = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check on clean submodule");
        assert!(
            clean_out.contains("dirty-lib: unchanged (matches configured state)"),
            "precondition: a freshly-added submodule must report a clean worktree, got:\n{clean_out}"
        );

        // Modify a tracked file in the submodule worktree without committing.
        fs::write(
            harness.work_dir.join("lib/dirty/LICENSE"),
            "MIT License\nlocal edit\n",
        )
        .expect("Failed to dirty submodule worktree");

        // Precondition: git itself sees the worktree as dirty.
        assert!(
            !harness
                .git_stdout(&["-C", "lib/dirty", "status", "--porcelain"])
                .is_empty(),
            "precondition: the submodule worktree must be dirty per git"
        );

        let before = harness.preservation_snapshot();
        let child_before = harness.git_stdout(&["-C", "lib/dirty", "status", "--porcelain=v1"]);
        let child_index_before = harness.git_stdout(&["-C", "lib/dirty", "ls-files", "--stage"]);
        let child_head_before = harness.git_stdout(&["-C", "lib/dirty", "rev-parse", "HEAD"]);
        let child_config_before =
            harness.git_stdout(&["-C", "lib/dirty", "config", "--local", "--list"]);
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run check on dirty submodule");
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(harness.preservation_snapshot(), before);
        assert_eq!(
            harness.git_stdout(&["-C", "lib/dirty", "status", "--porcelain=v1"]),
            child_before
        );
        assert_eq!(
            harness.git_stdout(&["-C", "lib/dirty", "ls-files", "--stage"]),
            child_index_before
        );
        assert_eq!(
            harness.git_stdout(&["-C", "lib/dirty", "rev-parse", "HEAD"]),
            child_head_before
        );
        assert_eq!(
            harness.git_stdout(&["-C", "lib/dirty", "config", "--local", "--list"]),
            child_config_before
        );
        let dirty_out = String::from_utf8_lossy(&output.stdout);
        assert_eq!(
            fs::read_to_string(harness.work_dir.join("lib/dirty/LICENSE")).unwrap(),
            "MIT License\nlocal edit\n"
        );
        assert!(
            dirty_out.contains("dirty-lib: drift: working tree has changes"),
            "check must report the modified submodule worktree as dirty, got:\n{dirty_out}"
        );
        assert!(
            !dirty_out.contains("dirty-lib: unchanged"),
            "check must not report the modified submodule worktree as clean, got:\n{dirty_out}"
        );
    }

    #[test]
    fn test_reset_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("reset_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add and initialize submodule
        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "reset-lib",
                "--path",
                "lib/reset",
            ])
            .expect("Failed to add submodule");

        let pin = harness.git_at(&harness.work_dir.join("lib/reset"), &["rev-parse", "HEAD"]);
        let parent_index =
            fs::read(harness.work_dir.join(".git/index")).expect("Failed to snapshot parent index");

        // Make some changes in the submodule
        fs::write(
            harness.work_dir.join("lib/reset/test_file.txt"),
            "This is a test change",
        )
        .expect("Failed to create test file");

        // Run reset command
        let stdout = harness
            .run_submod_success(&["reset", "reset-lib"])
            .expect("Failed to run reset");

        assert!(stdout.contains("Reset summary: 1 changed, 0 unchanged, 0 skipped, 0 failed."));
        assert!(stdout.contains(&format!("reset-lib reset to {pin}")));

        // Verify test file was removed
        assert!(!harness.file_exists("lib/reset/test_file.txt"));
        assert_eq!(
            harness.git_at(&harness.work_dir.join("lib/reset"), &["rev-parse", "HEAD"]),
            pin
        );
        assert_eq!(
            fs::read(harness.work_dir.join(".git/index")).unwrap(),
            parent_index
        );
        let stash = harness.git_at(
            &harness.work_dir.join("lib/reset"),
            &["rev-parse", "refs/stash"],
        );
        assert!(stdout.contains(&format!("Preserved local work in stash {stash}")));
        let short = &stash[..stash.len().min(12)];
        assert!(stdout.contains(&format!("git stash branch submod-recovery-{short} {stash}")));
        assert_eq!(
            harness.git_at(
                &harness.work_dir.join("lib/reset"),
                &["show", &format!("{stash}^3:test_file.txt")],
            ),
            "This is a test change"
        );
        harness.git_at(
            &harness.work_dir.join("lib/reset"),
            &[
                "stash",
                "branch",
                &format!("submod-recovery-{short}"),
                &stash,
            ],
        );
        assert_eq!(
            fs::read(harness.work_dir.join("lib/reset/test_file.txt")).unwrap(),
            b"This is a test change"
        );
        assert_eq!(
            fs::read(harness.work_dir.join(".git/index")).unwrap(),
            parent_index
        );
    }

    #[test]
    fn test_reset_all_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo1 = harness
            .create_test_remote("reset_lib1")
            .expect("Failed to create remote");
        let remote_repo2 = harness
            .create_test_remote("reset_lib2")
            .expect("Failed to create remote");
        let remote_url1 = format!("file://{}", remote_repo1.display());
        let remote_url2 = format!("file://{}", remote_repo2.display());

        // Add two submodules
        harness
            .run_submod_success(&[
                "add",
                &remote_url1,
                "--name",
                "reset-lib1",
                "--path",
                "lib/reset1",
            ])
            .expect("Failed to add submodule 1");

        harness
            .run_submod_success(&[
                "add",
                &remote_url2,
                "--name",
                "reset-lib2",
                "--path",
                "lib/reset2",
            ])
            .expect("Failed to add submodule 2");

        let pins: Vec<_> = ["lib/reset1", "lib/reset2"]
            .iter()
            .map(|path| harness.git_stdout(&["-C", path, "rev-parse", "HEAD"]))
            .collect();
        let parent_index = fs::read(harness.work_dir.join(".git/index")).unwrap();
        // Make changes in both submodules
        fs::write(harness.work_dir.join("lib/reset1/test1.txt"), "change1")
            .expect("Failed to create test file");
        fs::write(harness.work_dir.join("lib/reset2/test2.txt"), "change2")
            .expect("Failed to create test file");

        // Run reset all command
        let stdout = harness
            .run_submod_success(&["reset", "--all"])
            .expect("Failed to run reset all");

        assert!(stdout.contains("Reset summary: 2 changed, 0 unchanged, 0 skipped, 0 failed."));
        for (i, path) in ["lib/reset1", "lib/reset2"].iter().enumerate() {
            assert!(stdout.contains(&format!("reset-lib{} reset to {}", i + 1, pins[i])));
            assert_eq!(
                harness.git_stdout(&["-C", path, "rev-parse", "HEAD"]),
                pins[i]
            );
            assert_eq!(
                harness.git_stdout(&[
                    "-C",
                    path,
                    "show",
                    &format!("refs/stash^3:test{}.txt", i + 1)
                ]),
                format!("change{}", i + 1)
            );
        }
        assert_eq!(
            fs::read(harness.work_dir.join(".git/index")).unwrap(),
            parent_index
        );

        // Verify both test files were removed
        assert!(!harness.file_exists("lib/reset1/test1.txt"));
        assert!(!harness.file_exists("lib/reset2/test2.txt"));
    }

    #[test]
    fn test_sync_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("sync_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Create config manually without initializing
        let config_content = format!(
            r#"[sync-lib]
path = "lib/sync"
url = "{remote_url}"
active = true
"#
        );
        harness
            .create_config(&config_content)
            .expect("Failed to create config");

        // Run sync command (verbose to verify status messages)
        let stdout = harness
            .run_submod_success(&["sync", "--verbose"])
            .expect("Failed to run sync");

        assert!(stdout.contains("sync-lib at lib/sync: changed:"));
        assert!(
            stdout
                .contains("Sync summary: 1 changed, 0 unchanged, 0 skipped, 0 failed, 0 pending.")
        );
        assert!(!stdout.contains("Reconciling configured submodules"));

        // Verify submodule was initialized
        assert!(harness.dir_exists("lib/sync"));
        assert!(harness.file_exists("lib/sync/.git"));
        assert_eq!(
            harness.index_gitlink_mode("lib/sync").as_deref(),
            Some("160000")
        );
        assert_eq!(
            harness
                .git_stdout(&["-C", "lib/sync", "rev-parse", "HEAD"])
                .trim(),
            harness.git_at(&remote_repo, &["rev-parse", "HEAD"])
        );
        assert_eq!(
            std::fs::read_to_string(harness.work_dir.join("lib/sync/LICENSE"))
                .unwrap()
                .trim(),
            harness.git_at(&remote_repo, &["show", "HEAD:LICENSE"])
        );
    }

    #[test]
    fn test_config_with_defaults() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("defaults_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Create config with defaults
        let config_content = format!(
            r#"[defaults]
ignore = "dirty"

[defaults-lib]
path = "lib/defaults"
url = "{remote_url}"
active = true
"#
        );
        harness
            .create_config(&config_content)
            .expect("Failed to create config");

        harness
            .run_submod_success(&["init"])
            .expect("Failed to initialize inherited settings");
        let raw: toml::Value = toml::from_str(&harness.read_config().unwrap()).unwrap();
        assert!(raw["defaults-lib"].get("ignore").is_none());
        assert_eq!(raw["defaults"]["ignore"].as_str(), Some("dirty"));
        assert_eq!(
            harness
                .git_stdout(&[
                    "config",
                    "--local",
                    "--get",
                    "submodule.defaults-lib.ignore"
                ])
                .trim(),
            "dirty"
        );
        assert_eq!(
            harness
                .git_stdout(&[
                    "config",
                    "--file",
                    ".gitmodules",
                    "--get",
                    "submodule.defaults-lib.ignore"
                ])
                .trim(),
            "dirty"
        );

        // Run check to see if defaults are applied
        let stdout = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");

        assert!(stdout.contains("defaults-lib: unchanged (matches configured state)"));
    }

    #[test]
    fn test_custom_config_file() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let custom_config = harness.work_dir.join("custom.toml");
        fs::write(
            &custom_config,
            "[test-sub]\npath = \"test\"\nurl = \"https://example.com/test.git\"\nactive = true\n",
        )
        .expect("Failed to create custom config");

        // Run with custom config file (verbose to verify output)
        let before = harness.preservation_snapshot();
        let output = harness
            .run_submod(&["--config", "custom.toml", "check", "--verbose"])
            .expect("Failed to run with custom config");
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(harness.preservation_snapshot(), before);
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(stdout.contains("test-sub: drift: checkout is missing"));
        let custom = fs::read_to_string(&custom_config).unwrap();
        let parsed = submod::Config::parse(&custom).unwrap();
        assert_eq!(
            parsed.get_submodule("test-sub").unwrap().path.as_deref(),
            Some("test")
        );
        assert!(!harness.file_exists("submod.toml"));
    }

    #[test]
    fn test_error_handling_invalid_git_repo() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        // Don't initialize git repo

        // Should fail when not in a git repository
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run submod");
        assert_eq!(output.status.code(), Some(1));

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Cannot prepare check"),
            "error should name the operation that failed; got: {stderr}"
        );
        // The underlying cause must survive the wrapping, not be flattened away.
        assert!(
            stderr.contains("not a git repository"),
            "error should preserve the underlying cause; got: {stderr}"
        );
    }

    #[test]
    fn test_error_handling_invalid_url() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let before = harness.preservation_snapshot();
        // Try to add submodule with invalid URL
        let output = harness
            .run_submod(&[
                "add",
                "not-a-valid-url",
                "--name",
                "invalid-lib",
                "--path",
                "lib/invalid",
            ])
            .expect("Failed to run submod");

        assert_eq!(output.status.code(), Some(1));
        assert_eq!(harness.preservation_snapshot(), before);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("repo URL")
                && stderr.contains("not-a-valid-url")
                && stderr.contains("must be absolute or begin"),
            "{stderr}"
        );
        assert!(
            stderr.contains("Add failed"),
            "error should name the operation that failed; got: {stderr}"
        );
    }

    #[test]
    fn test_sparse_checkout_mismatch_detection() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("mismatch_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add submodule with specific sparse paths
        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "mismatch-lib",
                "--path",
                "lib/mismatch",
                "--sparse-paths",
                "src,docs",
            ])
            .expect("Failed to add submodule");

        // Manually modify sparse-checkout file to create mismatch
        let sparse_file = harness.get_sparse_checkout_file_path("lib/mismatch");
        fs::write(&sparse_file, "include\nLICENSE\n").expect("Failed to modify sparse file");

        // Run check command
        let before = harness.preservation_snapshot();
        let child_before = harness.git_stdout(&["-C", "lib/mismatch", "status", "--porcelain=v1"]);
        let child_index_before = harness.git_stdout(&["-C", "lib/mismatch", "ls-files", "--stage"]);
        let child_head_before = harness.git_stdout(&["-C", "lib/mismatch", "rev-parse", "HEAD"]);
        let child_config_before =
            harness.git_stdout(&["-C", "lib/mismatch", "config", "--local", "--list"]);
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run check");
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(harness.preservation_snapshot(), before);
        assert_eq!(
            harness.git_stdout(&["-C", "lib/mismatch", "status", "--porcelain=v1"]),
            child_before
        );
        assert_eq!(
            harness.git_stdout(&["-C", "lib/mismatch", "ls-files", "--stage"]),
            child_index_before
        );
        assert_eq!(
            harness.git_stdout(&["-C", "lib/mismatch", "rev-parse", "HEAD"]),
            child_head_before
        );
        assert_eq!(
            harness.git_stdout(&["-C", "lib/mismatch", "config", "--local", "--list"]),
            child_config_before
        );
        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(stdout.contains("mismatch-lib: drift: sparse patterns differ"));
        assert_eq!(
            fs::read_to_string(&sparse_file).unwrap(),
            "include\nLICENSE\n"
        );
        assert!(stdout.contains("expected [\"!/*\", \"src\", \"docs\"]"));
        assert!(stdout.contains(r#"current ["include", "LICENSE"]"#));
    }

    #[test]
    fn test_list_command_empty_config() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("# empty\n")
            .expect("Failed to create config");

        let stdout = harness
            .run_submod_success(&["list"])
            .expect("Failed to run list");

        assert!(stdout.contains("No submodules configured"));
    }

    #[test]
    fn test_list_command_shows_configured_submodules() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("list_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "list-lib",
                "--path",
                "lib/list",
            ])
            .expect("Failed to add submodule");

        let stdout = harness
            .run_submod_success(&["list"])
            .expect("Failed to run list");

        assert!(stdout.contains("list-lib"));
        assert!(stdout.contains("lib/list"));
        assert!(stdout.contains("active"));
    }

    #[test]
    fn test_list_recursive_queries_git() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("# empty\n")
            .expect("Failed to create config");

        // Even with empty config, --recursive should not fail and should list from git
        let output = harness
            .run_submod(&["list", "--recursive"])
            .expect("Failed to run list --recursive");

        // Should not crash (may succeed or fail gracefully)
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Either lists "No submodules configured" or something from git, but no panic
        let combined = format!("{stdout}{stderr}");
        assert!(
            combined.contains("No submodules configured")
                || combined.contains("Submodules")
                || combined.contains("Warning")
        );
    }

    #[test]
    fn test_disable_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("disable_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "disable-lib",
                "--path",
                "lib/disable",
            ])
            .expect("Failed to add submodule");

        let head = harness.git_stdout(&["-C", "lib/disable", "rev-parse", "HEAD"]);
        let refs = harness.git_stdout(&["-C", "lib/disable", "show-ref"]);
        let content = std::fs::read(harness.work_dir.join("lib/disable/docs/README.md")).unwrap();

        let stdout = harness
            .run_submod_success(&["disable", "disable-lib"])
            .expect("Failed to disable submodule");

        assert!(stdout.contains("Disabled submodule 'disable-lib'"));

        // Config should show active = false
        let config = harness.read_config().expect("Failed to read config");
        assert!(config.contains("active = false"));

        assert_eq!(
            harness.git_stdout(&["config", "--local", "--get", "submodule.disable-lib.active"]),
            "false"
        );
        assert!(!harness.gitmodules_entries().contains(".active"));
        assert_eq!(
            harness.git_stdout(&["-C", "lib/disable", "rev-parse", "HEAD"]),
            head
        );
        assert_eq!(
            std::fs::read(harness.work_dir.join("lib/disable/docs/README.md")).unwrap(),
            content
        );
        assert_eq!(harness.git_stdout(&["-C", "lib/disable", "show-ref"]), refs);
    }

    #[test]
    fn test_disable_command_matching_name() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Manually create a .gitmodules with a matching name
        let gitmodules_content = "\
[submodule \"my-lib\"]
\tpath = lib/my
\turl = https://example.com/my-lib.git
";
        std::fs::write(harness.work_dir.join(".gitmodules"), gitmodules_content)
            .expect("Failed to write .gitmodules");

        let config_content = "\
[my-lib]
path = \"lib/my\"
url = \"https://example.com/my-lib.git\"
active = true
";
        harness
            .create_config(config_content)
            .expect("Failed to create config");

        let stdout = harness
            .run_submod_success(&["disable", "my-lib"])
            .expect("Failed to disable submodule");

        assert!(stdout.contains("Disabled submodule 'my-lib'"));

        let gitmodules_updated = std::fs::read_to_string(harness.work_dir.join(".gitmodules"))
            .expect("Failed to read .gitmodules");
        assert_eq!(gitmodules_updated, gitmodules_content);
        assert!(harness.read_config().unwrap().contains("active = false"));
        assert_eq!(
            harness.git_stdout(&["config", "--local", "--get", "submodule.my-lib.active"]),
            "false"
        );
    }

    #[test]
    fn test_disable_command_preserves_comments() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Create a config with comments
        let config_content = "\
# My project submodules
[defaults]
# default settings
ignore = \"none\"

# This is my main library
[my-lib]
path = \"lib/my\"
url = \"https://example.com/my-lib.git\"
active = true
";
        harness
            .create_config(config_content)
            .expect("Failed to create config");

        harness
            .run_submod_success(&["disable", "my-lib"])
            .expect("Failed to disable submodule");

        let config = harness.read_config().expect("Failed to read config");

        // Comments must be preserved
        assert!(
            config.contains("# My project submodules"),
            "top-level comment lost"
        );
        assert!(
            config.contains("# This is my main library"),
            "submodule comment lost"
        );
        assert!(
            config.contains("# default settings"),
            "defaults comment lost"
        );
        // active must be updated
        assert!(config.contains("active = false"), "active not updated");
    }

    #[test]
    fn test_delete_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("delete_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "delete-lib",
                "--path",
                "lib/delete",
            ])
            .expect("Failed to add submodule");

        // Verify it was added
        let config_before = harness.read_config().expect("Failed to read config");
        assert!(config_before.contains("[delete-lib]"));

        let stdout = harness
            .run_submod_success(&["delete", "delete-lib"])
            .expect("Failed to delete submodule");

        assert!(stdout.contains("Deleted submodule 'delete-lib'"));

        // Verify it was removed from config
        let config_after = harness.read_config().expect("Failed to read config");
        assert!(!config_after.contains("[delete-lib]"));
    }

    #[test]
    fn test_delete_command_preserves_other_sections() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Config with two submodules and comments
        let config_content = "\
# Project submodules
[keep-me]
path = \"lib/keep\"
url = \"https://example.com/keep.git\"
active = true

# This one should be deleted
[delete-me]
path = \"lib/delete\"
url = \"https://example.com/delete.git\"
active = true
";
        harness
            .create_config(config_content)
            .expect("Failed to create config");

        harness
            .run_submod_success(&["delete", "delete-me"])
            .expect("Failed to delete submodule");

        let config = harness.read_config().expect("Failed to read config");

        // keep-me and its comment must still be present
        assert!(config.contains("[keep-me]"), "kept section was removed");
        assert!(config.contains("# Project submodules"), "top comment lost");
        // delete-me must be gone
        assert!(
            !config.contains("[delete-me]"),
            "deleted section still present"
        );
    }

    #[test]
    fn test_change_global_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("[defaults]\nignore = \"none\"\n")
            .expect("Failed to create config");

        let stdout = harness
            .run_submod_success(&["change-global", "--ignore", "dirty"])
            .expect("Failed to run change-global");

        let _ = stdout; // may be empty or have a message

        let config = harness.read_config().expect("Failed to read config");
        assert!(config.contains("ignore = \"dirty\""));
    }

    #[test]
    fn test_change_command_updates_field_preserves_comments() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("change_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Start with a config that has comments
        let config_content = format!(
            "# My project\n# Author: test\n[change-lib]\n# the path below\npath = \"lib/change\"\nurl = \"{remote_url}\"\nactive = true\n"
        );
        harness
            .create_config(&config_content)
            .expect("Failed to create config");

        // Change ignore setting
        harness
            .run_submod_success(&["change", "change-lib", "--ignore", "dirty"])
            .expect("Failed to change submodule");

        let config = harness.read_config().expect("Failed to read config");

        // Comments must be preserved
        assert!(config.contains("# My project"), "top comment lost");
        assert!(config.contains("# the path below"), "inline comment lost");
        // Updated field
        assert!(config.contains("ignore = \"dirty\""), "ignore not updated");
    }

    #[test]
    fn test_generate_config_template() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let output_path = harness.work_dir.join("generated.toml");

        let stdout = harness
            .run_submod_success(&[
                "--config",
                output_path.to_str().unwrap(),
                "generate-config",
                "--template",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .expect("Failed to generate template config");

        assert!(stdout.contains("Generated template config"));
        assert!(output_path.exists());
        let content = fs::read_to_string(&output_path).expect("Failed to read generated config");
        // Template should contain sample config content (at minimum a section or defaults)
        assert!(
            content.contains("[defaults]")
                || content.contains("vendor-utils")
                || content.contains("sparse_paths"),
            "Template config should contain sample content; got: {content}"
        );
    }

    #[test]
    fn test_generate_config_empty() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let output_path = harness.work_dir.join("empty_generated.toml");

        let stdout = harness
            .run_submod_success(&[
                "--config",
                output_path.to_str().unwrap(),
                "generate-config",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .expect("Failed to generate empty config");

        assert!(
            stdout.contains("Generated empty config"),
            "Expected 'Generated empty config' in stdout, got: {stdout}"
        );
        assert!(
            output_path.exists(),
            "Output file should exist at {}",
            output_path.display()
        );
    }

    #[test]
    fn test_generate_config_no_overwrite_without_force() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let output_path = harness.work_dir.join("existing.toml");
        fs::write(&output_path, "# existing\n").expect("Failed to create existing file");

        let output = harness
            .run_submod(&[
                "--config",
                output_path.to_str().unwrap(),
                "generate-config",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .expect("Failed to run generate-config");

        assert_eq!(output.status.code(), Some(2));
        assert_eq!(fs::read(&output_path).unwrap(), b"# existing\n");
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Both the refusal and the way out of it must be in the message.
        assert!(
            stderr.contains("already exists. Use --force to overwrite."),
            "error should refuse to clobber and name the override flag; got: {stderr}"
        );
    }

    #[test]
    fn test_nuke_command_with_kill() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("nuke_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "nuke-lib",
                "--path",
                "lib/nuke",
            ])
            .expect("Failed to add submodule");

        let gitdir = harness.git_stdout(&["-C", "lib/nuke", "rev-parse", "--absolute-git-dir"]);
        let head = harness.git_stdout(&["-C", "lib/nuke", "rev-parse", "HEAD"]);
        // Nuke with --kill (does not reinit)
        let stdout = harness
            .run_submod_success(&["nuke-it-from-orbit", "nuke-lib", "--kill"])
            .expect("Failed to nuke submodule");

        assert!(
            stdout.contains(
                "nuke-lib: changed: removed its checkout, Git registration, and TOML declaration"
            ),
            "nuke should report the submodule it is nuking; got: {stdout}"
        );

        assert!(stdout.contains("Nuke summary: 1 changed, 0 unchanged, 0 skipped, 0 failed."));
        assert!(!harness.dir_exists("lib/nuke"));
        assert_eq!(harness.index_gitlink_mode("lib/nuke"), None);
        assert!(!harness.gitmodules_entries().contains("nuke-lib"));
        assert!(!harness.submodule_config_entries().contains("nuke-lib"));
        assert_eq!(
            harness.git_stdout(&["--git-dir", &gitdir, "rev-parse", "HEAD"]),
            head
        );
        // Config should not contain the submodule anymore
        let config = harness.read_config().expect("Failed to read config");
        assert!(!config.contains("[nuke-lib]"));
    }

    #[test]
    fn test_add_submodule_shallow() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("shallow_lib")
            .expect("Failed to create remote");

        // We use a file URL since that works locally for Git.
        // Note: Git locally defaults to turning off full file-based shallow clone protocols,
        // so we need to enable it for testing.
        std::process::Command::new("git")
            .args(["config", "protocol.file.allow", "always"])
            .current_dir(&harness.work_dir)
            .output()
            .expect("Failed to configure git protocol");

        // Also enable `uploadpack.allowFilter` to let git clone shallowly from file URL
        std::process::Command::new("git")
            .args(["config", "uploadpack.allowFilter", "true"])
            .current_dir(&remote_repo)
            .output()
            .expect("Failed to configure git uploadpack");

        std::process::Command::new("git")
            .args(["config", "uploadpack.allowAnySHA1InWant", "true"])
            .current_dir(&remote_repo)
            .output()
            .expect("Failed to configure git uploadpack");

        let remote_url = format!("file://{}", remote_repo.display());

        // Add submodule with shallow flag (add branch argument to explicitly point to main)
        let stdout = harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "shallow-lib",
                "--path",
                "lib/shallow",
                "--shallow",
                "--branch",
                "main",
            ])
            .expect("Failed to add submodule");

        assert!(stdout.contains("Added submodule"));

        // Verify config includes shallow = true
        let config = harness.read_config().expect("Failed to read config");
        assert!(config.contains("shallow = true"));

        // Verify it is a shallow clone using git command
        let output = std::process::Command::new("git")
            .args(["rev-parse", "--is-shallow-repository"])
            .current_dir(harness.work_dir.join("lib/shallow"))
            .output()
            .expect("Failed to run git");

        let output_str = String::from_utf8_lossy(&output.stdout);
        let is_shallow = output_str.trim();
        assert_eq!(
            is_shallow, "true",
            "Repository at lib/shallow should be shallow"
        );
    }

    // ---------------------------------------------------------------------
    // Git-state assertions for add / delete / nuke (issue #62, P0-2).
    //
    // These tests assert on the *git state* submod manipulates — the index
    // gitlink, `.gitmodules`, the `submodule.*` config sections, and the
    // per-submodule `.git/modules/<path>` directory — rather than on printed
    // output or `submod.toml` text. The pre-delete assertions double as a guard
    // against the post-delete assertions passing vacuously.
    // ---------------------------------------------------------------------

    /// `add` must register real git state, not just write `submod.toml`:
    /// an index gitlink at mode 160000, a `.gitmodules` entry, a `submodule.*`
    /// config section, the `.git/modules/<path>` dir, and a worktree checked out
    /// at the gitlinked commit.
    #[test]
    fn test_add_registers_real_git_state() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("state_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "state-lib",
                "--path",
                "lib/state",
            ])
            .expect("Failed to add submodule");

        // Index gitlink: the path must be staged as a submodule (mode 160000).
        let stage = harness.git_stdout(&["ls-files", "--stage", "--", "lib/state"]);
        let fields: Vec<&str> = stage.split_whitespace().collect();
        assert_eq!(
            fields.first().copied(),
            Some("160000"),
            "expected an index gitlink at mode 160000 for lib/state, got: {stage:?}"
        );
        let gitlink_oid = fields.get(1).copied().expect("gitlink should carry an OID");

        // `.gitmodules` must carry the submodule's path and url.
        let gitmodules = harness.gitmodules_entries();
        assert!(
            gitmodules.contains("lib/state") && gitmodules.contains(&remote_url),
            "expected .gitmodules to record lib/state and its url, got:\n{gitmodules}"
        );

        // `.git/config` must carry a `submodule.*` section for the submodule.
        let config_entries = harness.submodule_config_entries();
        assert!(
            config_entries.contains("submodule.state-lib.url"),
            "expected a logical-name config section for state-lib, got:\n{config_entries}"
        );

        assert_eq!(
            harness.git_stdout(&[
                "config",
                "--file",
                ".gitmodules",
                "--get",
                "submodule.state-lib.path"
            ]),
            "lib/state"
        );
        let gitdir = harness.git_stdout(&["-C", "lib/state", "rev-parse", "--absolute-git-dir"]);
        assert!(std::path::Path::new(&gitdir).is_dir());

        // The worktree must be checked out at exactly the gitlinked commit.
        let head = harness.git_stdout(&["-C", "lib/state", "rev-parse", "HEAD"]);
        assert_eq!(
            head, gitlink_oid,
            "submodule worktree HEAD should match the index gitlink commit"
        );
    }

    /// Delete removes registration and checkout while retaining repository history.
    #[test]
    fn test_delete_cleans_up_git_state() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("del_state_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "del-state",
                "--path",
                "lib/delstate",
            ])
            .expect("Failed to add submodule");

        // Guard: the state we are about to assert is *gone* must first be present,
        // so the post-delete assertions cannot pass vacuously.
        assert_eq!(
            harness.index_gitlink_mode("lib/delstate").as_deref(),
            Some("160000"),
            "precondition: gitlink should exist before delete"
        );
        assert!(
            !harness.gitmodules_entries().is_empty(),
            "precondition: .gitmodules entry should exist before delete"
        );
        assert!(
            !harness.submodule_config_entries().is_empty(),
            "precondition: submodule.* config should exist before delete"
        );
        let gitdir = harness.git_stdout(&["-C", "lib/delstate", "rev-parse", "--absolute-git-dir"]);
        let head = harness.git_stdout(&["-C", "lib/delstate", "rev-parse", "HEAD"]);
        let refs = harness.git_stdout(&["-C", "lib/delstate", "show-ref"]);
        assert!(std::path::Path::new(&gitdir).is_dir());
        assert!(harness.dir_exists("lib/delstate"));

        harness
            .run_submod_success(&["delete", "del-state"])
            .expect("Failed to delete submodule");

        assert!(
            !harness.dir_exists("lib/delstate"),
            "worktree lib/delstate should be removed"
        );
        assert_eq!(
            harness.index_gitlink_mode("lib/delstate"),
            None,
            "index gitlink for lib/delstate should be cleared"
        );
        assert!(
            harness.gitmodules_entries().is_empty(),
            "no submodule entry should remain in .gitmodules, got:\n{}",
            harness.gitmodules_entries()
        );
        assert!(
            harness.submodule_config_entries().is_empty(),
            "no submodule.* config section should remain, got:\n{}",
            harness.submodule_config_entries()
        );
        assert!(
            std::path::Path::new(&gitdir).is_dir(),
            "retained history must survive removal"
        );
        assert_eq!(
            harness.git_stdout(&["--git-dir", &gitdir, "show-ref"]),
            refs
        );
        assert_eq!(
            harness.git_stdout(&["--git-dir", &gitdir, "cat-file", "-t", &head]),
            "commit"
        );
    }

    /// `nuke-it-from-orbit --kill` must clean up the same git state as `delete`.
    #[test]
    fn test_nuke_kill_cleans_up_git_state() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("nuke_state_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "nuke-state",
                "--path",
                "lib/nukestate",
            ])
            .expect("Failed to add submodule");

        // Guard against vacuous post-nuke assertions.
        assert_eq!(
            harness.index_gitlink_mode("lib/nukestate").as_deref(),
            Some("160000"),
            "precondition: gitlink should exist before nuke"
        );
        let gitdir =
            harness.git_stdout(&["-C", "lib/nukestate", "rev-parse", "--absolute-git-dir"]);
        let head = harness.git_stdout(&["-C", "lib/nukestate", "rev-parse", "HEAD"]);
        let refs = harness.git_stdout(&["-C", "lib/nukestate", "show-ref"]);
        assert!(std::path::Path::new(&gitdir).is_dir());

        harness
            .run_submod_success(&["nuke-it-from-orbit", "nuke-state", "--kill"])
            .expect("Failed to nuke submodule");

        assert!(
            !harness.dir_exists("lib/nukestate"),
            "worktree lib/nukestate should be removed after nuke --kill"
        );
        assert_eq!(
            harness.index_gitlink_mode("lib/nukestate"),
            None,
            "index gitlink for lib/nukestate should be cleared after nuke --kill"
        );
        assert!(
            harness.gitmodules_entries().is_empty(),
            "no submodule entry should remain in .gitmodules after nuke --kill, got:\n{}",
            harness.gitmodules_entries()
        );
        assert!(
            harness.submodule_config_entries().is_empty(),
            "no submodule.* config section should remain after nuke --kill, got:\n{}",
            harness.submodule_config_entries()
        );
        assert!(
            std::path::Path::new(&gitdir).is_dir(),
            "retained history must survive removal"
        );
        assert_eq!(
            harness.git_stdout(&["--git-dir", &gitdir, "show-ref"]),
            refs
        );
        assert_eq!(
            harness.git_stdout(&["--git-dir", &gitdir, "cat-file", "-t", &head]),
            "commit"
        );
    }

    /// After a `delete`, the on-disk git cleanup must be thorough enough that the
    /// same name+path can be added again — the real proof that no blocking
    /// `.git/modules/<path>` or stale config lingers.
    #[test]
    fn test_delete_then_readd_same_name_and_path_succeeds() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("readd_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        let add_args = [
            "add",
            remote_url.as_str(),
            "--name",
            "readd-lib",
            "--path",
            "lib/readd",
        ];

        harness
            .run_submod_success(&add_args)
            .expect("initial add should succeed");
        harness
            .run_submod_success(&["delete", "readd-lib"])
            .expect("delete should succeed");

        // Re-adding the same name+path must succeed and re-register the gitlink.
        harness
            .run_submod_success(&add_args)
            .expect("re-add of same name+path after delete should succeed");
        assert_eq!(
            harness.index_gitlink_mode("lib/readd").as_deref(),
            Some("160000"),
            "re-added submodule should be staged as a gitlink again"
        );
    }

    /// Committed gitlinks use Git's removal path and retain a reusable repository.
    #[test]
    fn test_delete_committed_gitlink_retains_history_and_readds() {
        let harness = TestHarness::new().unwrap();
        harness.init_git_repo().unwrap();
        let remote = harness.create_test_remote("committed-readd").unwrap();
        let url = format!("file://{}", remote.display());
        let args = ["add", &url, "--name", "retained", "--path", "lib/retained"];
        harness.run_submod_success(&args).unwrap();
        harness.git_stdout(&["commit", "-m", "Record submodule"]);
        let stage = harness.git_stdout(&["ls-files", "--stage", "--", "lib/retained"]);
        assert!(stage.starts_with("160000 "));
        let head = harness.git_stdout(&["-C", "lib/retained", "rev-parse", "HEAD"]);
        let gitdir = harness.git_stdout(&["-C", "lib/retained", "rev-parse", "--absolute-git-dir"]);
        let refs = harness.git_stdout(&["-C", "lib/retained", "show-ref"]);
        harness.run_submod_success(&["delete", "retained"]).unwrap();
        assert!(!harness.dir_exists("lib/retained"));
        assert_eq!(harness.index_gitlink_mode("lib/retained"), None);
        assert!(harness.gitmodules_entries().is_empty());
        assert!(
            harness.submodule_config_entries().is_empty(),
            "local registration remains: {}",
            harness.submodule_config_entries()
        );
        assert!(std::path::Path::new(&gitdir).is_dir());
        assert_eq!(
            harness.git_stdout(&[
                "--git-dir",
                &gitdir,
                "--work-tree",
                harness.work_dir.to_str().unwrap(),
                "show-ref"
            ]),
            refs
        );
        assert_eq!(
            harness.git_stdout(&[
                "--git-dir",
                &gitdir,
                "--work-tree",
                harness.work_dir.to_str().unwrap(),
                "cat-file",
                "-t",
                &head
            ]),
            "commit"
        );
        harness.run_submod_success(&args).unwrap();
        assert_eq!(
            harness.git_stdout(&["ls-files", "--stage", "--", "lib/retained"]),
            stage
        );
        assert_eq!(
            harness.git_stdout(&["-C", "lib/retained", "rev-parse", "HEAD"]),
            head
        );
        assert_eq!(
            harness.git_stdout(&["-C", "lib/retained", "rev-parse", "--absolute-git-dir"]),
            gitdir
        );
        assert_eq!(
            harness.git_stdout(&[
                "config",
                "--file",
                ".gitmodules",
                "--get",
                "submodule.retained.path"
            ]),
            "lib/retained"
        );
        assert_eq!(
            harness.git_stdout(&[
                "config",
                "--file",
                ".gitmodules",
                "--get",
                "submodule.retained.url"
            ]),
            url
        );
    }

    /// Repeated add refuses an existing declaration without changing its state.
    #[test]
    fn test_add_same_submodule_twice_refuses_without_changes() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("idem-remote")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&["add", &url, "--name", "idem", "--path", "lib/idem"])
            .expect("first add should succeed");
        let before = harness.preservation_snapshot();
        let refs = harness.git_stdout(&["-C", "lib/idem", "show-ref"]);
        let head = harness.git_stdout(&["-C", "lib/idem", "rev-parse", "HEAD"]);
        let content = std::fs::read(harness.work_dir.join("lib/idem/docs/README.md")).unwrap();
        let output = harness
            .run_submod(&["add", &url, "--name", "idem", "--path", "lib/idem"])
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stderr).contains("already declared"));
        assert_eq!(harness.preservation_snapshot(), before);
        assert_eq!(harness.git_stdout(&["-C", "lib/idem", "show-ref"]), refs);
        assert_eq!(
            harness.git_stdout(&["-C", "lib/idem", "rev-parse", "HEAD"]),
            head
        );
        assert_eq!(
            std::fs::read(harness.work_dir.join("lib/idem/docs/README.md")).unwrap(),
            content
        );

        // Exactly one entry must exist in .gitmodules, git config, and submod.toml.
        let gm_raw = std::fs::read_to_string(harness.work_dir.join(".gitmodules"))
            .expect("read .gitmodules");
        assert_eq!(
            gm_raw.matches("[submodule \"idem\"]").count(),
            1,
            ".gitmodules must hold exactly one section for the re-added submodule, got:\n{gm_raw}"
        );
        let cfg = harness.submodule_config_entries();
        assert_eq!(
            cfg.matches("submodule.idem.url").count(),
            1,
            "git config must hold exactly one entry for the submodule, got:\n{cfg}"
        );
        let toml = std::fs::read_to_string(harness.config_path()).expect("read submod.toml");
        assert_eq!(
            toml.matches("[idem]").count(),
            1,
            "submod.toml must hold exactly one [idem] section, got:\n{toml}"
        );
    }

    /// Deleting a submodule that does not exist must fail with a specific,
    /// informative error — not silently succeed.
    #[test]
    fn test_delete_nonexistent_submodule_fails() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        harness.create_config("# empty\n").unwrap();
        let before = harness.preservation_snapshot();
        let output = harness
            .run_submod(&["delete", "ghost"])
            .expect("Failed to run submod");
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(harness.preservation_snapshot(), before);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("ghost") && stderr.contains("not found"),
            "error must name the missing submodule and say it was not found, got: {stderr}"
        );
    }

    /// A failed `add` (e.g. an unreachable URL) must not leave partial state
    /// behind when a submodule already exists. Regression for the #62 audit (P2):
    /// the fallback cleanup matched the `.gitmodules` section by name, but git2
    /// writes the section keyed by path, so a stale `[submodule "<path>"]` entry
    /// lingered after the failed add.
    #[test]
    fn test_failed_add_leaves_no_partial_state() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("survivor-remote")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        // A real submodule so .gitmodules already exists when the bad add runs.
        harness
            .run_submod_success(&["add", &url, "--name", "good", "--path", "lib/good"])
            .expect("baseline add should succeed");

        // A failed add against an unreachable URL.
        let bad = harness
            .run_submod(&[
                "add",
                "file:///nonexistent/definitely-not-here.git",
                "--name",
                "bad",
                "--path",
                "lib/bad",
            ])
            .expect("Failed to run submod");
        assert!(
            !bad.status.success(),
            "an add against an unreachable URL must fail"
        );

        // No stale `bad`/`lib/bad` entry may remain in .gitmodules or git config.
        let gm = harness.gitmodules_entries();
        assert!(
            !gm.contains("lib/bad") && !gm.contains("\"bad\""),
            "failed add left a stale .gitmodules entry:\n{gm}"
        );
        let cfg = harness.submodule_config_entries();
        assert!(
            !cfg.contains("lib/bad") && !cfg.contains("submodule.bad."),
            "failed add left a stale git config entry:\n{cfg}"
        );
        assert!(
            !harness.dir_exists("lib/bad"),
            "failed add left an orphan working-tree directory"
        );
        assert!(
            !harness.git_modules_dir_exists("bad") && !harness.git_modules_dir_exists("lib/bad"),
            "failed add left a dangling .git/modules directory"
        );

        // Non-vacuity: the pre-existing good submodule must survive the cleanup.
        assert!(
            gm.contains("lib/good"),
            "cleanup of the failed add must not remove the existing submodule:\n{gm}"
        );
    }

    /// R11: a declaration without `path` uses its validated TOML nickname and
    /// initialization must leave a real gitlink, not just a cloned directory.
    #[test]
    fn r11_toml_only_init_defaults_path_and_registers_gitlink() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("parent repo");
        let remote = harness.create_test_remote("r11-remote").expect("remote");
        let url = remote.as_ref().to_string_lossy();
        harness
            .create_config(&format!("[library]\nurl = {url:?}\n"))
            .expect("config");

        let output = harness.run_submod(&["init"]).expect("run init");
        assert!(
            output.status.success(),
            "init must accept an omitted path; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            harness.index_gitlink_mode("library").as_deref(),
            Some("160000")
        );
        assert!(harness.work_dir.join("library/.git").is_file());
        assert!(harness.work_dir.join("library/src/main.c").is_file());
    }

    /// R12: onboarding a non-recursive clone must materialize the commit pinned
    /// by the parent even when the remote has advanced.
    #[test]
    fn r12_fresh_clone_init_materializes_parent_pin() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("parent repo");
        let remote = harness.create_test_remote("r12-remote").expect("remote");

        let add = harness
            .git_cmd()
            .args(["submodule", "add", "--name", "logical"])
            .arg(remote.as_ref())
            .arg("deps/library")
            .current_dir(&harness.work_dir)
            .output()
            .expect("native add");
        assert!(
            add.status.success(),
            "native add: {}",
            String::from_utf8_lossy(&add.stderr)
        );
        let pinned = harness.git_at(
            &harness.work_dir.join("deps/library"),
            &["rev-parse", "HEAD"],
        );
        harness
            .create_config(&format!(
                "[library]\npath = \"deps/library\"\nurl = {:?}\n",
                remote.as_ref().to_string_lossy()
            ))
            .expect("config");
        let commit = harness
            .git_cmd()
            .args(["add", ".gitmodules", "submod.toml", "deps/library"])
            .current_dir(&harness.work_dir)
            .output()
            .expect("stage parent");
        assert!(commit.status.success());
        let commit = harness
            .git_cmd()
            .args(["commit", "-m", "pin submodule"])
            .current_dir(&harness.work_dir)
            .output()
            .expect("commit parent");
        assert!(
            commit.status.success(),
            "parent commit: {}",
            String::from_utf8_lossy(&commit.stderr)
        );
        let advanced = harness
            .advance_test_remote("r12-remote")
            .expect("advance remote");
        assert_ne!(advanced, pinned);

        let fresh = harness.temp_dir.path().join("fresh-parent");
        let clone = harness
            .git_cmd()
            .arg("clone")
            .arg(&harness.work_dir)
            .arg(&fresh)
            .output()
            .expect("clone parent");
        assert!(
            clone.status.success(),
            "clone parent: {}",
            String::from_utf8_lossy(&clone.stderr)
        );

        let output = harness
            .run_submod_at(&fresh, &["init"])
            .expect("fresh init");
        assert!(
            output.status.success(),
            "fresh init: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            fresh.join("deps/library/.git").is_file(),
            "successful init must create a usable checkout"
        );
        assert_eq!(
            harness.git_at(&fresh.join("deps/library"), &["rev-parse", "HEAD"]),
            pinned
        );
        assert!(fresh.join("deps/library/src/main.c").is_file());
    }

    /// R13: the requested branch controls the initial checkout and is recorded
    /// for ordinary Git consumers.
    #[test]
    fn r13_add_branch_selects_and_records_named_branch() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("parent repo");
        let remote = harness.create_complex_remote("r13-remote").expect("remote");
        let url = format!("file://{}", remote.display());

        let output = harness
            .run_submod(&[
                "add",
                &url,
                "--name",
                "library",
                "--path",
                "deps/library",
                "--branch",
                "develop",
            ])
            .expect("branch add");
        assert!(
            output.status.success(),
            "branch add: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(harness.work_dir.join("deps/library/src/dev.rs").is_file());
        let remote_tip = harness.git_at(remote.as_ref(), &["rev-parse", "refs/heads/develop"]);
        let checkout = harness.git_at(
            &harness.work_dir.join("deps/library"),
            &["rev-parse", "HEAD"],
        );
        assert_eq!(checkout, remote_tip);
        assert!(
            harness.gitmodules_entries().lines().any(|line| {
                line.ends_with(".branch=develop") || line.ends_with(".branch develop")
            }),
            "branch missing from .gitmodules: {}",
            harness.gitmodules_entries()
        );
    }

    /// R13: `update=none` governs later automatic work; an explicit add still
    /// requests the initial checkout.
    #[test]
    fn r13_explicit_add_with_update_none_creates_initial_checkout() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("parent repo");
        let remote = harness.create_test_remote("r13-none").expect("remote");
        let url = format!("file://{}", remote.display());

        let output = harness
            .run_submod(&[
                "add",
                &url,
                "--name",
                "library",
                "--path",
                "deps/library",
                "--update",
                "none",
            ])
            .expect("add update none");
        assert!(
            output.status.success(),
            "add: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            harness.index_gitlink_mode("deps/library").as_deref(),
            Some("160000")
        );
        assert!(harness.work_dir.join("deps/library/src/main.c").is_file());
    }

    /// R14: an explicit managed URL edit followed by sync owns the portable,
    /// parent-local, and initialized-child URL while preserving unrelated config.
    #[test]
    fn r14_sync_reconciles_managed_url_everywhere() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("parent repo");
        let first = harness
            .create_test_remote("r14-first")
            .expect("first remote");
        let second = harness
            .create_test_remote("r14-second")
            .expect("second remote");
        let first_url = first.as_ref().to_string_lossy().into_owned();
        let second_url = second.as_ref().to_string_lossy().into_owned();
        harness
            .run_submod_success(&[
                "add",
                &first_url,
                "--name",
                "library",
                "--path",
                "deps/library",
            ])
            .expect("initial add");
        let set = harness
            .git_cmd()
            .args(["config", "submod.unrelated", "keep-me"])
            .current_dir(&harness.work_dir)
            .output()
            .expect("set unrelated config");
        assert!(set.status.success());

        harness
            .run_submod_success(&["change", "library", "--url", &second_url])
            .expect("change URL");
        let sync = harness.run_submod(&["sync"]).expect("sync");
        assert!(
            sync.status.success(),
            "sync: {}",
            String::from_utf8_lossy(&sync.stderr)
        );

        assert!(harness.gitmodules_entries().contains(&second_url));
        assert!(harness.submodule_config_entries().contains(&second_url));
        assert_eq!(
            harness.git_at(
                &harness.work_dir.join("deps/library"),
                &["remote", "get-url", "origin"]
            ),
            second_url
        );
        assert_eq!(
            harness.git_stdout(&["config", "--get", "submod.unrelated"]),
            "keep-me"
        );
    }
}

#[test]
fn phase2_acceptance_r03_disable_enable_init_preserves_local_history() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("activation-history").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "logical",
        remote.to_str().unwrap(),
        "vendor/child",
    ]);
    h.create_config(&format!(
        "[nickname]\npath = \"vendor/child\"\nurl = {:?}\nactive = true\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    h.git_stdout(&["config", "submodule.logical.active", "true"]);
    h.git_stdout(&["add", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "record child pin"]);
    let pin = h.git_stdout(&["rev-parse", "HEAD:vendor/child"]);
    let child = h.work_dir.join("vendor/child");
    fs::write(child.join("local-only"), b"local-only commit\0\xff").unwrap();
    h.git_at(&child, &["add", "local-only"]);
    h.git_at(&child, &["commit", "-m", "local-only child history"]);
    let local_commit = h.git_at(&child, &["rev-parse", "HEAD"]);
    h.git_at(&child, &["branch", "saved-local", &local_commit]);
    assert_ne!(local_commit, pin);
    fs::write(child.join("LICENSE"), b"existing stash bytes\0\xff").unwrap();
    h.git_at(&child, &["stash", "push", "-m", "existing recovery point"]);
    let stash = h.git_at(&child, &["rev-parse", "refs/stash"]);
    assert!(
        h.git_at(&child, &["--no-optional-locks", "status", "--porcelain"])
            .is_empty()
    );
    let gitdir = std::path::PathBuf::from(h.git_at(&child, &["rev-parse", "--absolute-git-dir"]));
    let files: Vec<_> = h
        .git_at(&child, &["ls-files", "-z"])
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| (path.to_owned(), fs::read(child.join(path)).unwrap()))
        .collect();
    let pointer = fs::read(child.join(".git")).unwrap();
    let index = fs::read(gitdir.join("index")).unwrap();
    let index_entries = h.git_at(&child, &["ls-files", "--stage"]);
    let config = fs::read(gitdir.join("config")).unwrap();
    let refs = h.git_at(&child, &["show-ref"]);
    let parent_index = h.git_stdout(&["ls-files", "--stage"]);
    let parent_refs = h.git_stdout(&["show-ref"]);
    let gitmodules = fs::read(h.work_dir.join(".gitmodules")).unwrap();

    for (args, active) in [
        (&["disable", "nickname"][..], "false"),
        (&["change", "nickname", "--active", "true"][..], "true"),
    ] {
        let output = h.run_submod(args).unwrap();
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert_eq!(
            h.git_stdout(&["config", "--local", "--get", "submodule.logical.active"]),
            active,
            "{args:?}"
        );
        assert!(
            h.read_config()
                .unwrap()
                .contains(&format!("active = {active}")),
            "{args:?}"
        );
        for (path, bytes) in &files {
            assert_eq!(
                fs::read(child.join(path)).unwrap(),
                *bytes,
                "{args:?}: {path}"
            );
        }
        assert_eq!(fs::read(child.join(".git")).unwrap(), pointer, "{args:?}");
        assert_eq!(fs::read(gitdir.join("index")).unwrap(), index, "{args:?}");
        assert_eq!(fs::read(gitdir.join("config")).unwrap(), config, "{args:?}");
        assert_eq!(
            h.git_at(&child, &["rev-parse", "HEAD"]),
            local_commit,
            "{args:?}"
        );
        assert_eq!(h.git_at(&child, &["show-ref"]), refs, "{args:?}");
        assert_eq!(
            h.git_at(&child, &["cat-file", "-t", &local_commit]),
            "commit"
        );
        assert_eq!(
            h.git_at(&child, &["rev-parse", "refs/stash"]),
            stash,
            "{args:?}"
        );
        assert_eq!(h.git_at(&child, &["cat-file", "-t", &stash]), "commit");
        assert_eq!(
            h.git_stdout(&["ls-files", "--stage"]),
            parent_index,
            "{args:?}"
        );
        assert_eq!(h.git_stdout(&["show-ref"]), parent_refs, "{args:?}");
        assert_eq!(h.git_stdout(&["rev-parse", "HEAD:vendor/child"]), pin);
        assert_eq!(
            fs::read(h.work_dir.join(".gitmodules")).unwrap(),
            gitmodules,
            "{args:?}"
        );
    }

    // Re-enabling preserves the checkout. Explicit init then applies the
    // selected checkout strategy to the parent pin while retaining every
    // recoverable ref, object, and pre-existing stash.
    let init = h.run_submod(&["init"]).unwrap();
    assert!(init.status.success(), "init: {init:?}");
    assert_eq!(
        h.git_stdout(&["config", "--local", "--get", "submodule.logical.active"]),
        "true"
    );
    assert_eq!(fs::read(child.join(".git")).unwrap(), pointer);
    assert_eq!(fs::read(gitdir.join("config")).unwrap(), config);
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), pin);
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(
        h.git_at(&child, &["rev-parse", "refs/heads/saved-local"]),
        local_commit
    );
    assert_eq!(
        h.git_at(&child, &["cat-file", "-t", &local_commit]),
        "commit"
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "refs/stash"]), stash);
    assert_eq!(h.git_at(&child, &["cat-file", "-t", &stash]), "commit");
    assert_eq!(h.git_stdout(&["ls-files", "--stage"]), parent_index);
    assert_eq!(h.git_stdout(&["show-ref"]), parent_refs);
    assert_eq!(h.git_stdout(&["rev-parse", "HEAD:vendor/child"]), pin);
    assert_eq!(
        fs::read(h.work_dir.join(".gitmodules")).unwrap(),
        gitmodules
    );
    assert!(!child.join("local-only").exists());
    h.git_at(&child, &["checkout", "saved-local"]);
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), local_commit);
    assert_eq!(h.git_at(&child, &["ls-files", "--stage"]), index_entries);
    for (path, bytes) in &files {
        assert_eq!(
            fs::read(child.join(path)).unwrap(),
            *bytes,
            "recovery: {path}"
        );
    }
    h.git_at(&child, &["stash", "apply", &stash]);
    assert_eq!(
        fs::read(child.join("LICENSE")).unwrap(),
        b"existing stash bytes\0\xff"
    );
}
