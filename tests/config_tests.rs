// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Integration tests focused on configuration management
//!
//! These tests verify TOML configuration parsing, serialization,
//! and the interaction between defaults and submodule-specific settings.

mod common;
use common::TestHarness;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization_roundtrip() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let original_config = r#"# Submodule configuration for gitoxide-based submodule manager
# Each section [name] defines a submodule

[defaults]
ignore = "dirty"
update = "checkout"
branch = "."
fetchRecurse = "on-demand"

[vendor-utils]
path = "vendor/utils"
url = "https://github.com/example/utils.git"
active = true
sparse_paths = ["src/", "include/", "*.md"]
ignore = "all"
update = "rebase"

[my-library]
path = "lib/my-library"
url = "https://github.com/example/my-library.git"
active = false
sparse_paths = ["src/", "docs/"]
"#;

        // Create config and verify it can be parsed
        harness
            .create_config(original_config)
            .expect("Failed to create config");

        // Run a command that loads and potentially saves the config
        let stdout = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");
        assert!(stdout.contains("Checking submodule configurations"));

        // Verify config content is preserved
        let config_content = harness.read_config().expect("Failed to read config");
        assert!(config_content.contains("[defaults]"));
        assert!(config_content.contains("ignore = \"dirty\""));
        assert!(config_content.contains("[vendor-utils]"));
        assert!(config_content.contains("active = true"));
        assert!(config_content.contains("[my-library]"));
        assert!(config_content.contains("active = false"));
    }

    #[test]
    fn test_defaults_inheritance() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let config_with_defaults = r#"[defaults]
ignore = "dirty"
update = "rebase"
fetchRecurse = "always"

[submodule-with-override]
path = "lib/override"
url = "https://github.com/example/override.git"
active = true
ignore = "all"  # Override default

[submodule-inherits-defaults]
path = "lib/inherits"
url = "https://github.com/example/inherits.git"
active = true
"#;

        harness
            .create_config(config_with_defaults)
            .expect("Failed to create config");

        // Run check to see effective settings
        let stdout = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");

        assert!(stdout.contains("Checking submodule configurations"));
        // Check should show that one submodule overrides defaults while another inherits them
    }

    #[test]
    fn test_invalid_config_handling() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Test invalid TOML syntax
        let invalid_toml = r#"[submodule
path = "broken
url = "https://github.com/example/test.git"
"#;

        harness
            .create_config(invalid_toml)
            .expect("Failed to create invalid config");

        // Should fail gracefully with a meaningful error
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run submod");
        assert!(!output.status.success());

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Failed to create manager") || stderr.contains("Failed to parse"));
    }

    #[test]
    fn test_config_with_all_git_options() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let comprehensive_config = r#"[defaults]
ignore = "none"
update = "checkout"
branch = "main"
fetchRecurse = "on-demand"

[comprehensive-submodule]
path = "lib/comprehensive"
url = "https://github.com/example/comprehensive.git"
active = true
sparse_paths = ["src/", "include/", "docs/", "*.md", "LICENSE"]
ignore = "dirty"
update = "merge"
branch = "develop"
fetchRecurse = "always"
"#;

        harness
            .create_config(comprehensive_config)
            .expect("Failed to create config");

        let stdout = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");
        assert!(stdout.contains("Checking submodule configurations"));

        // Verify config was parsed correctly
        let config_content = harness.read_config().expect("Failed to read config");
        assert!(config_content.contains("comprehensive-submodule"));
        assert!(config_content.contains("ignore = \"dirty\""));
        assert!(config_content.contains("update = \"merge\""));
        assert!(config_content.contains("branch = \"develop\""));
        assert!(config_content.contains("fetchRecurse = \"always\""));
    }

    #[test]
    fn test_config_modification_via_add_command() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("config_test")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Start with existing config
        let initial_config = r#"[defaults]
ignore = "dirty"

[existing-submodule]
path = "lib/existing"
url = "https://github.com/example/existing.git"
active = true
"#;

        harness
            .create_config(initial_config)
            .expect("Failed to create initial config");

        // Add a new submodule
        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "new-submodule",
                "--path",
                "lib/new",
                "--sparse-paths",
                "src,docs",
            ])
            .expect("Failed to add submodule");

        // Verify config was updated properly
        let updated_config = harness
            .read_config()
            .expect("Failed to read updated config");

        // Should preserve existing content
        assert!(updated_config.contains("[defaults]"));
        assert!(updated_config.contains("ignore = \"dirty\""));
        assert!(updated_config.contains("[existing-submodule]"));

        // Should add new submodule
        assert!(updated_config.contains("[new-submodule]"));
        assert!(updated_config.contains("path = \"lib/new\""));
        assert!(updated_config.contains(&format!("url = \"{remote_url}\"")));
        assert!(updated_config.contains("active = true"));
        assert!(updated_config.contains("sparse_paths = [\"src\", \"docs\"]"));
    }

    #[test]
    fn test_empty_defaults_section() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let config_with_empty_defaults = r#"[defaults]

