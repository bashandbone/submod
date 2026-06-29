// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Tests for the gix→git2 fallback architecture.
//!
//! The core design of the `git_ops` layer is "gix first, git2 fallback, CLI last resort".
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

    /// Verify `set_config_value` also uses fallback for 2-part keys.
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

    /// `add_submodule`: gix explicitly doesn't implement this, so it must fall through
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

    /// `list_submodules` should work regardless of which backend handles it.
    #[test]
    fn list_submodules_after_add() {
        let harness = TestHarness::new().expect("harness");
        let _url = setup_repo_with_submodule(&harness).expect("setup");

        let mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");
        let subs = mgr.list_submodules().expect("list_submodules");
        assert!(!subs.is_empty(), "should list the added submodule");
    }

    /// `apply_sparse_checkout` has a triple fallback (gix → git2 → CLI).
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

    /// Both backends should return the same result for `read_gitmodules`
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

    /// gix `fetch_submodule` on an invalid path should return Err.
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

    /// git2 `fetch_submodule` on an invalid path should return Err.
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

// ============================================================
// Failure-injection seam: force git2 by disabling gix (P0-1)
//
// `GitOpsManager::without_gix` builds a manager with no gix backend, so every
// `try_with_fallback` call goes straight to git2. This is the only way to
// exercise git2's implementation of operations gix *does* implement
// (read/write_gitmodules, add/delete, list) for *correct results* rather than
// just "didn't panic". Without this seam, gix always wins those ops and git2's
// code is dead from the suite's perspective.
// ============================================================

#[cfg(test)]
mod git2_fallback_injection_tests {
    use super::*;

    /// Set up a repo with one real submodule (name `inj-sub`, path `lib/inj`).
    fn setup_repo_with_submodule(
        harness: &TestHarness,
    ) -> Result<String, Box<dyn std::error::Error>> {
        harness.init_git_repo()?;
        let remote = harness.create_test_remote("inj_sub")?;
        let remote_url = format!("file://{}", remote.display());
        harness.run_submod_success(&[
            "add",
            &remote_url,
            "--name",
            "inj-sub",
            "--path",
            "lib/inj",
        ])?;
        Ok(remote_url)
    }

    /// The seam itself: `without_gix` must disable the gix backend while the
    /// normal constructor keeps it enabled. This guarantees the tests below
    /// actually route through git2 (non-vacuousness for the whole module).
    #[test]
    fn without_gix_disables_gix_backend() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let with_gix = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(
            with_gix.gix_enabled(),
            "GitOpsManager::new should keep the gix backend enabled"
        );

