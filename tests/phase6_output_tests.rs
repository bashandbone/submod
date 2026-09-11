// SPDX-FileCopyrightText: 2026 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! R26: assert display boundaries without modifying Git's machine-readable bytes.
mod common;
use common::TestHarness;
use std::process::Output;

fn fixture() -> TestHarness {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    h
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).expect("human output is UTF-8")
}

fn safe_human_output(out: &Output) {
    for bytes in [&out.stdout, &out.stderr] {
        let message = text(bytes);
        assert!(
            !message
                .chars()
                .any(|c| c.is_control() && c != '\n' && c != '\t'),
            "terminal controls emitted: {message:?}"
        );
        for secret in [
            "R26_FAKE_USER",
            "R26_FAKE_PASS",
            "USER_A",
            "PASS_A",
            "USER_B",
            "P%40SS_B",
        ] {
            assert!(
                !message.contains(secret),
                "URL userinfo leaked: {message:?}"
            );
        }
        for decoration in ["✅", "❌", "🔄", "⚠", "⏭", "ℹ", "📋", "💥"] {
            assert!(
                !message.contains(decoration),
                "decoration in redirected output: {message:?}"
            );
        }
    }
}

#[test]
fn r26_list_redacts_multiple_urls_and_preserves_unicode() {
    let h = fixture();
    let config = "['bibliothèque']\npath = 'dépendances/été'\nurl = 'https://USER_A:PASS_A@example.invalid/a'\nactive = false\n[other]\nurl = 'https://USER_B:P%40SS_B@example.invalid/b'\nactive = false\n[quoted]\nurl = 'https://R26_FAKE_USER:PA\"SS@example.invalid/quoted'\nactive = false\n";
    h.create_config(config).unwrap();
    let out = h.run_submod(&["list"]).unwrap();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(h.read_config().unwrap(), config);
    let stdout = text(&out.stdout);
    for expected in [
        "bibliothèque",
        "dépendances/été",
        "example.invalid/a",
        "example.invalid/b",
        "example.invalid/quoted",
    ] {
        assert!(
            stdout.contains(expected),
            "lost useful display context: {stdout:?}"
        );
    }
    assert!(
        out.stderr.is_empty(),
        "ordinary list must not produce progress/errors"
    );
    safe_human_output(&out);
}

#[test]
fn r26_repository_name_and_legacy_warning_escape_controls() {
    let h = fixture();
    let config = "[\"module-https://R26_FAKE_USER:R26_FAKE_PASS@example.invalid/warned\\u001b[2J\\r\\nFORGED_NAME\\u0008\\u007f\\u0085\"]\npath = \"lib\"\nurl = 'https://R26_FAKE_USER:R26_FAKE_PASS@example.invalid/repo.git'\nactive = false\nfetch = 'never'\n";
    h.create_config(config).unwrap();
    let out = h.run_submod(&["list"]).unwrap();
    assert!(
        !out.status.success(),
        "invalid administrative name must be rejected"
    );
    assert_eq!(h.read_config().unwrap(), config);
    assert!(
        out.stdout.is_empty(),
        "rejected config must not emit results"
    );
    let warning = text(&out.stderr);
    let legacy_line = warning
        .lines()
        .find(|line| line.contains("legacy"))
        .expect("legacy warning must be emitted");
    assert!(legacy_line.contains("fetch") && legacy_line.contains("module"));
    assert!(
        legacy_line.contains("example.invalid/warned"),
        "warning nickname context lost: {warning:?}"
    );
    assert!(
        !text(&out.stdout).contains("warning:"),
        "warnings belong to stderr"
    );
    for message in [text(&out.stdout), warning] {
        assert!(
            !message.contains("\nFORGED_"),
            "repository field forged an output line: {message:?}"
        );
    }
    safe_human_output(&out);
}

#[test]
fn r26_app_config_error_redacts_embedded_url_and_retains_field_cause() {
    let h = fixture();
    let config = "[module]\nurl = './remote.git'\nignore = 'https://R26_FAKE_USER:R26_FAKE_PASS@example.invalid/rejected'\n";
    h.create_config(config).unwrap();
    let before = h.preservation_snapshot();
    let out = h.run_submod(&["list"]).unwrap();
    assert!(!out.status.success());
    assert_eq!(h.preservation_snapshot(), before);
    assert!(
        out.stdout.is_empty(),
        "error must not appear as stdout result: {out:?}"
    );
    let error = text(&out.stderr);
    assert!(
        error.contains("module") && error.contains("ignore"),
        "field context lost: {error}"
    );
    assert!(
        error.contains("invalid") || error.contains("expected"),
        "validation cause lost: {error}"
    );
    safe_human_output(&out);
}

