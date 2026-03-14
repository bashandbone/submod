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
            .run_submod(&["check"])
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
            .run_submod(&["check"])
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
            .run_submod_success(&["add", &remote_url, "--name", "test-lib", "--path", "lib/test"])
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

        // Run init command
        let stdout = harness
            .run_submod_success(&["init"])
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
            .run_submod_success(&["add", &remote_url, "--name", "update-lib", "--path", "lib/update"])
            .expect("Failed to add submodule");

        // Run update command
        let stdout = harness
            .run_submod_success(&["update"])
            .expect("Failed to run update");

        assert!(stdout.contains("Updated") || stdout.contains("Already up to date"));
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
            .run_submod_success(&["add", &remote_url, "--name", "reset-lib", "--path", "lib/reset"])
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
            .run_submod_success(&["add", &remote_url1, "--name", "reset-lib1", "--path", "lib/reset1"])
            .expect("Failed to add submodule 1");

        harness
            .run_submod_success(&["add", &remote_url2, "--name", "reset-lib2", "--path", "lib/reset2"])
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

        // Run sync command (should check, init, and update)
        let stdout = harness
            .run_submod_success(&["sync"])
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
            .run_submod_success(&["check"])
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

        // Run with custom config file
        let stdout = harness
            .run_submod_success(&["--config", "custom.toml", "check"])
            .expect("Failed to run with custom config");

        assert!(stdout.contains("Checking submodule configurations"));
    }

    #[test]
    fn test_error_handling_invalid_git_repo() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        // Don't initialize git repo

        // Should fail when not in a git repository
        let output = harness
            .run_submod(&["check"])
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
            .run_submod(&["add", "not-a-valid-url", "--name", "invalid-lib", "--path", "lib/invalid"])
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
            .run_submod_success(&["check"])
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
            .run_submod_success(&["add", &remote_url, "--name", "list-lib", "--path", "lib/list"])
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
            .run_submod_success(&["add", &remote_url, "--name", "disable-lib", "--path", "lib/disable"])
            .expect("Failed to add submodule");

        let stdout = harness
            .run_submod_success(&["disable", "disable-lib"])
            .expect("Failed to disable submodule");

        assert!(stdout.contains("Disabled submodule 'disable-lib'"));

        // Config should show active = false
        let config = harness.read_config().expect("Failed to read config");
        assert!(config.contains("active = false"));
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
        assert!(config.contains("# My project submodules"), "top-level comment lost");
        assert!(config.contains("# This is my main library"), "submodule comment lost");
        assert!(config.contains("# default settings"), "defaults comment lost");
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
            .run_submod_success(&["add", &remote_url, "--name", "delete-lib", "--path", "lib/delete"])
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
        assert!(!config.contains("[delete-me]"), "deleted section still present");
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
            content.contains("[defaults]") || content.contains("vendor-utils") || content.contains("sparse_paths"),
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

        assert!(stdout.contains("Generated empty config"), "Expected 'Generated empty config' in stdout, got: {stdout}");
        assert!(output_path.exists(), "Output file should exist at {}", output_path.display());
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
            .run_submod_success(&["add", &remote_url, "--name", "nuke-lib", "--path", "lib/nuke"])
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
}
