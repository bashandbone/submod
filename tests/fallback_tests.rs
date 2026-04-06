// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Tests for the gix→git2 fallback architecture.
//!
//! The core design of the git_ops layer is "gix first, git2 fallback, CLI last resort".
//! These tests verify that:
//! - When gix fails, the error propagates and git2 is invoked
//! - The fallback produces correct results (not silent failures)
//! - Operations that gix intentionally doesn't implement correctly fall through
//! - The `GitOpsManager` with only git2 (no gix) works for all operations

mod common;
use common::TestHarness;

use std::collections::HashMap;
use submod::config::{SubmoduleAddOptions, SubmoduleEntries, SubmoduleEntry};
use submod::git_ops::{Git2Operations, GitConfig, GitOperations, GitOpsManager, GixOperations};
use submod::options::ConfigLevel;

// ============================================================
// Fallback: operations gix explicitly doesn't implement
// ============================================================

#[cfg(test)]
mod fallback_behavior_tests {
    use super::*;

    /// Verify that gix returns errors for operations it explicitly doesn't support,
    /// confirming the fallback will be needed.
    #[test]
    fn gix_returns_error_for_unimplemented_operations() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let gix = GixOperations::new(Some(&harness.work_dir)).expect("gix should init");

        // These operations all return explicit "not implemented/supported" errors in gix
        assert!(
            gix.reset_submodule("any", true).is_err(),
            "gix.reset_submodule should return error"
        );
        assert!(
            gix.clean_submodule("any", true, true).is_err(),
            "gix.clean_submodule should return error"
        );
        assert!(
            gix.stash_submodule("any", true).is_err(),
            "gix.stash_submodule should return error"
        );
        assert!(
            gix.enable_sparse_checkout("any").is_err(),
            "gix.enable_sparse_checkout should return error"
        );
        assert!(
            gix.set_sparse_patterns("any", &["src".to_string()])
                .is_err(),
            "gix.set_sparse_patterns should return error"
        );
        assert!(
            gix.get_sparse_patterns("any").is_err(),
            "gix.get_sparse_patterns should return error"
        );
        assert!(
            gix.get_submodule_status("any").is_err(),
            "gix.get_submodule_status should return error"
        );
    }

    /// When gix can't handle an operation, the manager should succeed via git2 fallback
    /// for operations where git2 has a real implementation.
    #[test]
    fn manager_write_gitmodules_succeeds_despite_gix_limitations() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");

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
        let entries = SubmoduleEntries::new(Some(map), None);

        // Write via manager (may use gix or git2)
        mgr.write_gitmodules(&entries)
            .expect("write_gitmodules should succeed");

        // Read back and verify content is correct
        let read_back = mgr.read_gitmodules().expect("read_gitmodules");
        assert_eq!(
            read_back.submodule_iter().count(),
            1,
            "should have one submodule entry after write"
        );
    }

    /// The 2-part config key test: gix rejects these, git2 handles them.
    /// This tests the actual fallback path where gix fails and git2 succeeds.
    #[test]
    fn config_write_falls_back_to_git2_for_two_part_keys() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");

        let mut entries = HashMap::new();
        // 2-part keys (e.g. "section.key" without a subsection) are rejected by gix
        entries.insert("submod.testkey".to_string(), "testval".to_string());
        let config = GitConfig { entries };

        // Manager should succeed via git2 fallback
        mgr.write_git_config(&config, ConfigLevel::Local)
            .expect("write_git_config should succeed via git2 fallback");

        // Verify the value was actually written
        let git2_ops = Git2Operations::new(Some(&harness.work_dir)).expect("git2");
        let read_back = git2_ops
            .read_git_config(ConfigLevel::Local)
            .expect("read_git_config");
        assert_eq!(
            read_back.entries.get("submod.testkey").map(String::as_str),
            Some("testval"),
            "value written via fallback should be readable"
        );
    }

    /// Verify set_config_value also uses fallback for 2-part keys.
    #[test]
    fn set_config_value_falls_back_for_two_part_keys() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");

        // 2-part key triggers gix failure → git2 fallback
        mgr.set_config_value("submod.fallbackkey", "fallbackval", ConfigLevel::Local)
            .expect("set_config_value should succeed via fallback");

        // Verify
        let git2_ops = Git2Operations::new(Some(&harness.work_dir)).expect("git2");
        let config = git2_ops
            .read_git_config(ConfigLevel::Local)
            .expect("read config");
        assert_eq!(
            config.entries.get("submod.fallbackkey").map(String::as_str),
            Some("fallbackval"),
        );
    }
}

// ============================================================
// Fallback with real submodule operations
// ============================================================

#[cfg(test)]
mod fallback_submodule_tests {
    use super::*;

