// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
//
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
use crate::options::{SerializableBranch as Branch, SerializableIgnore, SerializableFetchRecurse, SerializableUpdate};
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

            // Validate sparse paths for null bytes
            let sparse_paths_vec = match sparse_paths {
                Some(paths) => {
                    for path in &paths {
                        if path.contains('\0') {
                            return Err(anyhow::anyhow!(
                                "Invalid sparse path pattern: contains null byte"
                            ));
                        }
                    }
                    Some(paths)
                },
                None => None,
            };

            // Set the path - handle optional path
            let set_path_result = match path {
                Some(p) => set_path(p)
                    .map_err(|e| anyhow::anyhow!("Failed to set path: {}", e))?,
                None => {
                    // Derive path from URL if not provided
                    let cleaned_url = url.trim_end_matches('/').trim_end_matches(".git");
                    cleaned_url
                        .split('/')
                        .last()
                        .unwrap_or_else(|| {
                            cleaned_url
                                .split(':')
                                .last()
                                .unwrap_or("submodule")
                        })
                        .to_string()
                }
            };

            let set_branch = match branch {
                Some(ref b) => Some(Branch::from_str(b)
                    .map_err(|e| anyhow::anyhow!("Failed to set branch: {:#?}", e))?),
                None => Some(Branch::default()),
            };

            // Convert CLI enums to serializable types
            let serializable_ignore = Some(SerializableIgnore::try_from(ignore)
                .map_err(|_| anyhow::anyhow!("Failed to convert ignore setting"))?);
            let serializable_fetch = Some(SerializableFetchRecurse::try_from(fetch)
                .map_err(|_| anyhow::anyhow!("Failed to convert fetch setting"))?);
            let serializable_update = Some(SerializableUpdate::try_from(update)
                .map_err(|_| anyhow::anyhow!("Failed to convert update setting"))?);

            // Handle optional name - derive from path if not provided
            let submodule_name = name.unwrap_or_else(|| {
                // Use the path as the name
                set_path_result.clone()
            });

            manager
                .add_submodule(submodule_name, set_path_result, url, sparse_paths_vec, set_branch, serializable_ignore, serializable_fetch, serializable_update)
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
        // TODO: Implement missing commands
        Commands::Change { .. } => {
            return Err(anyhow::anyhow!("Change command not yet implemented"));
        }
        Commands::ChangeGlobal { .. } => {
            return Err(anyhow::anyhow!("ChangeGlobal command not yet implemented"));
        }
        Commands::List { .. } => {
            return Err(anyhow::anyhow!("List command not yet implemented"));
        }
        Commands::Delete => {
            return Err(anyhow::anyhow!("Delete command not yet implemented"));
        }
        Commands::Disable => {
            return Err(anyhow::anyhow!("Disable command not yet implemented"));
        }
        Commands::GenerateConfig { .. } => {
            return Err(anyhow::anyhow!("GenerateConfig command not yet implemented"));
        }
        Commands::NukeItFromOrbit { .. } => {
            return Err(anyhow::anyhow!("NukeItFromOrbit command not yet implemented"));
        }
        Commands::Completions => {
            return Err(anyhow::anyhow!("Completions command not yet implemented"));
        }
    }

    Ok(())
}
