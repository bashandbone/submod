// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Tests for the `git_ops` module: `git2_ops`, `gix_ops`, and `GitOpsManager`.
//!
//! These tests exercise the library API directly (not through the CLI binary)
//! to improve coverage of the git operations backend layer.

mod common;
use common::TestHarness;

use std::collections::HashMap;
use submod::config::{SubmoduleEntries, SubmoduleEntry, SubmoduleUpdateOptions};
use submod::git_ops::{
    Git2Operations, GitConfig, GitOpsManager, GitOperations, GixOperations, SubmoduleStatusFlags,
};
use submod::options::ConfigLevel;

/// Build a `SubmoduleEntries` with a single placeholder entry for write tests.
fn one_entry_entries() -> SubmoduleEntries {
    let entry = SubmoduleEntry::new(
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
    let mut map = HashMap::new();
    map.insert("test-lib".to_string(), entry);
    SubmoduleEntries::new(Some(map), None)
}

// ============================================================
// Git2Operations tests
// ============================================================

#[cfg(test)]
mod git2_ops_tests {
    use super::*;

    #[test]
    fn test_new_from_valid_path() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir));
        assert!(ops.is_ok(), "should open a valid repo");
    }

    #[test]
    fn test_new_from_invalid_path() {
        let ops = Git2Operations::new(Some(std::path::Path::new("/nonexistent_submod_test_path")));
        assert!(ops.is_err(), "should fail for a non-repo path");
    }

    #[test]
    fn test_read_gitmodules_empty_repo() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        let entries = ops.read_gitmodules().expect("read gitmodules");
        assert_eq!(
            entries.submodule_iter().count(),
            0,
            "fresh repo has no submodules"
        );
    }

    #[test]
    fn test_list_submodules_empty() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        let subs = ops.list_submodules().expect("list submodules");
        assert!(subs.is_empty(), "fresh repo has no submodules");
    }

    #[test]
    fn test_read_git_config_local() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        let config = ops.read_git_config(ConfigLevel::Local);
        assert!(config.is_ok(), "local config read should succeed");
        // init_git_repo sets user.name and user.email in local config
        let config = config.unwrap();
        assert!(
            !config.entries.is_empty(),
            "local config should have at least user entries"
        );
    }

    #[test]
    fn test_write_and_read_git_config_local() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");

        let mut entries = HashMap::new();
        entries.insert("submod.testkey".to_string(), "testvalue123".to_string());
        let config = GitConfig { entries };

        ops.write_git_config(&config, ConfigLevel::Local)
            .expect("write config should succeed");

        let read_back = ops
            .read_git_config(ConfigLevel::Local)
            .expect("read after write should succeed");
        assert_eq!(
            read_back.entries.get("submod.testkey").map(String::as_str),
            Some("testvalue123"),
            "written value should be readable"
        );
    }

    #[test]
    fn test_set_config_value_local() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");

        ops.set_config_value("submod.singlekey", "singlevalue", ConfigLevel::Local)
            .expect("set_config_value should succeed");

        let config = ops
            .read_git_config(ConfigLevel::Local)
            .expect("read config");
        assert_eq!(
            config
                .entries
                .get("submod.singlekey")
                .map(String::as_str),
            Some("singlevalue"),
        );
    }

    #[test]
    fn test_write_gitmodules_empty_entries() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        // Writing empty entries should silently succeed (nothing to do).
        ops.write_gitmodules(&SubmoduleEntries::default())
            .expect("writing empty entries should succeed");
    }

    #[test]
    fn test_write_gitmodules_skips_unknown_submodule() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        // Entries that reference a submodule not yet added are silently skipped.
        ops.write_gitmodules(&one_entry_entries())
            .expect("writing unknown submodule entry should not error");
    }

    // ---- Error paths (submodule not found) --------------------------------

    #[test]
    fn test_init_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.init_submodule("nonexistent").is_err());
    }

    #[test]
    fn test_deinit_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.deinit_submodule("nonexistent", true).is_err());
    }

    #[test]
    fn test_update_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(
            ops.update_submodule("nonexistent", &SubmoduleUpdateOptions::default())
                .is_err()
        );
    }

    #[test]
    fn test_delete_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.delete_submodule("nonexistent").is_err());
    }

    #[test]
    fn test_get_submodule_status_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.get_submodule_status("nonexistent").is_err());
    }

    #[test]
    fn test_fetch_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.fetch_submodule("nonexistent").is_err());
    }

    #[test]
    fn test_reset_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.reset_submodule("nonexistent", true).is_err());
    }

    #[test]
    fn test_clean_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.clean_submodule("nonexistent", true, true).is_err());
    }

    #[test]
    fn test_stash_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.stash_submodule("nonexistent", false).is_err());
    }

    #[test]
    fn test_enable_sparse_checkout_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.enable_sparse_checkout("nonexistent").is_err());
    }

    #[test]
    fn test_set_sparse_patterns_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(
            ops.set_sparse_patterns("nonexistent", &["src/".to_string()])
                .is_err()
        );
    }

    #[test]
    fn test_get_sparse_patterns_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.get_sparse_patterns("nonexistent").is_err());
    }

    #[test]
    fn test_apply_sparse_checkout_not_supported() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        // git2 apply_sparse_checkout is always an error.
        assert!(ops.apply_sparse_checkout("any").is_err());
    }

    // ---- Tests with a real submodule (set up via CLI) ----------------------

    #[test]
    fn test_with_submodule_list_and_read_gitmodules() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness.create_test_remote("g2_list_sub").expect("remote");
        let remote_url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "list-sub",
                "--path",
                "lib/listtest",
            ])
            .expect("add submodule");

        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");

        let subs = ops.list_submodules().expect("list_submodules");
        assert!(!subs.is_empty(), "should have at least one submodule");
        assert!(
            subs.iter().any(|s| s.contains("lib/listtest")),
            "submodule path should be present"
        );

        let entries = ops.read_gitmodules().expect("read_gitmodules");
        assert!(
            entries.submodule_iter().count() > 0,
            "should have submodule entries"
        );
        assert!(
            entries
                .submodule_iter()
                .any(|(_, entry)| entry.path.as_deref() == Some("lib/listtest")),
            "entry with path 'lib/listtest' should be present"
        );
    }

    #[test]
    fn test_with_submodule_get_status() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness.create_test_remote("g2_status_sub").expect("remote");
        let remote_url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "status-sub",
                "--path",
                "lib/statussub",
            ])
            .expect("add submodule");

        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        let status = ops
            .get_submodule_status("lib/statussub")
            .expect("get_submodule_status");
        assert_eq!(status.path, "lib/statussub");
        assert!(!status.name.is_empty(), "name should not be empty");
    }

    #[test]
    fn test_with_submodule_enable_sparse_checkout_and_patterns() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness
            .create_test_remote("g2_sparse_sub")
            .expect("remote");
        let remote_url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "sparse-sub",
                "--path",
                "lib/sparsesub",
            ])
            .expect("add submodule");

        let ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");

        ops.enable_sparse_checkout("lib/sparsesub")
            .expect("enable_sparse_checkout");

        let patterns = vec!["src/".to_string(), "include/".to_string()];
        ops.set_sparse_patterns("lib/sparsesub", &patterns)
            .expect("set_sparse_patterns");

        let read_back = ops
            .get_sparse_patterns("lib/sparsesub")
            .expect("get_sparse_patterns");
        assert_eq!(read_back, patterns, "patterns round-trip");
    }

    #[test]
    fn test_with_submodule_write_gitmodules_updates_existing() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness
            .create_test_remote("g2_write_sub")
            .expect("remote");
        let remote_url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "write-sub",
                "--path",
                "lib/writesub",
            ])
            .expect("add submodule");

        let mut ops = Git2Operations::new(Some(&harness.work_dir)).expect("ops");
        let entries = ops.read_gitmodules().expect("read_gitmodules");
        // write_gitmodules with the same entries should succeed without error
        ops.write_gitmodules(&entries).expect("write_gitmodules");
    }
}