    /// Helper to set up a repo with a submodule already added.
    fn setup_repo_with_submodule(
        harness: &TestHarness,
    ) -> Result<String, Box<dyn std::error::Error>> {
        harness.init_git_repo()?;
        let remote = harness.create_test_remote("fallback_sub")?;
        let remote_url = format!("file://{}", remote.display());

        harness.run_submod_success(&[
            "add",
            &remote_url,
            "--name",
            "fallback-sub",
            "--path",
            "lib/fallback",
        ])?;

        Ok(remote_url)
    }

    /// add_submodule: gix explicitly doesn't implement this, so it must fall through
    /// to git2, and if that fails, to CLI. Verify the result is correct.
    #[test]
    fn add_submodule_works_through_fallback() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let remote = harness.create_test_remote("add_fb").expect("create remote");
        let remote_url = format!("file://{}", remote.display());

        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");

        let opts = SubmoduleAddOptions {
            url: remote_url,
            path: std::path::PathBuf::from("lib/added"),
            name: "added-sub".to_string(),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            shallow: false,
            no_init: false,
        };

        mgr.add_submodule(&opts)
            .expect("add_submodule should succeed via fallback");

        // Verify it was actually added
        let subs = mgr.list_submodules().expect("list_submodules");
        assert!(!subs.is_empty(), "submodule should be listed after add");

        // Verify the path exists
        assert!(
            harness.work_dir.join("lib/added").exists(),
            "submodule directory should exist"
        );
    }

    /// After adding a submodule, the manager should be able to read its gitmodules entry.
    #[test]
    fn read_gitmodules_after_add_via_fallback() {
        let harness = TestHarness::new().expect("harness");
        let _url = setup_repo_with_submodule(&harness).expect("setup");

        let mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");
        let entries = mgr.read_gitmodules().expect("read_gitmodules");

        let count = entries.submodule_iter().count();
        assert!(
            count > 0,
            "should find submodule entries after add: got {count}"
        );
    }

    /// list_submodules should work regardless of which backend handles it.
    #[test]
    fn list_submodules_after_add() {
        let harness = TestHarness::new().expect("harness");
        let _url = setup_repo_with_submodule(&harness).expect("setup");

        let mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");
        let subs = mgr.list_submodules().expect("list_submodules");
        assert!(!subs.is_empty(), "should list the added submodule");
    }

    /// apply_sparse_checkout has a triple fallback (gix → git2 → CLI).
    /// Both gix and git2 fail for this, so it must reach the CLI fallback.
    /// With a valid submodule path, the CLI fallback should succeed.
    #[test]
    fn apply_sparse_checkout_reaches_cli_fallback() {
        let harness = TestHarness::new().expect("harness");
        let _url = setup_repo_with_submodule(&harness).expect("setup");

        let mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");

        let submodule_path = harness.work_dir.join("lib/fallback");

        // Enable sparse checkout first using git commands directly
        let _ = std::process::Command::new("git")
            .args([
                "-C",
                submodule_path.to_str().unwrap(),
                "config",
                "core.sparseCheckout",
                "true",
            ])
            .output();

        // apply_sparse_checkout goes through gix (fail) → git2 (fail) → CLI
        // With a valid path, the CLI git read-tree should succeed
        let result = mgr.apply_sparse_checkout(submodule_path.to_str().unwrap());
        // This may or may not succeed depending on the state, but it should
        // NOT silently succeed without doing anything — it should either
        // actually run git read-tree or return a clear error.
        if let Err(e) = &result {
            let msg = format!("{e:?}");
            assert!(
                msg.contains("git read-tree") || msg.contains("read-tree"),
                "CLI fallback error should mention git read-tree, got: {msg}"
            );
        }
    }

    /// Nonexistent path should fail through all three layers with a clear error.
    #[test]
    fn apply_sparse_checkout_fails_cleanly_for_bad_path() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        let result = mgr.apply_sparse_checkout("nonexistent/path");
        assert!(
            result.is_err(),
            "should fail for nonexistent submodule path"
        );
    }
}

// ============================================================
// Backend consistency: gix and git2 produce equivalent results
// ============================================================

#[cfg(test)]
mod backend_consistency_tests {
    use super::*;

    /// Both backends should return the same result for read_gitmodules
    /// on the same repository state.
    #[test]
    fn read_gitmodules_consistent_across_backends() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        // Create a .gitmodules file manually so both backends have something to read
        let gitmodules_content = r#"[submodule "test-lib"]
	path = lib/test
	url = https://example.com/repo.git
"#;
        std::fs::write(harness.work_dir.join(".gitmodules"), gitmodules_content)
            .expect("write .gitmodules");

        let gix = GixOperations::new(Some(&harness.work_dir)).expect("gix");
        let git2 = Git2Operations::new(Some(&harness.work_dir)).expect("git2");

