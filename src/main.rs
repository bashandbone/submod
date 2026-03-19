// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

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
mod commands;
mod config;
mod git_manager;
mod git_ops;
mod long_abouts;
mod options;
mod shells;
mod utilities;

use crate::commands::{Cli, Commands};
use crate::git_manager::GitManager;
use crate::options::SerializableBranch as Branch;
use crate::utilities::{get_name, get_sparse_paths, set_path};
use anyhow::Result;
use clap::Parser;
use clap_complete::generate;
#[cfg_attr(coverage_nightly, coverage(off))]
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

            let set_path = path
                .map(|p| set_path(p).map_err(|e| anyhow::anyhow!("Invalid path: {}", e)))
                .transpose()?
                .unwrap_or_else(|| set_name.clone());

            let set_url = url.trim().to_string();

            let set_branch = Branch::set_branch(branch)
                .map_err(|e| anyhow::anyhow!("Failed to set branch: {}", e))?;

            let mut manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;

            manager
                .add_submodule(
                    set_name,
                    set_path,
                    set_url,
                    sparse_paths_vec,
                    Some(set_branch),
                    ignore,
                    fetch,
                    update,
                    Some(shallow),
                    no_init,
                )
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
            let names: Vec<String> = manager
                .config()
                .get_submodules()
                .map(|(n, _)| n.clone())
                .collect();
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
            let names: Vec<String> = manager
                .config()
                .get_submodules()
                .map(|(n, _)| n.clone())
                .collect();
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
            let names: Vec<String> = manager
                .config()
                .get_submodules()
                .map(|(n, _)| n.clone())
                .collect();
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
        Commands::Change {
            name,
            path,
            branch,
            sparse_paths,
            append,
            ignore,
            fetch,
            update,
            shallow,
            url,
            active,
        } => {
            let mut manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;
            manager
                .change_submodule(
                    &name,
                    path,
                    branch,
                    sparse_paths,
                    append,
                    ignore,
                    fetch,
                    update,
                    Some(shallow),
                    url,
                    active,
                )
                .map_err(|e| anyhow::anyhow!("Failed to change submodule: {}", e))?;
        }
        Commands::ChangeGlobal {
            ignore,
            fetch,
            update,
        } => {
            let mut manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;
            manager
                .update_global_defaults(ignore, fetch, update)
                .map_err(|e| anyhow::anyhow!("Failed to update global settings: {}", e))?;
        }
        Commands::List { recursive } => {
            let manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;
            manager
                .list_submodules(recursive)
                .map_err(|e| anyhow::anyhow!("Failed to list submodules: {}", e))?;
        }
        Commands::Delete { name } => {
            let mut manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;
            manager
                .delete_submodule_by_name(&name)
                .map_err(|e| anyhow::anyhow!("Failed to delete submodule: {}", e))?;
        }
        Commands::Disable { name } => {
            let mut manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;
            manager
                .disable_submodule(&name)
                .map_err(|e| anyhow::anyhow!("Failed to disable submodule: {}", e))?;
        }
        Commands::GenerateConfig {
            output,
            from_setup,
            force,
            template,
        } => {
            GitManager::generate_config(&output, from_setup.is_some(), template, force)
                .map_err(|e| anyhow::anyhow!("Failed to generate config: {}", e))?;
        }
        Commands::NukeItFromOrbit { all, names, kill } => {
            let mut manager = GitManager::new(config_path)
                .map_err(|e| anyhow::anyhow!("Failed to create manager: {}", e))?;
            manager
                .nuke_submodules(all, names, kill)
                .map_err(|e| anyhow::anyhow!("Failed to nuke submodules: {}", e))?;
        }
        Commands::CompleteMe { shell } => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut std::io::stdout());
        }
    }

    Ok(())
}
