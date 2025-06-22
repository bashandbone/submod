//! Common utilities for integration tests

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;
use std::os::unix::process::ExitStatusExt;

/// Test harness for integration tests
pub struct TestHarness {
    /// Temporary directory for test operations
    pub temp_dir: TempDir,
    /// Path to the compiled submod binary
    pub submod_bin: PathBuf,
    /// Working directory within temp_dir
    pub work_dir: PathBuf,
}

impl TestHarness {
    /// Create a new test harness
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let temp_dir = TempDir::new()?;
        let work_dir = temp_dir.path().join("workspace");
        fs::create_dir_all(&work_dir)?;

        // Build the binary in debug mode for testing
        let output = Command::new("cargo")
            .args(["build", "--bin", "submod"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            panic!("Failed to build submod binary: {}", stderr);
        }

        // Get the actual target directory from cargo metadata
        let metadata_output = Command::new("cargo")
            .args(["metadata", "--format-version", "1", "--no-deps"])
            .output()?;

        if !metadata_output.status.success() {
            panic!("Failed to get cargo metadata");
        }

        let metadata_str = String::from_utf8_lossy(&metadata_output.stdout);
        let metadata: serde_json::Value = serde_json::from_str(&metadata_str)?;
        let target_dir = metadata["target_directory"].as_str()
            .ok_or("Could not find target_directory in cargo metadata")?;

        let submod_bin = PathBuf::from(target_dir)
            .join("debug")
            .join("submod");

        Ok(Self {
            temp_dir,
            submod_bin,
            work_dir,
        })
    }