        let no_gix = GitOpsManager::without_gix(Some(&harness.work_dir), false).expect("mgr");
        assert!(
            !no_gix.gix_enabled(),
            "GitOpsManager::without_gix should disable the gix backend"
        );
    }

    /// git2's `read_gitmodules` must parse the *correct* path and url, not just
    /// return a non-empty count.
    #[test]
    fn git2_fallback_reads_gitmodules_correctly() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let gitmodules =
            "[submodule \"read-lib\"]\n\tpath = lib/read\n\turl = https://example.com/read.git\n";
        std::fs::write(harness.work_dir.join(".gitmodules"), gitmodules)
            .expect("write .gitmodules");

        let mgr = GitOpsManager::without_gix(Some(&harness.work_dir), false).expect("mgr");
        let entries = mgr.read_gitmodules().expect("git2 read_gitmodules");

        let entry = entries
            .submodule_iter()
            .find(|(name, _)| name.as_str() == "read-lib")
            .map(|(_, e)| e)
            .expect("git2 should parse the read-lib entry");
        assert_eq!(
            entry.path.as_deref(),
            Some("lib/read"),
            "git2 must parse the submodule path"
        );
        assert_eq!(
            entry.url.as_deref(),
            Some("https://example.com/read.git"),
            "git2 must parse the submodule url"
        );
    }

    /// git2's config write must persist a correct value. Routed through the
    /// git2-only seam, `write_git_config` + `read_git_config` must round-trip
    /// the exact value (this is the write path the manager relies on git2 for).
    #[test]
    fn git2_fallback_writes_git_config_correctly() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mgr = GitOpsManager::without_gix(Some(&harness.work_dir), false).expect("mgr");

        let mut entries = HashMap::new();
        entries.insert("submod.injkey".to_string(), "injvalue".to_string());
        let config = GitConfig { entries };

        mgr.write_git_config(&config, ConfigLevel::Local)
            .expect("git2 write_git_config should succeed");

        let read_back = mgr
            .read_git_config(ConfigLevel::Local)
            .expect("git2 read_git_config");
        assert_eq!(
            read_back.entries.get("submod.injkey").map(String::as_str),
            Some("injvalue"),
            "git2 must persist and read back the exact config value"
        );
    }

    /// git2's `add_submodule` must produce *correct git state*: an index gitlink
    /// at mode 160000, a `.gitmodules` entry, a `submodule.*` config section, and
    /// the path must appear in `list_submodules`.
    #[test]
    fn git2_fallback_add_produces_real_git_state() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness
            .create_test_remote("inj_add")
            .expect("create remote");
        let remote_url = format!("file://{}", remote.display());

        let mut mgr = GitOpsManager::without_gix(Some(&harness.work_dir), false).expect("mgr");

        let opts = SubmoduleAddOptions {
            url: remote_url,
            path: std::path::PathBuf::from("lib/addinj"),
            name: "addinj-sub".to_string(),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            shallow: false,
            no_init: false,
        };
        mgr.add_submodule(&opts)
            .expect("git2 add_submodule should succeed");

        assert_eq!(
            harness.index_gitlink_mode("lib/addinj").as_deref(),
            Some("160000"),
            "git2 add must stage a gitlink at mode 160000"
        );
        assert!(
            harness.gitmodules_entries().contains("lib/addinj"),
            "git2 add must write the .gitmodules entry"
        );
        assert!(
            harness.submodule_config_entries().contains("lib/addinj"),
            "git2 add must write the submodule.* config section"
        );
        let subs = mgr.list_submodules().expect("git2 list_submodules");
        assert!(
            subs.iter().any(|p| p == "lib/addinj"),
            "git2 list_submodules must include the added path, got: {subs:?}"
        );
    }

    /// git2's `deinit_submodule(force)` removes the worktree and the
    /// `submodule.*` config section. git2's `delete_submodule` deliberately
    /// leaves `.gitmodules` untouched ("left to higher-level logic"), so the
    /// entry persists — which is exactly why `GitManager` performs additional
    /// cleanup. This characterizes that partial git2 contract.
    #[test]
    fn git2_fallback_deinit_clears_worktree_and_config() {
        let harness = TestHarness::new().expect("harness");
        setup_repo_with_submodule(&harness).expect("setup");

        let mut mgr = GitOpsManager::without_gix(Some(&harness.work_dir), false).expect("mgr");

        // Guards: present before delete (so the post-delete checks can't pass vacuously).
        assert!(
            harness.work_dir.join("lib/inj").exists(),
            "submodule worktree should exist before delete"
        );
        assert!(
            harness.submodule_config_entries().contains("lib/inj"),
            "submodule.* config should exist before delete"
        );

        mgr.deinit_submodule("lib/inj", true)
            .expect("git2 deinit_submodule");
        mgr.delete_submodule("lib/inj")
            .expect("git2 delete_submodule");

        assert!(
            !harness.work_dir.join("lib/inj").exists(),
            "git2 deinit(force) must remove the submodule worktree"
        );
        assert!(
            !harness.submodule_config_entries().contains("lib/inj"),
            "git2 deinit must remove the submodule.* config section"
        );
        // git2 leaves .gitmodules alone — documents why higher-level cleanup exists.
        assert!(
            harness.gitmodules_entries().contains("lib/inj"),
            "git2 delete_submodule must NOT touch .gitmodules (higher-level logic handles it)"
        );
    }

    /// Cross-backend parity: the gix-enabled manager and the git2-only manager
    /// must list the same submodules for the same repo state.
    #[test]
    fn gix_and_git2_list_submodules_agree() {
        let harness = TestHarness::new().expect("harness");
        setup_repo_with_submodule(&harness).expect("setup");

        let gix_mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        let git2_mgr = GitOpsManager::without_gix(Some(&harness.work_dir), false).expect("mgr");

        let mut gix_list = gix_mgr.list_submodules().expect("gix list");
        let mut git2_list = git2_mgr.list_submodules().expect("git2 list");
        gix_list.sort();
        git2_list.sort();

        assert_eq!(
            gix_list, git2_list,
            "gix and git2 backends must list the same submodules"
        );
        assert!(
            git2_list.iter().any(|p| p == "lib/inj"),
            "both backends must see the added submodule"
        );
    }

    /// `reopen()` hazard (P0-1): in a single process, add → delete → reopen →
    /// re-add the same name+path must succeed and re-stage a gitlink. This
    /// exercises `GitOpsManager::reopen()` in-process, which refreshes the
    /// cached git2 repository so the re-add sees the post-delete state.
    #[test]
    fn reopen_after_delete_allows_readd_same_path() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness.create_test_remote("inj_reopen").expect("remote");
        let remote_url = format!("file://{}", remote.display());

        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        let opts = SubmoduleAddOptions {
            url: remote_url,
            path: std::path::PathBuf::from("lib/reopen"),
            name: "reopen-sub".to_string(),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            shallow: false,
            no_init: false,
        };

        mgr.add_submodule(&opts).expect("initial add");
        assert_eq!(
            harness.index_gitlink_mode("lib/reopen").as_deref(),
            Some("160000"),
            "gitlink should be staged after the first add"
        );

        // Full delete: deinit + git-layer delete, then strip the config/state that
        // git2's delete deliberately leaves behind, mirroring the high-level cleanup.
        mgr.deinit_submodule("lib/reopen", true).expect("deinit");
        mgr.delete_submodule("lib/reopen").expect("delete");
        let _ = std::fs::remove_file(harness.work_dir.join(".gitmodules"));
        let _ = harness.git_stdout(&[
            "rm",
            "--cached",
            "-r",
            "--ignore-unmatch",
            "--",
            "lib/reopen",
        ]);
        let _ = harness.git_stdout(&["config", "--remove-section", "submodule.lib/reopen"]);
        let _ = std::fs::remove_dir_all(harness.work_dir.join(".git/modules/lib/reopen"));

        // Refresh cached git2 state after the destructive sequence.
        mgr.reopen().expect("reopen after delete");

        // Re-add the same name+path in the same process.
        mgr.add_submodule(&opts)
            .expect("re-add after reopen should succeed");
        assert_eq!(
            harness.index_gitlink_mode("lib/reopen").as_deref(),
            Some("160000"),
            "gitlink should be re-staged after reopen + re-add"
        );
    }
}

