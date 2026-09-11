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

        let before = harness.preservation_snapshot();
        // Should fail gracefully with a meaningful error
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run submod");
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        assert_eq!(harness.preservation_snapshot(), before);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("Cannot prepare check"), "{stderr}");
        assert!(
            stderr.contains("submod.toml") && stderr.contains("TOML parse error"),
            "{stderr}"
        );
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

        let before = harness.preservation_snapshot();
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run check");
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(harness.preservation_snapshot(), before);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("comprehensive-submodule: drift: checkout is missing"),
            "{stdout}"
        );
        let config = submod::Config::default()
            .load_from_file(Some(&harness.config_path()))
            .unwrap();
        assert_eq!(
            config
                .effective_entry("comprehensive-submodule")
                .unwrap()
                .path
                .as_deref(),
            Some("lib/comprehensive")
        );

        // Verify config was parsed correctly
        let config_content = harness.read_config().expect("Failed to read config");
        assert_eq!(config_content, comprehensive_config);
        let raw: toml::Value = toml::from_str(&config_content).unwrap();
        assert_eq!(raw["defaults"]["branch"].as_str(), Some("main"));
        let effective = config.effective_entry("comprehensive-submodule").unwrap();
        assert_eq!(
            effective.ignore,
            Some(submod::options::SerializableIgnore::Dirty)
        );
        assert_eq!(
            effective.update,
            Some(submod::options::SerializableUpdate::Merge)
        );
        assert_eq!(
            effective.fetch_recurse,
            Some(submod::options::SerializableFetchRecurse::Always)
        );
        assert_eq!(
            effective.branch,
            config
                .submodules
                .get("comprehensive-submodule")
                .unwrap()
                .branch
        );
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

        let before = harness.preservation_snapshot();
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run check");
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(harness.preservation_snapshot(), before);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("test-submodule: drift: checkout is missing"),
            "{stdout}"
        );
        let config = submod::Config::default()
            .load_from_file(Some(&harness.config_path()))
            .unwrap();
        assert_eq!(
            config
                .effective_entry("test-submodule")
                .unwrap()
                .path
                .as_deref(),
            Some("lib/test")
        );
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

        let before = harness.preservation_snapshot();
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run check");
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(harness.preservation_snapshot(), before);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("dev-tools: skipped-disabled at tools/dev (not materialized)"),
            "{stdout}"
        );
        assert!(!stdout.contains("dev-tools: drift:"), "{stdout}");
        assert!(
            stdout.contains("utils: drift: checkout is missing"),
            "{stdout}"
        );
        let config = submod::Config::default()
            .load_from_file(Some(&harness.config_path()))
            .unwrap();
        assert_eq!(
            config.effective_entry("utils").unwrap().path.as_deref(),
            Some("vendor/utils")
        );

        // Verify comments and formatting are preserved
        let config_content = harness.read_config().expect("Failed to read config");
        assert!(config_content.contains("# This is a test configuration file"));
        assert!(config_content.contains("# Main utility library"));
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

        let before = harness.preservation_snapshot();
        let output = harness
            .run_submod(&["check", "--verbose"])
            .expect("Failed to run check");
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert_eq!(harness.preservation_snapshot(), before);
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("special-chars: drift: checkout is missing"),
            "{stdout}"
        );
        let config = submod::Config::default()
            .load_from_file(Some(&harness.config_path()))
            .unwrap();
        assert_eq!(
            config
                .effective_entry("special-chars")
                .unwrap()
                .path
                .as_deref(),
            Some("lib/special-chars_123")
        );
    }

    #[test]
    fn test_public_surface_coverage() {
        use std::collections::HashMap;
        use submod::config::{
            OtherSubmoduleSettings, SubmoduleEntries, SubmoduleEntry, SubmoduleGitOptions,
            SubmoduleUpdateOptions,
        };
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
        assert!(
            !from_opts.recursive,
            "fetch policy must not opt into recursive materialization"
        );
        assert!(!from_opts.force);
        assert!(!from_opts.remote);

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
            SubmoduleEntry::from_options_and_settings(git_opts, other_settings.clone());
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

        // 4. Test raw Config loading using a real test harness
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

        assert_eq!(config.get_submodule("test-sub").unwrap().ignore, None);
        assert_eq!(
            config.effective_entry("test-sub").unwrap().ignore,
            Some(SerializableIgnore::Dirty)
        );
        assert!(!harness.work_dir.join(".gitmodules").exists());
    }
}

