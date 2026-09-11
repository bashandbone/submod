//! Phase 4 CLI acceptance: managed metadata, exact identity, and index ownership.
use std::{fs, path::Path, process::Output};
mod common;
use common::TestHarness;

const NAME: &str = "logical.name";
const CHILD: &str = "vendor/checkout";

fn ok(output: &Output) {
    assert!(output.status.success(), "CLI failed: {output:?}");
}

fn fixture() -> TestHarness {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("source").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        NAME,
        remote.to_str().unwrap(),
        CHILD,
    ]);
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.logical.name.custom",
        "portable-keep",
    ]);
    h.git_stdout(&[
        "config",
        "--local",
        "submodule.logical.name.custom",
        "local-keep",
    ]);
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.unmanaged.path",
        "vendor/checkout-extra",
    ]);
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.unmanaged.url",
        remote.to_str().unwrap(),
    ]);
    h.git_stdout(&[
        "config",
        "--local",
        "submodule.unmanaged.custom",
        "other-keep",
    ]);
    h.create_config(&format!(
        "[alias]\nurl = {:?}\npath = {CHILD:?}\n",
        remote.to_str().unwrap()
    ))
    .unwrap();
    h.git_stdout(&["add", ".gitmodules", "submod.toml"]);
    h.git_stdout(&[
        "commit",
        "-m",
        "native registration with distinct identities",
    ]);
    h
}

fn value(h: &TestHarness, cwd: &Path, scope: &[&str], key: &str) -> Option<String> {
    let out = h
        .git_cmd()
        .current_dir(cwd)
        .arg("config")
        .args(scope)
        .args(["--get", key])
        .output()
        .unwrap();
    assert!(
        out.status.success() || out.status.code() == Some(1),
        "{out:?}"
    );
    out.status
        .success()
        .then(|| String::from_utf8(out.stdout).unwrap().trim().to_owned())
}

fn assert_managed(h: &TestHarness, cwd: &Path, expected: &[(&str, Option<&str>)]) {
    for scope in [&["-f", ".gitmodules"][..], &["--local"][..], &[][..]] {
        for (field, expected) in expected {
            let key = format!("submodule.{NAME}.{field}");
            assert_eq!(
                value(h, cwd, scope, &key).as_deref(),
                *expected,
                "scope={scope:?} key={key}"
            );
        }
    }
    assert_eq!(
        value(
            h,
            cwd,
            &["-f", ".gitmodules"],
            "submodule.logical.name.custom"
        )
        .as_deref(),
        Some("portable-keep")
    );
    assert_eq!(
        value(h, cwd, &["--local"], "submodule.logical.name.custom").as_deref(),
        Some("local-keep")
    );
    assert_eq!(
        value(h, cwd, &["--local"], "submodule.unmanaged.custom").as_deref(),
        Some("other-keep")
    );
    assert_eq!(
        value(h, cwd, &["-f", ".gitmodules"], "submodule.unmanaged.path").as_deref(),
        Some("vendor/checkout-extra")
    );
    assert!(value(h, cwd, &["-f", ".gitmodules"], "submodule.alias.path").is_none());
    assert!(
        value(
            h,
            cwd,
            &["-f", ".gitmodules"],
            "submodule.vendor/checkout.path"
        )
        .is_none()
    );
}

fn child_state(h: &TestHarness, cwd: &Path) -> (String, String, Option<Vec<u8>>) {
    let child = cwd.join(CHILD);
    let gitdir = h.git_at(&child, &["rev-parse", "--absolute-git-dir"]);
    (
        h.git_at(&child, &["rev-parse", "HEAD"]),
        h.git_at(&child, &["show-ref"]),
        fs::read(Path::new(&gitdir).join("FETCH_HEAD")).ok(),
    )
}

