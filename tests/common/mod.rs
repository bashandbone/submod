//! Common utilities for integration tests

use std::env;
use std::fs;
use std::path::{PathBuf};
use std::process::Command;
use tempfile::TempDir;

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

        let submod_bin = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
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