/// Phase 1 contracts intentionally assert the repaired behavior against the audited CLI.
#[cfg(test)]
mod phase3_acceptance_config {
    use super::*;

    fn fixture(config: &str) -> TestHarness {
        let h = TestHarness::new().unwrap();
        h.init_git_repo().unwrap();
        h.create_config(config).unwrap();
        h
    }

    fn document(h: &TestHarness) -> toml::Value {
        toml::from_str(&h.read_config().unwrap()).expect("edited document must remain valid TOML")
    }

    #[test]
    fn r06_defaults_do_not_become_explicit_overrides() {
        let h = fixture(
            "[defaults]\nignore = \"dirty\"\n[inherits]\nurl = \"./remote.git\"\nactive = false\n[explicit]\nurl = \"./other.git\"\nactive = false\nignore = \"none\"\n",
        );
        h.run_submod_success(&["change-global", "--ignore", "all"])
            .unwrap();
        h.run_submod_success(&["change", "inherits", "--update", "none"])
            .unwrap();
        let value = document(&h);
        assert!(
            value["inherits"].get("ignore").is_none(),
            "inherited setting was pinned: {value}"
        );
        assert_eq!(value["defaults"]["ignore"].as_str(), Some("all"));
        assert_eq!(value["explicit"]["ignore"].as_str(), Some("none"));
        h.run_submod_success(&["list"])
            .expect("second process reload");
    }

    #[test]
    fn r08_checked_in_sample_loads_without_rewrite() {
        let sample = include_str!("../sample_config/submod.toml");
        let h = fixture(sample);
        h.run_submod_success(&["list"])
            .expect("exact shipped sample must load");
        assert_eq!(h.read_config().unwrap(), sample);
    }

    #[test]
    fn r08_generated_template_loads_without_rewrite() {
        let h = fixture("");
        h.run_submod_success(&["generate-config", "--template", "--force"])
            .unwrap();
        let before = h.read_config().unwrap();
        let _: toml::Value = toml::from_str(&before).unwrap();
        h.run_submod_success(&["list"])
            .expect("own generated template must load");
        assert_eq!(h.read_config().unwrap(), before);
    }

    macro_rules! supported_version {
        ($name:ident, $prefix:expr) => {
            #[test]
            fn $name() {
                let config = format!("{}[lib]\nurl = \"./remote.git\"\nactive = false\n", $prefix);
                let h = fixture(&config);
                h.run_submod_success(&["list"]).unwrap();
                assert_eq!(h.read_config().unwrap(), config);
            }
        };
    }
    supported_version!(r08_absent_schema_version_loads, "");
    supported_version!(r08_schema_1_0_loads, "schema_version = \"1.0.0\"\n");
    supported_version!(r08_schema_1_1_loads, "schema_version = \"1.1.0\"\n");

    macro_rules! rejected_config {
        ($name:ident, $config:expr, $context:expr) => {
            #[test]
            fn $name() {
                let h = fixture($config);
                let before = h.preservation_snapshot();
                let out = h.run_submod(&["change-global", "--ignore", "all"]).unwrap();
                assert_eq!(
                    h.preservation_snapshot(),
                    before,
                    "validation failure must not mutate config or Git"
                );
                assert!(!out.status.success(), "invalid config accepted: {out:?}");
                let error = String::from_utf8_lossy(&out.stderr);
                assert!(error.contains($context), "missing field context: {error}");
            }
        };
    }
    rejected_config!(
        r08_future_schema_rejected,
        "schema_version = \"99.0.0\"\n",
        "schema_version"
    );
    // spellchecker:off
    rejected_config!(
        r08_unknown_module_key_rejected,
        "[lib]\nurl = \"./remote.git\"\nactive = false\nignroe = \"dirty\"\n",
        "ignroe"
    );
    rejected_config!(
        r08_unknown_default_key_rejected,
        "[defaults]\nignroe = \"dirty\"\n",
        "ignroe"
    );
    // spellchecker:on
    rejected_config!(
        r08_conflicting_fetch_alias_rejected,
        "[lib]\nurl = \"./remote.git\"\nfetch = \"always\"\nfetchRecurse = \"never\"\n",
        "fetch"
    );
    rejected_config!(
        r08_conflicting_snake_alias_rejected,
        "[lib]\nurl = \"./remote.git\"\nfetch_recurse = \"always\"\nfetchRecurse = \"never\"\n",
        "fetch"
    );