#[test]
fn r14_r16_change_sets_exact_managed_identity_without_checkout_or_staging() {
    let h = fixture();
    let state = child_state(&h, &h.work_dir);
    let index = h.git_stdout(&["ls-files", "--stage"]);
    let new_url = h.temp_dir.path().join("unreachable-source");
    ok(&h
        .run_submod(&[
            "change",
            "alias",
            "--url",
            new_url.to_str().unwrap(),
            "--branch",
            "topic",
            "--ignore",
            "dirty",
            "--update",
            "merge",
            "--fetch",
            "on-demand",
            "--shallow",
            "true",
        ])
        .unwrap());
    assert_eq!(
        child_state(&h, &h.work_dir),
        state,
        "metadata-only change fetched or moved child"
    );
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage"]),
        index,
        "policy edit staged files"
    );
    assert_managed(
        &h,
        &h.work_dir,
        &[
            ("url", Some(new_url.to_str().unwrap())),
            ("branch", Some("topic")),
            ("ignore", Some("dirty")),
            ("update", Some("merge")),
            ("fetchRecurseSubmodules", Some("on-demand")),
            ("shallow", Some("true")),
        ],
    );
    assert_eq!(
        value(&h, &h.work_dir.join(CHILD), &[], "remote.origin.url").as_deref(),
        Some(new_url.to_str().unwrap())
    );
    let before = h.preservation_snapshot();
    ok(&h
        .run_submod(&[
            "change",
            "alias",
            "--url",
            new_url.to_str().unwrap(),
            "--branch",
            "topic",
            "--ignore",
            "dirty",
            "--update",
            "merge",
            "--fetch",
            "on-demand",
            "--shallow",
            "true",
        ])
        .unwrap());
    assert_eq!(
        h.preservation_snapshot(),
        before,
        "identical change rewrote metadata"
    );
    assert_eq!(child_state(&h, &h.work_dir), state);
}

#[test]
fn r14_unset_removes_stale_portable_and_local_overrides() {
    let h = fixture();
    h.git_stdout(&["config", "extensions.worktreeConfig", "true"]);
    h.git_stdout(&[
        "config",
        "--worktree",
        "submodule.logical.name.custom",
        "worktree-keep",
    ]);
    let mut toml = h.read_config().unwrap();
    toml.push_str(
        "branch = 'topic'\nignore = 'dirty'\nupdate = 'merge'\nfetch = 'always'\nshallow = true\n",
    );
    h.create_config(&toml).unwrap();
    for scope in [
        &["-f", ".gitmodules"][..],
        &["--local"][..],
        &["--worktree"][..],
    ] {
        for (field, val) in [
            ("branch", "topic"),
            ("ignore", "dirty"),
            ("update", "merge"),
            ("fetchRecurseSubmodules", "true"),
            ("shallow", "true"),
        ] {
            h.git_stdout(
                &[
                    ["config"].as_slice(),
                    scope,
                    &[&format!("submodule.{NAME}.{field}"), val],
                ]
                .concat(),
            );
        }
    }
    h.git_stdout(&["add", ".gitmodules", "submod.toml"]);
    h.git_stdout(&["commit", "-m", "seed stale managed overrides"]);
    let index = h.git_stdout(&["ls-files", "--stage"]);
    let state = child_state(&h, &h.work_dir);
    ok(&h
        .run_submod(&[
            "change",
            "alias",
            "--unset",
            "branch,ignore,update,fetch,shallow",
        ])
        .unwrap());
    assert_managed(&h, &h.work_dir, &[("branch", None)]);
    // Native defaults may be represented by absence or their explicit value.
    for scope in [&["-f", ".gitmodules"][..], &["--local"][..], &[][..]] {
        for (field, default) in [
            ("ignore", "none"),
            ("update", "checkout"),
            ("fetchRecurseSubmodules", "on-demand"),
            ("shallow", "false"),
        ] {
            let actual = value(&h, &h.work_dir, scope, &format!("submodule.{NAME}.{field}"));
            assert!(
                actual.is_none() || actual.as_deref() == Some(default),
                "stale {field}: {actual:?}"
            );
        }
    }
    for field in [
        "branch",
        "ignore",
        "update",
        "fetchRecurseSubmodules",
        "shallow",
    ] {
        assert_eq!(
            value(
                &h,
                &h.work_dir,
                &["--worktree"],
                &format!("submodule.{NAME}.{field}")
            ),
            None,
            "unset left a worktree override"
        );
    }
    assert_eq!(h.git_stdout(&["ls-files", "--stage"]), index);
    assert_eq!(
        value(
            &h,
            &h.work_dir,
            &["--worktree"],
            "submodule.logical.name.custom"
        )
        .as_deref(),
        Some("worktree-keep")
    );
    assert_eq!(child_state(&h, &h.work_dir), state);
}