#[test]
fn r26_clap_error_sanitizes_early_diagnostic() {
    let h = fixture();
    let out = h
        .run_submod(&[
            "change",
            "module",
            "--ignore",
            "https://R26_FAKE_USER:R26_FAKE_PASS@example.invalid/\u{1b}[2J\r\nFORGED_ARGUMENT",
        ])
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    let error = text(&out.stderr);
    assert!(
        error.contains("--ignore") && error.contains("invalid"),
        "argument cause lost: {error:?}"
    );
    assert!(!error.contains("\nFORGED_ARGUMENT"));
    safe_human_output(&out);
}

#[test]
fn r26_real_local_git_error_redacts_userinfo_without_false_success() {
    let h = fixture();
    // Native submodule add echoes the original file URL in its final failure.
    // Prove this in a separate fixture, without transport to any network host.
    let missing = format!(
        "file://R26_FAKE_USER:R26_FAKE_PASS@localhost{}/missing.git",
        h.work_dir.display()
    );
    let url = missing.as_str();
    let probe = fixture();
    let git = probe
        .git_cmd()
        .args(["submodule", "add", "--", url, "probe"])
        .current_dir(&probe.work_dir)
        .output()
        .unwrap();
    assert!(!git.status.success());
    let raw = text(&git.stderr);
    assert!(
        raw.contains("R26_FAKE_PASS"),
        "fixture must prove Git actually echoed the secret: {raw}"
    );
    h.create_config(&format!("[module]\npath = 'lib'\nurl = '{}'\n", url))
        .unwrap();
    let config_before = h.read_config().unwrap();
    let out = h.run_submod(&["init"]).unwrap();
    assert!(!out.status.success(), "missing remote must fail: {out:?}");
    assert_eq!(h.read_config().unwrap(), config_before);
    let error = text(&out.stderr);
    assert!(error.contains("module"), "module context lost: {error}");
    assert!(
        error.to_lowercase().contains("init") || error.to_lowercase().contains("clone"),
        "phase lost: {error}"
    );
    assert!(
        error.contains("does not appear to be a git repository")
            || error.contains("does not exist"),
        "actionable Git cause lost: {error}"
    );
    let stdout = text(&out.stdout).to_lowercase();
    assert!(
        !stdout.contains("successfully")
            && !stdout.contains("initialized 1")
            && !stdout.contains("sync complete"),
        "false success: {stdout}"
    );
    safe_human_output(&out);
}

#[test]
fn r26_redirected_init_sends_progress_to_stderr() {
    let h = fixture();
    let remote = h.create_test_remote("r26-progress").unwrap();
    h.create_config(&format!(
        "[module]\npath = 'lib'\nurl = '{}'\n",
        remote.display()
    ))
    .unwrap();
    let out = h.run_submod(&["init"]).unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(h.index_gitlink_mode("lib").is_some());
    let stdout = text(&out.stdout);
    assert!(
        stdout.contains("module"),
        "final result must identify module: {stdout}"
    );
    assert!(
        !stdout.contains("Initializing ") && !stdout.contains("Cloning into"),
        "progress on stdout: {stdout}"
    );
    safe_human_output(&out);
}

#[test]
fn r26_completion_stdout_remains_shell_program() {
    let h = TestHarness::new().unwrap();
    let out = h.run_submod(&["completeme", "bash"]).unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(
        out.stderr.is_empty(),
        "completion stderr contaminated: {out:?}"
    );
    let script = text(&out.stdout);
    assert!(
        script.starts_with("_submod()"),
        "unexpected human prefix: {script}"
    );
    assert!(script.contains("complete ") && script.contains("COMPREPLY="));
    // Shell parser validates literal newlines, quotes and backslashes survived.
    let path = h.work_dir.join("completion.bash");
    std::fs::write(&path, &out.stdout).unwrap();
    let syntax = std::process::Command::new("bash")
        .arg("-n")
        .arg(path)
        .output()
        .unwrap();
    assert!(syntax.status.success(), "completion corrupted: {syntax:?}");
    assert!(!out.stdout.contains(&0x1b));
}

