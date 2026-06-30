// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

//! Common utilities for integration tests

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::TempDir;

/// Test harness for integration tests
pub struct TestHarness {
    /// Temporary directory for test operations
    pub temp_dir: TempDir,
    /// Path to the compiled submod binary
    pub submod_bin: PathBuf,
    /// Working directory within `temp_dir`
    pub work_dir: PathBuf,
    /// Per-test git global config file (isolates tests from each other and from the user's ~/.gitconfig)
    git_config_global: PathBuf,
}

impl TestHarness {
    /// Create a new test harness
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let work_dir = temp_dir.path().join("workspace");
        fs::create_dir_all(&work_dir)?;

        // Use the binary path set by cargo at compile time. This ensures integration
        // tests use the same binary that cargo (and cargo-llvm-cov) built — including
        // the instrumented binary when running under coverage. Manually calling
        // `cargo build` here would produce a non-instrumented binary in target/debug/,
        // causing all subprocess-based tests to report zero coverage.
        let submod_bin = PathBuf::from(env!("CARGO_BIN_EXE_submod"));

        // Create a per-test global gitconfig so parallel tests don't race on ~/.gitconfig.
        // Pre-populate with protocol.file.allow=always (required for file:// submodule URLs).
        let git_config_global = temp_dir.path().join("gitconfig");
        fs::write(
            &git_config_global,
            "[protocol \"file\"]\n\tallow = always\n",
        )?;