#[test]
fn r14_defaults_reconcile_metadata_without_materializing_missing_checkout() {
    let h = fixture();
    h.git_stdout(&["submodule", "deinit", "-f", "--", CHILD]);
    h.git_stdout(&[
        "config",
        "--local",
        "submodule.logical.name.custom",
        "local-keep",
    ]);
    let index = h.git_stdout(&["ls-files", "--stage"]);
    ok(&h
        .run_submod(&[
            "change-global",
            "--ignore",
            "all",
            "--fetch",
            "never",
            "--update",
            "none",
        ])
        .unwrap());
    assert!(!h.work_dir.join(CHILD).join(".git").exists());
    assert_eq!(h.git_stdout(&["ls-files", "--stage"]), index);
    assert_managed(
        &h,
        &h.work_dir,
        &[
            ("ignore", Some("all")),
            ("fetchRecurseSubmodules", Some("false")),
            ("update", Some("none")),
        ],
    );
}

#[test]
fn r14_worktree_overrides_cannot_mask_managed_changes() {
    let h = fixture();
    h.git_stdout(&["config", "extensions.worktreeConfig", "true"]);
    for (field, val) in [
        ("url", "/missing/old"),
        ("branch", "stale"),
        ("ignore", "all"),
        ("update", "rebase"),
        ("fetchRecurseSubmodules", "true"),
        ("shallow", "true"),
        ("custom", "worktree-keep"),
    ] {
        h.git_stdout(&[
            "config",
            "--worktree",
            &format!("submodule.{NAME}.{field}"),
            val,
        ]);
    }
    let state = child_state(&h, &h.work_dir);
    let remote = value(
        &h,
        &h.work_dir,
        &["-f", ".gitmodules"],
        "submodule.logical.name.url",
    )
    .unwrap();
    ok(&h
        .run_submod(&[
            "change",
            "alias",
            "--url",
            &remote,
            "--branch",
            "main",
            "--ignore",
            "dirty",
            "--update",
            "none",
            "--fetch",
            "never",
            "--shallow",
            "false",
        ])
        .unwrap());
    assert_managed(
        &h,
        &h.work_dir,
        &[
            ("url", Some(&remote)),
            ("branch", Some("main")),
            ("ignore", Some("dirty")),
            ("update", Some("none")),
            ("fetchRecurseSubmodules", Some("false")),
            ("shallow", Some("false")),
        ],
    );
    assert_eq!(
        value(
            &h,
            &h.work_dir,
            &["--worktree"],
            "submodule.logical.name.custom"
        )
        .as_deref(),
        Some("worktree-keep")
    );
    assert_eq!(child_state(&h, &h.work_dir), state);
}

fn custom_config_invocation(linked: bool, nested: bool) {
    let h = fixture();
    fs::create_dir_all(h.work_dir.join("settings")).unwrap();
    fs::rename(h.config_path(), h.work_dir.join("settings/modules.toml")).unwrap();
    h.git_stdout(&["add", "-A"]);
    h.git_stdout(&["commit", "-m", "custom config"]);
    let root = if linked {
        let path = h.temp_dir.path().join("linked");
        h.git_stdout(&["worktree", "add", "-b", "linked", path.to_str().unwrap()]);
        path
    } else {
        h.work_dir.clone()
    };
    let cwd = if nested {
        root.join("nested/deep")
    } else {
        root.clone()
    };
    fs::create_dir_all(&cwd).unwrap();
    let config = if nested {
        "../../settings/modules.toml"
    } else {
        "settings/modules.toml"
    };
    ok(&h
        .run_submod_at(
            &cwd,
            &["--config", config, "change", "alias", "--ignore", "dirty"],
        )
        .unwrap());
    assert_eq!(
        value(
            &h,
            &root,
            &["-f", ".gitmodules"],
            "submodule.logical.name.ignore"
        )
        .as_deref(),
        Some("dirty")
    );
    assert!(!root.join("submod.toml").exists());
    if nested {
        assert!(!cwd.join(CHILD).exists());
    }
    if linked {
        assert!(
            !root.join(CHILD).join(".git").exists(),
            "metadata edit materialized linked checkout"
        );
    }
}

