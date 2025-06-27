// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: MIT
// Licensed under the [Plain MIT License][../LICENSE.md]
#![doc = r"
Main entry point for the submod CLI tool.

Parses command-line arguments and dispatches submodule management commands using the
[`GitoxideSubmoduleManager`]. Supports adding, checking, initializing, updating, resetting,
and syncing submodules with features like sparse checkout.

# Commands

- `add`: Add a new submodule with optional sparse paths.
- `check`: Check the status of all configured submodules.
- `init`: Initialize all submodules from config.
- `update`: Update all submodules.
- `reset`: Reset specified or all submodules.
- `sync`: Run check, init, and update in sequence.

Exits with an error if any operation fails.
"]

mod options;
mod commands;
mod config;
mod gitoxide_manager;
mod utilities;

use crate::commands::{Cli, Commands, set_path};
use crate::config::SerializableBranch as Branch;
use crate::gitoxide_manager::GitoxideSubmoduleManager;
use anyhow::Result;
use clap::Parser;
use std::str::FromStr;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Add {
            name,
            path,
            url,
            branch,
            sparse_paths,
            ignore,
            update,
            fetch,

        } => {
            let mut manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            let sparse_paths_vec = sparse_paths.map(|paths| {
                paths
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<String>>()
                    .into_iter()
                    .map(|s| {
                        if s.contains('\0') {
                            return Err(anyhow::anyhow!(
                                "Invalid sparse path pattern: contains null byte"
                            ));
                        }
                        Ok(s)
                    })
                    .collect::<Result<Vec<String>, _>>()
            });

            let sparse_paths_vec = match sparse_paths_vec {
                Some(result) => match result {
                    Ok(paths) => Some(paths),
                    Err(e) => return Err(e),
                },
                None => None,
            };

            // Set the path
            let set_path_result = set_path(path)
                .map_err(|e| anyhow::anyhow!("Failed to set path: {}", e))?;

            let set_branch = match branch {
                Some(ref b) => Some(Branch::from_str(b)
                    .map_err(|e| anyhow::anyhow!("Failed to set branch: {:#?}", e))?),
                None => Some(Branch::default()),
            };

            manager
                .add_submodule(name, set_path_result, url, sparse_paths_vec, set_branch, ignore, fetch, update)
                .map_err(|e| anyhow::anyhow!("Failed to add submodule: {}", e))?;
        }
        Commands::Check => {
            let manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;
            manager
                .check_all_submodules()
                .map_err(|e| anyhow::anyhow!("Failed to check submodules: {}", e))?;
        }
        Commands::Init => {
            let manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            // Initialize all submodules from config
            for (name, _) in manager.config().get_submodules() {
                manager
                    .init_submodule(name)
                    .map_err(|e| anyhow::anyhow!("Failed to init submodule {}: {}", name, e))?;
            }
        }
        Commands::Update => {
            let manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            // Update all submodules from config
            let submodules: Vec<_> = manager.config().get_submodules().collect();
            if submodules.is_empty() {
                println!("No submodules configured");
            } else {
                for (name, _) in submodules {
                    manager.update_submodule(name).map_err(|e| {
                        anyhow::anyhow!("Failed to update submodule {}: {}", name, e)
                    })?;
                }
                println!(
                    "Updated {} submodule(s)",
                    manager.config().get_submodules().count()
                );
            }
        }
        Commands::Reset { all, names } => {
            let manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            let submodules_to_reset: Vec<String> = if all {
                manager
                    .config()
                    .get_submodules()
                    .map(|(name, _)| name.clone())
                    .collect()
            } else {
                names
            };

            if submodules_to_reset.is_empty() {
                return Err(anyhow::anyhow!(
                    "No submodules specified for reset. Use --all to reset all submodules or specify submodule names."
                ));
            }

            for name in submodules_to_reset {
                manager
                    .reset_submodule(&name)
                    .map_err(|e| anyhow::anyhow!("Failed to reset submodule {}: {}", name, e))?;
            }
        }
        Commands::Sync => {
            let manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            // Run check, init, and update in sequence
            println!("🔄 Running full sync: check, init, update");

            manager
                .check_all_submodules()
                .map_err(|e| anyhow::anyhow!("Failed to check submodules: {}", e))?;

            for (name, _) in manager.config().get_submodules() {
                manager
                    .init_submodule(name)
                    .map_err(|e| anyhow::anyhow!("Failed to init submodule {}: {}", name, e))?;
            }

            for (name, _) in manager.config().get_submodules() {
                manager
                    .update_submodule(name)
                    .map_err(|e| anyhow::anyhow!("Failed to update submodule {}: {}", name, e))?;
            }

            println!("✅ Sync complete");
        }
        Commands::FromConfig => {
            let manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            // Generate submod.toml from existing git configuration
            manager
                .generate_from_git_config()
                .map_err(|e| anyhow::anyhow!("Failed to generate from config: {}", e))?;
        }
    }

    Ok(())
}