// ============================================================
// GixOperations tests
// ============================================================

#[cfg(test)]
mod gix_ops_tests {
    use super::*;
    use submod::config::SubmoduleAddOptions;

    #[test]
    fn test_new_from_valid_path() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir));
        assert!(ops.is_ok(), "should open a valid repo");
    }

    #[test]
    fn test_new_from_invalid_path() {
        let ops =
            GixOperations::new(Some(std::path::Path::new("/nonexistent_submod_gix_test_path")));
        assert!(ops.is_err(), "should fail for a non-repo path");
    }

    #[test]
    fn test_read_gitmodules_no_gitmodules_file() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        let entries = ops
            .read_gitmodules()
            .expect("read_gitmodules should return Ok");
        assert_eq!(
            entries.submodule_iter().count(),
            0,
            "fresh repo has no submodule entries"
        );
    }

    #[test]
    fn test_list_submodules_empty() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        let subs = ops.list_submodules().expect("list_submodules");
        assert!(subs.is_empty(), "fresh repo has no submodules");
    }

    #[test]
    fn test_read_git_config_local() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        let result = ops.read_git_config(ConfigLevel::Local);
        assert!(result.is_ok(), "local config read should succeed");
    }

    #[test]
    fn test_write_git_config_local() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");

        // gix write_git_config requires 3-part keys (section.subsection.name).
        // 2-part keys produce an empty "name" segment which gix rejects.
        let mut entries = HashMap::new();
        entries.insert(
            "remote.testremote.url".to_string(),
            "https://example.com".to_string(),
        );
        let config = GitConfig { entries };

        let result = ops.write_git_config(&config, ConfigLevel::Local);
        assert!(
            result.is_ok(),
            "writing a 3-part key to local config should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_write_git_config_global_level_fails() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");

        let config = GitConfig {
            entries: HashMap::new(),
        };
        let result = ops.write_git_config(&config, ConfigLevel::Global);
        assert!(
            result.is_err(),
            "gix only supports local config writing; global should fail"
        );
    }

    #[test]
    fn test_write_git_config_two_part_key_fails() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        // gix write_git_config splits on '.' with splitn(3, '.'), mapping
        // "section.name" → section="section", subsection=Some("name"), name="".
        // An empty name is invalid, so 2-part keys are rejected.
        let mut entries = HashMap::new();
        entries.insert("submod.gixkey".to_string(), "gixvalue".to_string());
        let config = GitConfig { entries };
        let result = ops.write_git_config(&config, ConfigLevel::Local);
        assert!(
            result.is_err(),
            "gix write_git_config rejects 2-part keys (name part is empty)"
        );
    }

    #[test]
    fn test_set_config_value_local() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        // set_config_value reads the existing config and then calls write_git_config.
        // The existing local config contains 2-part keys (e.g. user.name) that gix
        // cannot round-trip, so this call will fail.  We call it here to exercise
        // the full code path for coverage; the return value is intentionally ignored.
        let _ =
            ops.set_config_value("remote.gixremote.url", "https://gix.example.com", ConfigLevel::Local);
    }

    #[test]
    fn test_write_gitmodules_creates_file() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");

        ops.write_gitmodules(&one_entry_entries())
            .expect("write_gitmodules should succeed");

        // The .gitmodules file should be created.
        assert!(
            harness.work_dir.join(".gitmodules").exists(),
            ".gitmodules file should be created"
        );
        let content = std::fs::read_to_string(harness.work_dir.join(".gitmodules"))
            .expect("read .gitmodules");
        assert!(
            content.contains("lib/test"),
            ".gitmodules should contain the path we wrote"
        );
    }

    #[test]
    fn test_write_gitmodules_empty_entries() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        // Writing empty entries should succeed and create an empty .gitmodules file.
        ops.write_gitmodules(&SubmoduleEntries::default())
            .expect("write empty gitmodules should succeed");
    }

    // ---- Stubs that always return errors ----------------------------------

    #[test]
    fn test_add_submodule_not_implemented() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        let add_opts = SubmoduleAddOptions {
            name: "stub-sub".to_string(),
            path: std::path::PathBuf::from("lib/stub"),
            url: "https://example.com/repo.git".to_string(),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            shallow: false,
            no_init: false,
        };
        let result = ops.add_submodule(&add_opts);
        assert!(result.is_err(), "gix add_submodule should be not implemented");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("gix add_submodule not implemented"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn test_get_submodule_status_not_implemented() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.get_submodule_status("any").is_err());
    }

    #[test]
    fn test_reset_submodule_not_supported() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        let err = ops.reset_submodule("any", true).unwrap_err().to_string();
        assert!(
            err.contains("not yet supported"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn test_clean_submodule_not_supported() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        let err = ops
            .clean_submodule("any", true, true)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("not yet supported"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn test_stash_submodule_not_supported() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        let err = ops.stash_submodule("any", false).unwrap_err().to_string();
        assert!(
            err.contains("not yet supported"),
            "unexpected message: {err}"
        );
    }

    #[test]
    fn test_enable_sparse_checkout_deferred() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        // Deferred to git2 — always returns an error from gix.
        assert!(ops.enable_sparse_checkout("any").is_err());
    }

    #[test]
    fn test_set_sparse_patterns_deferred() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        assert!(
            ops.set_sparse_patterns("any", &["src/".to_string()])
                .is_err()
        );
    }

    #[test]
    fn test_get_sparse_patterns_deferred() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        assert!(ops.get_sparse_patterns("any").is_err());
    }

    #[test]
    fn test_apply_sparse_checkout_deferred() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        // apply_sparse_checkout calls get_sparse_patterns internally, which
        // also defers to git2 and errors immediately.
        assert!(ops.apply_sparse_checkout("any").is_err());
    }

    // ---- Tests with a real submodule (set up via CLI) --------------------

    #[test]
    fn test_with_submodule_list_and_read_gitmodules() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness
            .create_test_remote("gix_list_sub")
            .expect("remote");
        let remote_url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "gix-list-sub",
                "--path",
                "lib/gixlist",
            ])
            .expect("add submodule");

        let ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");

        let subs = ops.list_submodules().expect("list_submodules");
        assert!(!subs.is_empty(), "should find at least one submodule");

        let entries = ops.read_gitmodules().expect("read_gitmodules");
        assert!(
            entries.submodule_iter().count() > 0,
            "should have entries from .gitmodules"
        );
    }

    #[test]
    fn test_gix_deinit_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        // deinit reads .gitmodules first; if the path isn't there it should error.
        let result = ops.deinit_submodule("nonexistent", true);
        assert!(result.is_err(), "should fail for nonexistent submodule");
    }

    #[test]
    fn test_gix_delete_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = GixOperations::new(Some(&harness.work_dir)).expect("ops");
        let result = ops.delete_submodule("nonexistent");
        assert!(result.is_err(), "should fail for nonexistent submodule");
    }
}