#[test]
fn r17_custom_config_root() {
    custom_config_invocation(false, false);
}
#[test]
fn r17_custom_config_nested() {
    custom_config_invocation(false, true);
}
#[test]
fn r17_custom_config_linked_nested() {
    custom_config_invocation(true, true);
}

#[test]
fn r31_metadata_change_preserves_or_refuses_both_gitmodules_layers() {
    let h = fixture();
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.unmanaged.staged",
        "index-only",
    ]);
    h.git_stdout(&["add", ".gitmodules"]);
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.unmanaged.unstaged",
        "worktree-only",
    ]);
    let index = h
        .git_cmd()
        .current_dir(&h.work_dir)
        .args(["show", ":.gitmodules"])
        .output()
        .unwrap()
        .stdout;
    let bytes = fs::read(h.work_dir.join(".gitmodules")).unwrap();
    let snapshot = h.preservation_snapshot();
    let state = child_state(&h, &h.work_dir);
    let result = h
        .run_submod(&["change", "alias", "--ignore", "dirty"])
        .unwrap();
    assert_eq!(
        h.git_cmd()
            .current_dir(&h.work_dir)
            .args(["show", ":.gitmodules"])
            .output()
            .unwrap()
            .stdout,
        index,
        "changed staged blob"
    );
    if result.status.success() {
        assert_eq!(
            value(
                &h,
                &h.work_dir,
                &["-f", ".gitmodules"],
                "submodule.unmanaged.staged"
            )
            .as_deref(),
            Some("index-only")
        );
        assert_eq!(
            value(
                &h,
                &h.work_dir,
                &["-f", ".gitmodules"],
                "submodule.unmanaged.unstaged"
            )
            .as_deref(),
            Some("worktree-only")
        );
        assert_managed(&h, &h.work_dir, &[("ignore", Some("dirty"))]);
    } else {
        assert_eq!(fs::read(h.work_dir.join(".gitmodules")).unwrap(), bytes);
        assert_eq!(
            h.preservation_snapshot(),
            snapshot,
            "refusal mutated metadata"
        );
    }
    assert_eq!(child_state(&h, &h.work_dir), state);
}

#[test]
fn r14_second_update_none_sync_is_exact_noop() {
    let h = fixture();
    let mut config = h.read_config().unwrap();
    config.push_str("update = 'none'\n");
    h.create_config(&config).unwrap();
    ok(&h.run_submod(&["sync"]).unwrap());
    let before = h.preservation_snapshot();
    let child = child_state(&h, &h.work_dir);
    let child_common = h.git_at(
        &h.work_dir.join(CHILD),
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    );
    let config_paths = [
        h.config_path(),
        h.work_dir.join(".gitmodules"),
        h.work_dir.join(".git/config"),
        Path::new(&child_common).join("config"),
    ];
    let metadata: Vec<_> = config_paths
        .iter()
        .map(|p| fs::metadata(p).unwrap())
        .collect();
    ok(&h.run_submod(&["sync"]).unwrap());
    for (path, before) in config_paths.iter().zip(metadata) {
        let after = fs::metadata(path).unwrap();
        assert_eq!(
            after.modified().unwrap(),
            before.modified().unwrap(),
            "no-op changed mtime: {}",
            path.display()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(
                after.ino(),
                before.ino(),
                "no-op replaced inode: {}",
                path.display()
            );
        }
    }
    assert_eq!(h.preservation_snapshot(), before);
    assert_eq!(child_state(&h, &h.work_dir), child);
}

