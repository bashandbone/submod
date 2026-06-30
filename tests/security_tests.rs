// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Security tests to ensure robustness against various attack vectors.

use std::fs;

mod common;
use common::TestHarness;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_with_hyphen_injection() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("hyphen-remote")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // A path with a component starting with a hyphen that could be a git flag.
        // Note: Git itself has issues with paths starting with hyphen in the CWD
        // (even with --), so we use a sub-path.
        let malicious_path = "sub/-c";

        // This should not fail with "unknown option" or similar error from git -C
        // It might still fail for other reasons if the path is invalid for a submodule,
        // but it shouldn't be interpreted as a flag to the 'git' command itself.

        // Note: Using add_submodule via harness.
        // We need to make sure the directory doesn't exist or is handled.

        let result = harness.run_submod(&[
            "add",
            &remote_url,
            "--name",
            "hyphen-sub",
            "--path",
            malicious_path,
        ]);

        // The operation might fail because "sub/-c" is a weird path, but it shouldn't be a Command Injection.
        // If it was interpreted as `git -C sub/-c`, it would fail with "unknown option" or similar
        // if our fix wasn't working.

        match result {
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(
                    !stderr.contains("unknown option: -c"),
                    "Potential command injection detected: git interpreted path as a flag"
                );
            }
            Err(e) => {
                let err_msg = e.to_string();
                assert!(
                    !err_msg.contains("unknown option: -c"),
                    "Potential command injection detected: git interpreted path as a flag"
                );
            }
        }
    }

    #[test]
    fn test_sparse_checkout_with_hyphen_path() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("sparse-hyphen")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Path with a component starting with hyphen.
        // Note: Git itself has issues with paths starting with hyphen in the CWD
        // (even with --), so we use a sub-path.
        let path = "sub/-sparse";

        // Ensure the directory exists to trigger the CLI fallback in apply_sparse_checkout if needed,
        // although apply_sparse_checkout is usually called after gix/git2 which might fail or be bypassed.

        harness
            .run_submod_success(&[
                "add",
                &remote_url,
                "--name",
                "sparse-hyphen",
                "--path",
                path,
                "--sparse-paths",
                "src",
            ])
            .expect("Failed to add submodule with hyphenated path");

        // Verify it worked
        assert!(harness.dir_exists("sub/-sparse/src"));
    }

    /// CVE-2018-17456-class option injection: a submodule name or URL that mimics
    /// a `git clone`/`git submodule` flag (e.g. `--upload-pack=<cmd>`) must be
    /// treated as inert data, never as an option that triggers command execution.
    ///
    /// submod drives git via gix/git2 (no shell) and uses `--` before the URL/path
    /// on the CLI last-resort path, so the payload should never run. This test is
    /// non-vacuous: if any code path ever shelled the name/URL out unsafely, the
    /// sentinel file would be created.
    #[test]
    fn test_flag_like_name_and_url_do_not_inject_commands() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_test_remote("inject-remote")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Sentinels: `touch <relative>` would land in cwd (the working tree);
        // `touch <absolute>` would land at a fixed path. Neither must appear.
        let rel_sentinel = harness.work_dir.join("INJECTED_BY_NAME");
        let abs_sentinel = harness.work_dir.join("INJECTED_BY_URL");
        assert!(!rel_sentinel.exists() && !abs_sentinel.exists());

        // Flag-like NAME (passed via `--name=` so clap accepts the leading dashes).
        let _ = harness
            .run_submod(&[
                "add",
                &remote_url,
                "--name=--upload-pack=touch INJECTED_BY_NAME",
                "--path",
                "lib/inj-name",
            ])
            .expect("Failed to run submod");
        assert!(
            !rel_sentinel.exists(),
            "a flag-like submodule name must not inject a command"
        );

        // Malicious transport URL (ext:: would run a shell command if honored).
        let evil_url = format!("ext::sh -c \"touch {}\"", abs_sentinel.display());
        let _ = harness
            .run_submod(&[
                "add",
                &evil_url,
                "--name",
                "inj-url",
                "--path",
                "lib/inj-url",
            ])
            .expect("Failed to run submod");
        assert!(
            !abs_sentinel.exists(),
            "a malicious transport URL must not execute a command"
        );
    }

    /// A hostile `.gitmodules` fed to `generate-config --from-setup` must be parsed
    /// as data only: its url/branch values are serialized verbatim into the output
    /// config, never executed. Non-vacuous: the generated file must actually contain
    /// the hostile values (proving the parse path ran) while no sentinel is created.
    #[test]
    fn test_generate_config_from_malicious_gitmodules_does_not_execute() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let sentinel = harness.work_dir.join("GC_INJECTED");
        let gitmodules = format!(
            "[submodule \"evil\"]\n\tpath = lib/evil\n\turl = ext::sh -c \"touch {}\"\n\tbranch = --upload-pack=touch {}\n",
            sentinel.display(),
            sentinel.display()
        );
        fs::write(harness.work_dir.join(".gitmodules"), gitmodules)
            .expect("Failed to write .gitmodules");

        harness
            .run_submod(&[
                "generate-config",
                "--from-setup",
                "--output",
                "out.toml",
                "--force",
            ])
            .expect("Failed to run submod");

        assert!(
            !sentinel.exists(),
            "generate-config must not execute values read from .gitmodules"
        );

        // Non-vacuity: the hostile entry was actually parsed and captured as data.
        let generated = fs::read_to_string(harness.work_dir.join("out.toml"))
            .expect("generate-config should have written the output config");
        assert!(
            generated.contains("ext::sh -c"),
            "expected the hostile url to be captured as inert data, got:\n{generated}"
        );
    }
}
