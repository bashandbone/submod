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
    Git2Operations, GitConfig, GitOperations, GitOpsManager, GixOperations, SubmoduleStatusFlags,
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
        let harness = TestHarness::new().expect("harness");
        let non_repo_path = harness.temp_dir.path().join("not-a-repo");
        let ops = Git2Operations::new(Some(&non_repo_path));
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
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        let mut entries = HashMap::new();
        entries.insert("submod.testkey".to_string(), "testvalue123".to_string());
        let config = GitConfig { entries };

        mgr.write_git_config(&config, ConfigLevel::Local)
            .expect("write config should succeed");

        // The retained git2 reader must see the natively written value.
        let git2_ops = Git2Operations::new(Some(&harness.work_dir)).expect("git2");
        let read_back = git2_ops
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
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        mgr.set_config_value("submod.singlekey", "singlevalue", ConfigLevel::Local)
            .expect("set_config_value should succeed");

        assert_eq!(
            harness
                .git_stdout(&["config", "--local", "--get", "submod.singlekey"])
                .trim(),
            "singlevalue",
            "set value must land in real local Git config"
        );
    }

    #[test]
    fn test_write_gitmodules_persists_unregistered_entry() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        // Native writes persist declarations; completion of the registration
        // is the reconciliation layer's job, not the write path's.
        mgr.write_gitmodules(&one_entry_entries())
            .expect("writing an unregistered entry should not error");
        assert!(
            harness.gitmodules_entries().contains("lib/test"),
            "native write must persist lib/test in real .gitmodules state"
        );
    }

    // ---- Error paths (submodule not found) --------------------------------
    // Mutations run natively through the manager; the retained git2 backend
    // keeps only its read methods (status, sparse patterns) below.

    #[test]
    fn test_init_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(ops.init_submodule("nonexistent").is_err());
    }

    #[test]
    fn test_deinit_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(ops.deinit_submodule("nonexistent", true).is_err());
    }

    #[test]
    fn test_update_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(
            ops.update_submodule("nonexistent", &SubmoduleUpdateOptions::default())
                .is_err()
        );
    }

    #[test]
    fn test_delete_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(ops.delete_submodule("nonexistent", false).is_err());
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
        let ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(ops.fetch_submodule("nonexistent").is_err());
    }

    #[test]
    fn test_reset_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(ops.reset_submodule("nonexistent", true).is_err());
    }

    #[test]
    fn test_clean_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(ops.clean_submodule("nonexistent", true, true).is_err());
    }

    #[test]
    fn test_stash_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(ops.stash_submodule("nonexistent", false).is_err());
    }

    #[test]
    fn test_enable_sparse_checkout_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(ops.enable_sparse_checkout("nonexistent").is_err());
    }

    #[test]
    fn test_set_sparse_patterns_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
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

        // The OIDs and status flags are the substance of `get_submodule_status`,
        // yet were previously never asserted (#62 P1). Pin them against the real
        // git state of a freshly added submodule.

        // Index OID: the gitlink recorded in the superproject index.
        let stage = harness.git_stdout(&["ls-files", "--stage", "--", "lib/statussub"]);
        let gitlink_oid = stage
            .split_whitespace()
            .nth(1)
            .expect("ls-files --stage should report the gitlink OID");
        assert_eq!(
            status.index_oid.as_deref(),
            Some(gitlink_oid),
            "index_oid must equal the staged gitlink OID"
        );

        // Workdir OID: the commit the submodule worktree is actually checked out
        // at. For a fresh add it equals the recorded gitlink.
        let workdir_head = harness.git_stdout(&["-C", "lib/statussub", "rev-parse", "HEAD"]);
        assert_eq!(
            status.workdir_oid.as_deref(),
            Some(workdir_head.as_str()),
            "workdir_oid must equal the submodule worktree HEAD"
        );
        assert_eq!(
            status.index_oid, status.workdir_oid,
            "a freshly added submodule's worktree must sit at the recorded gitlink"
        );

        // Status flags: the submodule is staged in the index, recorded in
        // `.gitmodules`, and populated in the working directory.
        assert!(
            status.status_flags.contains(SubmoduleStatusFlags::IN_INDEX),
            "IN_INDEX must be set, got {:?}",
            status.status_flags
        );
        assert!(
            status
                .status_flags
                .contains(SubmoduleStatusFlags::IN_CONFIG),
            "IN_CONFIG must be set, got {:?}",
            status.status_flags
        );
        assert!(
            status.status_flags.contains(SubmoduleStatusFlags::IN_WD),
            "IN_WD must be set, got {:?}",
            status.status_flags
        );
    }

    #[test]
    fn test_with_submodule_enable_sparse_checkout_and_patterns() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness.create_test_remote("g2_sparse_sub").expect("remote");
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

        let ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

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
    fn test_native_metadata_update_preserves_checkout() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness.create_test_remote("g2_write_sub").expect("remote");
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

        let before_head = harness.git_stdout(&["-C", "lib/writesub", "rev-parse", "HEAD"]);
        let before_index = harness.git_stdout(&["ls-files", "--stage", "--", "lib/writesub"]);
        harness
            .run_submod_success(&["change", "write-sub", "--active", "false"])
            .expect("native metadata update");
        assert_eq!(
            harness.git_stdout(&["config", "--get", "submodule.write-sub.active"]),
            "false"
        );
        assert!(!harness.gitmodules_entries().contains(".active"));
        assert!(harness.read_config().unwrap().contains("active = false"));
        assert_eq!(
            harness.git_stdout(&["-C", "lib/writesub", "rev-parse", "HEAD"]),
            before_head
        );
        assert_eq!(
            harness.git_stdout(&["ls-files", "--stage", "--", "lib/writesub"]),
            before_index
        );
    }

    #[test]
    fn test_native_metadata_roundtrip_preserves_unspecified_activation() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness
            .create_test_remote("g2_write_sub_none")
            .expect("remote");
        let remote_url = format!("file://{}", remote.display());

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "write-sub-none",
                "--path",
                "lib/writesubnone",
            ])
            .expect("add submodule");

        harness.git_stdout(&["config", "submodule.write-sub-none.custom", "retained"]);
        let before_head = harness.git_stdout(&["-C", "lib/writesubnone", "rev-parse", "HEAD"]);
        let before_index = harness.git_stdout(&["ls-files", "--stage", "--", "lib/writesubnone"]);
        let before_active =
            harness.git_stdout(&["config", "--get", "submodule.write-sub-none.active"]);
        harness
            .run_submod_success(&["change", "write-sub-none", "--url", &remote_url])
            .expect("native metadata round-trip without activation option");
        assert_eq!(
            harness.git_stdout(&["config", "--get", "submodule.write-sub-none.active"]),
            before_active
        );
        assert_eq!(
            harness.git_stdout(&["config", "--get", "submodule.write-sub-none.custom"]),
            "retained"
        );
        assert!(!harness.gitmodules_entries().contains(".active"));
        assert_eq!(
            harness.git_stdout(&["-C", "lib/writesubnone", "rev-parse", "HEAD"]),
            before_head
        );
        assert_eq!(
            harness.git_stdout(&["ls-files", "--stage", "--", "lib/writesubnone"]),
            before_index
        );
    }
}