    macro_rules! legacy_config {
        ($name:ident, $key:expr, $value:expr, $canonical:expr) => {
            #[test]
            fn $name() {
                let config = format!("[lib]\nurl = \"./remote.git\"\nactive = false\nbranch = \"HEAD\"\n{} = \"{}\"\n", $key, $value);
                let h = fixture(&config);
                let out = h.run_submod(&["list"]).unwrap();
                assert!(out.status.success(), "legacy generated config rejected: {out:?}");
                assert_eq!(h.read_config().unwrap(), config);
                let message = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)).to_lowercase();
                assert!(message.contains("warn") && (message.contains("legacy") || message.contains("migrat")), "migration warning missing: {message}");
                assert!(message.contains("lib") && message.contains(&$key.to_lowercase()), "migration warning lacks module/field context: {message}");
                h.run_submod_success(&["change", "lib", "--ignore", "all"]).unwrap();
                assert_eq!(document(&h)["lib"]["branch"].as_str(), Some("HEAD"));
                assert_eq!(document(&h)["lib"][$key].as_str(), Some($value));
                h.run_submod_success(&["change", "lib", "--fetch", $canonical]).unwrap();
                assert_eq!(document(&h)["lib"]["fetchRecurse"].as_str(), Some($canonical));
                assert!(document(&h)["lib"].get("fetch").is_none());
                assert!(document(&h)["lib"].get("fetch_recurse").is_none());
                h.run_submod_success(&["list"]).unwrap();
            }
        };
    }
    legacy_config!(r08_legacy_fetch_true_and_head, "fetch", "true", "always");
    legacy_config!(
        r08_legacy_fetch_false_and_head,
        "fetch_recurse",
        "false",
        "never"
    );

    legacy_config!(r08_legacy_canonical_true, "fetchRecurse", "true", "always");
    legacy_config!(r08_legacy_canonical_false, "fetchRecurse", "false", "never");
    legacy_config!(r08_legacy_fetch_alias, "fetch", "on-demand", "on-demand");
    legacy_config!(r08_legacy_snake_alias, "fetch_recurse", "always", "always");

    macro_rules! editor_case {
        ($name:ident, $header:expr, $nickname:expr) => {
            #[test]
            fn $name() {
                let untouched = "# retain this section verbatim\n[other]\nurl = './other.git' # untouched\nactive = false\n";
                let config = format!("{}\nurl = '''./remote.git'''\nactive = false\nsparse_paths = [\n  \"src/\", # keep pattern comment\n  \"docs/\",\n]\nignore = \"dirty\" # retain explanation\n\n{untouched}", $header);
                let h = fixture(&config);
                h.run_submod_success(&["change", $nickname, "--ignore", "all"]).unwrap();
                let value = document(&h);
                assert_eq!(value[$nickname]["ignore"].as_str(), Some("all"));
                assert_eq!(value[$nickname]["sparse_paths"].as_array().unwrap().len(), 2);
                let after = h.read_config().unwrap();
                assert!(after.contains(untouched), "unrelated section changed: {after}");
                assert!(after.contains("# keep pattern comment") && after.contains("# retain explanation"));
                h.run_submod_success(&["list"]).unwrap();
            }
        };
    }
    editor_case!(
        r09_multiline_array_commented_header,
        "[lib] # module comment",
        "lib"
    );
    editor_case!(
        r09_quoted_dotted_name,
        "[\"lib.with.dots\"]",
        "lib.with.dots"
    );
    editor_case!(r09_literal_unicode_name, "['bibliothèque']", "bibliothèque");
    editor_case!(
        r09_escaped_quoted_name,
        "[\"lib\\\"quoted\"]",
        "lib\"quoted"
    );

    #[test]
    fn r10_identical_edit_keeps_exact_bytes() {
        let config = "# preserve spacing\n[lib]\nurl='./remote.git'\nactive=false\nignore  =  'all' # same value\n";
        let h = fixture(config);
        let modified = std::fs::metadata(h.config_path())
            .unwrap()
            .modified()
            .unwrap();
        h.run_submod_success(&["change", "lib", "--ignore", "all"])
            .unwrap();
        assert_eq!(h.read_config().unwrap(), config);
        assert_eq!(
            std::fs::metadata(h.config_path())
                .unwrap()
                .modified()
                .unwrap(),
            modified
        );
        h.run_submod_success(&["list"]).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn r10_symlink_config_output_is_refused() {
        let h = fixture("[defaults]\nignore = \"dirty\"\n");
        let target = h.work_dir.join("real.toml");
        std::fs::rename(h.config_path(), &target).unwrap();
        std::os::unix::fs::symlink(&target, h.config_path()).unwrap();
        let before = std::fs::read(&target).unwrap();
        let out = h.run_submod(&["change-global", "--ignore", "all"]).unwrap();
        assert_eq!(
            std::fs::read(&target).unwrap(),
            before,
            "symlink target modified"
        );
        assert!(
            std::fs::symlink_metadata(h.config_path())
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(!out.status.success(), "symlink output accepted");
    }

    #[test]
    fn r10_failed_template_write_preserves_existing_config() {
        let h = fixture("# original\n[defaults]\nignore = \"dirty\"\n");
        let before = h.preservation_snapshot();
        std::fs::write(h.work_dir.join("blocked"), "occupied ancestor").unwrap();
        let out = h
            .run_submod(&[
                "generate-config",
                "--template",
                "--force",
                "--output",
                "blocked/config.toml",
            ])
            .unwrap();
        assert!(!out.status.success(), "write beneath a file must fail");
        assert_eq!(h.preservation_snapshot(), before);
        assert_eq!(
            std::fs::read_to_string(h.work_dir.join("blocked")).unwrap(),
            "occupied ancestor"
        );
        h.run_submod_success(&["change-global", "--ignore", "all"])
            .expect("normal error releases any locks");
    }
}

#[test]
fn phase3_acceptance_r06_effective_defaults_applied_after_fresh_load() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("inheritance").unwrap();
    h.create_config(&format!("[defaults]\nignore='dirty'\n[inherits]\nurl='file://{}'\n[explicit]\nurl='file://{}'\nignore='none'\n", remote.display(), remote.display())).unwrap();
    h.run_submod_success(&["init"]).unwrap();
    h.run_submod_success(&["change-global", "--ignore", "all"])
        .unwrap();
    h.run_submod_success(&["change", "inherits", "--update", "none"])
        .unwrap();
    h.run_submod_success(&["sync"]).unwrap();
    let value: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
    assert!(value["inherits"].get("ignore").is_none());
    assert_eq!(value["explicit"]["ignore"].as_str(), Some("none"));
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get",
            "submodule.inherits.ignore"
        ])
        .trim(),
        "all"
    );
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get",
            "submodule.explicit.ignore"
        ])
        .trim(),
        "none"
    );
}