// ============================================================
// GitOpsManager tests
// ============================================================

#[cfg(test)]
mod git_ops_manager_tests {
    use super::*;

    #[test]
    fn test_new_from_valid_path() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir));
        assert!(mgr.is_ok(), "manager creation from valid path should succeed");
    }

    #[test]
    fn test_new_from_invalid_path() {
        let mgr = GitOpsManager::new(Some(std::path::Path::new("/nonexistent_submod_mgr_path")));
        assert!(mgr.is_err(), "should fail for an invalid path");
    }

    #[test]
    fn test_workdir_is_some() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir)).expect("mgr");
        assert!(mgr.workdir().is_some(), "workdir should be present");
    }

    #[test]
    fn test_reopen() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut mgr = GitOpsManager::new(Some(&harness.work_dir)).expect("mgr");
        let result = mgr.reopen();
        assert!(result.is_ok(), "reopen should succeed: {:?}", result.err());
    }

    #[test]
    fn test_read_gitmodules_empty_repo() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir)).expect("mgr");
        let entries = mgr.read_gitmodules().expect("read_gitmodules");
        assert_eq!(entries.submodule_iter().count(), 0);
    }

    #[test]
    fn test_list_submodules_empty_repo() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir)).expect("mgr");
        let subs = mgr.list_submodules().expect("list_submodules");
        assert!(subs.is_empty());
    }

    #[test]
    fn test_read_git_config_local() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir)).expect("mgr");
        let result = mgr.read_git_config(ConfigLevel::Local);
        assert!(result.is_ok(), "should read local config: {:?}", result.err());
    }

    #[test]
    fn test_write_and_read_git_config() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir)).expect("mgr");

        let mut entries = HashMap::new();
        entries.insert("submod.mgrkey".to_string(), "mgrvalue".to_string());
        let config = GitConfig { entries };

        // The manager falls back to git2 for this 2-part key (gix rejects it).
        mgr.write_git_config(&config, ConfigLevel::Local)
            .expect("write_git_config should succeed via git2 fallback");

        // Read back using git2 directly to avoid gix snapshot-caching issues.
        let git2_ops = Git2Operations::new(Some(&harness.work_dir)).expect("git2");
        let read_back = git2_ops
            .read_git_config(ConfigLevel::Local)
            .expect("read_git_config after write");
        assert_eq!(
            read_back.entries.get("submod.mgrkey").map(String::as_str),
            Some("mgrvalue"),
            "git2-written value should be readable via git2"
        );
    }

    #[test]
    fn test_set_config_value() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir)).expect("mgr");
        let result =
            mgr.set_config_value("submod.mgrsetkey", "mgrsetval", ConfigLevel::Local);
        assert!(
            result.is_ok(),
            "set_config_value should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_apply_sparse_checkout_fallback_chain() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir)).expect("mgr");
        // gix → git2 → CLI fallback; will ultimately fail since no submodule/path exists.
        let result = mgr.apply_sparse_checkout("nonexistent_path_xyz");
        assert!(
            result.is_err(),
            "should fail for a nonexistent submodule path"
        );
    }

    #[test]
    fn test_write_gitmodules_via_manager() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut mgr = GitOpsManager::new(Some(&harness.work_dir)).expect("mgr");
        mgr.write_gitmodules(&SubmoduleEntries::default())
            .expect("write empty gitmodules via manager");
    }

    #[test]
    fn test_manager_with_submodule_list() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness.create_test_remote("mgr_sub").expect("remote");
        let remote_url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "mgr-sub",
                "--path",
                "lib/mgrsub",
            ])
            .expect("add submodule");

        let mgr = GitOpsManager::new(Some(&harness.work_dir)).expect("mgr");
        let subs = mgr.list_submodules().expect("list_submodules");
        assert!(!subs.is_empty(), "should find the added submodule");

        let entries = mgr.read_gitmodules().expect("read_gitmodules");
        assert!(entries.submodule_iter().count() > 0);
    }
}

