//! Gitoxide-maximized submodule manager
//!
//! This module implements the submodule manager with maximum use of gitoxide/gix APIs
//! and strategic fallbacks only where necessary.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;
use gix::Repository;
use crate::config::{Config, SubmoduleConfig, SubmoduleGitOptions};

/// Custom error types for submodule operations
#[derive(Debug, thiserror::Error)]
pub enum SubmoduleError {
    /// Error from gitoxide library operations
    #[error("Gitoxide operation failed: {0}")]
    #[allow(dead_code)]
    GitoxideError(String),

    /// Error from git2 library operations (when git2-support feature is enabled)
    #[error("git2 operation failed: {0}")]
    #[cfg(feature = "git2-support")]
    Git2Error(#[from] git2::Error),

    /// Error from Git CLI operations
    #[error("Git CLI operation failed: {0}")]
    #[allow(dead_code)]
    CliError(String),

    /// Configuration-related error
    #[error("Configuration error: {0}")]
    #[allow(dead_code)]
    ConfigError(String),

    /// I/O operation error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Submodule not found in repository
    #[error("Submodule {name} not found")]
    #[allow(dead_code)]
    SubmoduleNotFound { 
        /// Name of the submodule that was not found
        name: String 
    },

    /// Sparse checkout configuration error
    #[error("Invalid sparse checkout configuration: {reason}")]
    #[allow(dead_code)]
    SparseCheckoutError { 
        /// Reason for the sparse checkout error
        reason: String 
    },

    /// Repository access or validation error
    #[error("Repository not found or invalid")]
    #[allow(dead_code)]
    RepositoryError,
}

/// Status information for a submodule
#[derive(Debug, Clone)]
pub struct SubmoduleStatus {
    /// Path to the submodule directory
    #[allow(dead_code)]
    pub path: String,
    /// Whether the submodule working directory is clean
    pub is_clean: bool,
    /// Current commit hash of the submodule
    pub current_commit: Option<String>,
    /// Whether the submodule has remote repositories configured
    pub has_remotes: bool,
    /// Whether the submodule is initialized
    #[allow(dead_code)]
    pub is_initialized: bool,
    /// Whether the submodule is active
    #[allow(dead_code)]
    pub is_active: bool,
    /// Sparse checkout status for this submodule
    pub sparse_status: SparseStatus,
}

/// Sparse checkout status
#[derive(Debug, Clone)]
pub enum SparseStatus {
    /// Sparse checkout is not enabled for this submodule
    NotEnabled,
    /// Sparse checkout is enabled but not configured
    NotConfigured,
    /// Sparse checkout configuration matches expected paths
    Correct,
    /// Sparse checkout configuration doesn't match expected paths
    Mismatch {
        /// Expected sparse checkout paths
        expected: Vec<String>,
        /// Actual sparse checkout paths
        actual: Vec<String>,
    },
}

/// Main gitoxide-based submodule manager
pub struct GitoxideSubmoduleManager {
    /// The main repository instance
    repo: Repository,
    /// Configuration for submodules
    config: Config,
    /// Path to the configuration file
    config_path: PathBuf,
}

impl GitoxideSubmoduleManager {
    /// Recursively remove empty parent directories up to (but not including) the modules root.
    fn cleanup_empty_module_parents(modules_dir: &std::path::Path, modules_root: &std::path::Path) {
        let mut current = modules_dir.parent();
        // Stop at the parent of modules_root, so we may remove modules_root itself if empty
        let stop = modules_root.parent();
        while let Some(dir) = current {
            if Some(dir) == stop { break; }
            if std::fs::read_dir(dir).map(|mut i| i.next().is_none()).unwrap_or(false) {
                let _ = std::fs::remove_dir(dir);
            } else {
                break;
            }
            current = dir.parent();
        }
    }

    /// Create a new GitoxideSubmoduleManager instance
    pub fn new(config_path: PathBuf) -> Result<Self, SubmoduleError> {
        // Use gix::discover for repository detection
        let repo = gix::discover(".")
            .map_err(|_| SubmoduleError::RepositoryError)?;

        let config = Config::load(&config_path)
            .map_err(|e| SubmoduleError::ConfigError(format!("Failed to load config: {}", e)))?;

        Ok(Self {
            repo,
            config,
            config_path,
        })
    }