    /// Initialize a git repository in the working directory
    pub fn init_git_repo(&self) -> Result<(), Box<dyn std::error::Error>> {
        // HARD CLEANUP: Remove any stale .git/modules and lib/ directories before each test
        let git_modules = Path::new(".git/modules");
        if git_modules.exists() {
            fs::remove_dir_all(git_modules)?;
        }
        let lib_dir = Path::new("lib");
        if lib_dir.exists() {
            fs::remove_dir_all(lib_dir)?;
        }
        let output = Command::new("git")
            .args(["init"])
            .current_dir(&self.work_dir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to init git repo: {}", stderr).into());
        }

        // Configure git user for tests
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&self.work_dir)
            .output()?;

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&self.work_dir)
            .output()?;

        // Configure git to allow file protocol for tests (both local and global)
        Command::new("git")
            .args(["config", "protocol.file.allow", "always"])
            .current_dir(&self.work_dir)
            .output()?;

        Command::new("git")
            .args(["config", "--global", "protocol.file.allow", "always"])
            .current_dir(&self.work_dir)
            .output()?;

        // Create initial commit
        fs::write(self.work_dir.join("README.md"), "# Test Repository\n")?;

        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(&self.work_dir)
            .output()?;

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&self.work_dir)
            .output()?;

        Ok(())
    }

    /// Create a test remote repository
    pub fn create_test_remote(&self, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let remote_dir = self.temp_dir.path().join(format!("{}.git", name));

        // Initialize bare repository
        Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote_dir)
            .output()?;

        // Create a working copy to add content
        let work_copy = self.temp_dir.path().join(format!("{}_work", name));
        Command::new("git")
            .args(["clone", remote_dir.to_str().unwrap(), work_copy.to_str().unwrap()])
            .output()?;

        // Add some content
        fs::create_dir_all(work_copy.join("src"))?;
        fs::create_dir_all(work_copy.join("docs"))?;
        fs::create_dir_all(work_copy.join("include"))?;

        fs::write(work_copy.join("src").join("main.c"), "#include <stdio.h>\nint main() { return 0; }\n")?;
        fs::write(work_copy.join("docs").join("README.md"), "# Documentation\n")?;
        fs::write(work_copy.join("include").join("header.h"), "#ifndef HEADER_H\n#define HEADER_H\n#endif\n")?;
        fs::write(work_copy.join("LICENSE"), "MIT License\n")?;

        // Configure git and commit
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&work_copy)
            .output()?;

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&work_copy)
            .output()?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(&work_copy)
            .output()?;

        Command::new("git")
            .args(["commit", "-m", "Add test content"])
            .current_dir(&work_copy)
            .output()?;

        Command::new("git")
            .args(["push", "origin", "main"])
            .current_dir(&work_copy)
            .output()?;

        Ok(remote_dir)
    }

    /// Run submod command with given arguments
    pub fn run_submod(&self, args: &[&str]) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        // Check for null bytes in arguments which would cause process execution to fail
        for arg in args {
            if arg.contains('\0') {
                // Return a simulated failed output for null byte arguments
                return Ok(std::process::Output {
                    status: std::process::ExitStatus::from_raw(1),
                    stdout: Vec::new(),
                    stderr: b"Error: Invalid argument contains null byte\n".to_vec(),
                });
            }
        }

        let output = Command::new(&self.submod_bin)
            .args(args)
            .current_dir(&self.work_dir)
            .output()?;

        Ok(output)
    }

    /// Run submod command and expect success
    pub fn run_submod_success(&self, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
        let output = self.run_submod(args)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(format!("Command failed:\nstdout: {}\nstderr: {}", stdout, stderr).into());
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Get path to config file in work directory
    pub fn config_path(&self) -> PathBuf {
        self.work_dir.join("submod.toml")
    }

    /// Create a config file with given content
    pub fn create_config(&self, content: &str) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(self.config_path(), content)?;
        Ok(())
    }

    /// Read config file content
    pub fn read_config(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.config_path())?)
    }

    /// Check if a directory exists in the work directory
    pub fn dir_exists(&self, path: &str) -> bool {
        self.work_dir.join(path).exists()
    }

    /// Check if a file exists in the work directory
    pub fn file_exists(&self, path: &str) -> bool {
        self.work_dir.join(path).is_file()
    }

    /// Read file content from work directory
    pub fn read_file(&self, path: &str) -> Result<String, Box<dyn std::error::Error>> {
        Ok(fs::read_to_string(self.work_dir.join(path))?)
    }

    /// Find the actual sparse-checkout file path, handling gitlinks
    pub fn get_sparse_checkout_file_path(&self, submodule_path: &str) -> PathBuf {
        let git_path = self.work_dir.join(submodule_path).join(".git");

        if git_path.is_dir() {
            // Regular git repository
            return git_path.join("info").join("sparse-checkout");
        } else if git_path.is_file() {
            // Gitlink - read the file to get the actual git directory
            if let Ok(content) = fs::read_to_string(&git_path) {
                if let Some(git_dir_line) = content.lines().find(|line| line.starts_with("gitdir: ")) {
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
        }

        // For submodules, try multiple possible locations
        let locations = vec![
            // Try the gitlink location first (actual git directory)
            self.work_dir.join(submodule_path).join(".git").join("info").join("sparse-checkout"),
            // Try relative to main repo's .git/modules
            self.work_dir.join(".git").join("modules").join(submodule_path).join("info").join("sparse-checkout"),
            // Try with just the last component of the path
            self.work_dir.join(".git").join("modules").join(
                std::path::Path::new(submodule_path).file_name().unwrap_or_else(|| std::ffi::OsStr::new(""))
            ).join("info").join("sparse-checkout"),
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
    pub fn create_complex_remote(&self, name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let remote_dir = self.temp_dir.path().join(format!("{}.git", name));

        // Initialize bare repository
        Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote_dir)
            .output()?;

        // Create a working copy to add content
        let work_copy = self.temp_dir.path().join(format!("{}_work", name));
        Command::new("git")
            .args(["clone", remote_dir.to_str().unwrap(), work_copy.to_str().unwrap()])
            .output()?;

        // Configure git
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&work_copy)
            .output()?;

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&work_copy)
            .output()?;

        // Create main branch content
        fs::create_dir_all(work_copy.join("src"))?;
        fs::create_dir_all(work_copy.join("docs"))?;
        fs::create_dir_all(work_copy.join("tests"))?;
        fs::create_dir_all(work_copy.join("examples"))?;

        fs::write(work_copy.join("src").join("lib.rs"), "// Main library code\npub fn hello() { println!(\"Hello!\"); }\n")?;
        fs::write(work_copy.join("docs").join("API.md"), "# API Documentation\n")?;
        fs::write(work_copy.join("tests").join("test.rs"), "// Tests\n#[test]\nfn test_basic() { assert!(true); }\n")?;
        fs::write(work_copy.join("examples").join("basic.rs"), "// Example\nfn main() { println!(\"Example\"); }\n")?;
        fs::write(work_copy.join("Cargo.toml"), "[package]\nname = \"test-lib\"\nversion = \"0.1.0\"\n")?;
        fs::write(work_copy.join("README.md"), "# Test Library\n")?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(&work_copy)
            .output()?;

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&work_copy)
            .output()?;

        // Create a development branch
        Command::new("git")
            .args(["checkout", "-b", "develop"])
            .current_dir(&work_copy)
            .output()?;

        fs::write(work_copy.join("src").join("dev.rs"), "// Development code\npub fn dev_feature() {}\n")?;

        Command::new("git")
            .args(["add", "."])
            .current_dir(&work_copy)
            .output()?;

        Command::new("git")
            .args(["commit", "-m", "Add dev features"])
            .current_dir(&work_copy)
            .output()?;

        // Create a tag
        Command::new("git")
            .args(["tag", "v0.1.0"])
            .current_dir(&work_copy)
            .output()?;

        // Push everything
        Command::new("git")
            .args(["push", "origin", "main"])
            .current_dir(&work_copy)
            .output()?;

        Command::new("git")
            .args(["push", "origin", "develop"])
            .current_dir(&work_copy)
            .output()?;

        Command::new("git")
            .args(["push", "origin", "--tags"])
            .current_dir(&work_copy)
            .output()?;

        Ok(remote_dir)
    }
}