// ============================================================
// SubmoduleStatusFlags and GitConfig unit-style tests
// ============================================================

#[cfg(test)]
mod data_types_tests {
    use super::*;

    #[test]
    fn test_status_flags_empty() {
        let flags = SubmoduleStatusFlags::empty();
        assert!(!flags.contains(SubmoduleStatusFlags::IN_HEAD));
        assert!(!flags.contains(SubmoduleStatusFlags::IN_INDEX));
        assert!(!flags.contains(SubmoduleStatusFlags::IN_CONFIG));
        assert!(!flags.contains(SubmoduleStatusFlags::IN_WD));
        assert!(!flags.contains(SubmoduleStatusFlags::WD_UNINITIALIZED));
    }

    #[test]
    fn test_status_flags_single() {
        let flags = SubmoduleStatusFlags::IN_HEAD;
        assert!(flags.contains(SubmoduleStatusFlags::IN_HEAD));
        assert!(!flags.contains(SubmoduleStatusFlags::IN_INDEX));
    }

    #[test]
    fn test_status_flags_combined() {
        let flags = SubmoduleStatusFlags::IN_HEAD
            | SubmoduleStatusFlags::IN_INDEX
            | SubmoduleStatusFlags::IN_CONFIG
            | SubmoduleStatusFlags::IN_WD;
        assert!(flags.contains(SubmoduleStatusFlags::IN_HEAD));
        assert!(flags.contains(SubmoduleStatusFlags::IN_INDEX));
        assert!(flags.contains(SubmoduleStatusFlags::IN_CONFIG));
        assert!(flags.contains(SubmoduleStatusFlags::IN_WD));
        assert!(!flags.contains(SubmoduleStatusFlags::WD_UNINITIALIZED));
    }