#[test]
fn r26_invalid_repository_path_reports_safe_validation_error() {
    let h = fixture();
    let config = "[module]\nurl = './remote.git'\npath = \"lib\\u001b[2J\\r\\nFORGED_PATH\"\n";
    h.create_config(config).unwrap();
    let before = h.preservation_snapshot();
    let out = h.run_submod(&["list"]).unwrap();
    assert!(!out.status.success());
    assert_eq!(h.preservation_snapshot(), before);
    assert!(out.stdout.is_empty());
    let error = text(&out.stderr);
    assert!(error.contains("module") && error.contains("path") && error.contains("invalid"));
    assert!(!error.contains("\nFORGED_PATH"));
    safe_human_output(&out);
}

#[test]
fn r26_redirected_sync_has_no_decoration_or_stdout_progress() {
    let h = fixture();
    h.create_config("[module]\nurl = './remote.git'\nactive = false\n")
        .unwrap();
    let out = h.run_submod(&["sync"]).unwrap();
    assert!(out.status.success(), "{out:?}");
    assert!(
        !text(&out.stdout).contains("Syncing submodules"),
        "progress belongs to stderr: {out:?}"
    );
    let stdout = text(&out.stdout).to_lowercase();
    assert!(
        stdout.lines().any(|line| line.contains("module")
            && line.contains("skipped")
            && line.contains("disabled")),
        "disabled module needs a skipped result: {stdout}"
    );
    for progress in [
        "syncing submodules",
        "reconciling configured",
        "initializing ",
        "cloning into",
    ] {
        assert!(!stdout.contains(progress), "progress on stdout: {stdout}");
    }
    safe_human_output(&out);
}

#[test]
fn r26_successful_list_escapes_url_suffix_controls() {
    let h = fixture();
    let config = r#"[module]
url = "https://example.invalid/repo.git/ESC\u001b[2J/TAB\t/BS\u0008/DEL\u007f/C1\u0085/end"
active = false
"#;
    h.create_config(config).unwrap();
    let out = h.run_submod(&["list"]).unwrap();
    assert!(out.status.success(), "{out:?}");
    assert_eq!(h.read_config().unwrap(), config);
    assert!(
        out.stderr.is_empty(),
        "ordinary list must succeed without diagnostics: {out:?}"
    );
    let stdout = text(&out.stdout);
    let url_line = stdout
        .lines()
        .find(|line| line.contains("example.invalid/repo.git"))
        .expect("URL display retained");
    for escaped in [
        r"ESC\u{1b}[2J",
        r"TAB\t",
        r"BS\u{8}",
        r"DEL\u{7f}",
        r"C1\u{85}",
        "/end",
    ] {
        assert!(
            url_line.contains(escaped),
            "missing recognizable escaped payload {escaped}: {stdout:?}"
        );
    }
    assert!(!stdout.contains("\nFORGED_URL") && !stdout.contains('\t'));
    safe_human_output(&out);
}

#[test]
fn r26_generate_preview_sanitizes_the_output_path() {
    let h = TestHarness::new().unwrap();
    let output = "generated\nFORGED.toml";
    let out = h
        .run_submod(&[
            "generate-config",
            "--template",
            "--output",
            output,
            "--dry-run",
        ])
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let stdout = text(&out.stdout);
    assert!(stdout.contains(r"generated\nFORGED.toml"), "{stdout:?}");
    assert!(!stdout.contains("\nFORGED"), "{stdout:?}");
    safe_human_output(&out);
}

// Result rows must describe observed Git postconditions, not the pre-fetch plan.
fn result_fixture(names: &[&str]) -> TestHarness {
    let h = fixture();
    let mut config = String::new();
    for name in names {
        let remote = h.create_test_remote(name).unwrap();
        h.git_stdout(&[
            "submodule",
            "add",
            "--name",
            name,
            remote.to_str().unwrap(),
            name,
        ]);
        config.push_str(&format!(
            "[{name}]\npath = '{name}'\nurl = '{}'\nbranch = 'main'\nignore = 'none'\n",
            remote.display()
        ));
    }
    h.create_config(&config).unwrap();
    h.run_submod_success(&["sync"]).unwrap();
    h.git_stdout(&["add", ".gitmodules"]);
    h.git_stdout(&["commit", "-am", "Record result fixture pins"]);
    h
}