// ============================================================
// GixOperations tests
// ============================================================

#[cfg(test)]
mod gix_ops_tests {
    use super::*;

    #[test]
    fn test_new_from_valid_path() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let ops = GixOperations::new(Some(&harness.work_dir));
        assert!(ops.is_ok(), "should open a valid repo");
    }

    #[test]
    fn test_new_from_invalid_path() {
        let harness = TestHarness::new().expect("harness");
        let non_repo_path = harness.temp_dir.path().join("not-a-repo");
        let ops = GixOperations::new(Some(&non_repo_path));
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
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        let mut entries = HashMap::new();
        entries.insert(
            "remote.testremote.url".to_string(),
            "https://example.com".to_string(),
        );
        let config = GitConfig { entries };

        mgr.write_git_config(&config, ConfigLevel::Local)
            .expect("writing a 3-part key to local config should succeed");
        assert_eq!(
            harness
                .git_stdout(&["config", "--local", "--get", "remote.testremote.url"])
                .trim(),
            "https://example.com",
            "written value must land in real local Git config"
        );
    }

    #[test]
    fn test_write_git_config_global_level_fails() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        let config = GitConfig {
            entries: HashMap::new(),
        };
        let result = mgr.write_git_config(&config, ConfigLevel::Global);
        assert!(
            result.is_err(),
            "the manager only supports local config writing; global should fail"
        );
    }

    #[test]
    fn test_write_git_config_two_part_key_succeeds() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        // Native `git config --local` accepts 2-part keys; the old gix
        // restriction no longer applies.
        let mut entries = HashMap::new();
        entries.insert("submod.gixkey".to_string(), "gixvalue".to_string());
        let config = GitConfig { entries };
        mgr.write_git_config(&config, ConfigLevel::Local)
            .expect("native write accepts 2-part keys");
        assert_eq!(
            harness
                .git_stdout(&["config", "--local", "--get", "submod.gixkey"])
                .trim(),
            "gixvalue",
            "written 2-part value must land in real local Git config"
        );
    }

    #[test]
    fn test_write_gitmodules_creates_file() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        mgr.write_gitmodules(&one_entry_entries())
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
    fn test_write_gitmodules_keeps_active_app_only() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        for active in [Some(false), None] {
            let mut entries = one_entry_entries();
            if let Some(mut entry) = entries.get("test-lib").cloned() {
                entry.active = active;
                entries.update_entry("test-lib".to_string(), entry);
            }

            mgr.write_gitmodules(&entries)
                .expect("write_gitmodules should succeed");

            let content = std::fs::read_to_string(harness.work_dir.join(".gitmodules"))
                .expect("read .gitmodules");
            assert!(
                !content.contains("active"),
                "active stays app-only and must not leak into .gitmodules (active={active:?})"
            );
            assert!(
                content.contains("lib/test"),
                ".gitmodules should still contain the path we wrote"
            );
        }
    }

    // ---- Tests with a real submodule (set up via CLI) --------------------

    #[test]
    fn test_with_submodule_list_and_read_gitmodules() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let remote = harness.create_test_remote("gix_list_sub").expect("remote");
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
        let mut ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        // deinit reads .gitmodules first; if the path isn't there it should error.
        let result = ops.deinit_submodule("nonexistent", true);
        assert!(result.is_err(), "should fail for nonexistent submodule");
    }

    #[test]
    fn test_gix_delete_submodule_not_found() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut ops = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        let result = ops.delete_submodule("nonexistent", false);
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
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false);
        assert!(
            mgr.is_ok(),
            "manager creation from valid path should succeed"
        );
    }

    #[test]
    fn test_new_from_invalid_path() {
        let harness = TestHarness::new().expect("harness");
        let invalid_path = harness.temp_dir.path().join("not-a-repo");
        let mgr = GitOpsManager::new(Some(&invalid_path), false);
        assert!(mgr.is_err(), "should fail for an invalid path");
    }

    #[test]
    fn test_workdir_is_some() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        assert!(mgr.workdir().is_some(), "workdir should be present");
    }

    #[test]
    fn test_reopen() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        let result = mgr.reopen();
        assert!(result.is_ok(), "reopen should succeed: {:?}", result.err());
    }

    #[test]
    fn test_read_gitmodules_empty_repo() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        let entries = mgr.read_gitmodules().expect("read_gitmodules");
        assert_eq!(entries.submodule_iter().count(), 0);
    }

    #[test]
    fn test_list_submodules_empty_repo() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        let subs = mgr.list_submodules().expect("list_submodules");
        assert!(subs.is_empty());
    }

    #[test]
    fn test_read_git_config_local() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        let result = mgr.read_git_config(ConfigLevel::Local);
        assert!(
            result.is_ok(),
            "should read local config: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_write_and_read_git_config() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");

        let mut entries = HashMap::new();
        entries.insert("submod.mgrkey".to_string(), "mgrvalue".to_string());
        let config = GitConfig { entries };

        // The manager writes natively; 2-part keys are accepted.
        mgr.write_git_config(&config, ConfigLevel::Local)
            .expect("write_git_config should succeed");

        // Read back through the retained git2 reader.
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
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        let result = mgr.set_config_value("submod.mgrsetkey", "mgrsetval", ConfigLevel::Local);
        assert!(
            result.is_ok(),
            "set_config_value should succeed: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_apply_sparse_checkout_rejects_unknown_path() {
        let harness = TestHarness::new().expect("harness");
        harness.init_git_repo().expect("init repo");
        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
        // Native sparse application; fails since no submodule/path exists.
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
        let mut mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
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

        let mgr = GitOpsManager::new(Some(&harness.work_dir), false).expect("mgr");
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

        assert!(
            flags.intersects(
                SubmoduleStatusFlags::WD_MODIFIED | SubmoduleStatusFlags::WD_WD_MODIFIED
            )
        );
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

#[test]
fn regression_r04_delete_preserves_prefix_sibling_index_entries() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("prefix").unwrap();
    std::fs::write(h.work_dir.join("library.txt"), b"sibling").unwrap();
    std::fs::create_dir(h.work_dir.join("lib-extra")).unwrap();
    std::fs::write(h.work_dir.join("lib-extra/file"), b"other sibling").unwrap();
    h.git_stdout(&["add", "library.txt", "lib-extra/file"]);
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "lib",
        remote.to_str().unwrap(),
        "lib",
    ]);
    h.create_config(&format!(
        "[m]\npath = \"lib\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let before = h.git_stdout(&["ls-files", "--stage", "--", "library.txt", "lib-extra/file"]);
    assert_eq!(h.index_gitlink_mode("lib").as_deref(), Some("160000"));
    h.run_submod_success(&["delete", "m"]).unwrap();
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "library.txt", "lib-extra/file"]),
        before
    );
    assert_eq!(
        std::fs::read(h.work_dir.join("library.txt")).unwrap(),
        b"sibling"
    );
    assert_eq!(
        std::fs::read(h.work_dir.join("lib-extra/file")).unwrap(),
        b"other sibling"
    );
    assert_eq!(h.index_gitlink_mode("lib"), None);
}

