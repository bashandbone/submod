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
        let output = harness.run_submod(&["check"]).expect("Failed to run submod");

        // Should succeed but show no submodules
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Checking submodule configurations"));
    }

    #[test]
    fn test_check_command_with_empty_config() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness.create_config("# Empty config\n").expect("Failed to create config");

        let output = harness.run_submod(&["check"]).expect("Failed to run submod");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("Checking submodule configurations"));
    }

    #[test]
    fn test_add_submodule_basic() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness.create_test_remote("test_lib").expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add a submodule
        let stdout = harness.run_submod_success(&[
            "add",
            "test-lib",
            "lib/test",
            &remote_url,
        ]).expect("Failed to add submodule");

        assert!(stdout.contains("Added submodule"));

        // Verify config file was created/updated
        let config = harness.read_config().expect("Failed to read config");
        assert!(config.contains("[test-lib]"));
        assert!(config.contains("path = \"lib/test\""));
        assert!(config.contains(&format!("url = \"{}\"", remote_url)));
        assert!(config.contains("active = true"));

        // Verify directory structure was created
        assert!(harness.dir_exists("lib/test"));
        assert!(harness.file_exists("lib/test/.git"));
    }

    #[test]
    fn test_add_submodule_with_sparse_paths() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness.create_test_remote("sparse_lib").expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add submodule with sparse paths
        let stdout = harness.run_submod_success(&[
            "add",
            "sparse-lib",
            "lib/sparse",
            &remote_url,
            "--sparse-paths", "src,docs",
        ]).expect("Failed to add submodule");

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

        let remote_repo = harness.create_test_remote("init_lib").expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Create config manually
        let config_content = format!(
            r#"[init-lib]
path = "lib/init"
url = "{}"
active = true
sparse_paths = ["src"]
"#,
            remote_url
        );
        harness.create_config(&config_content).expect("Failed to create config");

        // Run init command
        let stdout = harness.run_submod_success(&["init"]).expect("Failed to run init");

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

        let remote_repo = harness.create_test_remote("update_lib").expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add and initialize submodule first
        harness.run_submod_success(&[
            "add",
            "update-lib",
            "lib/update",
            &remote_url,
        ]).expect("Failed to add submodule");

        // Run update command
        let stdout = harness.run_submod_success(&["update"]).expect("Failed to run update");

        assert!(stdout.contains("Updated") || stdout.contains("Already up to date"));
    }

    #[test]
    fn test_reset_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness.create_test_remote("reset_lib").expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add and initialize submodule
        harness.run_submod_success(&[
            "add",
            "reset-lib",
            "lib/reset",
            &remote_url,
        ]).expect("Failed to add submodule");

        // Make some changes in the submodule
        fs::write(
            harness.work_dir.join("lib/reset/test_file.txt"),
            "This is a test change"
        ).expect("Failed to create test file");

        // Run reset command
        let stdout = harness.run_submod_success(&[
            "reset",
            "reset-lib"
        ]).expect("Failed to run reset");

        assert!(stdout.contains("Hard resetting"));
        assert!(stdout.contains("reset complete"));

        // Verify test file was removed
        assert!(!harness.file_exists("lib/reset/test_file.txt"));
    }

    #[test]
    fn test_reset_all_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo1 = harness.create_test_remote("reset_lib1").expect("Failed to create remote");
        let remote_repo2 = harness.create_test_remote("reset_lib2").expect("Failed to create remote");
        let remote_url1 = format!("file://{}", remote_repo1.display());
        let remote_url2 = format!("file://{}", remote_repo2.display());

        // Add two submodules
        harness.run_submod_success(&[
            "add", "reset-lib1", "lib/reset1", &remote_url1,
        ]).expect("Failed to add submodule 1");

        harness.run_submod_success(&[
            "add", "reset-lib2", "lib/reset2", &remote_url2,
        ]).expect("Failed to add submodule 2");

        // Make changes in both submodules
        fs::write(harness.work_dir.join("lib/reset1/test1.txt"), "change1").expect("Failed to create test file");
        fs::write(harness.work_dir.join("lib/reset2/test2.txt"), "change2").expect("Failed to create test file");

        // Run reset all command
        let stdout = harness.run_submod_success(&[
            "reset", "--all"
        ]).expect("Failed to run reset all");

        assert!(stdout.contains("Hard resetting"));

        // Verify both test files were removed
        assert!(!harness.file_exists("lib/reset1/test1.txt"));
        assert!(!harness.file_exists("lib/reset2/test2.txt"));
    }

    #[test]
    fn test_sync_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness.create_test_remote("sync_lib").expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Create config manually without initializing
        let config_content = format!(
            r#"[sync-lib]
path = "lib/sync"
url = "{}"
active = true
"#,
            remote_url
        );
        harness.create_config(&config_content).expect("Failed to create config");

        // Run sync command (should check, init, and update)
        let stdout = harness.run_submod_success(&["sync"]).expect("Failed to run sync");

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

        let remote_repo = harness.create_test_remote("defaults_lib").expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Create config with defaults
        let config_content = format!(
            r#"[defaults]
ignore = "dirty"

[defaults-lib]
path = "lib/defaults"
url = "{}"
active = true
"#,
            remote_url
        );
        harness.create_config(&config_content).expect("Failed to create config");

        // Run check to see if defaults are applied
        let stdout = harness.run_submod_success(&["check"]).expect("Failed to run check");

        assert!(stdout.contains("Checking submodule configurations"));
        // The output should show effective settings including defaults
    }

    #[test]
    fn test_custom_config_file() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let custom_config = harness.work_dir.join("custom.toml");
        fs::write(&custom_config, "[test-sub]\npath = \"test\"\nurl = \"https://example.com/test.git\"\nactive = true\n")
            .expect("Failed to create custom config");

        // Run with custom config file
        let stdout = harness.run_submod_success(&[
            "--config", "custom.toml",
            "check"
        ]).expect("Failed to run with custom config");

        assert!(stdout.contains("Checking submodule configurations"));
    }

    #[test]
    fn test_error_handling_invalid_git_repo() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        // Don't initialize git repo

        // Should fail when not in a git repository
        let output = harness.run_submod(&["check"]).expect("Failed to run submod");
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Failed to create manager") || stderr.contains("Repository not found"));
    }

    #[test]
    fn test_error_handling_invalid_url() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Try to add submodule with invalid URL
        let output = harness.run_submod(&[
            "add",
            "invalid-lib",
            "lib/invalid",
            "not-a-valid-url",
        ]).expect("Failed to run submod");

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Failed to add submodule") || stderr.contains("clone failed"));
    }

    #[test]
    fn test_sparse_checkout_mismatch_detection() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness.create_test_remote("mismatch_lib").expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add submodule with specific sparse paths
        harness.run_submod_success(&[
            "add",
            "mismatch-lib",
            "lib/mismatch",
            &remote_url,
            "--sparse-paths", "src,docs",
        ]).expect("Failed to add submodule");

        // Manually modify sparse-checkout file to create mismatch
        let sparse_file = harness.get_sparse_checkout_file_path("lib/mismatch");
        fs::write(&sparse_file, "include\nLICENSE\n").expect("Failed to modify sparse file");

        // Run check command
        let stdout = harness.run_submod_success(&["check"]).expect("Failed to run check");

        assert!(stdout.contains("Sparse checkout mismatch"));
    }
}