fn result_row<'a>(output: &'a str, name: &str, status: &str) -> &'a str {
    let prefix = format!("{name} at {name}: {status}:");
    let rows: Vec<_> = output
        .lines()
        .filter(|line| line.starts_with(&prefix))
        .collect();
    assert_eq!(
        rows.len(),
        1,
        "expected exactly one final row {prefix}: {output}"
    );
    rows[0]
}

fn assert_summary(output: &str, expected: &str) {
    assert_eq!(
        output
            .lines()
            .filter(|line| line.contains(" summary:"))
            .collect::<Vec<_>>(),
        vec![expected]
    );
}

#[test]
fn r26_result_remote_advance_then_repeat_uses_observed_target() {
    let h = result_fixture(&["module"]);
    let pin = h.git_stdout(&["-C", "module", "rev-parse", "HEAD"]);
    let index = h.git_stdout(&["ls-files", "--stage"]);
    let original = std::fs::read(h.work_dir.join("module/src/main.c")).unwrap();
    let target = h.advance_test_remote("module").unwrap();
    assert_ne!(target, pin);
    for (status, summary) in [
        (
            "changed",
            "Update summary: 1 changed, 0 unchanged, 0 skipped, 0 failed, 0 pending.",
        ),
        (
            "unchanged",
            "Update summary: 0 changed, 1 unchanged, 0 skipped, 0 failed, 0 pending.",
        ),
    ] {
        let out = h.run_submod(&["update", "--remote"]).unwrap();
        assert!(out.status.success(), "{out:?}");
        let stdout = text(&out.stdout);
        assert!(
            result_row(&stdout, "module", status).ends_with(&format!("(target {target})")),
            "wrong selected target: {stdout}"
        );
        assert_summary(&stdout, summary);
        assert_eq!(h.git_stdout(&["-C", "module", "rev-parse", "HEAD"]), target);
        assert_eq!(
            h.git_stdout(&["ls-files", "--stage"]),
            index,
            "remote tracking must preserve parent gitlink pin"
        );
        assert_eq!(
            std::fs::read(h.work_dir.join("module/src/main.c")).unwrap(),
            original
        );
        assert_eq!(
            std::fs::read_to_string(h.work_dir.join("module/ADVANCE.txt")).unwrap(),
            "advanced module\n"
        );
    }
}

#[test]
fn r26_result_recursive_noop_is_unchanged() {
    let h = result_fixture(&["module"]);
    let nested = h.create_test_remote("nested").unwrap();
    h.git_stdout(&[
        "-C",
        "module",
        "submodule",
        "add",
        nested.to_str().unwrap(),
        "nested",
    ]);
    h.git_stdout(&["-C", "module", "commit", "-am", "Record nested pin"]);
    h.git_stdout(&["add", "module"]);
    h.git_stdout(&["commit", "-m", "Pin parent with nested checkout"]);
    let nested_head = h.git_stdout(&["-C", "module/nested", "rev-parse", "HEAD"]);
    let nested_index = h.git_stdout(&["-C", "module", "ls-files", "--stage"]);
    let nested_content = std::fs::read(h.work_dir.join("module/nested/src/main.c")).unwrap();
    let before = h.preservation_snapshot();
    let target = h.git_stdout(&["-C", "module", "rev-parse", "HEAD"]);
    let content = std::fs::read(h.work_dir.join("module/src/main.c")).unwrap();
    let out = h.run_submod(&["update", "--recursive"]).unwrap();
    assert!(out.status.success(), "{out:?}");
    let stdout = text(&out.stdout);
    assert!(result_row(&stdout, "module", "unchanged").ends_with(&format!("(target {target})")));
    assert_summary(
        &stdout,
        "Update summary: 0 changed, 1 unchanged, 0 skipped, 0 failed, 0 pending.",
    );
    assert_eq!(h.preservation_snapshot(), before);
    assert_eq!(h.git_stdout(&["-C", "module", "rev-parse", "HEAD"]), target);
    assert_eq!(
        std::fs::read(h.work_dir.join("module/src/main.c")).unwrap(),
        content
    );
    assert_eq!(
        h.git_stdout(&["-C", "module/nested", "rev-parse", "HEAD"]),
        nested_head
    );
    assert_eq!(
        h.git_stdout(&["-C", "module", "ls-files", "--stage"]),
        nested_index
    );
    assert_eq!(
        std::fs::read(h.work_dir.join("module/nested/src/main.c")).unwrap(),
        nested_content
    );
}