#[test]
fn regression_r04_delete_refuses_held_index_lock() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("locked").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "m",
        remote.to_str().unwrap(),
        "lib",
    ]);
    h.create_config(&format!(
        "[m]\npath = \"lib\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let before = h.preservation_snapshot();
    let contents = std::fs::read(h.work_dir.join("lib/LICENSE")).unwrap();
    std::fs::write(h.work_dir.join(".git/index.lock"), "held by test\n").unwrap();
    let output = h.run_submod(&["delete", "m"]).unwrap();
    assert_eq!(
        h.preservation_snapshot(),
        before,
        "locked index must prevent all mutation: {output:?}"
    );
    assert_eq!(
        std::fs::read(h.work_dir.join("lib/LICENSE")).ok(),
        Some(contents)
    );
    assert!(!output.status.success(), "{output:?}");
}

#[test]
fn regression_r16_duplicate_paths_rejected_before_init() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("duplicate").unwrap();
    h.create_config(&format!(
        "[alias]\npath = \"lib\"\nurl = {0:?}\n[other]\npath = \"lib\"\nurl = {0:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let before = h.preservation_snapshot();
    let output = h.run_submod(&["init"]).unwrap();
    assert_eq!(
        h.preservation_snapshot(),
        before,
        "duplicate paths mutated state: {output:?}"
    );
    assert!(!h.work_dir.join("lib").exists());
    assert!(!output.status.success(), "{output:?}");
}

#[test]
fn test_fixture_path_display_preserves_drive_and_relative_paths() {
    for spelling in [
        "C:/fixtures/remote.git",
        "relative/remote.git",
        "/tmp/remote.git",
    ] {
        assert_eq!(
            common::TestPath(std::path::PathBuf::from(spelling))
                .display()
                .to_string(),
            spelling
        );
    }
}

#[test]
fn phase2_r16_overlapping_paths_rejected_before_init() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("overlap").unwrap();
    h.create_config(&format!(
        "[parent]\npath = \"lib\"\nurl = {0:?}\n[child]\npath = \"lib/nested\"\nurl = {0:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let before = h.preservation_snapshot();
    let output = h.run_submod(&["init"]).unwrap();
    assert_eq!(h.preservation_snapshot(), before, "{output:?}");
    assert!(!h.work_dir.join("lib").exists(), "{output:?}");
    assert!(!output.status.success(), "{output:?}");
}

#[test]
fn phase2_r16_formatted_logical_name_reused_by_exact_path() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("logical").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "logical.name",
        remote.to_str().unwrap(),
        "vendor/child",
    ]);
    h.create_config(&format!(
        "[alias]\npath = \"vendor/child\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    // Git accepts quoted values and compact assignment; substring matching does not.
    std::fs::write(h.work_dir.join(".gitmodules"), format!("# preserve this comment\n[submodule \"logical.name\"]\n\tpath=\"vendor/child\"\n\turl={:?}\n", remote.to_str().unwrap())).unwrap();
    h.git_stdout(&["add", ".gitmodules", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "registered logical identity"]);
    let child = h.work_dir.join("vendor/child");
    let head = h.git_at(&child, &["rev-parse", "HEAD"]);
    let gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
    let index = h.git_stdout(&["ls-files", "--stage"]);
    let output = h.run_submod(&["init"]).unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get-regexp",
            r"^submodule\..*\.path$"
        ]),
        "submodule.logical.name.path vendor/child"
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        h.git_at(&child, &["rev-parse", "--absolute-git-dir"]),
        gitdir
    );
    assert_eq!(h.git_stdout(&["ls-files", "--stage"]), index);
    assert!(!h.work_dir.join("alias").exists());
}

