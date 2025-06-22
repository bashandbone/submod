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
            .run_submod_success(&["check"])
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
            .run_submod_success(&["check"])
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
            .run_submod(&["check"])
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
            .run_submod_success(&["check"])
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
                "new-submodule",
                "lib/new",
                &remote_url,
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
            .run_submod_success(&["check"])
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
            .run_submod_success(&["check"])
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
            .run_submod_success(&["check"])
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
            .run_submod_success(&["check"])
            .expect("Failed to run check");
        assert!(stdout.contains("Checking submodule configurations"));
    }
}
