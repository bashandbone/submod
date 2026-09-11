// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]
// See the note in lib.rs: `deny` is opt-out-able per file, `forbid` is not.
#![forbid(unsafe_code)]
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
mod long_abouts;

use crate::commands::{Cli, Commands};
use clap::{CommandFactory, FromArgMatches, error::ErrorKind, parser::ValueSource};
use clap_complete::generate;
use submod::git_manager::{GitManager, SubmoduleError};
use submod::options::SerializableBranch as Branch;
use submod::utilities::{get_name, get_sparse_paths, safe_human_text, set_path};

struct AppError {
    code: u8,
    message: String,
    structured: bool,
}

impl AppError {
    fn validation(message: impl Into<String>) -> Self {
        Self {
            code: 2,
            message: message.into(),
            structured: false,
        }
    }

    fn operation(context: &str, error: SubmoduleError) -> Self {
        let code = error.exit_code();
        if let SubmoduleError::IncompleteBatch { summary, cause } = error {
            Self {
                code,
                message: format!("{context}:\n{summary}Cause: {}", safe_human_text(&cause)),
                structured: true,
            }
        } else {
            Self {
                code,
                message: format!("{context}: {error}"),
                structured: false,
            }
        }
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            if error.structured {
                eprintln!("{}", error.message);
            } else {
                eprintln!("{}", safe_human_text(&error.message));
            }
            std::process::ExitCode::from(error.code)
        }
    }
}

