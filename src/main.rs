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
mod git_ops;
mod options;
mod commands;
mod config;
mod gitoxide_manager;
mod utilities;

use crate::commands::{Cli, Commands};
use crate::utilities::set_path;
use crate::options::SerializableBranch as Branch;
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
            shallow,
            no_init,
        } => {
            let mut manager = GitoxideSubmoduleManager::new(cli.config)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            let sparse_paths_vec = sparse_paths.map(|paths| {
                paths
                    .into_iter()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
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

            // Set the path - if not provided, derive from URL
            let set_path_result = match path {
                Some(p) => set_path(p)
                    .map_err(|e| anyhow::anyhow!("Failed to set path: {}", e))?,
                None => {
                    // Derive path from URL - take the last component without .git
                    let path_from_url = url.split('/').last()
                        .unwrap_or("submodule")
                        .trim_end_matches(".git");
                    path_from_url.to_string()
                },
            };

            let set_branch = match branch {
                Some(ref b) => Some(Branch::from_str(b)
                    .map_err(|e| anyhow::anyhow!("Failed to set branch: {:#?}", e))?),
                None => Some(Branch::default()),
            };

            // Set the name - if not provided, use the path
            let set_name = name.unwrap_or_else(|| set_path_result.clone());

            // Convert CLI types to serializable types
            use crate::options::{SerializableIgnore, SerializableFetchRecurse, SerializableUpdate};
            let set_ignore = Some(SerializableIgnore::try_from(ignore)
                .map_err(|_| anyhow::anyhow!("Failed to convert ignore option"))?);
            let set_fetch = Some(SerializableFetchRecurse::try_from(fetch)
                .map_err(|_| anyhow::anyhow!("Failed to convert fetch option"))?);
            let set_update = Some(SerializableUpdate::try_from(update)
                .map_err(|_| anyhow::anyhow!("Failed to convert update option"))?);

            manager
                .add_submodule(set_name, set_path_result, url, sparse_paths_vec, set_branch, set_ignore, set_fetch, set_update)
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
        Commands::Change { .. } => {
            return Err(anyhow::anyhow!("Change command is not yet implemented"));
        }
        Commands::ChangeGlobal { .. } => {
            return Err(anyhow::anyhow!("ChangeGlobal command is not yet implemented"));
        }
        Commands::List { .. } => {
            return Err(anyhow::anyhow!("List command is not yet implemented"));
        }
        Commands::Delete => {
            return Err(anyhow::anyhow!("Delete command is not yet implemented"));
        }
        Commands::Disable => {
            return Err(anyhow::anyhow!("Disable command is not yet implemented"));
        }
        Commands::GenerateConfig { .. } => {
            return Err(anyhow::anyhow!("GenerateConfig command is not yet implemented"));
        }
        Commands::NukeItFromOrbit { .. } => {
            return Err(anyhow::anyhow!("NukeItFromOrbit command is not yet implemented"));
        }
        Commands::Completions => {
            return Err(anyhow::anyhow!("Completions command is not yet implemented"));
        }
        // TODO: Implement FromConfig command
        // Commands::FromConfig => {
        //     let manager = GitoxideSubmoduleManager::new(cli.config)
        //         .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;
        //
        //     // Generate submod.toml from existing git configuration
        //     manager
        //         .generate_from_git_config()
        //         .map_err(|e| anyhow::anyhow!("Failed to generate from config: {}", e))?;
        // }
    }

    Ok(())
}
