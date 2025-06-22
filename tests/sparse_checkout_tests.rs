//! Integration tests focused on sparse checkout functionality
//!
//! These tests verify sparse checkout configuration, detection of mismatches,
//! and proper handling of sparse checkout patterns.

use std::fs;

mod common;
use common::TestHarness;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_checkout_basic_setup() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("sparse_basic")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add submodule with basic sparse paths
        harness
            .run_submod_success(&[
                "add",
                "sparse-basic",
                "lib/sparse-basic",
                &remote_url,
                "--sparse-paths",
                "src,docs",
            ])
            .expect("Failed to add submodule");

        // Verify sparse-checkout file exists and has correct content
        let sparse_file = harness.get_sparse_checkout_file_path("lib/sparse-basic");
        assert!(sparse_file.exists());

        let sparse_content = fs::read_to_string(&sparse_file).expect("Failed to read sparse file");
        assert!(sparse_content.contains("src"));
        assert!(sparse_content.contains("docs"));

        // Verify git config shows sparse checkout enabled
        let git_config_output = std::process::Command::new("git")
            .args(["config", "core.sparseCheckout"])
            .current_dir(harness.work_dir.join("lib/sparse-basic"))
            .output()
            .expect("Failed to run git config");

        let config_value = String::from_utf8_lossy(&git_config_output.stdout);
        assert!(config_value.trim() == "true");

        // Verify only specified directories exist in working tree
        assert!(harness.dir_exists("lib/sparse-basic/src"));
        assert!(harness.dir_exists("lib/sparse-basic/docs"));
        // These should not exist due to sparse checkout
        assert!(!harness.dir_exists("lib/sparse-basic/tests"));
        assert!(!harness.dir_exists("lib/sparse-basic/examples"));
    }

    #[test]
    fn test_sparse_checkout_with_patterns() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("sparse_patterns")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add submodule with pattern-based sparse paths
        harness
            .run_submod_success(&[
                "add",
                "sparse-patterns",
                "lib/sparse-patterns",
                &remote_url,
                "--sparse-paths",
                "src/,*.md,Cargo.toml",
            ])
            .expect("Failed to add submodule");

        let sparse_file = harness.get_sparse_checkout_file_path("lib/sparse-patterns");
        let sparse_content = fs::read_to_string(&sparse_file).expect("Failed to read sparse file");

        assert!(sparse_content.contains("src/"));
        assert!(sparse_content.contains("*.md"));
        assert!(sparse_content.contains("Cargo.toml"));

        // Verify pattern matching works
        assert!(harness.dir_exists("lib/sparse-patterns/src"));
        assert!(harness.file_exists("lib/sparse-patterns/README.md"));
        assert!(harness.file_exists("lib/sparse-patterns/Cargo.toml"));
    }

    #[test]
    fn test_sparse_checkout_mismatch_detection() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("sparse_mismatch")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add submodule with specific sparse paths
        harness
            .run_submod_success(&[
                "add",
                "sparse-mismatch",
                "lib/sparse-mismatch",
                &remote_url,
                "--sparse-paths",
                "src,docs",
            ])
            .expect("Failed to add submodule");

        // Manually modify sparse-checkout file to create mismatch
        let sparse_file = harness.get_sparse_checkout_file_path("lib/sparse-mismatch");
        fs::write(&sparse_file, "tests\nexamples\n").expect("Failed to modify sparse file");

        // Run check command to detect mismatch
        let stdout = harness
            .run_submod_success(&["check"])
            .expect("Failed to run check");

        assert!(stdout.contains("Sparse checkout mismatch"));
        assert!(stdout.contains("Expected:"));
        assert!(stdout.contains("Current:"));
    }

    #[test]
    fn test_sparse_checkout_disabled_detection() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("sparse_disabled")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add submodule normally first
        harness
            .run_submod_success(&["add", "sparse-disabled", "lib/sparse-disabled", &remote_url])
            .expect("Failed to add submodule");

        // Update config to include sparse paths
        let config_content = format!(
            r#"[sparse-disabled]
path = "lib/sparse-disabled"
url = "{remote_url}"
active = true
sparse_paths = ["src", "docs"]
"#
        );
        harness
            .create_config(&config_content)
            .expect("Failed to create config");

        // Remove sparse-checkout file to simulate it not being configured
        let sparse_file = harness.get_sparse_checkout_file_path("lib/sparse-disabled");
        if sparse_file.exists() {
            fs::remove_file(&sparse_file).expect("Failed to remove sparse file");
        }

        // Run check to detect missing sparse configuration
        let stdout = harness
            .run_submod_success(&["check"])
            .expect("Failed to run check");
        assert!(stdout.contains("Sparse checkout not configured"));
    }

    #[test]
    fn test_sparse_checkout_complex_patterns() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("sparse_complex")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Test complex patterns including negation
        harness
            .run_submod_success(&[
                "add",
                "sparse-complex",
                "lib/sparse-complex",
                &remote_url,
                "--sparse-paths",
                "src/,docs/,*.md,!tests/,!examples/",
            ])
            .expect("Failed to add submodule");

        let sparse_file = harness.get_sparse_checkout_file_path("lib/sparse-complex");
        let sparse_content = fs::read_to_string(&sparse_file).expect("Failed to read sparse file");

        assert!(sparse_content.contains("src/"));
        assert!(sparse_content.contains("docs/"));
        assert!(sparse_content.contains("*.md"));
        assert!(sparse_content.contains("!tests/"));
        assert!(sparse_content.contains("!examples/"));
    }

    #[test]
    fn test_init_with_sparse_checkout() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("sparse_init")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Create config with sparse paths but don't initialize yet
        let config_content = format!(
            r#"[sparse-init]
path = "lib/sparse-init"
url = "{remote_url}"
active = true
sparse_paths = ["src", "docs", "*.md"]
"#
        );
        harness
            .create_config(&config_content)
            .expect("Failed to create config");

        // Run init command
        harness
            .run_submod_success(&["init"])
            .expect("Failed to run init");

        // Verify sparse checkout was configured during init
        let sparse_file = harness.get_sparse_checkout_file_path("lib/sparse-init");
        assert!(sparse_file.exists());

        let sparse_content = fs::read_to_string(&sparse_file).expect("Failed to read sparse file");
        assert!(sparse_content.contains("src"));
        assert!(sparse_content.contains("docs"));
        assert!(sparse_content.contains("*.md"));

        // Verify only specified paths are checked out
        assert!(harness.dir_exists("lib/sparse-init/src"));
        assert!(harness.dir_exists("lib/sparse-init/docs"));
        assert!(harness.file_exists("lib/sparse-init/README.md"));
    }

    #[test]
    fn test_sparse_checkout_status_reporting() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("sparse_status")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add submodule without sparse checkout
        harness
            .run_submod_success(&["add", "no-sparse", "lib/no-sparse", &remote_url])
            .expect("Failed to add submodule");

        // Add submodule with sparse checkout
        harness
            .run_submod_success(&[
                "add",
                "with-sparse",
                "lib/with-sparse",
                &remote_url,
                "--sparse-paths",
                "src,docs",
            ])
            .expect("Failed to add submodule");

        // Run check to see status reporting
        let stdout = harness
            .run_submod_success(&["check"])
            .expect("Failed to run check");

        // Should show different status for each submodule
        assert!(stdout.contains("no-sparse"));
        assert!(stdout.contains("with-sparse"));
        assert!(stdout.contains("Sparse checkout configured correctly"));
    }

    #[test]
    fn test_sparse_checkout_empty_patterns() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("sparse_empty")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Try to add with empty sparse paths - should handle gracefully
        let output = harness.run_submod(&[
            "add",
            "sparse-empty",
            "lib/sparse-empty",
            &remote_url,
            "--sparse-paths",
            "",
        ]);

        // Should either succeed without sparse checkout or provide clear error
        if let Ok(process_output) = output {
            if process_output.status.success() {
                // If successful, sparse checkout should not be enabled
                let sparse_file = harness.get_sparse_checkout_file_path("lib/sparse-empty");
                assert!(
                    !sparse_file.exists()
                        || fs::read_to_string(&sparse_file).unwrap().trim().is_empty()
                );
            }
        } else {
            // If it fails, that's also acceptable for empty patterns
        }
    }

    #[test]
    fn test_sparse_checkout_with_sync_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("sparse_sync")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Create config with sparse paths
        let config_content = format!(
            r#"[sparse-sync]
path = "lib/sparse-sync"
url = "{remote_url}"
active = true
sparse_paths = ["src", "docs", "README.md"]
"#
        );
        harness
            .create_config(&config_content)
            .expect("Failed to create config");

        // Run sync command
        harness
            .run_submod_success(&["sync"])
            .expect("Failed to run sync");

        // Verify sparse checkout was configured during sync
        let sparse_file = harness.get_sparse_checkout_file_path("lib/sparse-sync");
        assert!(sparse_file.exists());

        let sparse_content = fs::read_to_string(&sparse_file).expect("Failed to read sparse file");
        assert!(sparse_content.contains("src"));
        assert!(sparse_content.contains("docs"));
        assert!(sparse_content.contains("README.md"));

        // Verify working tree matches sparse configuration
        assert!(harness.dir_exists("lib/sparse-sync/src"));
        assert!(harness.dir_exists("lib/sparse-sync/docs"));
        assert!(harness.file_exists("lib/sparse-sync/README.md"));
    }
}