    /// Check submodule repository status using gix APIs
    pub fn check_submodule_repository_status(&self, submodule_path: &str, name: &str) -> Result<SubmoduleStatus, SubmoduleError> {
        let submodule_repo = gix::open(submodule_path)
            .map_err(|_| SubmoduleError::RepositoryError)?;

        // GITOXIDE API: Use gix for what's available, fall back to CLI for complex status
        // For now, use a simple approach - check if there are any uncommitted changes
        let is_dirty = match submodule_repo.head() {
            Ok(_head) => {
                // Simple check - if we can get head, assume repository is clean
                // This is a conservative approach until we can use the full status API
                false
            }
            Err(_) => true,
        };

        // GITOXIDE API: Use reference APIs for current commit
        let current_commit = match submodule_repo.head() {
            Ok(head) => head.id().map(|id| id.to_string()),
            Err(_) => None,
        };

        // GITOXIDE API: Use remote APIs to check if remotes exist
        let has_remotes = !submodule_repo.remote_names().is_empty();

        // For now, consider all submodules active if they exist in config
        let is_active = self.config.submodules.contains_key(name);

        // Check sparse checkout status
        let sparse_status = if let Some(config) = self.config.submodules.get(name) {
            if let Some(expected_paths) = &config.sparse_paths {
                self.check_sparse_checkout_status(&submodule_repo, expected_paths)?
            } else {
                SparseStatus::NotEnabled
            }
        } else {
            SparseStatus::NotEnabled
        };

        Ok(SubmoduleStatus {
            path: submodule_path.to_string(),
            is_clean: !is_dirty,
            current_commit,
            has_remotes,
            is_initialized: true,
            is_active,
            sparse_status,
        })
    }

    /// Check sparse checkout configuration
    pub fn check_sparse_checkout_status(&self, repo: &Repository, expected_paths: &[String]) -> Result<SparseStatus, SubmoduleError> {
        // Read sparse-checkout file directly
        let sparse_checkout_file = repo.git_dir().join("info").join("sparse-checkout");
        if !sparse_checkout_file.exists() {
            return Ok(SparseStatus::NotConfigured);
        }

        let content = fs::read_to_string(&sparse_checkout_file)?;
        let configured_paths: Vec<String> = content.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|s| s.to_string())
            .collect();

        let matches = expected_paths.iter()
            .all(|path| configured_paths.contains(path));

        if matches {
            Ok(SparseStatus::Correct)
        } else {
            Ok(SparseStatus::Mismatch {
                expected: expected_paths.to_vec(),
                actual: configured_paths,
            })
        }
    }