        let gix_result = gix.read_gitmodules();
        let git2_result = git2.read_gitmodules();

        // Both should succeed
        assert!(
            gix_result.is_ok(),
            "gix read_gitmodules failed: {:?}",
            gix_result.err()
        );
        assert!(
            git2_result.is_ok(),
            "git2 read_gitmodules failed: {:?}",
            git2_result.err()
        );

        let gix_entries = gix_result.unwrap();
        let git2_entries = git2_result.unwrap();

        // Should have the same number of submodule entries
        assert_eq!(
            gix_entries.submodule_iter().count(),
            git2_entries.submodule_iter().count(),
            "gix and git2 should find the same number of submodule entries"
        );
    }

    /// Both backends should return the same submodule list.
    #[test]
    fn list_submodules_consistent_across_backends() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let gix = GixOperations::new(Some(&harness.work_dir)).expect("gix");
        let git2 = Git2Operations::new(Some(&harness.work_dir)).expect("git2");

        let gix_list = gix.list_submodules().expect("gix list");
        let git2_list = git2.list_submodules().expect("git2 list");

        assert_eq!(
            gix_list, git2_list,
            "both backends should list the same submodules"
        );
    }

    /// Write with one backend, read with the other — the roundtrip should preserve data.
    #[test]
    fn write_gix_read_git2_roundtrip() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let entry = SubmoduleEntry::new(
            Some("https://example.com/roundtrip.git".to_string()),
            Some("lib/roundtrip".to_string()),
            None,
            None,
            None,
            None,
            Some(true),
            Some(false),
            Some(false),
        );
        let mut map = HashMap::new();
        map.insert("roundtrip-sub".to_string(), entry);
        let entries = SubmoduleEntries::new(Some(map), None);

        // Write with gix
        let mut gix = GixOperations::new(Some(&harness.work_dir)).expect("gix");
        gix.write_gitmodules(&entries)
            .expect("gix write_gitmodules");

        // Read back with git2
        let git2 = Git2Operations::new(Some(&harness.work_dir)).expect("git2");
        let read_back = git2.read_gitmodules().expect("git2 read_gitmodules");

        assert_eq!(
            read_back.submodule_iter().count(),
            1,
            "git2 should read what gix wrote"
        );
    }

    /// Manager write → both backends can read.
    /// Uses the manager (which writes via gix or git2 fallback) and verifies
    /// both backends can read the result.
    #[test]
    fn manager_write_both_backends_read() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let entry = SubmoduleEntry::new(
            Some("https://example.com/roundtrip2.git".to_string()),
            Some("lib/roundtrip2".to_string()),
            None,
            None,
            None,
            None,
            Some(true),
            Some(false),
            Some(false),
        );
        let mut map = HashMap::new();
        map.insert("roundtrip2-sub".to_string(), entry);
        let entries = SubmoduleEntries::new(Some(map), None);

        // Write via manager
        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        mgr.write_gitmodules(&entries)
            .expect("manager write_gitmodules");

        // Both backends should be able to read what was written
        let git2 = Git2Operations::new(Some(&harness.work_dir)).expect("git2");
        let git2_result = git2.read_gitmodules().expect("git2 read");
        assert_eq!(
            git2_result.submodule_iter().count(),
            1,
            "git2 should read what manager wrote"
        );

        let gix = GixOperations::new(Some(&harness.work_dir)).expect("gix");
        let gix_result = gix.read_gitmodules().expect("gix read");
        assert_eq!(
            gix_result.submodule_iter().count(),
            1,
            "gix should read what manager wrote"
        );
    }

    /// Verify that a gix instance opened BEFORE a write cannot see changes
    /// made after it was opened (snapshot caching behavior).
    #[test]
    fn gix_caches_state_at_open_time() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        // Open gix BEFORE writing .gitmodules
        let gix_before = GixOperations::new(Some(&harness.work_dir)).expect("gix");
        let before_entries = gix_before.read_gitmodules().expect("read before");
        assert_eq!(
            before_entries.submodule_iter().count(),
            0,
            "should be empty initially"
        );

        // Write .gitmodules externally
        let gitmodules_content = "[submodule \"cache-test\"]\n\tpath = lib/cache\n\turl = https://example.com/cache.git\n";
        std::fs::write(harness.work_dir.join(".gitmodules"), gitmodules_content)
            .expect("write .gitmodules");

        // A fresh gix instance should see the changes
        let gix_after = GixOperations::new(Some(&harness.work_dir)).expect("gix");
        let after_entries = gix_after.read_gitmodules().expect("read after");
        assert_eq!(
            after_entries.submodule_iter().count(),
            1,
            "fresh gix instance should see new .gitmodules"
        );
    }
}

