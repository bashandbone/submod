// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Read-backend compatibility and native CLI mutation state tests.

mod common;
use common::TestHarness;

use std::collections::HashMap;
use submod::config::{SubmoduleEntries, SubmoduleEntry};
use submod::git_ops::{Git2Operations, GitConfig, GitOperations, GitOpsManager, GixOperations};
use submod::options::ConfigLevel;

// ============================================================
// Fallback: operations gix explicitly doesn't implement
// ============================================================

#[cfg(test)]
mod fallback_behavior_tests {
    use super::*;

    /// The gix backend is read-only: it serves the read contract the manager's
    /// inspection fallback relies on (gitmodules, config, list). Mutations go
    /// through the manager's native Git path instead.
    #[test]
    fn gix_serves_manager_read_contract() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let gix = GixOperations::new(Some(&harness.work_dir)).expect("gix should init");

        let entries = gix.read_gitmodules().expect("gix read_gitmodules");
        assert_eq!(
            entries.submodule_iter().count(),
            0,
            "fresh repo has no submodule entries"
        );
        let subs = gix.list_submodules().expect("gix list_submodules");
        assert!(subs.is_empty(), "fresh repo lists no submodules");
        let config = gix
            .read_git_config(ConfigLevel::Local)
            .expect("gix read_git_config");
        assert!(
            !config.entries.is_empty(),
            "init_git_repo seeds local config entries"
        );
    }

    /// The manager writes natively; the write must be visible through the
    /// retained backend read APIs and real Git state.
    #[test]
    fn manager_write_gitmodules_visible_to_backend_readers() {
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

    /// Native config writes accept 2-part keys; the value must be visible
    /// through the retained git2 read API and real Git state.
    #[test]
    fn config_write_accepts_two_part_keys() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");

        let mut entries = HashMap::new();
        // 2-part keys (e.g. "section.key" without a subsection) are rejected by gix
        entries.insert("submod.testkey".to_string(), "testval".to_string());
        let config = GitConfig { entries };

        // Manager writes natively; the value must land in the local config
        mgr.write_git_config(&config, ConfigLevel::Local)
            .expect("write_git_config should succeed");

        // Verify the value was actually written
        let git2_ops = Git2Operations::new(Some(&harness.work_dir)).expect("git2");
        let read_back = git2_ops
            .read_git_config(ConfigLevel::Local)
            .expect("read_git_config");
        assert_eq!(
            read_back.entries.get("submod.testkey").map(String::as_str),
            Some("testval"),
            "natively written value should be readable"
        );
    }

    /// `set_config_value` persists 2-part keys through the native write path.
    #[test]
    fn set_config_value_persists_two_part_keys() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");

        // 2-part keys are written natively and readable via the git2 reader
        mgr.set_config_value("submod.fallbackkey", "fallbackval", ConfigLevel::Local)
            .expect("set_config_value should succeed");

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

    /// Native CLI additions remain visible through the manager read API.
    #[test]
    fn native_add_is_visible_to_manager() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let remote = harness.create_test_remote("add_fb").expect("create remote");
        let remote_url = format!("file://{}", remote.display());

        let mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "added-sub",
                "--path",
                "lib/added",
            ])
            .expect("native add");

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

    /// Native manager write must land in real Git state: the `.gitmodules`
    /// file on disk must contain the written path and url.
    #[test]
    fn native_write_visible_in_git_state_and_git2_reader() {
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

        // Write via the native manager path
        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        mgr.write_gitmodules(&entries)
            .expect("manager write_gitmodules");

        // Real Git state must contain the written registration
        assert!(
            harness.gitmodules_entries().contains("lib/roundtrip"),
            "native write must persist lib/roundtrip in .gitmodules"
        );

        // Read back with the retained git2 reader
        let git2 = Git2Operations::new(Some(&harness.work_dir)).expect("git2");
        let read_back = git2.read_gitmodules().expect("git2 read_gitmodules");

        assert_eq!(
            read_back.submodule_iter().count(),
            1,
            "git2 should read what the manager wrote"
        );
    }

    /// Manager write → both backends can read.
    /// Uses the manager (which writes natively) and verifies both retained
    /// backend readers can read the result.
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

    /// Reopen preserves the backend policy: a gix-less manager must stay
    /// gix-less, so git2-only test routing cannot silently gain a gix backend.
    #[test]
    fn reopen_preserves_without_gix_policy() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let mut mgr = GitOpsManager::without_gix(Some(&harness.work_dir), false).expect("mgr");
        assert!(!mgr.gix_enabled(), "without_gix starts gix-less");

        mgr.reopen().expect("reopen should succeed");
        assert!(
            !mgr.gix_enabled(),
            "reopen must not enable gix on a without_gix manager"
        );

        // The git2 read path still works after the refresh.
        let subs = mgr.list_submodules().expect("list after reopen");
        assert!(subs.is_empty(), "fresh repo lists no submodules");
    }
}