#[test]
fn r14_linked_url_change_updates_selected_child_only() {
    let h = fixture();
    h.git_stdout(&["config", "extensions.worktreeConfig", "true"]);
    let linked = h.temp_dir.path().join("linked");
    h.git_stdout(&["worktree", "add", "-b", "linked", linked.to_str().unwrap()]);
    h.git_at(&linked, &["submodule", "update", "--init", "--", CHILD]);
    h.git_at(
        &linked,
        &[
            "config",
            "--worktree",
            "submodule.logical.name.url",
            "/stale/worktree-url",
        ],
    );
    h.git_at(
        &linked,
        &[
            "config",
            "--worktree",
            "submodule.logical.name.active",
            "false",
        ],
    );
    h.git_at(
        &linked,
        &[
            "config",
            "--worktree",
            "submodule.logical.name.custom",
            "linked-keep",
        ],
    );
    let main_url = value(&h, &h.work_dir.join(CHILD), &[], "remote.origin.url");
    let main_state = child_state(&h, &h.work_dir);
    let linked_state = child_state(&h, &linked);
    let index = h.git_at(&linked, &["ls-files", "--stage"]);
    let new_url = h.temp_dir.path().join("unreachable-replacement");
    ok(&h
        .run_submod_at(
            &linked,
            &["change", "alias", "--url", new_url.to_str().unwrap()],
        )
        .unwrap());
    for cwd in [&linked, &linked.join(CHILD)] {
        let key = if cwd == &linked {
            "submodule.logical.name.url"
        } else {
            "remote.origin.url"
        };
        assert_eq!(
            value(&h, cwd, &[], key).as_deref(),
            Some(new_url.to_str().unwrap())
        );
    }
    assert_eq!(
        value(&h, &h.work_dir.join(CHILD), &[], "remote.origin.url"),
        main_url
    );
    assert_eq!(
        value(
            &h,
            &linked,
            &["--worktree"],
            "submodule.logical.name.custom"
        )
        .as_deref(),
        Some("linked-keep")
    );
    assert_eq!(child_state(&h, &h.work_dir), main_state);
    assert_eq!(child_state(&h, &linked), linked_state);
    assert_eq!(h.git_at(&linked, &["ls-files", "--stage"]), index);
}

fn refuse_structural_edit_with_two_layers(operation: &str) {
    let h = fixture();
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.unmanaged.staged",
        "index-only",
    ]);
    h.git_stdout(&["add", ".gitmodules"]);
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.unmanaged.unstaged",
        "worktree-only",
    ]);
    let index_blob = h
        .git_cmd()
        .current_dir(&h.work_dir)
        .args(["show", ":.gitmodules"])
        .output()
        .unwrap()
        .stdout;
    let snapshot = h.preservation_snapshot();
    let state = child_state(&h, &h.work_dir);
    let remote = value(
        &h,
        &h.work_dir,
        &["-f", ".gitmodules"],
        "submodule.logical.name.url",
    )
    .unwrap();
    let args = match operation {
        "add" => vec![
            "add",
            &remote,
            "--name",
            "another",
            "--path",
            "vendor/another",
        ],
        "move" => vec!["change", "alias", "--path", "vendor/moved"],
        "delete" => vec!["delete", "alias"],
        _ => unreachable!(),
    };
    let output = h.run_submod(&args).unwrap();
    assert!(
        !output.status.success(),
        "structural edit must refuse unresolved .gitmodules layers: {output:?}"
    );
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        diagnostic.contains(".gitmodules"),
        "refusal must identify the conflicting metadata layers: {diagnostic}"
    );
    assert_eq!(
        h.preservation_snapshot(),
        snapshot,
        "refusal changed TOML, working .gitmodules, local config, refs or index entries"
    );
    assert_eq!(
        h.git_cmd()
            .current_dir(&h.work_dir)
            .args(["show", ":.gitmodules"])
            .output()
            .unwrap()
            .stdout,
        index_blob
    );
    assert_eq!(child_state(&h, &h.work_dir), state);
    assert!(!h.work_dir.join("vendor/another").exists());
    assert!(!h.work_dir.join("vendor/moved").exists());
}

#[test]
fn r31_add_refuses_two_gitmodules_layers() {
    refuse_structural_edit_with_two_layers("add");
}
#[test]
fn r31_move_refuses_two_gitmodules_layers() {
    refuse_structural_edit_with_two_layers("move");
}
#[test]
fn r31_delete_refuses_two_gitmodules_layers() {
    refuse_structural_edit_with_two_layers("delete");
}

fn linked_metadata_fixture(h: &TestHarness) -> std::path::PathBuf {
    h.git_stdout(&["config", "extensions.worktreeConfig", "true"]);
    let linked = h.temp_dir.path().join("review-linked");
    h.git_stdout(&[
        "worktree",
        "add",
        "-b",
        "review-linked",
        linked.to_str().unwrap(),
    ]);
    h.git_at(&linked, &["submodule", "update", "--init", "--", CHILD]);
    h.git_at(
        &linked,
        &[
            "config",
            "--worktree",
            "submodule.logical.name.custom",
            "worktree-keep",
        ],
    );
    linked
}

