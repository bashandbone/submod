// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Integration tests focused on error handling and edge cases
//!
//! These tests verify that the tool handles various error conditions gracefully
//! and provides meaningful error messages to users.

use std::fs;
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

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Repository not found") || stderr.contains("Failed to create manager")
        );
    }

    #[test]
    fn test_invalid_git_url() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Try various invalid URLs
        let invalid_urls = vec![
            "not-a-url",
            "http://nonexistent.domain.invalid/repo.git",
            "file:///nonexistent/path.git",
            "git@invalid-host:user/repo.git",
        ];

        for invalid_url in invalid_urls {
            let output = harness
                .run_submod(&["add", "invalid-test", "lib/invalid", invalid_url])
                .expect("Failed to run submod");

            assert!(!output.status.success());
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(stderr.contains("Failed to add submodule") || stderr.contains("clone failed"));
        }
    }

    #[test]
    fn test_invalid_config_file_path() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Try to use a non-existent config file
        harness
            .run_submod(&["--config", "/nonexistent/path/config.toml", "check"])
            .expect("Failed to run submod");

        // Should handle missing config file gracefully (create default or error)
        // The exact behavior depends on implementation
    }

    #[test]
    fn test_permission_denied_scenarios() {
        // Check if running as root - permission tests don't work as root
        let is_root = std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "0")
            .unwrap_or(false);

        if is_root {
            println!("Skipping permission test - running as root");
            return;
        }

        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Create a directory we can't write to
        let readonly_dir = harness.work_dir.join("readonly");
        fs::create_dir_all(&readonly_dir).expect("Failed to create readonly dir");

        // Make directory read-only
        let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&readonly_dir, perms).expect("Failed to set permissions");

        let remote_repo = harness
            .create_test_remote("perm_test")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Try to add submodule to read-only directory
        let output = harness
            .run_submod(&["add", "perm-test", "readonly/submodule", &remote_url])
            .expect("Failed to run submod");

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Permission denied") || stderr.contains("Failed to add submodule"));

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&readonly_dir).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&readonly_dir, perms).expect("Failed to restore permissions");
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

            let output = harness
                .run_submod(&["check"])
                .expect("Failed to run submod");
            assert!(!output.status.success());

            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("Failed to create manager") || stderr.contains("Failed to parse")
            );
        }
    }

    #[test]
    fn test_missing_submodule_for_operations() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

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
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    assert!(stderr.contains("not found") || stderr.contains("Failed to reset"));
                }
                "update" => {
                    // Update should succeed but do nothing
                    assert!(output.status.success());
                }
                _ => {}
            }
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

        // Test various potentially problematic sparse patterns
        let problematic_patterns = vec![
            "//invalid//path",
            "..",
            "../../../escape",
            "path/with/\0/null",
        ];

        for pattern in problematic_patterns {
            let output = harness
                .run_submod(&[
                    "add",
                    "sparse-test",
                    "lib/sparse-test",
                    &remote_url,
                    "--sparse-paths",
                    pattern,
                ])
                .expect("Failed to run submod");

            // Should either succeed and handle the pattern safely, or fail gracefully
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(!stderr.is_empty());
            }

            // Clean up for next iteration
            let _ = fs::remove_dir_all(harness.work_dir.join("lib/sparse-test"));
        }
    }

    #[test]
    fn test_concurrent_operations() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("concurrent")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add a submodule
        harness
            .run_submod_success(&["add", "concurrent-test", "lib/concurrent", &remote_url])
            .expect("Failed to add submodule");

        // Simulate concurrent access by modifying config externally
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

        // Run check to see if it handles the externally modified config
        let stdout = harness
            .run_submod_success(&["check"])
            .expect("Failed to run check");
        assert!(stdout.contains("concurrent-test"));
        assert!(stdout.contains("external-addition"));
    }

    #[test]
    fn test_disk_space_exhaustion_simulation() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Create a very large remote repository to potentially trigger space issues
        // This is more of a stress test than a true disk space test
        let remote_repo = harness
            .create_test_remote("large_repo")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add submodule - should handle any space issues gracefully
        let output = harness
            .run_submod(&["add", "large-repo", "lib/large", &remote_url])
            .expect("Failed to run submod");

        // Should either succeed or fail with a meaningful error
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!stderr.is_empty());
        }
    }

    #[test]
    fn test_invalid_command_line_arguments() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let invalid_args = vec![
            vec!["--invalid-flag"],
            vec!["add"],                 // Missing required arguments
            vec!["add", "name"],         // Missing required arguments
            vec!["add", "name", "path"], // Missing required arguments
            vec!["reset"],               // Missing submodule name when not using --all
            vec!["nonexistent-command"],
        ];

        for args in invalid_args {
            let output = harness.run_submod(&args).expect("Failed to run submod");
            assert!(!output.status.success());

            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!stderr.is_empty());
            // Should provide helpful error messages
            assert!(
                stderr.contains("error") || stderr.contains("Usage") || stderr.contains("help")
            );
        }
    }

    #[test]
    fn test_network_timeout_simulation() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Use a URL that should fail quickly (invalid domain)
        let timeout_url = "http://nonexistent.invalid.domain.test/repo.git";

        let output = harness
            .run_submod(&["add", "timeout-test", "lib/timeout", timeout_url])
            .expect("Failed to run submod");

        assert!(!output.status.success());
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

        let output = harness
            .run_submod(&["add", "fake-repo", "lib/fake", &fake_url])
            .expect("Failed to run submod");

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Failed to add submodule") || stderr.contains("not a git repository")
        );
    }

    #[test]
    fn test_config_file_locked() {
        // Check if running as root - permission tests don't work as root
        let is_root = std::process::Command::new("id")
            .arg("-u")
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim() == "0")
            .unwrap_or(false);

        if is_root {
            println!("Skipping config file lock test - running as root");
            return;
        }

        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Create initial config
        harness
            .create_config(
                "[test]\npath = \"test\"\nurl = \"https://example.com/test.git\"\nactive = true\n",
            )
            .expect("Failed to create config");

        // Make config file read-only to simulate lock
        let config_path = harness.config_path();
        let mut perms = fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o444);
        fs::set_permissions(&config_path, perms).expect("Failed to set permissions");

        let remote_repo = harness
            .create_test_remote("locked_config")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Try to add submodule (which requires writing to config)
        let output = harness
            .run_submod(&["add", "locked-test", "lib/locked", &remote_url])
            .expect("Failed to run submod");

        assert!(!output.status.success());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Permission denied") || stderr.contains("Failed to save config"));

        // Restore permissions for cleanup
        let mut perms = fs::metadata(&config_path).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&config_path, perms).expect("Failed to restore permissions");
    }

    #[test]
    fn test_recovery_from_partial_operations() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("partial_recovery")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Simulate partial operation by creating directory but not git repo
        let partial_dir = harness.work_dir.join("lib/partial");
        fs::create_dir_all(&partial_dir).expect("Failed to create partial dir");
        fs::write(partial_dir.join("partial_file.txt"), "partial content")
            .expect("Failed to create partial file");

        // Try to add submodule to existing directory
        let output = harness
            .run_submod(&["add", "partial-test", "lib/partial", &remote_url])
            .expect("Failed to run submod");

        // Should handle existing directory appropriately
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!stderr.is_empty());
            assert!(
                stderr.contains("already exists") || stderr.contains("Failed to add submodule")
            );
        }
    }
}