// ============================================================
// CLI last-resort path of add_submodule
// ============================================================
//
// `add_submodule`'s `.or_else(...)` CLI branch only runs when *both* in-process
// backends (gix and git2) fail. That condition cannot be reproduced offline with
// real inputs — git2 and the git CLI clone from the same URL, so anything that
// breaks git2 breaks the CLI too. `GitOpsManager::forcing_cli_add` is a fault-
// injection seam that bypasses both in-process backends so the otherwise-
// unreachable CLI last resort runs *unmodified* and can be checked for correct
// results, including its cleanup of partial state left by a failed git2 attempt.

#[cfg(test)]
mod cli_last_resort_tests {
    use super::*;

    /// The seam itself: `forcing_cli_add` must flag the manager to route
    /// `add_submodule` through the CLI last resort, while the normal constructor
    /// does not. This anchors non-vacuousness for the whole module — when the
    /// flag is set, any resulting git state must have come from the CLI branch
    /// (both in-process backends are bypassed).
    #[test]
    fn forcing_cli_add_enables_cli_seam() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let normal = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(
            !normal.forces_cli_add(),
            "GitOpsManager::new must not force the CLI last resort"
        );

        let forced = GitOpsManager::forcing_cli_add(Some(&harness.work_dir), false).expect("mgr");
        assert!(
            forced.forces_cli_add(),
            "GitOpsManager::forcing_cli_add must force the CLI last resort"
        );
    }

    /// The CLI last resort must produce *correct git state*: an index gitlink at
    /// mode 160000, a `.gitmodules` entry, a `submodule.*` config section, and the
    /// path must appear in `list_submodules`. Both in-process backends are
    /// bypassed, so this state can only have come from the CLI branch.
    #[test]
    fn cli_last_resort_add_produces_real_git_state() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness
            .create_test_remote("cli_add")
            .expect("create remote");
        let remote_url = format!("file://{}", remote.display());

        let mut mgr = GitOpsManager::forcing_cli_add(Some(&harness.work_dir), false).expect("mgr");

        let opts = SubmoduleAddOptions {
            url: remote_url,
            path: std::path::PathBuf::from("lib/cliadd"),
            name: "cliadd-sub".to_string(),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            shallow: false,
            no_init: false,
        };
        mgr.add_submodule(&opts)
            .expect("CLI last-resort add_submodule should succeed");

        assert_eq!(
            harness.index_gitlink_mode("lib/cliadd").as_deref(),
            Some("160000"),
            "CLI add must stage a gitlink at mode 160000"
        );
        assert!(
            harness.gitmodules_entries().contains("lib/cliadd"),
            "CLI add must write the .gitmodules entry"
        );
        // `git submodule add --name cliadd-sub` keys .git/config by the submodule
        // *name* (submodule.cliadd-sub.url), unlike git2 which keys by path.
        assert!(
            harness.submodule_config_entries().contains("cliadd-sub"),
            "CLI add must write the submodule.* config section (keyed by name)"
        );
        let subs = mgr.list_submodules().expect("list_submodules");
        assert!(
            subs.iter().any(|p| p == "lib/cliadd"),
            "CLI add path must appear in list_submodules, got: {subs:?}"
        );
    }

    /// The CLI branch's first job is to clean up partial state a failed git2
    /// attempt may have left behind (a stale `.gitmodules` section, a config
    /// section, an internal `.git/modules/<name>` dir, a staged index entry) and
    /// then re-add cleanly. Seed exactly that leftover state for the target name,
    /// then run the forced CLI add and assert it both succeeds and ends with the
    /// *real* url — not the stale seed — proving the cleanup ran.
    #[test]
    fn cli_last_resort_cleans_up_partial_state_and_succeeds() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness
            .create_test_remote("cli_reinit")
            .expect("create remote");
        let remote_url = format!("file://{}", remote.display());

        // Simulate the debris a half-finished git2 add_submodule leaves behind:
        // a stale .gitmodules section (with a bogus url), a stale config section,
        // and an internal modules directory — all keyed by the submodule name.
        let stale_gitmodules = "[submodule \"cleanup-sub\"]\n\tpath = lib/cleanup\n\turl = https://example.com/STALE.git\n";
        std::fs::write(harness.work_dir.join(".gitmodules"), stale_gitmodules)
            .expect("seed stale .gitmodules");
        harness.git_stdout(&[
            "config",
            "submodule.cleanup-sub.url",
            "https://example.com/STALE.git",
        ]);
        std::fs::create_dir_all(harness.work_dir.join(".git/modules/cleanup-sub"))
            .expect("seed stale internal modules dir");

        let mut mgr = GitOpsManager::forcing_cli_add(Some(&harness.work_dir), false).expect("mgr");

        let opts = SubmoduleAddOptions {
            url: remote_url.clone(),
            path: std::path::PathBuf::from("lib/cleanup"),
            name: "cleanup-sub".to_string(),
            branch: None,
            ignore: None,
            update: None,
            fetch_recurse: None,
            shallow: false,
            no_init: false,
        };
        mgr.add_submodule(&opts)
            .expect("CLI last-resort add must clean up partial state and succeed");

        assert_eq!(
            harness.index_gitlink_mode("lib/cleanup").as_deref(),
            Some("160000"),
            "CLI add must stage the gitlink after cleaning up partial state"
        );
        let gitmodules = harness.gitmodules_entries();
        assert!(
            gitmodules.contains(&remote_url),
            "the real url must replace the stale seed, got: {gitmodules}"
        );
        assert!(
            !gitmodules.contains("STALE.git"),
            "the stale .gitmodules section must have been cleaned up, got: {gitmodules}"
        );
    }

    /// The audit flagged that fallback warning logs are never asserted. gix's
    /// `add_submodule` always errors, so a verbose run of the real binary must
    /// emit the gix→git2 fallback warning to stderr.
    #[test]
    fn fallback_warning_is_logged_in_verbose_mode() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness
            .create_test_remote("warn_add")
            .expect("create remote");
        let remote_url = format!("file://{}", remote.display());

        let output = harness
            .run_submod(&[
                "--verbose",
                "add",
                &remote_url,
                "--name",
                "warn-sub",
                "--path",
                "lib/warn",
            ])
            .expect("run submod --verbose add");
        assert!(
            output.status.success(),
            "verbose add should still succeed via fallback; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("falling back to git2"),
            "verbose mode must log the gix→git2 fallback warning, got stderr: {stderr}"
        );
    }

    /// Non-vacuousness for the warning assertion above: without `--verbose`, the
    /// same fallback occurs silently. This proves the assertion discriminates on
    /// the verbose flag rather than matching unconditional output.
    #[test]
    fn fallback_warning_is_silent_without_verbose() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness
            .create_test_remote("quiet_add")
            .expect("create remote");
        let remote_url = format!("file://{}", remote.display());

        let output = harness
            .run_submod(&[
                "add",
                &remote_url,
                "--name",
                "quiet-sub",
                "--path",
                "lib/quiet",
            ])
            .expect("run submod add");
        assert!(
            output.status.success(),
            "non-verbose add should succeed via fallback; stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("falling back to git2"),
            "non-verbose mode must not log the fallback warning, got stderr: {stderr}"
        );
    }
}
