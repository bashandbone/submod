// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Security tests to ensure robustness against various attack vectors.

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

        // A path starting with a hyphen that could be a git flag.
        // This should be handled as a literal path component, not interpreted
        // as an option to git or to any cleanup command that operates on paths.
        let malicious_path = "-c";

        harness.run_submod_success(&[
            "add",
            &remote_url,
            "--name",
            "hyphen-sub",
            "--path",
            malicious_path,
        ]).expect("Failed to add submodule with hyphenated path");

        // Verify the submodule was actually created at the requested path.
        assert!(harness.dir_exists("-c"));
    }

    #[test]
    fn test_sparse_checkout_with_hyphen_path() {
        let harness = TestHarness::new().expect("Failed to create test harness");
        harness.init_git_repo().expect("Failed to init git repo");

        let remote_repo = harness
            .create_complex_remote("sparse-hyphen")
            .expect("Failed to create remote");
        let remote_url = format!("file://{}", remote_repo.display());

        // Path starting with hyphen
        let path = "-sparse";

        // Ensure the directory exists to trigger the CLI fallback in apply_sparse_checkout if needed,
        // although apply_sparse_checkout is usually called after gix/git2 which might fail or be bypassed.

        harness.run_submod_success(&[
            "add",
            &remote_url,
            "--name",
            "sparse-hyphen",
            "--path",
            path,
            "--sparse-paths",
            "src",
        ]).expect("Failed to add submodule with hyphenated path");

        // Verify it worked
        assert!(harness.dir_exists("-sparse/src"));
    }
}