[test-submodule]
path = "lib/test"
url = "https://github.com/example/test.git"
active = true
"#;

        harness
            .create_config(config_with_empty_defaults)
            .expect("Failed to create config");

        let stdout = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");
        assert!(stdout.contains("Checking submodule configurations"));
    }

    #[test]
    fn test_config_with_comments_and_formatting() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let formatted_config = r#"# This is a test configuration file
# It demonstrates proper formatting and comments

[defaults]
# Set default ignore behavior
ignore = "dirty"
# Default update strategy
update = "checkout"

# Main utility library
[utils]
path = "vendor/utils"
url = "https://github.com/example/utils.git"
active = true
# Only checkout specific directories
sparse_paths = [
    "src/",
    "include/",
    "docs/",
    "*.md"
]
# Override default ignore setting
ignore = "all"

# Development dependency
[dev-tools]
path = "tools/dev"
url = "https://github.com/example/dev-tools.git"
active = false  # Not active by default
"#;

        harness
            .create_config(formatted_config)
            .expect("Failed to create config");

        let stdout = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");
        assert!(stdout.contains("Checking submodule configurations"));

        // Verify comments and formatting are preserved
        let config_content = harness.read_config().expect("Failed to read config");
        assert!(config_content.contains("# This is a test configuration file"));
        assert!(config_content.contains("# Main utility library"));
    }

    #[test]
    fn test_config_validation_missing_required_fields() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Config with missing required fields
        let incomplete_config = r"[incomplete-submodule]
# Missing path and url
active = true
";

        harness
            .create_config(incomplete_config)
            .expect("Failed to create config");

        // Should handle missing fields gracefully
        let stdout = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");
        assert!(stdout.contains("Checking submodule configurations"));
        // The check should report issues with incomplete configuration
    }

    #[test]
    fn test_config_with_special_characters_in_paths() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let special_config = r#"[special-chars]