#[test]
fn phase2_r16_prefix_registration_does_not_match_new_path() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("prefix-match").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "existing",
        remote.to_str().unwrap(),
        "library",
    ]);
    h.create_config(&format!(
        "[alias]\npath = \"lib\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let existing_head = h.git_at(&h.work_dir.join("library"), &["rev-parse", "HEAD"]);
    let existing_index = h.git_stdout(&["ls-files", "--stage", "--", "library"]);
    let output = h.run_submod(&["init"]).unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get",
            "submodule.alias.path"
        ]),
        "lib"
    );
    assert_eq!(h.index_gitlink_mode("lib").as_deref(), Some("160000"));
    assert!(h.work_dir.join("lib/LICENSE").is_file());
    assert_eq!(
        h.git_at(&h.work_dir.join("library"), &["rev-parse", "HEAD"]),
        existing_head
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "library"]),
        existing_index
    );
}

#[test]
fn phase2_context_nested_cwd_add_uses_repository_root() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h.create_config("[defaults]\n").unwrap();
    let remote = h.create_test_remote("nested-context").unwrap();
    let nested = h.work_dir.join("nested/deeper");
    std::fs::create_dir_all(&nested).unwrap();
    let refs = h.git_stdout(&["show-ref"]);
    let root_index = h.git_stdout(&["ls-files", "--stage", "--", "README.md"]);
    let output = h
        .run_submod_at(
            &nested,
            &[
                "add",
                remote.to_str().unwrap(),
                "--name",
                "alias",
                "--path",
                "vendor/child",
            ],
        )
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        h.index_gitlink_mode("vendor/child").as_deref(),
        Some("160000")
    );
    assert!(h.work_dir.join("vendor/child/LICENSE").is_file());
    assert!(h.read_config().unwrap().contains("[alias]"));
    assert!(!nested.join("submod.toml").exists());
    assert!(!nested.join("vendor").exists());
    assert_eq!(h.git_stdout(&["show-ref"]), refs);
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "README.md"]),
        root_index
    );
}

