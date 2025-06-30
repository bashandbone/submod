// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

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
mod long_abouts;
mod shells;
mod git_ops;
mod options;
mod commands;
mod config;
mod gitoxide_manager;
mod utilities;

use crate::commands::{Cli, Commands};
use crate::utilities::{name_from_osstring, name_from_url, set_path, get_sparse_paths};
use crate::options::{SerializableBranch as Branch, SerializableIgnore, SerializableFetchRecurse, SerializableUpdate};
use crate::gitoxide_manager::GitoxideSubmoduleManager;
use anyhow::Result;
use clap::Parser;
use submod::options::SerializableBranch;
use std::ffi::OsString;
use std::str::FromStr;

fn main() -> Result<()> {
    let cli = Cli::parse();
    // config-path is always set because it has a default value, "submod.toml"
    let config_path = cli.config.clone();

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
            // Validate sparse paths for null bytes
            let sparse_paths_vec = get_sparse_paths(sparse_paths)
                .map_err(|e| anyhow::anyhow!("Invalid sparse paths: {}", e))?;

            let set_name = get_name(&name, &url, &path)
                .map_err(|e| anyhow::anyhow!("Failed to get submodule name: {}", e))?;

            let set_path = path.map(|p| set_path(p).map_err(|e| anyhow::anyhow!("Invalid path: {}", e))).transpose()?;

            let set_url = url.trim().to_string();

            let set_branch = Branch::set_branch(branch.as_deref())
                .map_err(|e| anyhow::anyhow!("Failed to set branch: {}", e))?;

            let mut manager = GitoxideSubmoduleManager::new(config_path);

            manager
                .add_submodule(set_name, set_path, set_url, sparse_paths_vec, set_branch, ignore, fetch, update, shallow, no_init)
                .map_err(|e| anyhow::anyhow!("Failed to add submodule: {}", e))?;
        }
        Commands::Check => {
            let manager = GitoxideSubmoduleManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;
            manager
                .check_all_submodules()
                .map_err(|e| anyhow::anyhow!("Failed to check submodules: {}", e))?;
        }
        Commands::Init => {
            let manager = GitoxideSubmoduleManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            // Initialize all submodules from config
            for (name, _) in manager.config().get_submodules() {
                manager
                    .init_submodule(name)
                    .map_err(|e| anyhow::anyhow!("Failed to init submodule {}: {}", name, e))?;
            }
        }
        Commands::Update => {
            let manager = GitoxideSubmoduleManager::new(config_path)
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
            let manager = GitoxideSubmoduleManager::new(config_path)
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
        Commands::CompleteMe { .. } => {
            return Err(anyhow::anyhow!("Completions command not yet implemented"));
        }
    }

    Ok(())
}