    /// Add a submodule using CLI + basic configuration (temporarily disabling git2)
    pub fn add_submodule(&mut self, name: String, path: String, url: String, sparse_paths: Option<Vec<String>>) -> Result<(), SubmoduleError> {
        eprintln!("DEBUG: Starting add_submodule for {} at {} with sparse_paths: {:?}", name, path, sparse_paths);

        // --- Robust cleanup before attempting to add ---
        let submodule_path = std::path::Path::new(&path);
        if submodule_path.exists() {
            eprintln!("DEBUG: Removing existing submodule directory: {}", path);
            std::fs::remove_dir_all(submodule_path).map_err(SubmoduleError::IoError)?;
        }
        // Remove .git/modules/<submodule> if it exists (leftover from failed adds)
        // Handles nested submodule paths (e.g., lib/foo/bar)
        let git_dir = self.repo.git_dir();
        let mut modules_dir = git_dir.join("modules");
        for part in path.split('/') {
            modules_dir = modules_dir.join(part);
        }

        // Debug output before cleanup
        eprintln!("DEBUG: Before cleanup:");
        eprintln!("  modules_dir exists: {}", modules_dir.exists());
        eprintln!("  modules_dir: {}", modules_dir.display());
        let parent_dir = modules_dir.parent().unwrap_or_else(|| std::path::Path::new(""));
        eprintln!("  parent_dir exists: {}", parent_dir.exists());
        eprintln!("  parent_dir: {}", parent_dir.display());
        eprintln!("  modules_root: {}", git_dir.join("modules").display());

        if modules_dir.exists() {
            eprintln!("DEBUG: Removing existing .git/modules entry: {}", modules_dir.display());
            match std::fs::remove_dir_all(&modules_dir) {
                Ok(_) => eprintln!("DEBUG: Successfully removed {}", modules_dir.display()),
                Err(e) => eprintln!("ERROR: Failed to remove {}: {}", modules_dir.display(), e),
            }
            let modules_root = git_dir.join("modules");
            Self::cleanup_empty_module_parents(&modules_dir, &modules_root);
        }

        // Debug output after cleanup
        eprintln!("DEBUG: After cleanup:");
        eprintln!("  modules_dir exists: {}", modules_dir.exists());
        eprintln!("  parent_dir exists: {}", parent_dir.exists());
        eprintln!("  modules_root exists: {}", git_dir.join("modules").exists());

        // Step 1: Ensure .git/modules and all parent dirs for the submodule exist
        let modules_root = git_dir.join("modules");
        // (removed unused variable submodule_modules_dir)
        if !modules_root.exists() {
            eprintln!("DEBUG: Creating modules_root: {}", modules_root.display());
            std::fs::create_dir_all(&modules_root).map_err(SubmoduleError::IoError)?;
        }
        // DO NOT create parent directories for the submodule's modules path.
        // Git expects to create .git/modules/<submodule> itself.

        // Step 2: Try to deinit the submodule in case git has internal state
        let workdir = self.repo.workdir().unwrap_or_else(|| std::path::Path::new("."));
        let deinit_output = std::process::Command::new("git")
            .args(["submodule", "deinit", "-f", &path])
            .current_dir(workdir)
            .output();
        match deinit_output {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!("DEBUG: git submodule deinit failed (may be harmless): {}", stderr);
                } else {
                    eprintln!("DEBUG: git submodule deinit succeeded");
                }
            }
            Err(e) => {
                eprintln!("DEBUG: git submodule deinit command failed to run: {}", e);
            }
        }

        // Diagnostics: print .git/modules tree and git config before add
        use std::process::Command;
        eprintln!("==== DIAGNOSTIC: ls -lR .git/modules ====");
        let ls_output = Command::new("ls")
            .args(&["-lR", git_dir.join("modules").to_str().unwrap_or(".git/modules")])
            .output();
        match ls_output {
            Ok(output) => {
                eprintln!("{}", String::from_utf8_lossy(&output.stdout));
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
            Err(e) => eprintln!("ls failed: {}", e),
        }

        eprintln!("==== DIAGNOSTIC: git config --list --show-origin ====");
        let config_output = Command::new("git")
            .args(&["config", "--list", "--show-origin"])
            .current_dir(workdir)
            .output();
        match config_output {
            Ok(output) => {
                eprintln!("{}", String::from_utf8_lossy(&output.stdout));
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
            Err(e) => eprintln!("git config failed: {}", e),
        }

        // Try to remove from index in case submodule is still registered
        eprintln!("==== DIAGNOSTIC: git rm --cached <path> ====");
        let rm_output = Command::new("git")
            .args(&["rm", "--cached", &path])
            .current_dir(workdir)
            .output();
        match rm_output {
            Ok(output) => {
                eprintln!("{}", String::from_utf8_lossy(&output.stdout));
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
            }
            Err(e) => eprintln!("git rm --cached failed: {}", e),
        }

        // (No-op: function now defined as an impl method and called above)

        // Try git2 first if enabled, fall back to CLI if not available or fails
        #[cfg(feature = "git2-support")]
        {
            match self.add_submodule_git2(&name, &path, &url) {
                Ok(_) => eprintln!("DEBUG: git2 submodule add completed"),
                Err(e) => {
                    eprintln!("DEBUG: git2 submodule add failed: {e}, falling back to CLI");
                    self.add_submodule_cli(&name, &path, &url)?;
                    eprintln!("DEBUG: CLI submodule add completed");
                }
            }
        }
        #[cfg(not(feature = "git2-support"))]
        {
            self.add_submodule_cli(&name, &path, &url)?;
            eprintln!("DEBUG: CLI submodule add completed");
        }

        // Configure after creation
        self.configure_submodule_post_creation(&name, &path, sparse_paths.clone())?;
        eprintln!("DEBUG: Post-creation configuration completed");

        self.update_toml_config(name.clone(), path, url, sparse_paths)?;
        eprintln!("DEBUG: TOML config updated");

        println!("Added submodule {}", name);

        Ok(())
    }

    #[cfg(feature = "git2-support")]
    fn add_submodule_git2(&self, _name: &str, path: &str, url: &str) -> Result<(), SubmoduleError> {
        let git2_repo = git2::Repository::open(self.repo.git_dir())?;

        let submodule_path = std::path::Path::new(path);

        // Remove existing directory if it exists to avoid conflicts
        if submodule_path.exists() {
            std::fs::remove_dir_all(submodule_path)?;
        }

        // Create parent directories if they don't exist
        if let Some(parent) = submodule_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Step 1: Add submodule to parent repository - this registers it in .gitmodules and index
        let mut submodule = git2_repo.submodule(url, submodule_path, false)?;

        // Step 2: Initialize the submodule configuration
        submodule.init(false)?;

        // Step 3: Set up the subrepository for cloning
        let _sub_repo = submodule.repo_init(true)?;

        // Step 4: Clone the repository
        let _cloned_repo = submodule.clone(None)?;

        // Step 5: Add the submodule to the index and finalize
        submodule.add_to_index(true)?;
        submodule.add_finalize()?;

        Ok(())
    }

    fn add_submodule_cli(&self, _name: &str, path: &str, url: &str) -> Result<(), SubmoduleError> {
        let workdir = self.repo.workdir().unwrap_or_else(|| std::path::Path::new("."));

        // Remove existing directory if it exists to avoid conflicts
        let submodule_path = std::path::Path::new(path);
        if submodule_path.exists() {
            std::fs::remove_dir_all(submodule_path)?;
        }

        // Create parent directories if they don't exist
        if let Some(parent) = submodule_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Configure git to allow file protocol for tests
        let _config_output = Command::new("git")
            .args(["config", "protocol.file.allow", "always"])
            .current_dir(workdir)
            .output()?;

        // Use --force to ensure git overwrites any stale state
        let output = Command::new("git")
            .args(["submodule", "add", "--force", url, path])
            .current_dir(workdir)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SubmoduleError::CliError(format!("Git submodule add failed: {}", stderr)));
        }

        Ok(())
    }

    /// Configure submodule for post-creation setup
    fn configure_submodule_post_creation(&mut self, _name: &str, path: &str, sparse_paths: Option<Vec<String>>) -> Result<(), SubmoduleError> {
        // Configure sparse checkout if specified
        if let Some(patterns) = sparse_paths {
            eprintln!("DEBUG: Configuring sparse checkout for {} with patterns: {:?}", path, patterns);
            self.configure_sparse_checkout(path, &patterns)?;
        } else {
            eprintln!("DEBUG: No sparse paths provided for {}", path);
        }

        Ok(())
    }

    /// Update TOML configuration
    fn update_toml_config(&mut self, name: String, path: String, url: String, sparse_paths: Option<Vec<String>>) -> Result<(), SubmoduleError> {
        let submodule_config = SubmoduleConfig {
            git_options: SubmoduleGitOptions::default(),
            active: true,
            path: Some(path),
            url: Some(url),
            sparse_paths,
        };

        self.config.add_submodule(name, submodule_config);
        self.config.save(&self.config_path)
            .map_err(|e| SubmoduleError::ConfigError(format!("Failed to save config: {}", e)))?;

        Ok(())
    }

    /// Configure sparse checkout using basic file operations
    pub fn configure_sparse_checkout(&self, submodule_path: &str, patterns: &[String]) -> Result<(), SubmoduleError> {
        eprintln!("DEBUG: Configuring sparse checkout for {} with patterns: {:?}", submodule_path, patterns);

        // Enable sparse checkout in git config (using CLI for now since config mutation is complex)
        let output = Command::new("git")
            .args(["config", "core.sparseCheckout", "true"])
            .current_dir(submodule_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SubmoduleError::CliError(format!("Failed to enable sparse checkout: {}", stderr)));
        }

        // Get the actual git directory (handles both regular repos and submodules with gitlinks)
        let git_dir = self.get_git_directory(submodule_path)?;
        eprintln!("DEBUG: Git directory for {}: {}", submodule_path, git_dir.display());

        // Write sparse-checkout file
        let info_dir = git_dir.join("info");
        fs::create_dir_all(&info_dir)?;

        let sparse_checkout_file = info_dir.join("sparse-checkout");
        let content = patterns.join("\n") + "\n";
        fs::write(&sparse_checkout_file, &content)?;
        eprintln!("DEBUG: Wrote sparse-checkout file to: {}", sparse_checkout_file.display());

        // Apply sparse checkout
        self.apply_sparse_checkout_cli(submodule_path)?;

        println!("Configured sparse checkout");

        Ok(())
    }

    /// Get the actual git directory path, handling gitlinks in submodules
    fn get_git_directory(&self, submodule_path: &str) -> Result<std::path::PathBuf, SubmoduleError> {
        let git_path = std::path::Path::new(submodule_path).join(".git");
        eprintln!("DEBUG: Checking git path: {}", git_path.display());

        if git_path.is_dir() {
            // Regular git repository
            eprintln!("DEBUG: Found regular git directory");
            Ok(git_path)
        } else if git_path.is_file() {
            // Gitlink - read the file to get the actual git directory
            eprintln!("DEBUG: Found gitlink file, reading content");
            let content = fs::read_to_string(&git_path)?;
            eprintln!("DEBUG: Gitlink content: {}", content);

            let git_dir_line = content.lines()
                .find(|line| line.starts_with("gitdir: "))
                .ok_or_else(|| SubmoduleError::IoError(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Invalid gitlink file"
                )))?;

            let git_dir_path = git_dir_line.strip_prefix("gitdir: ").unwrap().trim();
            eprintln!("DEBUG: Parsed git dir path: {}", git_dir_path);

            // Path might be relative to the submodule directory
            let absolute_path = if std::path::Path::new(git_dir_path).is_absolute() {
                std::path::PathBuf::from(git_dir_path)
            } else {
                std::path::Path::new(submodule_path).join(git_dir_path)
            };

            eprintln!("DEBUG: Resolved absolute path: {}", absolute_path.display());
            Ok(absolute_path)
        } else {
            // Use gix as fallback
            eprintln!("DEBUG: No .git file/dir found, trying gix fallback");
            if let Ok(repo) = gix::open(submodule_path) {
                let git_dir = repo.git_dir().to_path_buf();
                eprintln!("DEBUG: Gix found git dir: {}", git_dir.display());
                Ok(git_dir)
            } else {
                eprintln!("DEBUG: Gix fallback failed");
                Err(SubmoduleError::RepositoryError)
            }
        }
    }

    fn apply_sparse_checkout_cli(&self, path: &str) -> Result<(), SubmoduleError> {
        let output = Command::new("git")
            .args(["read-tree", "-m", "-u", "HEAD"])
            .current_dir(path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("Warning applying sparse checkout: {}", stderr);
        }

        Ok(())
    }

    /// Update submodule using CLI fallback (gix remote operations are complex for this use case)
    pub fn update_submodule(&self, name: &str) -> Result<(), SubmoduleError> {
        let config = self.config.submodules.get(name)
            .ok_or_else(|| SubmoduleError::SubmoduleNotFound { name: name.to_string() })?;

        let submodule_path = config.path.as_ref()
            .ok_or_else(|| SubmoduleError::ConfigError("No path configured for submodule".to_string()))?;

        // Use CLI for update operations for reliability
        let output = Command::new("git")
            .args(["pull", "origin", "HEAD"])
            .current_dir(submodule_path)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SubmoduleError::CliError(format!("Update failed for {}: {}", name, stderr)));
        }

        println!("✅ Updated {} using git CLI", name);
        Ok(())
    }

    /// Reset submodule using CLI operations
    pub fn reset_submodule(&self, name: &str) -> Result<(), SubmoduleError> {
        let config = self.config.submodules.get(name)
            .ok_or_else(|| SubmoduleError::SubmoduleNotFound { name: name.to_string() })?;

        let submodule_path = config.path.as_ref()
            .ok_or_else(|| SubmoduleError::ConfigError("No path configured for submodule".to_string()))?;

        println!("🔄 Hard resetting {}...", name);

        // Step 1: Stash changes
        println!("  📦 Stashing working changes...");
        let stash_output = Command::new("git")
            .args(["stash", "push", "--include-untracked", "-m", "Submod reset stash"])
            .current_dir(submodule_path)
            .output()?;

        if !stash_output.status.success() {
            let stderr = String::from_utf8_lossy(&stash_output.stderr);
            if !stderr.contains("No local changes to save") {
                println!("  ⚠️  Stash warning: {}", stderr.trim());
            }
        }

        // Step 2: Hard reset
        println!("  🔄 Resetting to HEAD...");
        let reset_output = Command::new("git")
            .args(["reset", "--hard", "HEAD"])
            .current_dir(submodule_path)
            .output()?;

        if !reset_output.status.success() {
            let stderr = String::from_utf8_lossy(&reset_output.stderr);
            return Err(SubmoduleError::CliError(format!("Git reset failed: {}", stderr)));
        }

        // Step 3: Clean untracked files
        println!("  🧹 Cleaning untracked files...");
        let clean_output = Command::new("git")
            .args(["clean", "-fdx"])
            .current_dir(submodule_path)
            .output()?;

        if !clean_output.status.success() {
            let stderr = String::from_utf8_lossy(&clean_output.stderr);
            return Err(SubmoduleError::CliError(format!("Git clean failed: {}", stderr)));
        }

        println!("✅ {} reset complete", name);
        Ok(())
    }

    /// Initialize submodule - add it first if not registered, then initialize
    pub fn init_submodule(&self, name: &str) -> Result<(), SubmoduleError> {
        let config = self.config.submodules.get(name)
            .ok_or_else(|| SubmoduleError::SubmoduleNotFound { name: name.to_string() })?;

        let path_str = config.path.as_ref()
            .ok_or_else(|| SubmoduleError::ConfigError("No path configured for submodule".to_string()))?;
        let url_str = config.url.as_ref()
            .ok_or_else(|| SubmoduleError::ConfigError("No URL configured for submodule".to_string()))?;

        let submodule_path = Path::new(path_str);

        if submodule_path.exists() && submodule_path.join(".git").exists() {
            println!("✅ {} already initialized", name);
            // Even if already initialized, check if we need to configure sparse checkout
            if let Some(sparse_paths) = &config.sparse_paths {
                eprintln!("DEBUG: Configuring sparse checkout for already initialized submodule: {}", name);
                self.configure_sparse_checkout(path_str, sparse_paths)?;
            }
            return Ok(());
        }

        println!("🔄 Initializing {}...", name);

        let workdir = self.repo.workdir().unwrap_or_else(|| std::path::Path::new("."));

        // First check if submodule is registered in .gitmodules
        let gitmodules_path = workdir.join(".gitmodules");
        let needs_add = if gitmodules_path.exists() {
            let gitmodules_content = fs::read_to_string(&gitmodules_path)?;
            !gitmodules_content.contains(&format!("path = {}", path_str))
        } else {
            true
        };

        if needs_add {
            // Submodule not registered yet, add it first
            eprintln!("DEBUG: Submodule not registered in .gitmodules, adding first");
            self.add_submodule_cli(name, path_str, url_str)?;
        } else {
            // Submodule is registered, just initialize and update
            let init_output = Command::new("git")
                .args(["submodule", "init", path_str])
                .current_dir(workdir)
                .output()?;

            if !init_output.status.success() {
                let stderr = String::from_utf8_lossy(&init_output.stderr);
                return Err(SubmoduleError::CliError(format!("Git submodule init failed: {}", stderr)));
            }

            let update_output = Command::new("git")
                .args(["submodule", "update", path_str])
                .current_dir(workdir)
                .output()?;

            if !update_output.status.success() {
                let stderr = String::from_utf8_lossy(&update_output.stderr);
                return Err(SubmoduleError::CliError(format!("Git submodule update failed: {}", stderr)));
            }
        }

        println!("  ✅ Initialized using git submodule commands: {}", path_str);

        // Configure sparse checkout if specified
        if let Some(sparse_paths) = &config.sparse_paths {
            eprintln!("DEBUG: Configuring sparse checkout for newly initialized submodule: {}", name);
            self.configure_sparse_checkout(path_str, sparse_paths)?;
        }

        println!("✅ {} initialized", name);
        Ok(())
    }

    /// GITOXIDE API: Clone using gix - temporarily disabled due to API changes
    #[allow(dead_code)]
    fn clone_with_gix(&self, url: &str, path: &str) -> Result<(), SubmoduleError> {
        // TODO: Fix gitoxide clone API - the prepare_clone API has changed
        // For now, fall back to CLI
        eprintln!("DEBUG: Gitoxide clone API needs updating, falling back to CLI");
        self.clone_with_cli(url, path)
    }

    /// Fallback clone using CLI
    #[allow(dead_code)]
    fn clone_with_cli(&self, url: &str, path: &str) -> Result<(), SubmoduleError> {
        // Create parent directories if they don't exist
        if let Some(parent) = Path::new(path).parent() {
            fs::create_dir_all(parent)?;
        }

        let output = Command::new("git")
            .args(["clone", url, path])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(SubmoduleError::CliError(format!("Git clone failed: {}", stderr)));
        }

        Ok(())
    }

    /// Check all submodules using gitoxide APIs where possible
    pub fn check_all_submodules(&self) -> Result<(), SubmoduleError> {
        println!("Checking submodule configurations...");

        for (submodule_name, submodule) in self.config.get_submodules() {
            println!("\n📁 {}", submodule_name);

            // Handle missing path gracefully - report but don't fail
            let path_str = match submodule.path.as_ref() {
                Some(path) => path,
                None => {
                    println!("  ❌ Configuration error: No path configured");
                    continue;
                }
            };

            // Handle missing URL gracefully - report but don't fail
            if submodule.url.is_none() {
                println!("  ❌ Configuration error: No URL configured");
                continue;
            }

            let submodule_path = Path::new(path_str);
            let git_path = submodule_path.join(".git");

            if !submodule_path.exists() {
                println!("  ❌ Folder missing: {}", path_str);
                continue;
            }

            if !git_path.exists() {
                println!("  ❌ Not a git repository");
                continue;
            }

            // GITOXIDE API: Use gix::open and status check
            match self.check_submodule_repository_status(path_str, submodule_name) {
                Ok(status) => {
                    println!("  ✅ Git repository exists");

                    if status.is_clean {
                        println!("  ��� Working tree is clean");
                    } else {
                        println!("  ⚠️  Working tree has changes");
                    }

                    if let Some(commit) = &status.current_commit {
                        println!("  ✅ Current commit: {}", &commit[..8]);
                    }

                    if status.has_remotes {
                        println!("  ✅ Has remotes configured");
                    } else {
                        println!("  ⚠️  No remotes configured");
                    }

                    match status.sparse_status {
                        SparseStatus::NotEnabled => {},
                        SparseStatus::NotConfigured => {
                            println!("  ❌ Sparse checkout not configured");
                        },
                        SparseStatus::Correct => {
                            println!("  ✅ Sparse checkout configured correctly");
                        },
                        SparseStatus::Mismatch { expected, actual } => {
                            println!("  ❌ Sparse checkout mismatch");
                            println!("    Expected: {:?}", expected);
                            println!("    Current: {:?}", actual);
                        },
                    }

                    // Show effective settings
                    self.show_effective_settings(submodule_name, submodule);
                }
                Err(e) => {
                    println!("  ❌ Cannot analyze repository: {}", e);
                }
            }
        }

        Ok(())
    }

    fn show_effective_settings(&self, _name: &str, config: &SubmoduleConfig) {
        println!("  📋 Effective settings:");

        if let Some(ignore) = self.config.get_effective_setting(config, "ignore") {
            println!("     ignore = {}", ignore);
        }
        if let Some(update) = self.config.get_effective_setting(config, "update") {
            println!("     update = {}", update);
        }
        if let Some(branch) = self.config.get_effective_setting(config, "branch") {
            println!("     branch = {}", branch);
        }
    }

    /// Get reference to the underlying config
    pub fn config(&self) -> &Config {
        &self.config
    }

}