#[test]
fn phase2_context_linked_worktree_mutates_only_selected_checkout() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h.create_config("[defaults]\n").unwrap();
    h.git_stdout(&["add", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "shared initial configuration"]);
    let remote = h.create_test_remote("linked-context").unwrap();
    let linked = h.temp_dir.path().join("linked");
    h.git_stdout(&[
        "worktree",
        "add",
        "-b",
        "linked-branch",
        linked.to_str().unwrap(),
    ]);
    let main_index = h.git_stdout(&["ls-files", "--stage"]);
    let main_config = std::fs::read(h.config_path()).unwrap();
    let refs = h.git_stdout(&["show-ref"]);
    let nested = linked.join("nested/deeper");
    std::fs::create_dir_all(&nested).unwrap();
    let output = h
        .run_submod_at(
            &nested,
            &[
                "add",
                remote.to_str().unwrap(),
                "--name",
                "logical",
                "--path",
                "vendor/child",
            ],
        )
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        h.git_at(
            &linked,
            &[
                "config",
                "--file",
                ".gitmodules",
                "--get",
                "submodule.logical.path"
            ]
        ),
        "vendor/child"
    );
    assert!(
        h.git_at(&linked, &["ls-files", "--stage", "--", "vendor/child"])
            .starts_with("160000 ")
    );
    assert!(linked.join("vendor/child/LICENSE").is_file());
    assert!(
        std::fs::read_to_string(linked.join("submod.toml"))
            .unwrap()
            .contains("[logical]")
    );
    let child_gitdir = h.git_at(
        &linked.join("vendor/child"),
        &["rev-parse", "--absolute-git-dir"],
    );
    let expected_gitdir = h.git_at(
        &linked,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-path",
            "modules/logical",
        ],
    );
    assert_eq!(
        std::fs::canonicalize(child_gitdir).unwrap(),
        std::fs::canonicalize(expected_gitdir).unwrap()
    );
    assert!(!h.work_dir.join("vendor").exists());
    assert!(!nested.join("vendor").exists());
    assert!(!nested.join("submod.toml").exists());
    assert_eq!(h.git_stdout(&["ls-files", "--stage"]), main_index);
    assert_eq!(std::fs::read(h.config_path()).unwrap(), main_config);
    assert_eq!(h.git_stdout(&["show-ref"]), refs);
}

#[test]
fn phase2_r16_duplicate_git_registrations_rejected_before_mutation() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("ambiguous-git").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "logical",
        remote.to_str().unwrap(),
        "child",
    ]);
    h.git_stdout(&[
        "config",
        "--file",
        ".gitmodules",
        "submodule.other.path",
        "child",
    ]);
    h.git_stdout(&[
        "config",
        "--file",
        ".gitmodules",
        "submodule.other.url",
        remote.to_str().unwrap(),
    ]);
    h.git_stdout(&["add", ".gitmodules"]);
    h.create_config(&format!(
        "[alias]\npath = \"child\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let before = h.preservation_snapshot();
    let child = h.work_dir.join("child");
    let refs = h.git_at(&child, &["show-ref"]);
    let contents = std::fs::read(child.join("LICENSE")).unwrap();
    let output = h.run_submod(&["init"]).unwrap();
    assert_eq!(h.preservation_snapshot(), before, "{output:?}");
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(std::fs::read(child.join("LICENSE")).unwrap(), contents);
    assert!(
        !output.status.success(),
        "ambiguous registration accepted: {output:?}"
    );
}

#[test]
fn phase2_r16_case_colliding_paths_rejected_on_insensitive_filesystem() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    std::fs::write(h.work_dir.join("CaseProbe"), b"probe").unwrap();
    if !h.work_dir.join("caseprobe").exists() {
        return; // Case-sensitive filesystems permit these distinct paths.
    }
    let remote = h.create_test_remote("case-collision").unwrap();
    h.create_config(&format!(
        "[first]\npath = \"Lib\"\nurl = {0:?}\n[second]\npath = \"lib\"\nurl = {0:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let before = h.preservation_snapshot();
    let output = h.run_submod(&["init"]).unwrap();
    assert_eq!(h.preservation_snapshot(), before, "{output:?}");
    assert!(!h.work_dir.join("Lib").exists(), "{output:?}");
    assert!(!output.status.success(), "{output:?}");
}

#[test]
fn phase2_r16_case_normalized_ancestor_overlap_rejected_before_init() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("case-ancestor-overlap").unwrap();
    h.create_config(&format!(
        "[parent]\npath = \"Lib\"\nurl = {0:?}\n[child]\npath = \"lib/nested\"\nurl = {0:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let before = h.preservation_snapshot();
    let output = h.run_submod(&["init"]).unwrap();
    assert_eq!(h.preservation_snapshot(), before, "{output:?}");
    assert!(!h.work_dir.join("Lib").exists(), "{output:?}");
    assert!(!h.work_dir.join("lib").exists(), "{output:?}");
    assert!(!output.status.success(), "{output:?}");
}

#[test]
fn phase2_r16_existing_nested_logical_name_is_accepted_without_mutation() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("nested-logical-name").unwrap();
    h.git_stdout(&["submodule", "add", remote.to_str().unwrap(), "vendor/lib"]);
    h.create_config(&format!(
        "[nickname]\npath = \"vendor/lib\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    h.git_stdout(&["add", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "native nested logical name"]);
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get-regexp",
            r"^submodule\..*\.path$"
        ]),
        "submodule.vendor/lib.path vendor/lib"
    );
    let child = h.work_dir.join("vendor/lib");
    let before = h.preservation_snapshot();
    let head = h.git_at(&child, &["rev-parse", "HEAD"]);
    let refs = h.git_at(&child, &["show-ref"]);
    let index = h.git_at(&child, &["ls-files", "--stage"]);
    let config = h.git_at(&child, &["config", "--local", "--list"]);
    let gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
    let contents = std::fs::read(child.join("LICENSE")).unwrap();
    let output = h.run_submod(&["init"]).unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(h.preservation_snapshot(), before, "{output:?}");
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), head);
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(h.git_at(&child, &["ls-files", "--stage"]), index);
    assert_eq!(h.git_at(&child, &["config", "--local", "--list"]), config);
    assert_eq!(
        h.git_at(&child, &["rev-parse", "--absolute-git-dir"]),
        gitdir
    );
    assert_eq!(std::fs::read(child.join("LICENSE")).unwrap(), contents);
}