#[test]
fn r14_relative_url_remains_portable_and_resolves_selected_child_without_fetch() {
    let h = fixture();
    let parent_remote = h.temp_dir.path().join("remotes/parent.git");
    h.git_stdout(&["remote", "add", "origin", parent_remote.to_str().unwrap()]);
    let linked = linked_metadata_fixture(&h);
    let expected = h.temp_dir.path().join("remotes/replacement.git");
    let state = child_state(&h, &linked);
    let main_state = child_state(&h, &h.work_dir);
    let main_url = value(&h, &h.work_dir.join(CHILD), &[], "remote.origin.url");
    let index = h.git_at(&linked, &["ls-files", "--stage"]);
    let parent_head = h.git_at(&linked, &["rev-parse", "HEAD"]);
    let parent_refs = h.git_at(&linked, &["show-ref"]);
    ok(&h
        .run_submod_at(&linked, &["change", "alias", "--url", "../replacement.git"])
        .unwrap());
    assert_eq!(
        value(
            &h,
            &linked,
            &["-f", ".gitmodules"],
            "submodule.logical.name.url"
        )
        .as_deref(),
        Some("../replacement.git")
    );
    let config: toml::Value =
        toml::from_str(&fs::read_to_string(linked.join("submod.toml")).unwrap()).unwrap();
    assert_eq!(config["alias"]["url"].as_str(), Some("../replacement.git"));
    for scope in [&["--local"][..], &[][..]] {
        assert_eq!(
            value(&h, &linked, scope, "submodule.logical.name.url").as_deref(),
            Some(expected.to_str().unwrap())
        );
    }
    assert_eq!(
        value(&h, &linked.join(CHILD), &[], "remote.origin.url").as_deref(),
        Some(expected.to_str().unwrap())
    );
    assert_eq!(
        value(&h, &h.work_dir.join(CHILD), &[], "remote.origin.url"),
        main_url
    );
    assert_eq!(child_state(&h, &linked), state);
    assert_eq!(child_state(&h, &h.work_dir), main_state);
    assert_eq!(h.git_at(&linked, &["ls-files", "--stage"]), index);
    assert_eq!(h.git_at(&linked, &["rev-parse", "HEAD"]), parent_head);
    assert_eq!(h.git_at(&linked, &["show-ref"]), parent_refs);
}

fn held_metadata_lock_refuses_before_mutation(parent_worktree_lock: bool) {
    let h = fixture();
    let linked = linked_metadata_fixture(&h);
    let parent_gitdir =
        std::path::PathBuf::from(h.git_at(&linked, &["rev-parse", "--absolute-git-dir"]));
    let parent_common = std::path::PathBuf::from(h.git_at(
        &linked,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    ));
    let child_common = std::path::PathBuf::from(h.git_at(
        &linked.join(CHILD),
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    ));
    let lock = if parent_worktree_lock {
        parent_gitdir.join("config.worktree.lock")
    } else {
        child_common.join("config.lock")
    };
    fs::write(&lock, b"held by another Git writer\n").unwrap();
    let files = [
        linked.join("submod.toml"),
        linked.join(".gitmodules"),
        parent_common.join("config"),
        parent_gitdir.join("config.worktree"),
        child_common.join("config"),
        parent_gitdir.join("index"),
    ];
    let before: Vec<_> = files.iter().map(|p| fs::read(p).unwrap()).collect();
    let child_before = child_state(&h, &linked);
    let main_before = child_state(&h, &h.work_dir);
    let parent_refs = h.git_at(&linked, &["show-ref"]);
    let output = h
        .run_submod_at(
            &linked,
            &[
                "change",
                "alias",
                "--url",
                "/unreachable/replacement",
                "--ignore",
                "dirty",
            ],
        )
        .unwrap();
    assert!(
        !output.status.success(),
        "held Git lock must refuse before mutation: {output:?}"
    );
    let diagnostic = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        diagnostic.contains("lock"),
        "missing lock diagnostic: {diagnostic}"
    );
    for (path, bytes) in files.iter().zip(before) {
        assert_eq!(
            fs::read(path).unwrap(),
            bytes,
            "modified {} despite held {}",
            path.display(),
            lock.display()
        );
    }
    assert_eq!(fs::read(&lock).unwrap(), b"held by another Git writer\n");
    assert_eq!(child_state(&h, &linked), child_before);
    assert_eq!(child_state(&h, &h.work_dir), main_before);
    assert_eq!(h.git_at(&linked, &["show-ref"]), parent_refs);
}