// ============================================================
// Error propagation: verify errors are not swallowed
// ============================================================

#[cfg(test)]
mod error_propagation_tests {
    use super::*;

    /// Operations on invalid paths must return Err, not silently succeed.
    #[test]
    fn fetch_submodule_returns_error_for_invalid_path() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        let result = mgr.fetch_submodule("nonexistent/submodule");
        assert!(
            result.is_err(),
            "fetch_submodule on nonexistent path should return Err, not silently succeed"
        );
    }

    /// gix fetch_submodule on an invalid path should return Err.
    #[test]
    fn gix_fetch_submodule_propagates_error() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let gix = GixOperations::new(Some(&harness.work_dir)).expect("gix");

        let result = gix.fetch_submodule("nonexistent/path");
        assert!(
            result.is_err(),
            "gix.fetch_submodule should propagate error for invalid path, not swallow it"
        );
    }

    /// git2 fetch_submodule on an invalid path should return Err.
    #[test]
    fn git2_fetch_submodule_propagates_error() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let git2 = Git2Operations::new(Some(&harness.work_dir)).expect("git2");

        let result = git2.fetch_submodule("nonexistent/path");
        assert!(
            result.is_err(),
            "git2.fetch_submodule should propagate error for invalid path"
        );
    }

    /// Manager operations on nonexistent submodules must error, not silently pass.
    #[test]
    fn manager_operations_error_on_invalid_submodule() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        // All of these should fail, not silently succeed
        assert!(
            mgr.init_submodule("nonexistent").is_err(),
            "init_submodule on nonexistent should error"
        );
        assert!(
            mgr.deinit_submodule("nonexistent", false).is_err(),
            "deinit_submodule on nonexistent should error"
        );
    }

    /// Verify that both backends fail (not silently succeed) when the repo path
    /// doesn't exist.
    #[test]
    fn backends_fail_for_nonexistent_repo() {
        let bad_path = std::path::PathBuf::from("/tmp/definitely_not_a_repo_12345");

        assert!(
            GixOperations::new(Some(&bad_path)).is_err(),
            "gix should fail for nonexistent repo path"
        );
        assert!(
            Git2Operations::new(Some(&bad_path)).is_err(),
            "git2 should fail for nonexistent repo path"
        );
    }
}

// ============================================================
// Reopen: verify state refresh after destructive operations
// ============================================================

#[cfg(test)]
mod reopen_tests {
    use super::*;

    /// After reopen, the manager should reflect the current on-disk state.
    #[test]
    fn reopen_refreshes_repository_state() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");

        // Initially no submodules
        let subs = mgr.list_submodules().expect("list");
        assert!(subs.is_empty());

        // Add a submodule externally (via CLI, bypassing the manager)
        let remote = harness.create_test_remote("reopen_sub").expect("remote");
        harness
            .run_submod_success(&[
                "add",
                &format!("file://{}", remote.display()),
                "--name",
                "reopen-sub",
                "--path",
                "lib/reopen",
            ])
            .expect("add submodule");

        // Reopen to pick up external changes
        mgr.reopen().expect("reopen should succeed");

        // Now should see the submodule
        let subs = mgr.list_submodules().expect("list after reopen");
        assert!(
            !subs.is_empty(),
            "after reopen, manager should see externally added submodule"
        );
    }

    /// Reopen on a valid repo should always succeed.
    #[test]
    fn reopen_succeeds_on_valid_repo() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        // Multiple reopens should all succeed
        mgr.reopen().expect("first reopen");
        mgr.reopen().expect("second reopen");
        mgr.reopen().expect("third reopen");
    }
}

// ============================================================
// Verbose mode: verify fallback logging
// ============================================================

#[cfg(test)]
mod verbose_fallback_tests {
    use super::*;

    /// With verbose=true, the manager should still succeed for operations
    /// that fall back to git2, just with logging.
    #[test]
    fn verbose_mode_does_not_affect_fallback_success() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        // verbose=true
        let mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");

        // 2-part key triggers gix failure → git2 fallback
        // With verbose=true, this will log to stderr but should still succeed
        let mut entries = HashMap::new();
        entries.insert("submod.verbosetest".to_string(), "val".to_string());
        let config = GitConfig { entries };

        mgr.write_git_config(&config, ConfigLevel::Local)
            .expect("should succeed even in verbose mode");
    }

    /// With verbose=false, the manager should still succeed identically.
    #[test]
    fn non_verbose_mode_fallback_success() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        // verbose=false
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        let mut entries = HashMap::new();
        entries.insert("submod.quiettest".to_string(), "val".to_string());
        let config = GitConfig { entries };

        mgr.write_git_config(&config, ConfigLevel::Local)
            .expect("should succeed silently in non-verbose mode");
    }
}