#[cfg(unix)] // Windows filenames cannot contain the literal '*' fixture component.
#[test]
fn phase2_r04_delete_literal_glob_preserves_unrelated_submodule() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("literal-pathspec").unwrap();
    for (name, path) in [("logical", "lib*"), ("sibling", "lib-extra")] {
        h.git_stdout(&[
            "--literal-pathspecs",
            "submodule",
            "add",
            "--name",
            name,
            remote.to_str().unwrap(),
            path,
        ]);
    }
    h.git_stdout(&["commit", "-m", "Record literal and sibling gitlinks"]);
    h.create_config(&format!(
        "[logical]\npath = \"lib*\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let exact_stage =
        |path: &str| h.git_stdout(&["--literal-pathspecs", "ls-files", "--stage", "--", path]);
    assert!(exact_stage("lib*").starts_with("160000 "));
    let sibling_stage = exact_stage("lib-extra");
    assert!(sibling_stage.starts_with("160000 "));
    let sibling_section = || {
        h.git_stdout(&[
            "config",
            "-f",
            ".gitmodules",
            "--get-regexp",
            "^submodule[.]sibling[.]",
        ])
    };
    let section = sibling_section();
    let local_config = h.git_stdout(&[
        "config",
        "--local",
        "--get-regexp",
        "^submodule[.]sibling[.]",
    ]);
    let child = h.work_dir.join("lib-extra");
    let head = h.git_at(&child, &["rev-parse", "HEAD"]);
    let refs = h.git_at(&child, &["show-ref"]);
    let child_index = h.git_at(&child, &["ls-files", "--stage"]);
    let gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
    let config_bytes = std::fs::read(std::path::Path::new(&gitdir).join("config")).unwrap();
    let paths = h.git_at(&child, &["ls-files", "-z"]);
    let files: Vec<_> = paths
        .split('\0')
        .filter(|path| !path.is_empty())
        .chain(std::iter::once(".git"))
        .map(|path| (path.to_owned(), std::fs::read(child.join(path)).unwrap()))
        .collect();

    h.run_submod_success(&["delete", "logical"]).unwrap();

    assert_eq!(exact_stage("lib*"), "");
    assert!(!h.work_dir.join("lib*").exists());
    assert_eq!(exact_stage("lib-extra"), sibling_stage);
    assert_eq!(sibling_section(), section);
    assert_eq!(h.gitmodules_entries(), section);
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--local",
            "--get-regexp",
            "^submodule[.]sibling[.]",
        ]),
        local_config
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), head);
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(h.git_at(&child, &["ls-files", "--stage"]), child_index);
    assert_eq!(
        h.git_at(&child, &["rev-parse", "--absolute-git-dir"]),
        gitdir
    );
    assert_eq!(
        std::fs::read(std::path::Path::new(&gitdir).join("config")).unwrap(),
        config_bytes
    );
    for (path, bytes) in files {
        assert_eq!(
            std::fs::read(child.join(&path)).unwrap(),
            bytes,
            "sibling file changed: {path}"
        );
    }
}

