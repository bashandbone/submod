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

        // Should succeed but show no submodules
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Checking submodule configurations"));
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
        assert!(stdout.contains("Checking submodule configurations"));
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
        assert!(config.contains("active = true"));

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

        assert!(stdout.contains("Initializing init-lib"));
        assert!(stdout.contains("initialized"));

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

        assert!(stdout.contains("Updated") || stdout.contains("Already up to date"));
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

    /// Characterizes `update` against a remote that has moved forward: the fetch
    /// genuinely happens (the new commit's object is pulled into the submodule),
    /// but HEAD follows the superproject-recorded gitlink — it does NOT jump to
    /// the remote tip. That is plain `git submodule update` semantics (only
    /// `--remote` would follow the branch tip). Replaces the prior no-op smoke
    /// test that updated against a remote which never advanced (#62 P1).
    #[test]
    fn update_against_advanced_remote_fetches_without_moving_head() {
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
        assert!(
            harness
                .git_stdout(&["-C", "lib/adv", "cat-file", "-t", &advanced])
                .is_empty(),
            "precondition: the advanced commit must be unknown before update"
        );

        harness
            .run_submod_success(&["update"])
            .expect("Failed to run update");

        // The fetch really ran: the advanced commit's object is now present in
        // the submodule's object store. (This is the non-vacuous part — it is
        // false unless update actually fetched, since the object was absent in
        // the precondition above.)
        assert_eq!(
            harness.git_stdout(&["-C", "lib/adv", "cat-file", "-t", &advanced]),
            "commit",
            "update must fetch the advanced remote commit into the submodule"
        );

        // But HEAD stayed at the recorded gitlink — update tracks the recorded
        // commit, not the remote tip.
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
            clean_out.contains("Working tree is clean"),
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

        let dirty_out = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check on dirty submodule");
        assert!(
            dirty_out.contains("Working tree has changes"),
            "check must report the modified submodule worktree as dirty, got:\n{dirty_out}"
        );
        assert!(
            !dirty_out.contains("Working tree is clean"),
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

        assert!(stdout.contains("Hard resetting"));
        assert!(stdout.contains("reset complete"));

        // Verify test file was removed
        assert!(!harness.file_exists("lib/reset/test_file.txt"));
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

        // Make changes in both submodules
        fs::write(harness.work_dir.join("lib/reset1/test1.txt"), "change1")
            .expect("Failed to create test file");
        fs::write(harness.work_dir.join("lib/reset2/test2.txt"), "change2")
            .expect("Failed to create test file");

        // Run reset all command
        let stdout = harness
            .run_submod_success(&["reset", "--all"])
            .expect("Failed to run reset all");

        assert!(stdout.contains("Hard resetting"));

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

        assert!(stdout.contains("Running full sync"));
        assert!(stdout.contains("Checking submodule configurations"));
        assert!(stdout.contains("Initializing"));
        assert!(stdout.contains("Sync complete"));

        // Verify submodule was initialized
        assert!(harness.dir_exists("lib/sync"));
        assert!(harness.file_exists("lib/sync/.git"));
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

        // Run check to see if defaults are applied
        let stdout = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");

        assert!(stdout.contains("Checking submodule configurations"));
        // The output should show effective settings including defaults
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
        let stdout = harness
            .run_submod_success(&["--config", "custom.toml", "check", "--verbose"])
            .expect("Failed to run with custom config");

        assert!(stdout.contains("Checking submodule configurations"));
    }

    #[test]
    fn test_error_handling_invalid_git_repo() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        // Don't initialize git repo

        // Should fail when not in a git repository
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run submod");
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Failed to create manager") || stderr.contains("Repository not found")
        );
    }

    #[test]
    fn test_error_handling_invalid_url() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

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

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Failed to add submodule") || stderr.contains("clone failed"));
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
        let stdout = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");

        assert!(stdout.contains("Sparse checkout mismatch"));
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

        let stdout = harness
            .run_submod_success(&["disable", "disable-lib"])
            .expect("Failed to disable submodule");

        assert!(stdout.contains("Disabled submodule 'disable-lib'"));

        // Config should show active = false
        let config = harness.read_config().expect("Failed to read config");
        assert!(config.contains("active = false"));

        // .gitmodules should show active = false
        let gitmodules_path = harness.work_dir.join(".gitmodules");
        let gitmodules_content =
            std::fs::read_to_string(&gitmodules_path).expect("Failed to read .gitmodules");
        println!("GITMODULES CONTENT:\n{gitmodules_content}");
        assert!(gitmodules_content.contains("active = false"));
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
        assert!(gitmodules_updated.contains("active = false"));
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

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("already exists") || stderr.contains("Use --force"));
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

        // Nuke with --kill (does not reinit)
        let stdout = harness
            .run_submod_success(&["nuke-it-from-orbit", "nuke-lib", "--kill"])
            .expect("Failed to nuke submodule");

        assert!(stdout.contains("Nuking") || stdout.contains("💥"));

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
            config_entries.contains("lib/state"),
            "expected a submodule.* config section for lib/state, got:\n{config_entries}"
        );

        // The per-submodule git directory must exist.
        assert!(
            harness.git_modules_dir_exists("lib/state"),
            ".git/modules/lib/state should exist after add"
        );

        // The worktree must be checked out at exactly the gitlinked commit.
        let head = harness.git_stdout(&["-C", "lib/state", "rev-parse", "HEAD"]);
        assert_eq!(
            head, gitlink_oid,
            "submodule worktree HEAD should match the index gitlink commit"
        );
    }

    /// `delete` must remove all git state, not just the `submod.toml` section:
    /// the worktree, the index gitlink, the `.gitmodules` entry, the `submodule.*`
    /// config section, and the `.git/modules/<path>` directory.
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
        assert!(
            harness.git_modules_dir_exists("lib/delstate"),
            "precondition: .git/modules/lib/delstate should exist before delete"
        );
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
            !harness.git_modules_dir_exists("lib/delstate"),
            ".git/modules/lib/delstate should be removed"
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
        assert!(harness.git_modules_dir_exists("lib/nukestate"));

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
            !harness.git_modules_dir_exists("lib/nukestate"),
            ".git/modules/lib/nukestate should be removed after nuke --kill"
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

    /// Adding the same submodule (same name + path) a second time is idempotent:
    /// it succeeds without creating duplicate `.gitmodules` or config entries.
    #[test]
    fn test_add_same_submodule_twice_is_idempotent() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("idem-remote")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&["add", &url, "--name", "idem", "--path", "lib/idem"])
            .expect("first add should succeed");
        harness
            .run_submod_success(&["add", &url, "--name", "idem", "--path", "lib/idem"])
            .expect("re-adding the same submodule should be a graceful no-op");

        // Exactly one entry must exist in .gitmodules, git config, and submod.toml.
        let gm_raw = std::fs::read_to_string(harness.work_dir.join(".gitmodules"))
            .expect("read .gitmodules");
        assert_eq!(
            gm_raw.matches("[submodule \"lib/idem\"]").count(),
            1,
            ".gitmodules must hold exactly one section for the re-added submodule, got:\n{gm_raw}"
        );
        let cfg = harness.submodule_config_entries();
        assert_eq!(
            cfg.matches("submodule.lib/idem.url").count(),
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

        let output = harness
            .run_submod(&["delete", "ghost"])
            .expect("Failed to run submod");
        assert!(
            !output.status.success(),
            "deleting a nonexistent submodule must exit non-zero"
        );
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
}
