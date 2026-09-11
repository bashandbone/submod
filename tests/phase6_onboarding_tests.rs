// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
//! README onboarding commands, exercised against fixture-local repositories.
mod common;

use common::TestHarness;
use std::{fs, process::Output};
use submod::Config;

fn success(output: Output) -> String {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn readme_toml_workflow_from_nested_directory_repairs_drift() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("readme-library").unwrap();
    h.create_config(&format!(
        r#"[defaults]
ignore = "dirty"

[my-submodule]
path = "vendor/my-lib"
url = "{}"
sparse_paths = ["src/", "include/", "*.md"]
"#,
        remote.display()
    ))
    .unwrap();
    let nested = h.work_dir.join("app/nested");
    fs::create_dir_all(&nested).unwrap();
    success(h.run_submod_at(&nested, &["init"]).unwrap());
    success(h.run_submod_at(&nested, &["check"]).unwrap());
    assert!(h.file_exists("vendor/my-lib/src/main.c"));
    assert!(h.file_exists("vendor/my-lib/include/header.h"));
    assert!(!h.file_exists("vendor/my-lib/LICENSE"));
    assert_eq!(
        h.index_gitlink_mode("vendor/my-lib").as_deref(),
        Some("160000")
    );
    h.git_stdout(&[
        "config",
        "-f",
        ".gitmodules",
        "submodule.my-submodule.ignore",
        "all",
    ]);
    h.git_stdout(&["add", ".gitmodules"]);
    assert_eq!(
        h.run_submod_at(&nested, &["check"]).unwrap().status.code(),
        Some(1)
    );
    success(h.run_submod_at(&nested, &["sync"]).unwrap());
    success(h.run_submod_at(&nested, &["check"]).unwrap());
    success(h.run_submod_at(&nested, &["update"]).unwrap());
    assert_eq!(
        h.git_stdout(&[
            "config",
            "-f",
            ".gitmodules",
            "submodule.my-submodule.ignore"
        ]),
        "dirty"
    );
}

#[test]
fn readme_boolean_from_setup_imports_registration_without_checkout() {
    let h = TestHarness::new().unwrap();
    h.init_git_repo().unwrap();
    let remote = h.create_test_remote("existing-library").unwrap();
    h.git_stdout(&[
        "submodule",
        "add",
        "--name",
        "library",
        remote.to_str().unwrap(),
        "vendor/library",
    ]);
    h.git_stdout(&["submodule", "deinit", "-f", "--", "vendor/library"]);
    success(h.run_submod(&["generate-config", "--from-setup"]).unwrap());
    let config = Config::default()
        .load_from_file(Some(h.config_path()))
        .unwrap();
    assert!(config.submodules.contains_key("library"));
    assert_eq!(h.run_submod(&["check"]).unwrap().status.code(), Some(1));
    success(h.run_submod(&["sync"]).unwrap());
    success(h.run_submod(&["check"]).unwrap());
    assert!(h.file_exists("vendor/library/src/main.c"));
    assert_eq!(
        h.run_submod(&["generate-config", "--from-setup", "."])
            .unwrap()
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn readme_template_outside_repository_reloads_and_protects_existing_output() {
    let h = TestHarness::new().unwrap();
    let args = [
        "generate-config",
        "--template",
        "--output",
        "my-config.toml",
    ];
    success(h.run_submod(&args).unwrap());
    let path = h.work_dir.join("my-config.toml");
    let original = fs::read(&path).unwrap();
    Config::default().load_from_file(Some(&path)).unwrap();
    assert!(!h.work_dir.join(".git").exists());
    assert!(!h.run_submod(&args).unwrap().status.success());
    assert_eq!(fs::read(&path).unwrap(), original);
    success(
        h.run_submod(&[
            "generate-config",
            "--template",
            "--output",
            "my-config.toml",
            "--force",
        ])
        .unwrap(),
    );
    Config::default().load_from_file(Some(&path)).unwrap();
}

#[test]
fn completion_scripts_for_all_supported_shells_and_nu_alias_work_outside_repository() {
    let h = TestHarness::new().unwrap();
    for shell in ["bash", "zsh", "fish", "powershell", "elvish", "nushell"] {
        let output = h.run_submod(&["completeme", shell]).unwrap();
        assert!(output.stderr.is_empty(), "unexpected stderr for {shell}");
        let script = success(output);
        assert!(
            script.contains("submod") && script.contains("generate-config"),
            "incomplete {shell} script"
        );
    }
    let canonical = success(h.run_submod(&["completeme", "nushell"]).unwrap());
    assert_eq!(
        success(h.run_submod(&["completeme", "nu"]).unwrap()),
        canonical
    );
    assert_eq!(
        success(h.run_submod(&["complete-me", "nu"]).unwrap()),
        canonical
    );
}

#[test]
fn onboarding_help_documents_boolean_import_reset_and_global_branch() {
    let h = TestHarness::new().unwrap();
    for command in [
        "init",
        "check",
        "sync",
        "update",
        "generate-config",
        "completeme",
        "disable",
        "reset",
        "change-global",
    ] {
        assert!(success(h.run_submod(&[command, "--help"]).unwrap()).contains("Usage:"));
    }
    let import = success(h.run_submod(&["generate-config", "--help"]).unwrap());
    let flag = import
        .lines()
        .find(|line| line.contains("--from-setup"))
        .unwrap();
    assert!(!flag.contains('<'), "from-setup must be boolean: {flag}");
    let reset = success(h.run_submod(&["reset", "--help"]).unwrap());
    assert!(reset.contains("parent gitlinks"), "{reset}");
    assert!(success(h.run_submod(&["change-global", "--help"]).unwrap()).contains("--branch"));
    assert_eq!(
        h.run_submod(&["generate-config", "--from-setup", "--template"])
            .unwrap()
            .status
            .code(),
        Some(2)
    );
}