fn metadata_skip_result(setting: &str, status: &str) {
    let h = result_fixture(&["module"]);
    let target = h.git_stdout(&["-C", "module", "rev-parse", "HEAD"]);
    let gitlink = h.git_stdout(&["ls-files", "--stage", "--", "module"]);
    let content = std::fs::read(h.work_dir.join("module/src/main.c")).unwrap();
    let config = h
        .read_config()
        .unwrap()
        .replace("ignore = 'none'", "ignore = 'all'");
    h.create_config(&format!("{config}{setting}\n")).unwrap();
    let remote_target = h.advance_test_remote("module").unwrap();
    assert_ne!(target, remote_target);
    let out = h.run_submod(&["update", "--remote"]).unwrap();
    assert!(out.status.success(), "{out:?}");
    let stdout = text(&out.stdout);
    result_row(&stdout, "module", status);
    assert_summary(
        &stdout,
        "Update summary: 1 changed, 0 unchanged, 1 skipped, 0 failed, 0 pending.",
    );
    assert_eq!(
        h.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get",
            "submodule.module.ignore"
        ]),
        "all"
    );
    assert_eq!(
        h.git_stdout(&["config", "--get", "submodule.module.ignore"]),
        "all"
    );
    assert_eq!(h.git_stdout(&["-C", "module", "rev-parse", "HEAD"]), target);
    assert_eq!(
        h.git_stdout(&["ls-files", "--stage", "--", "module"]),
        gitlink
    );
    assert_eq!(
        std::fs::read(h.work_dir.join("module/src/main.c")).unwrap(),
        content
    );
    assert!(!h.work_dir.join("module/ADVANCE.txt").exists());
}

#[test]
fn r26_result_disabled_metadata_repair_counts_changed_and_skipped() {
    metadata_skip_result("active = false", "changed/skipped-disabled");
}

#[test]
fn r26_result_policy_metadata_repair_counts_changed_and_skipped() {
    metadata_skip_result("update = 'none'", "changed/skipped-policy");
}

#[test]
fn r26_result_runtime_failure_preserves_observed_completed_and_pending() {
    let h = result_fixture(&["alpha", "beta", "gamma"]);
    let index = h.git_stdout(&["ls-files", "--stage"]);
    let pins: Vec<_> = ["alpha", "beta", "gamma"]
        .iter()
        .map(|name| h.git_stdout(&["-C", name, "rev-parse", "HEAD"]))
        .collect();
    let contents: Vec<_> = ["alpha", "beta", "gamma"]
        .iter()
        .map(|name| std::fs::read(h.work_dir.join(name).join("src/main.c")).unwrap())
        .collect();
    let target = h.advance_test_remote("alpha").unwrap();
    h.advance_test_remote("gamma").unwrap();
    std::fs::rename(
        h.temp_dir.path().join("beta.git"),
        h.temp_dir.path().join("beta-offline.git"),
    )
    .unwrap();
    let out = h.run_submod(&["update", "--remote"]).unwrap();
    assert!(!out.status.success(), "offline beta must fail: {out:?}");
    let error = text(&out.stderr);
    assert!(
        result_row(&error, "alpha", "changed").ends_with(&format!("(target {target})")),
        "completed row must contain observed target: {error}"
    );
    result_row(&error, "beta", "failed");
    result_row(&error, "gamma", "pending");
    assert_summary(
        &error,
        "Update incomplete summary: 1 changed, 0 unchanged, 0 skipped, 1 failed, 1 pending.",
    );
    assert!(
        !text(&out.stdout).contains("summary:"),
        "failed batch must not emit a success summary"
    );
    for (index, name) in ["alpha", "beta", "gamma"].iter().enumerate() {
        assert_eq!(
            h.git_stdout(&["-C", name, "rev-parse", "HEAD"]),
            if index == 0 { &target } else { &pins[index] }.as_str()
        );
        assert_eq!(
            std::fs::read(h.work_dir.join(name).join("src/main.c")).unwrap(),
            contents[index]
        );
    }
    assert_eq!(h.git_stdout(&["ls-files", "--stage"]), index);
    assert_eq!(
        std::fs::read_to_string(h.work_dir.join("alpha/ADVANCE.txt")).unwrap(),
        "advanced alpha\n"
    );
    assert!(!h.work_dir.join("beta/ADVANCE.txt").exists());
    assert!(!h.work_dir.join("gamma/ADVANCE.txt").exists());
}
