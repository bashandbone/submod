// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Integration tests focused on command contracts, output accuracy, and
//! behaviors not covered by existing tests.
//!
//! Covers:
//! - `completeme` shell completion output
//! - `add --no-init` (config-only add)
//! - `nuke-it-from-orbit` without `--kill` (reinit) and `--all`
//! - `generate-config --from-setup` and `--force`
//! - `change` command: path relocation, URL update, active toggle,
//!   sparse-path replace and append
//! - Error contracts for nonexistent targets (delete, disable, reset, change)
//! - Output contracts: exact phrases expected for each command

use std::fs;

mod common;
use common::TestHarness;

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // completeme – shell completions
    // =========================================================================

    #[test]
    fn test_completeme_bash_outputs_completion_script() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let output = harness
            .run_submod(&["completeme", "bash"])
            .expect("Failed to run completeme bash");

        assert!(
            output.status.success(),
            "completeme bash should succeed; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Bash completion scripts always contain the function/complete keyword
        assert!(
            stdout.contains("complete") || stdout.contains("_submod") || stdout.contains("submod"),
            "bash completion script should reference submod; got: {stdout}"
        );
    }

    #[test]
    fn test_completeme_zsh_outputs_completion_script() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let output = harness
            .run_submod(&["completeme", "zsh"])
            .expect("Failed to run completeme zsh");

        assert!(
            output.status.success(),
            "completeme zsh should succeed; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.is_empty(),
            "zsh completion script should not be empty"
        );
    }

    #[test]
    fn test_completeme_fish_outputs_completion_script() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let output = harness
            .run_submod(&["completeme", "fish"])
            .expect("Failed to run completeme fish");

        assert!(
            output.status.success(),
            "completeme fish should succeed; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.is_empty(),
            "fish completion script should not be empty"
        );
    }

    #[test]
    fn test_completeme_nushell_outputs_completion_script() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let output = harness
            .run_submod(&["completeme", "nushell"])
            .expect("Failed to run completeme nushell");

        assert!(
            output.status.success(),
            "completeme nushell should succeed; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.is_empty(),
            "nushell completion script should not be empty"
        );
    }

    #[test]
    fn test_completeme_completions_include_subcommand_names() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let output = harness
            .run_submod(&["completeme", "bash"])
            .expect("Failed to run completeme bash");

        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);

        // Key subcommands should appear somewhere in the completion output
        for subcmd in &["add", "check", "init", "update", "reset", "sync", "list"] {
            assert!(
                stdout.contains(subcmd),
                "bash completion should reference subcommand '{subcmd}'; output: {stdout}"
            );
        }
    }

    // =========================================================================
    // add --no-init
    // =========================================================================

    #[test]
    fn test_add_no_init_writes_config_only() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("noinit_lib")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "no-init-lib",
                "--path",
                "lib/noinit",
                "--no-init",
            ])
            .expect("Failed to add submodule with --no-init");

        // --no-init produces no stdout; success of the command is the signal.

        // Config should be written
        let config = harness.read_config().expect("Failed to read config");
        assert!(
            config.contains("[no-init-lib]"),
            "Config should contain the submodule section"
        );
        assert!(
            config.contains("path = \"lib/noinit\""),
            "Config should contain path"
        );
        assert!(
            config.contains(&format!("url = \"{remote_url}\"")),
            "Config should contain URL"
        );

        // Directory should NOT have been cloned
        assert!(
            !harness.file_exists("lib/noinit/.git"),
            "With --no-init the submodule directory should not be cloned"
        );
    }

    #[test]
    fn test_add_no_init_then_init_initializes_submodule() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("noinit_then_init")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Add without init
        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "lazy-lib",
                "--path",
                "lib/lazy",
                "--no-init",
            ])
            .expect("Failed to add submodule with --no-init");

        assert!(
            !harness.file_exists("lib/lazy/.git"),
            "Should not be initialized yet"
        );

        // Now init it
        let _init_stdout = harness
            .run_submod_success(&["init"])
            .expect("Failed to run init");

        assert!(
            harness.file_exists("lib/lazy/.git"),
            "After init the submodule should exist"
        );

        // Verbose init should mention the submodule
        let init_verbose = harness
            .run_submod_success(&["init", "--verbose"])
            .expect("Failed to run init --verbose");
        assert!(
            init_verbose.contains("lazy-lib") || init_verbose.contains("already initialized"),
            "Verbose init should mention the submodule; got: {init_verbose}"
        );
    }

    // =========================================================================
    // nuke-it-from-orbit – all flag
    // =========================================================================

    #[test]
    fn test_nuke_all_removes_all_submodules() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote1 = harness
            .create_test_remote("nuke_all_a")
            .expect("Failed to create remote");
        let remote2 = harness
            .create_test_remote("nuke_all_b")
            .expect("Failed to create remote");
        let url1 = format!("file://{}", remote1.display());
        let url2 = format!("file://{}", remote2.display());

        harness
            .run_submod_success(&["add", &url1, "--name", "nuke-a", "--path", "lib/nuke-a"])
            .expect("Failed to add submodule A");
        harness
            .run_submod_success(&["add", &url2, "--name", "nuke-b", "--path", "lib/nuke-b"])
            .expect("Failed to add submodule B");

        let config_before = harness.read_config().expect("Failed to read config");
        assert!(config_before.contains("[nuke-a]"));
        assert!(config_before.contains("[nuke-b]"));

        let stdout = harness
            .run_submod_success(&["nuke-it-from-orbit", "--all", "--kill"])
            .expect("Failed to nuke all");

        assert!(
            stdout.contains("Nuking") || stdout.contains("💥"),
            "Expected nuke progress output; got: {stdout}"
        );

        let config_after = harness.read_config().expect("Failed to read config");
        assert!(
            !config_after.contains("[nuke-a]"),
            "nuke-a should be removed from config"
        );
        assert!(
            !config_after.contains("[nuke-b]"),
            "nuke-b should be removed from config"
        );
    }

    // =========================================================================
    // nuke-it-from-orbit – without --kill (reinit)
    // =========================================================================

    #[test]
    fn test_nuke_without_kill_reinitializes_submodule() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("nuke_reinit")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&["add", &url, "--name", "reinit-lib", "--path", "lib/reinit"])
            .expect("Failed to add submodule");

        assert!(
            harness.file_exists("lib/reinit/.git"),
            "Submodule should exist after add"
        );

        // Nuke without --kill: should delete then reinitialize
        let stdout = harness
            .run_submod_success(&["nuke-it-from-orbit", "reinit-lib"])
            .expect("Failed to nuke-and-reinit");

        assert!(
            stdout.contains("Nuking") || stdout.contains("Reinitializing"),
            "Expected nuke/reinit progress; got: {stdout}"
        );

        // After reinit, submodule should exist again
        assert!(
            harness.file_exists("lib/reinit/.git"),
            "After nuke-without-kill the submodule should be reinitialized"
        );

        // Config should still contain the submodule
        let config = harness.read_config().expect("Failed to read config");
        assert!(
            config.contains("[reinit-lib]"),
            "Config should retain the submodule entry after reinit"
        );
    }

    #[test]
    fn test_nuke_without_kill_preserves_sparse_paths() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_complex_remote("nuke_sparse_reinit")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &url,
                "--name",
                "sparse-reinit",
                "--path",
                "lib/sparse-reinit",
                "--sparse-paths",
                "src,docs",
            ])
            .expect("Failed to add submodule");

        // Nuke without --kill
        harness
            .run_submod_success(&["nuke-it-from-orbit", "sparse-reinit"])
            .expect("Failed to nuke-and-reinit");

        // Sparse checkout should be reconfigured
        let sparse_file = harness.get_sparse_checkout_file_path("lib/sparse-reinit");
        assert!(
            sparse_file.exists(),
            "Sparse checkout file should exist after reinit"
        );

        let sparse_content = fs::read_to_string(&sparse_file).expect("Failed to read sparse file");
        assert!(
            sparse_content.contains("src"),
            "Sparse checkout should contain 'src' after reinit"
        );
        assert!(
            sparse_content.contains("docs"),
            "Sparse checkout should contain 'docs' after reinit"
        );
    }

    // =========================================================================
    // generate-config --from-setup
    // =========================================================================

    #[test]
    fn test_generate_config_from_setup_reads_gitmodules() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("genconf_setup")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        // Add a real submodule so .gitmodules is populated
        harness
            .run_submod_success(&["add", &url, "--name", "setup-lib", "--path", "lib/setup"])
            .expect("Failed to add submodule");

        // Verify .gitmodules was created
        assert!(
            harness.work_dir.join(".gitmodules").exists(),
            ".gitmodules should exist after adding a submodule"
        );

        let output_path = harness.work_dir.join("from_setup.toml");

        let stdout = harness
            .run_submod_success(&[
                "--config",
                output_path.to_str().unwrap(),
                "generate-config",
                "--from-setup",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .expect("Failed to generate config from setup");

        assert!(
            stdout.contains("Generated config from .gitmodules"),
            "Expected success message; got: {stdout}"
        );

        assert!(output_path.exists(), "Output config file should exist");

        let content = fs::read_to_string(&output_path).expect("Failed to read generated config");
        // The generated config should reference the submodule's URL (the most reliable identifier)
        assert!(
            content.contains(remote.to_str().unwrap_or("")),
            "Generated config should reference the submodule URL; got: {content}"
        );
    }

    #[test]
    fn test_generate_config_force_overwrites_existing() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let output_path = harness.work_dir.join("force_gen.toml");
        fs::write(&output_path, "# existing content\n").expect("Failed to write existing file");

        // Without --force should fail
        let no_force = harness
            .run_submod(&[
                "--config",
                output_path.to_str().unwrap(),
                "generate-config",
                "--output",
                output_path.to_str().unwrap(),
            ])
            .expect("Failed to run generate-config");
        assert!(
            !no_force.status.success(),
            "generate-config without --force on existing file should fail"
        );

        // With --force should succeed and overwrite
        let stdout = harness
            .run_submod_success(&[
                "--config",
                output_path.to_str().unwrap(),
                "generate-config",
                "--output",
                output_path.to_str().unwrap(),
                "--force",
            ])
            .expect("Failed to generate config with --force");

        assert!(
            stdout.contains("Generated"),
            "Expected success message after --force; got: {stdout}"
        );

        let content = fs::read_to_string(&output_path).expect("Failed to read generated config");
        // The existing content must be gone and replaced with generated defaults.
        assert!(
            !content.contains("# existing content") && content.contains("[defaults]"),
            "Content should have been overwritten by --force; got: {content}"
        );
    }

    // =========================================================================
    // change command – path change (delete + re-add)
    // =========================================================================

    #[test]
    fn test_change_path_reclones_at_new_location() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("change_path")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &url,
                "--name",
                "movable-lib",
                "--path",
                "lib/original",
            ])
            .expect("Failed to add submodule");

        assert!(harness.file_exists("lib/original/.git"));

        let stdout = harness
            .run_submod_success(&["change", "movable-lib", "--path", "lib/moved"])
            .expect("Failed to change submodule path");

        // Should confirm the update
        assert!(
            stdout.contains("Added submodule") || stdout.contains("movable-lib"),
            "Expected confirmation of path change; got: {stdout}"
        );

        // New path should be cloned
        assert!(
            harness.file_exists("lib/moved/.git"),
            "Submodule should exist at new path"
        );

        // Config should reflect the new path
        let config = harness.read_config().expect("Failed to read config");
        assert!(
            config.contains("path = \"lib/moved\""),
            "Config should have the new path"
        );
        assert!(
            !config.contains("path = \"lib/original\""),
            "Config should not have the old path"
        );
    }

    // =========================================================================
    // change command – URL update
    // =========================================================================

    #[test]
    fn test_change_url_updates_config() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Create two remotes
        let remote1 = harness
            .create_test_remote("url_orig")
            .expect("Failed to create remote 1");
        let remote2 = harness
            .create_test_remote("url_new")
            .expect("Failed to create remote 2");
        let url1 = format!("file://{}", remote1.display());
        let url2 = format!("file://{}", remote2.display());

        harness
            .run_submod_success(&["add", &url1, "--name", "url-lib", "--path", "lib/urltest"])
            .expect("Failed to add submodule");

        harness
            .run_submod_success(&["change", "url-lib", "--url", &url2])
            .expect("Failed to change URL");

        let config = harness.read_config().expect("Failed to read config");
        assert!(
            config.contains(&format!("url = \"{url2}\"")),
            "Config should contain the new URL"
        );
        assert!(
            !config.contains(&format!("url = \"{url1}\"")),
            "Config should not contain the old URL"
        );
    }

    // =========================================================================
    // change command – active flag
    // =========================================================================

    #[test]
    fn test_change_active_false_disables_submodule() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("active_toggle")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&["add", &url, "--name", "toggle-lib", "--path", "lib/toggle"])
            .expect("Failed to add submodule");

        // Disable via `change`
        harness
            .run_submod_success(&["change", "toggle-lib", "--active", "false"])
            .expect("Failed to change active flag");

        let config = harness.read_config().expect("Failed to read config");
        assert!(
            config.contains("active = false"),
            "Config should show active = false"
        );
    }

    #[test]
    fn test_change_active_true_reenables_submodule() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("reenable_lib")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        // Start disabled
        let config_content =
            format!("[reenable-lib]\npath = \"lib/reenable\"\nurl = \"{url}\"\nactive = false\n");
        harness
            .create_config(&config_content)
            .expect("Failed to create config");

        harness
            .run_submod_success(&["change", "reenable-lib", "--active", "true"])
            .expect("Failed to change active flag to true");

        let config = harness.read_config().expect("Failed to read config");
        assert!(
            config.contains("active = true"),
            "Config should show active = true"
        );
    }

    // =========================================================================
    // change command – sparse paths: replace vs append
    // =========================================================================

    #[test]
    fn test_change_sparse_paths_replaces_existing() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_complex_remote("sparse_replace")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &url,
                "--name",
                "sparse-replace",
                "--path",
                "lib/sparse-replace",
                "--sparse-paths",
                "src,docs",
            ])
            .expect("Failed to add submodule");

        // Replace sparse paths (no --append)
        harness
            .run_submod_success(&["change", "sparse-replace", "--sparse-paths", "tests"])
            .expect("Failed to change sparse paths");

        let config = harness.read_config().expect("Failed to read config");
        assert!(
            config.contains("\"tests\""),
            "Config should contain new sparse path 'tests'"
        );
        // Old paths must be gone after a replace (not an append)
        assert!(
            !config.contains("\"src\"") && config.contains("\"tests\""),
            "Old sparse path 'src' should be replaced (gone), new path 'tests' should be present; config: {config}"
        );
    }

    #[test]
    fn test_change_sparse_paths_append_adds_to_existing() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_complex_remote("sparse_append")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &url,
                "--name",
                "sparse-append",
                "--path",
                "lib/sparse-append",
                "--sparse-paths",
                "src,docs",
            ])
            .expect("Failed to add submodule");

        // Append more sparse paths
        harness
            .run_submod_success(&[
                "change",
                "sparse-append",
                "--sparse-paths",
                "tests",
                "--append",
            ])
            .expect("Failed to append sparse paths");

        let config = harness.read_config().expect("Failed to read config");
        assert!(
            config.contains("\"tests\""),
            "Config should contain appended path 'tests'"
        );
        assert!(
            config.contains("\"src\""),
            "Config should still contain original path 'src'"
        );
        assert!(
            config.contains("\"docs\""),
            "Config should still contain original path 'docs'"
        );
    }

    // =========================================================================
    // change command – multiple fields in one call
    // =========================================================================

    #[test]
    fn test_change_multiple_fields_at_once() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("multi_change")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        let config_content =
            format!("[multi-lib]\npath = \"lib/multi\"\nurl = \"{url}\"\nactive = true\n");
        harness
            .create_config(&config_content)
            .expect("Failed to create config");

        harness
            .run_submod_success(&[
                "change",
                "multi-lib",
                "--ignore",
                "dirty",
                "--update",
                "rebase",
                "--branch",
                "main",
            ])
            .expect("Failed to change multiple fields");

        let config = harness.read_config().expect("Failed to read config");
        assert!(config.contains("ignore = \"dirty\""), "ignore not updated");
        assert!(config.contains("update = \"rebase\""), "update not updated");
        assert!(config.contains("branch = \"main\""), "branch not updated");
    }

    // =========================================================================
    // change-global – all fields
    // =========================================================================

    #[test]
    fn test_change_global_sets_update_and_fetch() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("[defaults]\nignore = \"none\"\n")
            .expect("Failed to create config");

        harness
            .run_submod_success(&["change-global", "--update", "rebase", "--fetch", "always"])
            .expect("Failed to run change-global with update and fetch");

        let config = harness.read_config().expect("Failed to read config");
        assert!(
            config.contains("update = \"rebase\""),
            "Global update not saved"
        );
        // The `--fetch always` CLI value maps to SerializableFetchRecurse::Always.
        // That variant's TOML serialization (via to_gitmodules()) uses git's native
        // fetchRecurseSubmodules encoding: "true" means "always fetch".
        // The TOML config therefore stores `fetch = "true"` not `fetch = "always"`.
        assert!(
            config.contains("fetch = \"true\""),
            "Global fetch not saved; config: {config}"
        );
    }

    #[test]
    fn test_change_global_without_args_fails() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("[defaults]\n")
            .expect("Failed to create config");

        let output = harness
            .run_submod(&["change-global"])
            .expect("Failed to run change-global");

        // Should fail because no arguments were given
        assert!(
            !output.status.success(),
            "change-global with no args should fail"
        );
    }

    // =========================================================================
    // Error contracts: nonexistent targets
    // =========================================================================

    #[test]
    fn test_delete_nonexistent_submodule_fails_with_message() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("[defaults]\n")
            .expect("Failed to create config");

        let output = harness
            .run_submod(&["delete", "does-not-exist"])
            .expect("Failed to run delete");

        assert!(
            !output.status.success(),
            "delete on nonexistent submodule should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("not found")
                || stderr.contains("does-not-exist")
                || stderr.contains("Failed to delete"),
            "Expected informative error; got: {stderr}"
        );
    }

    #[test]
    fn test_disable_nonexistent_submodule_fails_with_message() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("[defaults]\n")
            .expect("Failed to create config");

        let output = harness
            .run_submod(&["disable", "ghost-lib"])
            .expect("Failed to run disable");

        assert!(
            !output.status.success(),
            "disable on nonexistent submodule should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("not found")
                || stderr.contains("ghost-lib")
                || stderr.contains("Failed to disable"),
            "Expected informative error; got: {stderr}"
        );
    }

    #[test]
    fn test_change_nonexistent_submodule_fails_with_message() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("[defaults]\n")
            .expect("Failed to create config");

        let output = harness
            .run_submod(&["change", "phantom-lib", "--ignore", "dirty"])
            .expect("Failed to run change");

        assert!(
            !output.status.success(),
            "change on nonexistent submodule should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("not found")
                || stderr.contains("phantom-lib")
                || stderr.contains("Failed to change"),
            "Expected informative error; got: {stderr}"
        );
    }

    #[test]
    fn test_nuke_nonexistent_submodule_fails_with_message() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("[defaults]\n")
            .expect("Failed to create config");

        let output = harness
            .run_submod(&["nuke-it-from-orbit", "vanished-lib", "--kill"])
            .expect("Failed to run nuke");

        assert!(
            !output.status.success(),
            "nuke on nonexistent submodule should fail"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("not found")
                || stderr.contains("vanished-lib")
                || stderr.contains("Failed to nuke"),
            "Expected informative error; got: {stderr}"
        );
    }

    // =========================================================================
    // Output contracts: exact phrases for core commands
    // =========================================================================

    #[test]
    fn test_add_output_contract() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("add_contract")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        let stdout = harness
            .run_submod_success(&[
                "add",
                &url,
                "--name",
                "add-contract",
                "--path",
                "lib/addcnt",
            ])
            .expect("Failed to add submodule");

        assert!(
            stdout.contains("Added submodule"),
            "add output should contain 'Added submodule'; got: {stdout}"
        );
        assert!(
            stdout.contains("add-contract"),
            "add output should name the submodule; got: {stdout}"
        );
    }

    #[test]
    fn test_check_output_contract() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("# empty\n")
            .expect("Failed to create config");

        // Default (non-verbose) check with no submodules produces minimal output
        let stdout = harness
            .run_submod_success(&["check"])
            .expect("Failed to run check");
        assert!(
            !stdout.contains("FAIL"),
            "check output should not contain errors; got: {stdout}"
        );

        // Verbose check should show the detailed header
        let stdout_verbose = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check --verbose");
        assert!(
            stdout_verbose.contains("Checking submodule configurations"),
            "verbose check should say 'Checking submodule configurations'; got: {stdout_verbose}"
        );
    }

    #[test]
    fn test_init_output_contract() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("init_contract")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        let config_content =
            format!("[init-contract]\npath = \"lib/ic\"\nurl = \"{url}\"\nactive = true\n");
        harness
            .create_config(&config_content)
            .expect("Failed to create config");

        // Default init succeeds silently
        let _stdout = harness
            .run_submod_success(&["init"])
            .expect("Failed to run init");

        // Verbose init should mention initialization
        let stdout_verbose = harness
            .run_submod_success(&["init", "--verbose"])
            .expect("Failed to run init --verbose");
        assert!(
            stdout_verbose.contains("Initializing") || stdout_verbose.contains("initialized"),
            "verbose init output should mention initialization; got: {stdout_verbose}"
        );
    }

    #[test]
    fn test_reset_output_contract() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("reset_contract")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&["add", &url, "--name", "reset-contract", "--path", "lib/rc"])
            .expect("Failed to add submodule");

        let stdout = harness
            .run_submod_success(&["reset", "reset-contract"])
            .expect("Failed to run reset");

        assert!(
            stdout.contains("Hard resetting"),
            "reset output should say 'Hard resetting'; got: {stdout}"
        );
        assert!(
            stdout.contains("reset complete"),
            "reset output should say 'reset complete'; got: {stdout}"
        );
        assert!(
            stdout.contains("reset-contract"),
            "reset output should name the submodule; got: {stdout}"
        );
    }

    #[test]
    fn test_sync_output_contract() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("sync_contract")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        let config_content =
            format!("[sync-contract]\npath = \"lib/sc\"\nurl = \"{url}\"\nactive = true\n");
        harness
            .create_config(&config_content)
            .expect("Failed to create config");

        // Default sync shows concise output
        let stdout = harness
            .run_submod_success(&["sync"])
            .expect("Failed to run sync");
        assert!(
            stdout.contains("Syncing submodules:"),
            "sync output should say 'Syncing submodules:'; got: {stdout}"
        );
        assert!(
            stdout.contains("Sync complete"),
            "sync output should say 'Sync complete'; got: {stdout}"
        );

        // Verbose sync shows detailed output
        let stdout_verbose = harness
            .run_submod_success(&["sync", "--verbose"])
            .expect("Failed to run sync --verbose");
        assert!(
            stdout_verbose.contains("Running full sync"),
            "verbose sync should say 'Running full sync'; got: {stdout_verbose}"
        );
    }

    #[test]
    fn test_update_output_contract_no_submodules() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");
        harness
            .create_config("# empty\n")
            .expect("Failed to create config");

        let stdout = harness
            .run_submod_success(&["update"])
            .expect("Failed to run update");

        assert!(
            stdout.contains("No submodules configured"),
            "update with no submodules should say 'No submodules configured'; got: {stdout}"
        );
    }

    #[test]
    fn test_list_output_shows_url_and_status() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("list_full")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&["add", &url, "--name", "list-full", "--path", "lib/lf"])
            .expect("Failed to add submodule");

        let stdout = harness
            .run_submod_success(&["list"])
            .expect("Failed to run list");

        assert!(
            stdout.contains("list-full"),
            "list should show submodule name"
        );
        assert!(stdout.contains("lib/lf"), "list should show submodule path");
        assert!(stdout.contains(&url), "list should show submodule URL");
        assert!(stdout.contains("active"), "list should show active status");
    }

    #[test]
    fn test_disable_output_contract() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("disable_contract")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &url,
                "--name",
                "dis-contract",
                "--path",
                "lib/discon",
            ])
            .expect("Failed to add submodule");

        let stdout = harness
            .run_submod_success(&["disable", "dis-contract"])
            .expect("Failed to disable");

        assert!(
            stdout.contains("Disabled submodule 'dis-contract'"),
            "disable output should confirm; got: {stdout}"
        );
    }

    #[test]
    fn test_delete_output_contract() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("del_contract")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &url,
                "--name",
                "del-contract",
                "--path",
                "lib/delcon",
            ])
            .expect("Failed to add submodule");

        let stdout = harness
            .run_submod_success(&["delete", "del-contract"])
            .expect("Failed to delete");

        assert!(
            stdout.contains("Deleted submodule 'del-contract'"),
            "delete output should confirm; got: {stdout}"
        );
    }

    // =========================================================================
    // Config accuracy: add writes all specified fields
    // =========================================================================

    #[test]
    fn test_add_with_all_options_writes_complete_config() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote = harness
            .create_test_remote("full_options")
            .expect("Failed to create remote");
        let url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &url,
                "--name",
                "full-opts",
                "--path",
                "lib/full",
                "--branch",
                "main",
                "--ignore",
                "dirty",
                "--update",
                "rebase",
                "--fetch",
                "always",
                "--sparse-paths",
                "src,docs",
            ])
            .expect("Failed to add submodule with all options");

        let config = harness.read_config().expect("Failed to read config");
        assert!(config.contains("[full-opts]"), "section header missing");
        assert!(config.contains("path = \"lib/full\""), "path missing");
        assert!(config.contains(&format!("url = \"{url}\"")), "url missing");
        assert!(config.contains("active = true"), "active missing");
        assert!(
            config.contains("\"src\"") && config.contains("\"docs\""),
            "sparse_paths missing"
        );
    }

    // =========================================================================
    // Config persistence across successive operations
    // =========================================================================

    #[test]
    fn test_multiple_adds_all_appear_in_config() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let names_and_paths = [
            ("alpha-lib", "lib/alpha"),
            ("beta-lib", "lib/beta"),
            ("gamma-lib", "lib/gamma"),
        ];

        for (name, path) in &names_and_paths {
            let remote = harness
                .create_test_remote(name)
                .expect("Failed to create remote");
            let url = format!("file://{}", remote.display());
            harness
                .run_submod_success(&["add", &url, "--name", name, "--path", path])
                .expect("Failed to add submodule");
        }

        let config = harness.read_config().expect("Failed to read config");

        for (name, path) in &names_and_paths {
            assert!(
                config.contains(&format!("[{name}]")),
                "Config should contain section [{name}]"
            );
            assert!(
                config.contains(&format!("path = \"{path}\"")),
                "Config should contain path for {name}"
            );
        }
    }

    #[test]
    fn test_delete_preserves_remaining_submodules_in_config() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote1 = harness
            .create_test_remote("preserve_a")
            .expect("Failed to create remote");
        let remote2 = harness
            .create_test_remote("preserve_b")
            .expect("Failed to create remote");
        let remote3 = harness
            .create_test_remote("preserve_c")
            .expect("Failed to create remote");

        let url1 = format!("file://{}", remote1.display());
        let url2 = format!("file://{}", remote2.display());
        let url3 = format!("file://{}", remote3.display());

        harness
            .run_submod_success(&["add", &url1, "--name", "keep-a", "--path", "lib/keep-a"])
            .expect("Failed to add keep-a");
        harness
            .run_submod_success(&["add", &url2, "--name", "remove-b", "--path", "lib/remove-b"])
            .expect("Failed to add remove-b");
        harness
            .run_submod_success(&["add", &url3, "--name", "keep-c", "--path", "lib/keep-c"])
            .expect("Failed to add keep-c");

        harness
            .run_submod_success(&["delete", "remove-b"])
            .expect("Failed to delete remove-b");

        let config = harness.read_config().expect("Failed to read config");
        assert!(config.contains("[keep-a]"), "keep-a should remain");
        assert!(config.contains("[keep-c]"), "keep-c should remain");
        assert!(!config.contains("[remove-b]"), "remove-b should be gone");
    }
}