    #[test]
    fn test_status_flags_modification_group() {
        let flags = SubmoduleStatusFlags::WD_MODIFIED
            | SubmoduleStatusFlags::WD_INDEX_MODIFIED
            | SubmoduleStatusFlags::WD_WD_MODIFIED
            | SubmoduleStatusFlags::WD_UNTRACKED
            | SubmoduleStatusFlags::INDEX_ADDED
            | SubmoduleStatusFlags::INDEX_DELETED
            | SubmoduleStatusFlags::INDEX_MODIFIED
            | SubmoduleStatusFlags::WD_ADDED
            | SubmoduleStatusFlags::WD_DELETED;

        assert!(flags.intersects(
            SubmoduleStatusFlags::WD_MODIFIED | SubmoduleStatusFlags::WD_WD_MODIFIED
        ));
        assert!(flags.contains(SubmoduleStatusFlags::INDEX_ADDED));
        assert!(!flags.contains(SubmoduleStatusFlags::IN_HEAD));
    }

    #[test]
    fn test_git_config_construction() {
        let mut entries = HashMap::new();
        entries.insert("section.key1".to_string(), "value1".to_string());
        entries.insert("section.key2".to_string(), "value2".to_string());
        let config = GitConfig { entries };
        assert_eq!(config.entries.len(), 2);
        assert_eq!(
            config.entries.get("section.key1").map(String::as_str),
            Some("value1")
        );
        assert_eq!(
            config.entries.get("section.key2").map(String::as_str),
            Some("value2")
        );
    }

    #[test]
    fn test_git_config_empty() {
        let config = GitConfig {
            entries: HashMap::new(),
        };
        assert!(config.entries.is_empty());
    }
}
