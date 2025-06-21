mod config;
mod commands;

use anyhow::{Context, Result};
use clap::Parser;
use gix::Repository;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use crate::commands::{Cli, Commands};
use crate::config::{Config, SubmoduleConfig};

struct SubmoduleManager {
    repo: Repository,
    config: Config,
    config_path: PathBuf,
}

impl SubmoduleManager {
    fn new(config_path: PathBuf) -> Result<Self> {
        let repo = gix::discover(".")
            .context("Not in a git repository")?;

        let config = Config::load(&config_path)?;

        Ok(SubmoduleManager {
            repo,
            config,
            config_path,
        })
    }

    fn add_submodule(
        &mut self,
        name: String,
        folder: String,
        url: String,
        sparse_paths: Option<String>,
        _settings: Option<String>,
    ) -> Result<()> {
        let sparse_paths = sparse_paths.map(|paths| {
            paths.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        });

        let submodule = SubmoduleConfig {
            git_options: crate::config::SubmoduleGitOptions::default(),
            active: true,
            path: Some(folder),
            url: Some(url),
            sparse_paths,
        };

        self.config.add_submodule(name.clone(), submodule);
        self.config.save(&self.config_path)?;

        println!("✅ Added submodule configuration for '{}'", name);
        println!("   Configuration saved with clean section-based structure");
        Ok(())
    }

    fn check_submodules(&self) -> Result<()> {
        println!("Checking submodule configurations...");

        for (submodule_name, submodule) in self.config.get_submodules() {
            println!("\n📁 {}", submodule_name);

            let default_path = String::new();
            let path_str = submodule.path.as_ref().unwrap_or(&default_path);
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

            // Check if it's a proper git repository
            if let Ok(submodule_repo) = gix::open(&submodule_path) {
                println!("  ✅ Git repository exists");

                // Check sparse checkout configuration
                if let Some(sparse_paths) = &submodule.sparse_paths {
                    self.check_sparse_checkout(&submodule_repo, sparse_paths)?;
                }

                // Check submodule state using gitoxide
                self.check_submodule_state(&submodule_repo, submodule_name, submodule)?;
            } else {
                println!("  ❌ Cannot open git repository");
            }
        }

        Ok(())
    }

    fn check_submodule_state(&self, repo: &Repository, name: &str, _config: &SubmoduleConfig) -> Result<()> {
        // Check if repository has remotes configured
        let remotes = repo.remote_names();
        if remotes.is_empty() {
            println!("  ⚠️  No remotes configured");
        } else {
            println!("  ✅ Remotes configured: {:?}", remotes);
        }

        // Check current HEAD
        if let Ok(head) = repo.head() {
            if let Some(name) = head.referent_name() {
                println!("  ✅ On branch: {}", name.shorten());
            } else if let Some(id) = head.id() {
                println!("  ✅ Detached HEAD at: {}", &id.to_string()[..8]);
            }
        }

        // Show effective settings (defaults + overrides)
        self.show_effective_settings(name, _config);

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

    fn check_sparse_checkout(&self, repo: &Repository, expected_paths: &[String]) -> Result<()> {
        let sparse_checkout_file = repo.git_dir().join("info").join("sparse-checkout");

        if !sparse_checkout_file.exists() {
            println!("  ❌ Sparse checkout not configured");
            return Ok(());
        }

        let content = fs::read_to_string(&sparse_checkout_file)
            .context("Failed to read sparse-checkout file")?;

        let configured_paths: Vec<&str> = content.lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();

        let matches = expected_paths.iter()
            .all(|path| configured_paths.contains(&path.as_str()));

        if matches {
            println!("  ✅ Sparse checkout configured correctly");
        } else {
            println!("  ❌ Sparse checkout mismatch");
            println!("    Expected: {:?}", expected_paths);
            println!("    Current: {:?}", configured_paths);
        }

        Ok(())
    }

    fn init_submodules(&self) -> Result<()> {
        println!("Initializing missing submodules...");

        for (submodule_name, submodule) in self.config.get_submodules() {
            let default_path = String::new();
            let path_str = submodule.path.as_ref().unwrap_or(&default_path);
            let submodule_path = Path::new(path_str);

            if submodule_path.exists() && submodule_path.join(".git").exists() {
                println!("✅ {} already initialized", submodule_name);
                continue;
            }

            println!("🔄 Initializing {}...", submodule_name);

            // Clone the repository using git command
            self.clone_submodule(submodule)?;

            // Configure sparse checkout if specified
            if let Some(sparse_paths) = &submodule.sparse_paths {
                self.configure_sparse_checkout(submodule, sparse_paths)?;
            }

            println!("✅ {} initialized", submodule_name);
        }

        Ok(())
    }

    fn clone_submodule(&self, submodule: &SubmoduleConfig) -> Result<()> {
        let default_path = String::new();
        let default_url = String::new();
        let path_str = submodule.path.as_ref().unwrap_or(&default_path);
        let url_str = submodule.url.as_ref().unwrap_or(&default_url);
        let submodule_path = Path::new(path_str);

        // Create parent directories if they don't exist
        if let Some(parent) = submodule_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Use git clone command for reliability
        let output = Command::new("git")
            .args(["clone", url_str, path_str])
            .output()
            .context("Failed to execute git clone")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Git clone failed: {}", stderr));
        }

        println!("  ✅ Cloned repository to: {}", path_str);

        Ok(())
    }