#[test]
fn phase2_r01_materialization_refuses_redirected_child_gitfile() {
    for command in [vec!["update"], vec!["init"]] {
        let h = TestHarness::new().unwrap();
        h.init_git_repo().unwrap();
        let other = TestHarness::new().unwrap();
        other.init_git_repo().unwrap();
        let remote = h.create_test_remote("materialization-redirect").unwrap();
        h.git_stdout(&[
            "submodule",
            "add",
            "--name",
            "logical",
            remote.to_str().unwrap(),
            "child",
        ]);
        h.create_config(&format!(
            "[logical]\npath = \"child\"\nurl = {:?}\n",
            remote.to_str().unwrap()
        ))
        .unwrap();
        h.git_stdout(&["add", "submod.toml"]);
        h.git_stdout(&["commit", "-m", "Register intended child"]);
        let child = h.work_dir.join("child");
        let intended_head = h.git_at(&child, &["rev-parse", "HEAD"]);
        let child_paths = h.git_at(&child, &["ls-files", "-z"]);
        let child_files: Vec<_> = child_paths
            .split('\0')
            .filter(|path| !path.is_empty())
            .map(|path| (path.to_owned(), std::fs::read(child.join(path)).unwrap()))
            .collect();
        std::fs::write(
            other.work_dir.join("sentinel"),
            b"unrelated committed bytes\0\xff",
        )
        .unwrap();
        other.git_stdout(&["add", "sentinel"]);
        other.git_stdout(&["commit", "-m", "Distinct unrelated history"]);
        other.git_stdout(&["branch", "unrelated-history"]);
        other.git_stdout(&["config", "core.worktree", other.work_dir.to_str().unwrap()]);
        other.git_stdout(&["config", "submod.sentinel", "unrelated configuration"]);
        let other_head = other.git_stdout(&["rev-parse", "HEAD"]);
        assert_ne!(other_head, intended_head);
        let other_gitdir = other.git_stdout(&["rev-parse", "--absolute-git-dir"]);
        let pointer = format!("gitdir: {other_gitdir}\n");
        std::fs::write(child.join(".git"), pointer.as_bytes()).unwrap();
        let before = h.preservation_snapshot();
        let parent_index = std::fs::read(h.work_dir.join(".git/index")).unwrap();
        let other_before = other.preservation_snapshot();
        let other_bytes: Vec<_> = [
            ".git/HEAD",
            ".git/index",
            ".git/config",
            ".git/refs/heads/unrelated-history",
            "sentinel",
        ]
        .into_iter()
        .map(|path| (path, std::fs::read(other.work_dir.join(path)).unwrap()))
        .collect();

        let output = h.run_submod(&command).unwrap();

        assert!(
            !output.status.success(),
            "{command:?} accepted redirected child: {output:?}"
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            !stderr.contains("unexpected argument"),
            "{command:?}: {stderr}"
        );
        assert!(
            stderr.contains("child") || stderr.contains("worktree") || stderr.contains("gitdir"),
            "missing repository-boundary diagnostic: {stderr}"
        );
        assert_eq!(h.preservation_snapshot(), before, "{command:?}: {output:?}");
        assert_eq!(
            std::fs::read(h.work_dir.join(".git/index")).unwrap(),
            parent_index
        );
        assert_eq!(
            other.preservation_snapshot(),
            other_before,
            "{command:?}: {output:?}"
        );
        assert_eq!(other.git_stdout(&["rev-parse", "HEAD"]), other_head);
        assert_eq!(
            std::fs::read(child.join(".git")).unwrap(),
            pointer.as_bytes()
        );
        for (path, bytes) in &child_files {
            assert_eq!(
                std::fs::read(child.join(path)).unwrap(),
                *bytes,
                "{command:?} changed child {path}"
            );
        }
        for (path, bytes) in other_bytes {
            assert_eq!(
                std::fs::read(other.work_dir.join(path)).unwrap(),
                bytes,
                "{command:?} changed unrelated {path}"
            );
        }
    }
}

#[cfg(unix)] // Windows filenames cannot contain the literal '*' fixture component.
#[test]
fn phase2_r23_move_literal_glob_preserves_unrelated_submodule() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("literal-move-pathspec").unwrap();
    for (name, path) in [("logical", "move*"), ("sibling", "move-extra")] {
        h.git_stdout(&[
            "--literal-pathspecs",
            "submodule",
            "add",
            "--name",
            name,
            remote.to_str().unwrap(),
            path,
        ]);
    }
    h.git_stdout(&["commit", "-m", "Record literal and sibling gitlinks"]);
    h.create_config(&format!(
        "[logical]\npath = \"move*\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let exact_stage =
        |path: &str| h.git_stdout(&["--literal-pathspecs", "ls-files", "--stage", "--", path]);
    let source_stage = exact_stage("move*");
    assert!(source_stage.starts_with("160000 "));
    let source = h.work_dir.join("move*");
    let source_head = h.git_at(&source, &["rev-parse", "HEAD"]);
    let source_refs = h.git_at(&source, &["show-ref"]);
    let source_gitdir = h.git_at(&source, &["rev-parse", "--absolute-git-dir"]);
    let source_paths = h.git_at(&source, &["ls-files", "-z"]);
    let source_files: Vec<_> = source_paths
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(|path| (path.to_owned(), std::fs::read(source.join(path)).unwrap()))
        .collect();
    let sibling_stage = exact_stage("move-extra");
    assert!(sibling_stage.starts_with("160000 "));
    let sibling_section = || {
        h.git_stdout(&[
            "config",
            "-f",
            ".gitmodules",
            "--get-regexp",
            "^submodule[.]sibling[.]",
        ])
    };
    let section = sibling_section();
    let local_config = h.git_stdout(&[
        "config",
        "--local",
        "--get-regexp",
        "^submodule[.]sibling[.]",
    ]);
    let child = h.work_dir.join("move-extra");
    let head = h.git_at(&child, &["rev-parse", "HEAD"]);
    let refs = h.git_at(&child, &["show-ref"]);
    let child_index = h.git_at(&child, &["ls-files", "--stage"]);
    let gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
    let config_bytes = std::fs::read(std::path::Path::new(&gitdir).join("config")).unwrap();
    let paths = h.git_at(&child, &["ls-files", "-z"]);
    let files: Vec<_> = paths
        .split('\0')
        .filter(|path| !path.is_empty())
        .chain(std::iter::once(".git"))
        .map(|path| (path.to_owned(), std::fs::read(child.join(path)).unwrap()))
        .collect();

    h.run_submod_success(&["change", "logical", "--path", "moved"])
        .unwrap();

    assert_eq!(exact_stage("move*"), "");
    assert!(!h.work_dir.join("move*").exists());
    assert_eq!(exact_stage("move-extra"), sibling_stage);
    assert_eq!(sibling_section(), section);
    assert_eq!(
        exact_stage("moved"),
        source_stage.replace("\tmove*", "\tmoved")
    );
    assert_eq!(
        h.git_stdout(&[
            "config",
            "-f",
            ".gitmodules",
            "--get",
            "submodule.logical.path"
        ]),
        "moved"
    );
    let moved = h.work_dir.join("moved");
    assert_eq!(h.git_at(&moved, &["rev-parse", "HEAD"]), source_head);
    assert_eq!(h.git_at(&moved, &["show-ref"]), source_refs);
    assert_eq!(
        h.git_at(&moved, &["rev-parse", "--absolute-git-dir"]),
        source_gitdir
    );
    for (path, bytes) in source_files {
        assert_eq!(
            std::fs::read(moved.join(&path)).unwrap(),
            bytes,
            "moved file changed: {path}"
        );
    }
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--local",
            "--get-regexp",
            "^submodule[.]sibling[.]",
        ]),
        local_config
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), head);
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(h.git_at(&child, &["ls-files", "--stage"]), child_index);
    assert_eq!(
        h.git_at(&child, &["rev-parse", "--absolute-git-dir"]),
        gitdir
    );
    assert_eq!(
        std::fs::read(std::path::Path::new(&gitdir).join("config")).unwrap(),
        config_bytes
    );
    for (path, bytes) in files {
        assert_eq!(
            std::fs::read(child.join(&path)).unwrap(),
            bytes,
            "sibling file changed: {path}"
        );
    }
}