#[test]
fn r14_parent_worktree_config_lock_refuses_before_mutation() {
    held_metadata_lock_refuses_before_mutation(true);
}

#[test]
fn r14_selected_child_common_config_lock_refuses_before_mutation() {
    held_metadata_lock_refuses_before_mutation(false);
}

#[test]
fn r14_child_branch_upstream_remote_selected_and_origin_preserved() {
    let h = fixture();
    let child = h.work_dir.join(CHILD);
    h.git_at(&child, &["checkout", "-b", "selected"]);
    let old_url = value(&h, &child, &[], "remote.origin.url").unwrap();
    h.git_at(&child, &["remote", "add", "upstream", &old_url]);
    h.git_at(&child, &["config", "branch.selected.remote", "upstream"]);
    h.git_at(
        &child,
        &[
            "remote",
            "set-url",
            "origin",
            "/deliberately/different-origin",
        ],
    );
    let expected = h.temp_dir.path().join("unreachable-upstream");
    let state = child_state(&h, &h.work_dir);
    let index = h.git_stdout(&["ls-files", "--stage"]);
    ok(&h
        .run_submod(&["change", "alias", "--url", expected.to_str().unwrap()])
        .unwrap());
    assert_eq!(
        value(&h, &child, &[], "remote.upstream.url").as_deref(),
        Some(expected.to_str().unwrap())
    );
    assert_eq!(
        value(&h, &child, &[], "remote.origin.url").as_deref(),
        Some("/deliberately/different-origin")
    );
    assert_eq!(
        value(&h, &child, &[], "branch.selected.remote").as_deref(),
        Some("upstream")
    );
    assert_eq!(
        value(&h, &h.work_dir, &["--local"], "submodule.logical.name.url").as_deref(),
        Some(expected.to_str().unwrap())
    );
    assert_eq!(child_state(&h, &h.work_dir), state);
    assert_eq!(h.git_stdout(&["ls-files", "--stage"]), index);
}

#[test]
fn r14_relative_parent_remote_resolves_distinct_nested_child_url() {
    let h = fixture();
    let child = h.work_dir.join(CHILD);
    h.git_stdout(&["remote", "add", "origin", "../remotes/parent.git"]);
    // Ask native Git for the platform's path-relative URLs, then restore all
    // oracle writes before exercising Submod from the original state.
    let child_common = h.git_at(
        &child,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    );
    let files = [
        h.work_dir.join(".gitmodules"),
        h.work_dir.join(".git/config"),
        Path::new(&child_common).join("config"),
    ];
    let original: Vec<_> = files.iter().map(|p| fs::read(p).unwrap()).collect();
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.logical.name.url",
        "../replacement.git",
    ]);
    h.git_stdout(&["submodule", "sync", "--", CHILD]);
    let expected_parent =
        value(&h, &h.work_dir, &["--local"], "submodule.logical.name.url").unwrap();
    let expected_child = value(&h, &child, &[], "remote.origin.url").unwrap();
    assert_ne!(
        expected_parent, expected_child,
        "fixture must exercise distinct relative URL bases"
    );
    for (path, bytes) in files.iter().zip(original) {
        fs::write(path, bytes).unwrap();
    }
    let state = child_state(&h, &h.work_dir);
    let index = h.git_stdout(&["ls-files", "--stage"]);
    ok(&h
        .run_submod(&["change", "alias", "--url", "../replacement.git"])
        .unwrap());
    assert_eq!(
        value(
            &h,
            &h.work_dir,
            &["-f", ".gitmodules"],
            "submodule.logical.name.url"
        )
        .as_deref(),
        Some("../replacement.git")
    );
    for scope in [&["--local"][..], &[][..]] {
        assert_eq!(
            value(&h, &h.work_dir, scope, "submodule.logical.name.url").as_deref(),
            Some(expected_parent.as_str())
        );
    }
    assert_eq!(
        value(&h, &child, &[], "remote.origin.url").as_deref(),
        Some(expected_child.as_str())
    );
    assert_eq!(child_state(&h, &h.work_dir), state);
    assert_eq!(h.git_stdout(&["ls-files", "--stage"]), index);
}