// ============================================================
// Verbose mode: flag affects logging only, never operation outcome
// ============================================================

#[cfg(test)]
mod verbose_fallback_tests {
    use super::*;

    /// With verbose=true, native writes still succeed, just with logging.
    #[test]
    fn verbose_mode_does_not_affect_operation_success() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        // verbose=true
        let mgr = GitOpsManager::new(Some(&harness.work_dir), true).expect("mgr");

        // Native write path; verbose mode only affects logging, not success
        let mut entries = HashMap::new();
        entries.insert("submod.verbosetest".to_string(), "val".to_string());
        let config = GitConfig { entries };

        mgr.write_git_config(&config, ConfigLevel::Local)
            .expect("should succeed even in verbose mode");
    }

    /// With verbose=false, the manager should still succeed identically.
    #[test]
    fn non_verbose_mode_operation_success() {
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
// `try_with_fallback` read goes straight to git2. This exercises git2's
// implementation of the retained reads (git config, detailed status, list)
// for *correct results* rather than just "didn't panic". Without this seam,
// gix always wins those reads and git2's code is dead from the suite's
// perspective. Mutations are native in both constructors.
// ============================================================

#[cfg(test)]
mod git2_read_path_tests {
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
    /// return a non-empty count. Uses the retained git2 reader directly.
    #[test]
    fn git2_reader_parses_gitmodules_fields() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");

        let gitmodules =
            "[submodule \"read-lib\"]\n\tpath = lib/read\n\turl = https://example.com/read.git\n";
        std::fs::write(harness.work_dir.join(".gitmodules"), gitmodules)
            .expect("write .gitmodules");

        let git2 = Git2Operations::new(Some(&harness.work_dir)).expect("git2");
        let entries = git2.read_gitmodules().expect("git2 read_gitmodules");

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

    /// Native config writes must persist a correct value: `write_git_config`
    /// plus the retained git2 `read_git_config` must round-trip the exact value.
    #[test]
    fn native_write_roundtrips_through_git2_reader() {
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

    /// A native addition is registered correctly and visible to the git2 reader.
    #[test]
    fn native_add_is_visible_to_git2_reader() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness
            .create_test_remote("inj_add")
            .expect("create remote");
        let remote_url = format!("file://{}", remote.display());

        let mgr = GitOpsManager::without_gix(Some(&harness.work_dir), false).expect("mgr");

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "addinj-sub",
                "--path",
                "lib/addinj",
            ])
            .expect("native add");

        assert_eq!(
            harness.index_gitlink_mode("lib/addinj").as_deref(),
            Some("160000"),
            "native add must stage a gitlink at mode 160000"
        );
        assert!(
            harness.gitmodules_entries().contains("lib/addinj"),
            "native add must write the .gitmodules entry"
        );
        assert!(
            harness
                .submodule_config_entries()
                .contains("submodule.addinj-sub.url"),
            "native add must write the submodule.* config section"
        );
        let subs = mgr.list_submodules().expect("git2 list_submodules");
        assert!(
            subs.iter().any(|p| p == "lib/addinj"),
            "git2 list_submodules must include the added path, got: {subs:?}"
        );
    }

    #[test]
    fn native_delete_clears_registration_and_retains_history() {
        let harness = TestHarness::new().expect("harness");
        setup_repo_with_submodule(&harness).expect("setup");
        let gitdir = harness.git_stdout(&["-C", "lib/inj", "rev-parse", "--absolute-git-dir"]);
        let oid = harness.git_stdout(&["-C", "lib/inj", "rev-parse", "HEAD"]);
        assert!(
            harness
                .submodule_config_entries()
                .contains("submodule.inj-sub.url")
        );
        harness
            .run_submod_success(&["delete", "inj-sub"])
            .expect("native delete");
        assert!(!harness.work_dir.join("lib/inj").exists());
        assert!(
            !harness
                .submodule_config_entries()
                .contains("submodule.inj-sub.")
        );
        assert!(!harness.gitmodules_entries().contains("lib/inj"));
        assert_eq!(harness.index_gitlink_mode("lib/inj"), None);
        assert_eq!(
            harness
                .git_stdout(&["--git-dir", gitdir.trim(), "cat-file", "-t", oid.trim()])
                .trim(),
            "commit"
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

    #[test]
    fn reopen_after_delete_preserves_retained_history() {
        let harness = TestHarness::new().expect("harness");
        setup_repo_with_submodule(&harness).expect("setup");
        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        let gitdir = harness.git_stdout(&["-C", "lib/inj", "rev-parse", "--absolute-git-dir"]);
        let oid = harness.git_stdout(&["-C", "lib/inj", "rev-parse", "HEAD"]);
        harness
            .run_submod_success(&["delete", "inj-sub"])
            .expect("delete");
        mgr.reopen().expect("reopen after delete");
        assert!(
            !mgr.list_submodules()
                .expect("list")
                .iter()
                .any(|p| p == "lib/inj")
        );
        assert_eq!(harness.index_gitlink_mode("lib/inj"), None);
        assert_eq!(
            harness
                .git_stdout(&["--git-dir", gitdir.trim(), "cat-file", "-t", oid.trim()])
                .trim(),
            "commit"
        );
    }
}

// ============================================================
// Native CLI mutation state and preservation
// ============================================================

#[cfg(test)]
mod native_mutation_tests {
    use super::*;

    /// Native add stages a real gitlink and registers the logical identity.
    #[test]
    fn native_add_produces_real_git_state() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness
            .create_test_remote("cli_add")
            .expect("create remote");
        let remote_url = format!("file://{}", remote.display());

        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "cliadd-sub",
                "--path",
                "lib/cliadd",
            ])
            .expect("native add");

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

    #[test]
    fn native_add_refuses_partial_state_without_cleanup() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness.create_test_remote("partial").expect("remote");
        let remote_url = format!("file://{}", remote.display());
        let stale = "[submodule \"cleanup-sub\"]\n\tpath = lib/cleanup\n\turl = https://example.com/STALE.git\n";
        std::fs::write(harness.work_dir.join(".gitmodules"), stale).expect("seed declaration");
        harness.git_stdout(&[
            "config",
            "submodule.cleanup-sub.url",
            "https://example.com/STALE.git",
        ]);
        let retained = harness.work_dir.join(".git/modules/cleanup-sub");
        std::fs::create_dir_all(&retained).expect("seed partial gitdir");
        std::fs::write(retained.join("sentinel"), b"retain partial bytes").expect("sentinel");
        let before = harness.preservation_snapshot();
        let output = harness
            .run_submod(&[
                "add",
                &remote_url,
                "--name",
                "cleanup-sub",
                "--path",
                "lib/cleanup",
            ])
            .expect("run add");
        assert!(
            !output.status.success(),
            "partial state must require explicit recovery"
        );
        assert_eq!(harness.preservation_snapshot(), before);
        assert_eq!(
            std::fs::read(retained.join("sentinel")).expect("retained sentinel"),
            b"retain partial bytes"
        );
    }
}
