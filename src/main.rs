mod config;
mod commands;
mod gitoxide_manager;

use anyhow::Result;
use clap::Parser;
use crate::commands::{Cli, Commands};
use crate::gitoxide_manager::GitoxideSubmoduleManager;

// Old SubmoduleManager removed - now using GitoxideSubmoduleManager

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add { name, path, url, sparse_paths, settings: _ } => {
            let mut manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            let sparse_paths_vec = sparse_paths.map(|paths| {
                paths.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            });

            manager.add_submodule(name, path, url, sparse_paths_vec)
                .map_err(|e| anyhow::anyhow!("Failed to add submodule: {}", e))?;
        }
        Commands::Check => {
            let manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;
            manager.check_all_submodules()
                .map_err(|e| anyhow::anyhow!("Failed to check submodules: {}", e))?;
        }
        Commands::Init => {
            let manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            // Initialize all submodules from config
            for (name, _) in manager.config().get_submodules() {
                manager.init_submodule(name)
                    .map_err(|e| anyhow::anyhow!("Failed to init submodule {}: {}", name, e))?;
            }
        }
        Commands::Update => {
            let manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            // Update all submodules from config
            for (name, _) in manager.config().get_submodules() {
                manager.update_submodule(name)
                    .map_err(|e| anyhow::anyhow!("Failed to update submodule {}: {}", name, e))?;
            }
        }
        Commands::Reset { all, names } => {
            let manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            let submodules_to_reset: Vec<String> = if all {
                manager.config().get_submodules().map(|(name, _)| name.clone()).collect()
            } else {
                names
            };

            if submodules_to_reset.is_empty() {
                println!("No submodules to reset");
                return Ok(());
            }

            for name in submodules_to_reset {
                manager.reset_submodule(&name)
                    .map_err(|e| anyhow::anyhow!("Failed to reset submodule {}: {}", name, e))?;
            }
        }
        Commands::Sync => {
            let manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            // Run check, init, and update in sequence
            println!("🔄 Running full sync: check, init, update");

            manager.check_all_submodules()
                .map_err(|e| anyhow::anyhow!("Failed to check submodules: {}", e))?;

            for (name, _) in manager.config().get_submodules() {
                manager.init_submodule(name)
                    .map_err(|e| anyhow::anyhow!("Failed to init submodule {}: {}", name, e))?;
            }

            for (name, _) in manager.config().get_submodules() {
                manager.update_submodule(name)
                    .map_err(|e| anyhow::anyhow!("Failed to update submodule {}: {}", name, e))?;
            }

            println!("✅ Sync complete");
        }
    }

    Ok(())
}