        Ok(Self {
            temp_dir,
            submod_bin,
            work_dir,
            git_config_global,
        })
    }

    /// Return a `Command` for git with per-test config isolation.
    ///
    /// Sets `GIT_CONFIG_GLOBAL` to a test-local file and `GIT_CONFIG_SYSTEM` to
    /// `/dev/null` so that tests never read or write the real user/system config.
    fn git_cmd(&self) -> Command {
        let mut cmd = Command::new("git");
        cmd.env("GIT_CONFIG_GLOBAL", &self.git_config_global);
        cmd.env("GIT_CONFIG_SYSTEM", "/dev/null");
        cmd
    }

    /// Initialize a git repository in the working directory
    pub fn init_git_repo(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Use git commands for cleanup instead of direct filesystem operations
        let _ = self
            .git_cmd()
            .args(["submodule", "deinit", "--all", "-f"])
            .current_dir(&self.work_dir)
            .output();

        let _ = self
            .git_cmd()
            .args(["clean", "-fdx"])
            .current_dir(&self.work_dir)
            .output();
        let output = self
            .git_cmd()
            .args(["init"])
            .current_dir(&self.work_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to init git repo: {stderr}").into());
        }

        // Ensure we're on the main branch
        self.git_cmd()
            .args(["checkout", "-b", "main"])
            .current_dir(&self.work_dir)
            .output()?;

        // Configure git user for tests
        self.git_cmd()
            .args(["config", "user.name", "Test User"])
            .current_dir(&self.work_dir)
            .output()?;

        self.git_cmd()
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&self.work_dir)
            .output()?;

        // protocol.file.allow=always is set in the per-test global gitconfig
        // (created in TestHarness::new), so no --global write needed here.
        self.git_cmd()
            .args(["config", "protocol.file.allow", "always"])
            .current_dir(&self.work_dir)
            .output()?;

        // Create initial commit
        fs::write(self.work_dir.join("README.md"), "# Test Repository\n")?;

        self.git_cmd()
            .args(["add", "README.md"])
            .current_dir(&self.work_dir)
            .output()?;

        self.git_cmd()
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&self.work_dir)
            .output()?;

        Ok(())
    }

    /// Create a test remote repository
    pub fn create_test_remote(&self, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let remote_dir = self.temp_dir.path().join(format!("{name}.git"));

        // Initialize bare repository
        self.git_cmd()
            .args(["init", "--bare"])
            .arg(&remote_dir)
            .output()?;

        // Set the default branch to main for the bare repository
        self.git_cmd()
            .args(["symbolic-ref", "HEAD", "refs/heads/main"])
            .current_dir(&remote_dir)
            .output()?;

        // Create a working copy to add content
        let work_copy = self.temp_dir.path().join(format!("{name}_work"));
        self.git_cmd().args(["init"]).arg(&work_copy).output()?;

        // Set the default branch to main for the working copy
        self.git_cmd()
            .args(["checkout", "-b", "main"])
            .current_dir(&work_copy)
            .output()?;

        // Set up remote
        self.git_cmd()
            .args(["remote", "add", "origin", remote_dir.to_str().unwrap()])
            .current_dir(&work_copy)
            .output()?;

        // Add some content
        fs::create_dir_all(work_copy.join("src"))?;
        fs::create_dir_all(work_copy.join("docs"))?;
        fs::create_dir_all(work_copy.join("include"))?;

        fs::write(
            work_copy.join("src").join("main.c"),
            "#include <stdio.h>\nint main() { return 0; }\n",
        )?;
        fs::write(
            work_copy.join("docs").join("README.md"),
            "# Documentation\n",
        )?;
        fs::write(
            work_copy.join("include").join("header.h"),
            "#ifndef HEADER_H\n#define HEADER_H\n#endif\n",
        )?;
        fs::write(work_copy.join("LICENSE"), "MIT License\n")?;

        // Configure git and commit
        self.git_cmd()
            .args(["config", "user.name", "Test User"])
            .current_dir(&work_copy)
            .output()?;

        self.git_cmd()
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&work_copy)
            .output()?;

        self.git_cmd()
            .args(["add", "."])
            .current_dir(&work_copy)
            .output()?;

        self.git_cmd()
            .args(["commit", "-m", "Add test content"])
            .current_dir(&work_copy)
            .output()?;

        let push_output = self
            .git_cmd()
            .args(["push", "--no-verify", "origin", "main"])
            .current_dir(&work_copy)
            .output()?;

        // Check if push was successful
        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            return Err(format!("Failed to push to remote: {stderr}").into());
        }

        Ok(remote_dir)
    }

    /// Advance the bare remote created by [`create_test_remote`] by one commit on
    /// `main`, returning the new commit's full SHA. Reuses the working copy left
    /// at `<temp>/<name>_work` so callers can move the remote forward and then
    /// observe how `update`/`sync` react to a remote that has new history.
    pub fn advance_test_remote(&self, name: &str) -> Result<String, Box<dyn std::error::Error>> {
        let work_copy = self.temp_dir.path().join(format!("{name}_work"));

        fs::write(work_copy.join("ADVANCE.txt"), format!("advanced {name}\n"))?;
        self.git_cmd()
            .args(["add", "."])
            .current_dir(&work_copy)
            .output()?;
        self.git_cmd()
            .args(["commit", "-m", "Advance remote"])
            .current_dir(&work_copy)
            .output()?;

        let push_output = self
            .git_cmd()
            .args(["push", "--no-verify", "origin", "main"])
            .current_dir(&work_copy)
            .output()?;
        if !push_output.status.success() {
            let stderr = String::from_utf8_lossy(&push_output.stderr);
            return Err(format!("Failed to push advance to remote: {stderr}").into());
        }

        let rev = self
            .git_cmd()
            .args(["rev-parse", "HEAD"])
            .current_dir(&work_copy)
            .output()?;
        Ok(String::from_utf8_lossy(&rev.stdout).trim().to_string())
    }

    /// Run submod command with given arguments
    pub fn run_submod(
        &self,
        args: &[&str],
    ) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        // NOTE: arguments containing an interior NUL byte cannot be passed to a
        // process at all — std's Command rejects them before spawn, so `.output()`
        // below returns an Err. We deliberately do NOT fabricate a fake failure
        // here; tests assert the real process-boundary rejection.
        let output = Command::new(&self.submod_bin)
            .args(args)
            .current_dir(&self.work_dir)
            .env("GIT_CONFIG_GLOBAL", &self.git_config_global)
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .output()?;

        Ok(output)
    }

    /// Run submod command and expect success
    pub fn run_submod_success(&self, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
        let output = self.run_submod(args)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!("Command failed:\nstdout: {stdout}\nstderr: {stderr}").into());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get path to config file in work directory
    pub fn config_path(&self) -> PathBuf {
        self.work_dir.join("submod.toml")
    }

    /// Create a config file with given content
    #[allow(dead_code)] // Used by integration tests; required for test harness
    pub fn create_config(&self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(self.config_path(), content)?;
        Ok(())
    }

    /// Read config file content
    #[allow(dead_code)] // Used by integration tests; required for test harness
    pub fn read_config(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(std::fs::read_to_string(self.config_path())?)
    }

    /// Check if a directory exists in the work directory
    #[allow(dead_code)] // Used by integration tests; required for test harness
    pub fn dir_exists(&self, path: &str) -> bool {
        self.work_dir.join(path).exists()
    }

    /// Check if a file exists in the work directory
    #[allow(dead_code)] // Used by integration tests; required for test harness
    pub fn file_exists(&self, path: &str) -> bool {
        self.work_dir.join(path).is_file()
    }

    /// Run a git command in the work directory (with per-test config isolation) and
    /// return its stdout, trimmed. Panics if git cannot be spawned. Intended for
    /// asserting on real git state rather than on printed output or `submod.toml` text.
    #[allow(dead_code)] // Used by integration tests; required for test harness
    pub fn git_stdout(&self, args: &[&str]) -> String {
        let output = self
            .git_cmd()
            .args(args)
            .current_dir(&self.work_dir)
            .output()
            .expect("failed to run git command");
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    }

    /// Return the index gitlink mode for `path` (e.g. `"160000"`), or `None` if the
    /// path is not staged. A registered submodule is staged as mode `160000`.
    #[allow(dead_code)] // Used by integration tests; required for test harness
    pub fn index_gitlink_mode(&self, path: &str) -> Option<String> {
        let out = self.git_stdout(&["ls-files", "--stage", "--", path]);
        if out.is_empty() {
            return None;
        }
        out.split_whitespace().next().map(str::to_string)
    }

    /// Return all `submodule.*` entries from the superproject's `.git/config`
    /// (`git config --get-regexp ^submodule\.`), or an empty string if there are none.
    #[allow(dead_code)] // Used by integration tests; required for test harness
    pub fn submodule_config_entries(&self) -> String {
        self.git_stdout(&["config", "--get-regexp", r"^submodule\."])
    }

    /// Return all `submodule.*` entries from the `.gitmodules` file, or an empty string
    /// if the file is absent/empty. Queries via git so the result reflects parsed config,
    /// not raw text.
    #[allow(dead_code)] // Used by integration tests; required for test harness
    pub fn gitmodules_entries(&self) -> String {
        if !self.work_dir.join(".gitmodules").exists() {
            return String::new();
        }
        self.git_stdout(&[
            "config",
            "--file",
            ".gitmodules",
            "--get-regexp",
            r"^submodule\.",
        ])
    }

    /// Check whether the per-submodule git directory `.git/modules/<subpath>` exists.
    /// This is the artifact whose lingering presence can block a clean re-add.
    #[allow(dead_code)] // Used by integration tests; required for test harness
    pub fn git_modules_dir_exists(&self, subpath: &str) -> bool {
        self.work_dir
            .join(".git")
            .join("modules")
            .join(subpath)
            .exists()
    }

    /// Find the actual sparse-checkout file path, handling gitlinks
    #[allow(dead_code)] // Used by integration tests; required for test harness
    pub fn get_sparse_checkout_file_path(&self, submodule_path: &str) -> std::path::PathBuf {
        let git_path = self.work_dir.join(submodule_path).join(".git");

        if git_path.is_dir() {
            // Regular git repository
            return git_path.join("info").join("sparse-checkout");
        } else if git_path.is_file() {
            // Gitlink - read the file to get the actual git directory
            if let Ok(content) = std::fs::read_to_string(&git_path)
                && let Some(git_dir_line) =
                    content.lines().find(|line| line.starts_with("gitdir: "))
            {
                let git_dir_path = git_dir_line.strip_prefix("gitdir: ").unwrap().trim();

                // Path might be relative to the submodule directory
                let absolute_path = if std::path::Path::new(git_dir_path).is_absolute() {
                    std::path::PathBuf::from(git_dir_path)
                } else {
                    self.work_dir.join(submodule_path).join(git_dir_path)
                };

                let sparse_file = absolute_path.join("info").join("sparse-checkout");

                // Check if the file exists in the actual git directory
                if sparse_file.exists() {
                    return sparse_file;
                }
            }
        }

        // For submodules, try multiple possible locations
        let locations = vec![
            // Try the gitlink location first (actual git directory)
            self.work_dir
                .join(submodule_path)
                .join(".git")
                .join("info")
                .join("sparse-checkout"),
            // Try relative to main repo's .git/modules
            self.work_dir
                .join(".git")
                .join("modules")
                .join(submodule_path)
                .join("info")
                .join("sparse-checkout"),
            // Try with just the last component of the path
            self.work_dir
                .join(".git")
                .join("modules")
                .join(
                    std::path::Path::new(submodule_path)
                        .file_name()
                        .unwrap_or_else(|| std::ffi::OsStr::new("")),
                )
                .join("info")
                .join("sparse-checkout"),
        ];

        // Return the first location that exists, or the first one as fallback
        for location in &locations {
            if location.exists() {
                return location.clone();
            }
        }

        // Fallback to the expected location for tests
        locations[0].clone()
    }

    /// Create a complex test repository with multiple branches and tags
    #[allow(dead_code)] // Used by integration tests; required for test harness
    pub fn create_complex_remote(
        &self,
        name: &str,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let remote_dir = self.temp_dir.path().join(format!("{name}.git"));

        // Initialize bare repository
        self.git_cmd()
            .args(["init", "--bare"])
            .arg(&remote_dir)
            .output()?;

        // Set the default branch to main for the bare repository
        self.git_cmd()
            .args(["symbolic-ref", "HEAD", "refs/heads/main"])
            .current_dir(&remote_dir)
            .output()?;

        // Create a working copy to add content
        let work_copy = self.temp_dir.path().join(format!("{name}_work"));
        self.git_cmd().args(["init"]).arg(&work_copy).output()?;

        // Set up the main branch and remote
        self.git_cmd()
            .args(["checkout", "-b", "main"])
            .current_dir(&work_copy)
            .output()?;

        self.git_cmd()
            .args(["remote", "add", "origin", remote_dir.to_str().unwrap()])
            .current_dir(&work_copy)
            .output()?;

        // Configure git
        self.git_cmd()
            .args(["config", "user.name", "Test User"])
            .current_dir(&work_copy)
            .output()?;

        self.git_cmd()
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&work_copy)
            .output()?;

        // Create main branch content
        fs::create_dir_all(work_copy.join("src"))?;
        fs::create_dir_all(work_copy.join("docs"))?;
        fs::create_dir_all(work_copy.join("tests"))?;
        fs::create_dir_all(work_copy.join("examples"))?;

        fs::write(
            work_copy.join("src").join("lib.rs"),
            "// Main library code\npub fn hello() { println!(\"Hello!\"); }\n",
        )?;
        fs::write(
            work_copy.join("docs").join("API.md"),
            "# API Documentation\n",
        )?;
        fs::write(
            work_copy.join("tests").join("test.rs"),
            "// Tests\n#[test]\nfn test_basic() { assert!(true); }\n",
        )?;
        fs::write(
            work_copy.join("examples").join("basic.rs"),
            "// Example\nfn main() { println!(\"Example\"); }\n",
        )?;
        fs::write(
            work_copy.join("Cargo.toml"),
            "[package]\nname = \"test-lib\"\nversion = \"0.1.0\"\n",
        )?;
        fs::write(work_copy.join("README.md"), "# Test Library\n")?;

        self.git_cmd()
            .args(["add", "."])
            .current_dir(&work_copy)
            .output()?;

        self.git_cmd()
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&work_copy)
            .output()?;

        // Create a development branch
        self.git_cmd()
            .args(["checkout", "-b", "develop"])
            .current_dir(&work_copy)
            .output()?;

        fs::write(
            work_copy.join("src").join("dev.rs"),
            "// Development code\npub fn dev_feature() {}\n",
        )?;

        self.git_cmd()
            .args(["add", "."])
            .current_dir(&work_copy)
            .output()?;

        self.git_cmd()
            .args(["commit", "-m", "Add dev features"])
            .current_dir(&work_copy)
            .output()?;

        // Create a tag
        self.git_cmd()
            .args(["tag", "v0.1.0"])
            .current_dir(&work_copy)
            .output()?;

        // Push everything with error checking
        let push_main = self
            .git_cmd()
            .args(["push", "origin", "main"])
            .current_dir(&work_copy)
            .output()?;

        if !push_main.status.success() {
            let stderr = String::from_utf8_lossy(&push_main.stderr);
            return Err(format!("Failed to push main branch: {stderr}").into());
        }

        let push_develop = self
            .git_cmd()
            .args(["push", "origin", "develop"])
            .current_dir(&work_copy)
            .output()?;

        if !push_develop.status.success() {
            let stderr = String::from_utf8_lossy(&push_develop.stderr);
            return Err(format!("Failed to push develop branch: {stderr}").into());
        }

        let push_tags = self
            .git_cmd()
            .args(["push", "origin", "--tags"])
            .current_dir(&work_copy)
            .output()?;

        if !push_tags.status.success() {
            let stderr = String::from_utf8_lossy(&push_tags.stderr);
            return Err(format!("Failed to push tags: {stderr}").into());
        }

        Ok(remote_dir)
    }
}