#[test]
fn phase3_acceptance_r09_multiline_strings_and_escaped_values() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let untouched = "url = \"\"\"\n./remote.git\"\"\" # multiline URL\nactive = false\nsparse_paths = [\"src/\\\"quoted\\\"\", 'docs/é'] # escaped value\n";
    h.create_config(&format!("['bibliothèque.with.dots'] # literal dotted name\n{untouched}ignore = 'dirty' # retain inline\n")).unwrap();
    let before: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
    h.run_submod_success(&["change", "bibliothèque.with.dots", "--ignore", "all"])
        .unwrap();
    let after = h.read_config().unwrap();
    let value: toml::Value = toml::from_str(&after).unwrap();
    assert_eq!(
        value["bibliothèque.with.dots"]["ignore"].as_str(),
        Some("all")
    );
    for field in ["url", "active", "sparse_paths"] {
        assert_eq!(
            value["bibliothèque.with.dots"][field],
            before["bibliothèque.with.dots"][field]
        );
    }
    assert!(
        after.contains(untouched),
        "untouched values reformatted: {after}"
    );
    assert!(after.contains("# retain inline"));
    h.run_submod_success(&["list"]).unwrap();
}

#[test]
fn phase3_acceptance_r09_dotted_keys_edit_without_duplicate_table() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h.create_config("# dotted field spelling\nlib.url = './remote.git'\nlib.active = false\nlib.ignore = 'dirty' # policy\n").unwrap();
    h.run_submod_success(&["change", "lib", "--ignore", "all"])
        .unwrap();
    let raw: toml::Value = toml::from_str(&h.read_config().unwrap()).unwrap();
    assert_eq!(raw["lib"]["ignore"].as_str(), Some("all"));
    assert_eq!(raw["lib"]["url"].as_str(), Some("./remote.git"));
    assert_eq!(raw["lib"]["active"].as_bool(), Some(false));
    assert!(h.read_config().unwrap().contains("# policy"));
    h.run_submod_success(&["list"]).unwrap();
}
