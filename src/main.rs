// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

#![doc = r"
Main entry point for the submod CLI tool.

Parses command-line arguments and dispatches submodule management commands using the
[`GitManager`]. Supports adding, checking, initializing, updating, resetting,
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
mod git_manager;
mod utilities;

use crate::commands::{Cli, Commands};
use crate::utilities::{set_path, get_sparse_paths, get_name};
use crate::options::SerializableBranch as Branch;
use crate::git_manager::GitManager;
use anyhow::Result;
use clap::Parser;
use clap_complete::generate;


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

            let set_name = get_name(name, Some(url.clone()), path.clone())
                .map_err(|e| anyhow::anyhow!("Failed to get submodule name: {}", e))?;

            let set_path = path.map(|p| set_path(p).map_err(|e| anyhow::anyhow!("Invalid path: {}", e))).transpose()?
                .unwrap_or_else(|| set_name.clone());

            let set_url = url.trim().to_string();

            let set_branch = Branch::set_branch(branch)
                .map_err(|e| anyhow::anyhow!("Failed to set branch: {}", e))?;

            let mut manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            manager
                .add_submodule(set_name, set_path, set_url, sparse_paths_vec, Some(set_branch), Some(ignore), Some(fetch), Some(update), Some(shallow), no_init)
                .map_err(|e| anyhow::anyhow!("Failed to add submodule: {}", e))?;
        }
        Commands::Check => {
            let manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;
            manager
                .check_all_submodules()
                .map_err(|e| anyhow::anyhow!("Failed to check submodules: {}", e))?;
        }
        Commands::Init => {
            let mut manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            // Collect names first to avoid borrow conflict
            let names: Vec<String> = manager.config().get_submodules().map(|(n, _)| n.clone()).collect();
            for name in &names {
                manager
                    .init_submodule(name)
                    .map_err(|e| anyhow::anyhow!("Failed to init submodule {}: {}", name, e))?;
            }
        }
        Commands::Update => {
            let mut manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            // Collect names first to avoid borrow conflict
            let names: Vec<String> = manager.config().get_submodules().map(|(n, _)| n.clone()).collect();
            if names.is_empty() {
                println!("No submodules configured");
            } else {
                let count = names.len();
                for name in &names {
                    manager.update_submodule(name).map_err(|e| {
                        anyhow::anyhow!("Failed to update submodule {}: {}", name, e)
                    })?;
                }
                println!("Updated {} submodule(s)", count);
            }
        }
        Commands::Reset { all, names } => {
            let mut manager = GitManager::new(config_path)
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
            let mut manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            // Run check, init, and update in sequence
            println!("🔄 Running full sync: check, init, update");

            manager
                .check_all_submodules()
                .map_err(|e| anyhow::anyhow!("Failed to check submodules: {}", e))?;

            // Collect names first to avoid borrow conflict
            let names: Vec<String> = manager.config().get_submodules().map(|(n, _)| n.clone()).collect();
            for name in &names {
                manager
                    .init_submodule(name)
                    .map_err(|e| anyhow::anyhow!("Failed to init submodule {}: {}", name, e))?;
            }

            for name in &names {
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
        Commands::CompleteMe { shell } => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut std::io::stdout());
        }
    }

    Ok(())
}