#[cfg(unix)] // Windows filenames cannot contain the literal ':' fixture component.
#[test]
fn phase2_r04_delete_literal_magic_preserves_unrelated_submodule() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("literal-magic-pathspec").unwrap();
    for (name, path) in [("logical", ":(top)magic"), ("sibling", "lib-extra")] {
        h.git_stdout(&[
            "--literal-pathspecs",
            "submodule",
            "add",
            "--name",
            name,
            remote.to_str().unwrap(),
            path,
        ]);
    }
    h.git_stdout(&["commit", "-m", "Record literal and sibling gitlinks"]);
    h.create_config(&format!(
        "[logical]\npath = \":(top)magic\"\nurl = {:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    let exact_stage =
        |path: &str| h.git_stdout(&["--literal-pathspecs", "ls-files", "--stage", "--", path]);
    assert!(exact_stage(":(top)magic").starts_with("160000 "));
    let sibling_stage = exact_stage("lib-extra");
    assert!(sibling_stage.starts_with("160000 "));
    let sibling_section = || {
        h.git_stdout(&[
            "config",
            "-f",
            ".gitmodules",
            "--get-regexp",
            "^submodule[.]sibling[.]",
        ])
    };
    let section = sibling_section();
    let local_config = h.git_stdout(&[
        "config",
        "--local",
        "--get-regexp",
        "^submodule[.]sibling[.]",
    ]);
    let child = h.work_dir.join("lib-extra");
    let head = h.git_at(&child, &["rev-parse", "HEAD"]);
    let refs = h.git_at(&child, &["show-ref"]);
    let child_index = h.git_at(&child, &["ls-files", "--stage"]);
    let gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
    let config_bytes = std::fs::read(std::path::Path::new(&gitdir).join("config")).unwrap();
    let paths = h.git_at(&child, &["ls-files", "-z"]);
    let files: Vec<_> = paths
        .split('\0')
        .filter(|path| !path.is_empty())
        .chain(std::iter::once(".git"))
        .map(|path| (path.to_owned(), std::fs::read(child.join(path)).unwrap()))
        .collect();

    h.run_submod_success(&["delete", "logical"]).unwrap();

    assert_eq!(exact_stage(":(top)magic"), "");
    assert!(!h.work_dir.join(":(top)magic").exists());
    assert_eq!(exact_stage("lib-extra"), sibling_stage);
    assert_eq!(sibling_section(), section);
    assert_eq!(h.gitmodules_entries(), section);
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--local",
            "--get-regexp",
            "^submodule[.]sibling[.]",
        ]),
        local_config
    );
    assert_eq!(h.git_at(&child, &["rev-parse", "HEAD"]), head);
    assert_eq!(h.git_at(&child, &["show-ref"]), refs);
    assert_eq!(h.git_at(&child, &["ls-files", "--stage"]), child_index);
    assert_eq!(
        h.git_at(&child, &["rev-parse", "--absolute-git-dir"]),
        gitdir
    );
    assert_eq!(
        std::fs::read(std::path::Path::new(&gitdir).join("config")).unwrap(),
        config_bytes
    );
    for (path, bytes) in files {
        assert_eq!(
            std::fs::read(child.join(&path)).unwrap(),
            bytes,
            "sibling file changed: {path}"
        );
    }
}