path = "lib/special-chars_123"
url = "https://github.com/user-name/repo-name.git"
active = true
sparse_paths = ["src/**", "docs/*", "*.{md,txt,rst}"]
"#;

        harness
            .create_config(special_config)
            .expect("Failed to create config");

        let stdout = harness
            .run_submod_success(&["check", "--verbose"])
            .expect("Failed to run check");
        assert!(stdout.contains("Checking submodule configurations"));
    }

    #[test]
    fn test_public_surface_coverage() {
        use std::collections::HashMap;
        use submod::config::{
            OtherSubmoduleSettings, SubmoduleEntries, SubmoduleEntry, SubmoduleGitOptions,
            SubmoduleUpdateOptions,
        };
        use submod::git_ops::Git2Operations;
        use submod::options::{
            SerializableBranch, SerializableFetchRecurse, SerializableIgnore, SerializableUpdate,
        };

        // 1. Test SubmoduleUpdateOptions methods
        let update_opts = SubmoduleUpdateOptions::new(SerializableUpdate::Rebase, true, false);
        assert_eq!(update_opts.strategy, SerializableUpdate::Rebase);
        assert!(update_opts.recursive);
        assert!(!update_opts.force);

        let forced_opts = update_opts.forced();
        assert!(forced_opts.force);
        assert_eq!(forced_opts.strategy, SerializableUpdate::Rebase);
        assert!(forced_opts.recursive);

        let git_opts = SubmoduleGitOptions {
            ignore: Some(SerializableIgnore::Dirty),
            fetch_recurse: Some(SerializableFetchRecurse::Always),
            branch: Some(SerializableBranch::set_branch(Some("main".to_string())).unwrap()),
            update: Some(SerializableUpdate::Merge),
        };
        let from_opts = SubmoduleUpdateOptions::from_options(git_opts.clone());
        assert_eq!(from_opts.strategy, SerializableUpdate::Merge);
        assert!(from_opts.recursive);
        assert!(!from_opts.force);

        // 2. Test SubmoduleEntry constructors and updater methods
        let entry = SubmoduleEntry::new(
            Some("https://example.com/repo.git".to_string()),
            Some("lib/test".to_string()),
            Some(SerializableBranch::set_branch(Some("main".to_string())).unwrap()),
            Some(SerializableIgnore::Dirty),
            Some(SerializableUpdate::Merge),
            Some(SerializableFetchRecurse::Always),
            Some(true),
            Some(false),
            Some(false),
        );
        assert_eq!(entry.url.as_deref(), Some("https://example.com/repo.git"));

        let other_settings = OtherSubmoduleSettings {
            url: Some("https://example.com/repo-new.git".to_string()),
            path: Some("lib/test-new".to_string()),
            name: Some("test-new".to_string()),
            active: false,
            shallow: true,
            no_init: true,
        };

        let entry_from_opts =
            SubmoduleEntry::from_options_and_settings(git_opts.clone(), other_settings.clone());
        assert_eq!(
            entry_from_opts.url.as_deref(),
            Some("https://example.com/repo-new.git")
        );
        assert_eq!(entry_from_opts.path.as_deref(), Some("lib/test-new"));
        assert_eq!(entry_from_opts.active, Some(false));
        assert_eq!(entry_from_opts.shallow, Some(true));
        assert_eq!(entry_from_opts.no_init, Some(true));

        let updated_entry = entry.update_with_settings(other_settings);
        assert_eq!(
            updated_entry.url.as_deref(),
            Some("https://example.com/repo-new.git")
        );
        assert_eq!(updated_entry.path.as_deref(), Some("lib/test-new"));
        assert_eq!(updated_entry.active, Some(false));
        assert_eq!(updated_entry.shallow, Some(true));
        assert_eq!(updated_entry.no_init, Some(true));

        // 3. Test SubmoduleEntries::set_sparse_paths_for
        let mut entries = SubmoduleEntries::new(Some(HashMap::new()), Some(HashMap::new()));
        let entry_to_insert = SubmoduleEntry::new(
            Some("https://example.com/repo.git".to_string()),
            Some("lib/test".to_string()),
            None,
            None,
            None,
            None,
            Some(true),
            Some(false),
            Some(false),
        );
        entries.update_entry("test-sub".to_string(), entry_to_insert);

        // Test set_sparse_paths_for when paths is not empty
        entries.set_sparse_paths_for("test-sub", vec!["src/".to_string(), "docs/".to_string()]);
        let stored_entry = entries
            .iter()
            .find(|(name, _)| *name == "test-sub")
            .unwrap()
            .1
            .0;
        assert_eq!(
            stored_entry.sparse_paths,
            Some(vec!["src/".to_string(), "docs/".to_string()])
        );

        // Test set_sparse_paths_for when paths is empty
        entries.set_sparse_paths_for("test-sub", vec![]);
        let stored_entry_empty = entries
            .iter()
            .find(|(name, _)| *name == "test-sub")
            .unwrap()
            .1
            .0;
        assert_eq!(stored_entry_empty.sparse_paths, None);

        // 4. Test Config loading and sync methods using a real test harness
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        // Write a test config file
        let config_toml = r#"[defaults]
ignore = "dirty"
update = "checkout"

[test-sub]
path = "lib/test"
url = "https://github.com/example/test.git"
active = true
"#;
        let config_path = harness.config_path();
        std::fs::write(&config_path, config_toml).expect("Failed to write test config");

        // Load config from file using load_from_file
        let config = submod::Config::default()
            .load_from_file(Some(&config_path))
            .expect("Failed to load_from_file");
        assert_eq!(config.defaults.ignore, Some(SerializableIgnore::Dirty));

        // Test sync_with_git_config and load_with_git_sync
        let mut git_ops =
            Git2Operations::new(Some(&harness.work_dir)).expect("Failed to open git_ops");

        // Initially, config has a submodule but gitmodules has nothing.
        // We sync config with git config, which should write the submodule to .gitmodules
        config
            .sync_with_git_config(&mut git_ops)
            .expect("Failed to sync_with_git_config");

        // Verify .gitmodules was written
        let gitmodules_content = harness.gitmodules_entries();
        assert!(gitmodules_content.contains("submodule.test-sub.path"));

        // Now test load_with_git_sync
        let loaded_sync_config = submod::Config::default()
            .load_with_git_sync(&config_path, &mut git_ops, submod::Config::default())
            .expect("Failed to load_with_git_sync");
        assert_eq!(
            loaded_sync_config.defaults.ignore,
            Some(SerializableIgnore::Dirty)
        );
    }
}