fn run() -> Result<(), AppError> {
    let matches = match Cli::command().try_get_matches() {
        Ok(matches) => matches,
        Err(error)
            if matches!(
                error.kind(),
                ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
            ) =>
        {
            error.print().map_err(|print_error| AppError {
                code: 1,
                message: format!("Failed to print command help: {print_error}"),
                structured: false,
            })?;
            return Ok(());
        }
        Err(error) => return Err(AppError::validation(error.to_string())),
    };
    let config_explicit = matches.value_source("config") == Some(ValueSource::CommandLine);
    let cli =
        Cli::from_arg_matches(&matches).map_err(|error| AppError::validation(error.to_string()))?;
    cli.validate()
        .map_err(|error| AppError::validation(error.to_string()))?;
    // config-path is always set because it has a default value, "submod.toml"
    let config_path = cli.config.clone();
    let verbose = cli.verbose;
    let dry_run = cli.dry_run;

    match cli.command {
        Commands::Add {
            name,
            path,
            url,
            branch,
            sparse_paths,
            use_git_default_sparse_checkout,
            ignore,
            update,
            fetch,
            shallow,
            no_init,
        } => {
            let sparse_paths_vec = get_sparse_paths(sparse_paths)
                .map_err(|e| AppError::validation(format!("Invalid sparse paths: {e}")))?;

            let set_name = get_name(name, Some(url.clone()), path.clone())
                .map_err(|e| AppError::validation(format!("Invalid submodule name: {e}")))?;

            let set_path = path.map_or_else(|| set_name.clone(), set_path);

            let set_url = url.trim().to_string();

            let set_branch = branch
                .map(|branch| Branch::set_branch(Some(branch)))
                .transpose()
                .map_err(|e| AppError::validation(format!("Invalid branch: {e}")))?;

            let mut manager =
                GitManager::with_verbose_config(config_path, verbose, config_explicit)
                    .map_err(|e| AppError::operation("Cannot prepare add", e))?;

            let result = if dry_run {
                manager.preview_add_submodule(
                    set_name,
                    set_path,
                    set_url,
                    sparse_paths_vec,
                    set_branch,
                    ignore,
                    fetch,
                    update,
                    Some(shallow),
                    no_init,
                    use_git_default_sparse_checkout,
                )
            } else {
                manager.add_submodule(
                    set_name,
                    set_path,
                    set_url,
                    sparse_paths_vec,
                    set_branch,
                    ignore,
                    fetch,
                    update,
                    Some(shallow),
                    no_init,
                    use_git_default_sparse_checkout,
                )
            };
            result.map_err(|e| AppError::operation("Add failed", e))?;
        }
        Commands::Check => {
            let manager = GitManager::with_verbose_config(config_path, verbose, config_explicit)
                .map_err(|e| AppError::operation("Cannot prepare check", e))?;
            manager
                .require_config()
                .map_err(|e| AppError::operation("Check failed", e))?;
            manager
                .check_all_submodules()
                .map_err(|e| AppError::operation("Check failed", e))?;
        }
        Commands::Init { recursive } => {
            let mut manager =
                GitManager::with_verbose_config(config_path, verbose, config_explicit)
                    .map_err(|e| AppError::operation("Cannot prepare initialization", e))?;
            manager
                .require_config()
                .map_err(|e| AppError::operation("Initialization failed", e))?;
            let summary = if dry_run {
                manager.preview_init_all_submodules(recursive)
            } else {
                manager.init_all_submodules(recursive)
            }
            .map_err(|e| AppError::operation("Initialization failed", e))?;
            print!("{summary}");
        }
        Commands::Update { remote, recursive } => {
            let mut manager =
                GitManager::with_verbose_config(config_path, verbose, config_explicit)
                    .map_err(|e| AppError::operation("Cannot prepare update", e))?;
            manager
                .require_config()
                .map_err(|e| AppError::operation("Update failed", e))?;
            let summary = if dry_run {
                manager.preview_update_all_submodules(remote, recursive)
            } else {
                manager.update_all_submodules(remote, recursive)
            }
            .map_err(|e| AppError::operation("Update failed", e))?;
            print!("{summary}");
        }
        Commands::Reset { all, names } => {
            let mut manager =
                GitManager::with_verbose_config(config_path, verbose, config_explicit)
                    .map_err(|e| AppError::operation("Cannot prepare reset", e))?;
            manager
                .require_config()
                .map_err(|e| AppError::operation("Reset failed", e))?;
            let result = if dry_run {
                manager.preview_reset_submodules(all, names)
            } else {
                manager.reset_submodules(all, names)
            };
            result.map_err(|e| AppError::operation("Reset failed", e))?;
        }
        Commands::Sync { recursive } => {
            let mut manager =
                GitManager::with_verbose_config(config_path, verbose, config_explicit)
                    .map_err(|e| AppError::operation("Cannot prepare sync", e))?;
            manager
                .require_config()
                .map_err(|e| AppError::operation("Sync failed", e))?;
            if dry_run {
                let summary = manager
                    .preview_sync_all_submodules(recursive)
                    .map_err(|e| AppError::operation("Sync preview failed", e))?;
                print!("{summary}");
            } else {
                eprintln!("Reconciling configured submodules...");
                let summary = manager
                    .sync_all_submodules(recursive)
                    .map_err(|e| AppError::operation("Sync failed", e))?;
                print!("{summary}");
            }
        }
        Commands::Change {
            name,
            path,
            branch,
            sparse_paths,
            append,
            clear_sparse_paths,
            unset,
            use_git_default_sparse_checkout,
            ignore,
            fetch,
            update,
            shallow,
            url,
            active,
        } => {
            let mut manager =
                GitManager::with_verbose_config(config_path, verbose, config_explicit)
                    .map_err(|e| AppError::operation("Cannot prepare change", e))?;
            manager
                .require_config()
                .map_err(|e| AppError::operation("Change failed", e))?;
            let unset = unset
                .iter()
                .map(|setting| setting.as_str())
                .collect::<Vec<_>>();
            let result = if dry_run {
                manager.preview_change_submodule(
                    &name,
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
                    use_git_default_sparse_checkout,
                    &unset,
                    clear_sparse_paths,
                )
            } else {
                manager.change_submodule(
                    &name,
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
                    use_git_default_sparse_checkout,
                    &unset,
                    clear_sparse_paths,
                )
            };
            result.map_err(|e| AppError::operation("Change failed", e))?;
        }
        Commands::ChangeGlobal {
            unset,
            branch,
            ignore,
            fetch,
            update,
            use_git_default_sparse_checkout,
        } => {
            let branch = branch
                .map(|value| Branch::set_branch(Some(value)))
                .transpose()
                .map_err(|error| AppError::validation(format!("Invalid global branch: {error}")))?;
            let mut manager =
                GitManager::with_verbose_config(config_path, verbose, config_explicit)
                    .map_err(|e| AppError::operation("Cannot prepare global change", e))?;
            manager
                .require_config()
                .map_err(|e| AppError::operation("Global change failed", e))?;
            let unset = unset
                .iter()
                .map(|setting| setting.as_str())
                .collect::<Vec<_>>();
            let result = if dry_run {
                manager.preview_global_defaults(
                    branch,
                    ignore,
                    fetch,
                    update,
                    use_git_default_sparse_checkout,
                    &unset,
                )
            } else {
                manager.update_global_defaults(
                    branch,
                    ignore,
                    fetch,
                    update,
                    use_git_default_sparse_checkout,
                    &unset,
                )
            };
            result.map_err(|e| AppError::operation("Global change failed", e))?;
        }
        Commands::List { recursive } => {
            let manager = GitManager::with_verbose_config(config_path, verbose, config_explicit)
                .map_err(|e| AppError::operation("Cannot prepare list", e))?;
            manager
                .require_config()
                .map_err(|e| AppError::operation("List failed", e))?;
            manager
                .list_submodules(recursive)
                .map_err(|e| AppError::operation("List failed", e))?;
        }
        Commands::Delete { name, force } => {
            let mut manager =
                GitManager::with_verbose_config(config_path, verbose, config_explicit)
                    .map_err(|e| AppError::operation("Cannot prepare delete", e))?;
            manager
                .require_config()
                .map_err(|e| AppError::operation("Delete failed", e))?;
            let result = if dry_run {
                manager.preview_delete_submodule_by_name(&name, force)
            } else {
                manager.delete_submodule_by_name(&name, force)
            };
            result.map_err(|e| AppError::operation("Delete failed", e))?;
        }
        Commands::Disable { name } => {
            let mut manager =
                GitManager::with_verbose_config(config_path, verbose, config_explicit)
                    .map_err(|e| AppError::operation("Cannot prepare disable", e))?;
            manager
                .require_config()
                .map_err(|e| AppError::operation("Disable failed", e))?;
            let result = if dry_run {
                manager.preview_disable_submodule(&name)
            } else {
                manager.disable_submodule(&name)
            };
            result.map_err(|e| AppError::operation("Disable failed", e))?;
        }
        Commands::GenerateConfig {
            output,
            from_setup,
            force,
            template,
        } => {
            let result = if dry_run {
                GitManager::preview_generate_config(&output, from_setup, template, force)
            } else {
                GitManager::generate_config(&output, from_setup, template, force)
            };
            result.map_err(|e| AppError::operation("Config generation failed", e))?;
        }
        Commands::NukeItFromOrbit {
            all,
            names,
            kill,
            force,
        } => {
            let mut manager =
                GitManager::with_verbose_config(config_path, verbose, config_explicit)
                    .map_err(|e| AppError::operation("Cannot prepare rebuild", e))?;
            manager
                .require_config()
                .map_err(|e| AppError::operation("Rebuild failed", e))?;
            let result = if dry_run {
                manager.preview_nuke_submodules(all, names, kill, force)
            } else {
                manager.nuke_submodules(all, names, kill, force)
            };
            result.map_err(|e| AppError::operation("Rebuild failed", e))?;
        }
        Commands::CompleteMe { shell } => {
            let mut cmd = <Cli as clap::CommandFactory>::command();
            let name = cmd.get_name().to_string();
            generate(shell, &mut cmd, name, &mut std::io::stdout());
        }
    }

    Ok(())
}