    fn configure_sparse_checkout(&self, submodule: &SubmoduleConfig, sparse_paths: &[String]) -> Result<()> {
        let default_path = String::new();
        let path_str = submodule.path.as_ref().unwrap_or(&default_path);
        let submodule_repo = gix::open(path_str)
            .with_context(|| format!("Failed to open submodule repository: {}", path_str))?;

        // Enable sparse checkout in git config
        let config_path = submodule_repo.git_dir().join("config");
        let mut config_content = fs::read_to_string(&config_path).unwrap_or_default();

        if !config_content.contains("sparseCheckout = true") {
            config_content.push_str("\n[core]\n\tsparseCheckout = true\n");
            fs::write(&config_path, config_content)?;
        }

        // Write sparse-checkout file
        let info_dir = submodule_repo.git_dir().join("info");
        fs::create_dir_all(&info_dir)?;

        let sparse_checkout_file = info_dir.join("sparse-checkout");
        let sparse_content = sparse_paths.join("\n") + "\n";
        fs::write(&sparse_checkout_file, sparse_content)?;

        // Apply sparse checkout using git command
        let output = Command::new("git")
            .args(["read-tree", "-m", "-u", "HEAD"])
            .current_dir(path_str)
            .output()
            .context("Failed to apply sparse checkout")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("  ⚠️  Warning applying sparse checkout: {}", stderr);
        }

        println!("  ✅ Configured sparse checkout");

        Ok(())
    }

    fn update_submodules(&self) -> Result<()> {
        println!("Updating submodules...");

        for (submodule_name, submodule) in self.config.get_submodules() {
            let default_path = String::new();
            let path_str = submodule.path.as_ref().unwrap_or(&default_path);
            let submodule_path = Path::new(path_str);

            if !submodule_path.exists() {
                println!("❌ {} not found, run init first", submodule_name);
                continue;
            }

            println!("🔄 Updating {}...", submodule_name);

            // Use git pull to update
            let output = Command::new("git")
                .args(["pull", "origin", "HEAD"])
                .current_dir(path_str)
                .output()
                .context("Failed to execute git pull")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                println!("  ⚠️  Update warning: {}", stderr);
            } else {
                println!("  ✅ {} updated", submodule_name);
            }
        }

        Ok(())
    }

    fn reset_submodules(&self, all: bool, names: Vec<String>) -> Result<()> {
        let submodules_to_reset: Vec<(&String, &SubmoduleConfig)> = if all {
            self.config.submodules.iter().collect()
        } else {
            self.config.submodules.iter()
                .filter(|(name, _)| names.contains(name))
                .collect()
        };

        if submodules_to_reset.is_empty() {
            println!("No submodules to reset");
            return Ok(());
        }

        for (name, submodule) in submodules_to_reset {
            println!("🔄 Hard resetting {}...", name);

            let default_path = String::new();
            let path_str = submodule.path.as_ref().unwrap_or(&default_path);
            let submodule_path = Path::new(path_str);
            if !submodule_path.exists() {
                println!("❌ {} not found", name);
                continue;
            }

            self.reset_submodule_fallback(path_str)?;

            println!("✅ {} reset complete", name);
        }

        Ok(())
    }

    fn reset_submodule_fallback(&self, submodule_path: &str) -> Result<()> {
        println!("  ⚠️  Using git command fallback for reset operations");

        // Step 1: Stash changes
        println!("  📦 Stashing working changes...");
        let stash_output = Command::new("git")
            .args(["stash", "push", "--include-untracked", "-m", "Submod reset stash"])
            .current_dir(submodule_path)
            .output()
            .context("Failed to execute git stash")?;

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
            .output()
            .context("Failed to execute git reset")?;

        if !reset_output.status.success() {
            let stderr = String::from_utf8_lossy(&reset_output.stderr);
            return Err(anyhow::anyhow!("Git reset failed: {}", stderr));
        }

        // Step 3: Clean untracked files
        println!("  🧹 Cleaning untracked files...");
        let clean_output = Command::new("git")
            .args(["clean", "-fdx"])
            .current_dir(submodule_path)
            .output()
            .context("Failed to execute git clean")?;

        if !clean_output.status.success() {
            let stderr = String::from_utf8_lossy(&clean_output.stderr);
            return Err(anyhow::anyhow!("Git clean failed: {}", stderr));
        }

        Ok(())
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { name, folder, url, sparse_paths, settings } => {
            let mut manager = SubmoduleManager::new(cli.config)?;
            manager.add_submodule(name, folder, url, sparse_paths, settings)?;
        }
        Commands::Check => {
            let manager = SubmoduleManager::new(cli.config)?;
            manager.check_submodules()?;
        }
        Commands::Init => {
            let manager = SubmoduleManager::new(cli.config)?;
            manager.init_submodules()?;
        }
        Commands::Update => {
            let manager = SubmoduleManager::new(cli.config)?;
            manager.update_submodules()?;
        }
        Commands::Reset { all, names } => {
            let manager = SubmoduleManager::new(cli.config)?;
            manager.reset_submodules(all, names)?;
        }
        Commands::Sync => {
            let manager = SubmoduleManager::new(cli.config)?;
            manager.check_submodules()?;
            manager.init_submodules()?;
            manager.update_submodules()?;
        }
    }

    Ok(())
}
