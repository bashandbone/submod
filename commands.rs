// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: MIT
// Licensed under the [Plain MIT License][../LICENSE.md]
#![doc = r#"
Command-line argument definitions for the `submod` tool.

Defines the CLI structure and subcommands using [`clap`] for managing git submodules with sparse checkout support.

# Overview

- Parses CLI arguments for the `submod` binary.
- Supports commands for adding, checking, initializing, updating, resetting, and syncing submodules.
- Allows specifying a custom configuration file (default: `submod.toml`).

# Commands

- [`Commands::Add`](src/commands.rs): Adds a new submodule configuration.
- [`Commands::Change`](src/commands.rs): Changes the configuration of an existing submodule.
- [`Commands::ChangeGlobal`](src/commands.rs): Changes global settings for all submodules in the current repository.
- [`Commands::Check`](src/commands.rs): Checks submodule status and configuration.
- [`Commands::Delete`](src/commands.rs): Deletes a submodule by name.
- [`Commands::Disable`](src/commands.rs): Disables a submodule by name.
- [`Commands::List`](src/commands.rs): Lists all submodules, optionally recursively.
- [`Commands::Init`](src/commands.rs): Initializes missing submodules.
- [`Commands::Update`](src/commands.rs): Updates all submodules.
- [`Commands::Reset`](src/commands.rs): Hard resets submodules (stash, reset --hard, clean).
- [`Commands::Sync`](src/commands.rs): Runs a full sync (check, init, update).
- [`Commands::GenerateConfig`](src/commands.rs): Generates a new configuration file.
- [`Commands::NukeItFromOrbit`](src/commands.rs): Deletes all submodules or specific ones, optionally leaving them dead. (reinits by default)
- [`Commands::Completions`](src/commands.rs): Generates shell completions for the specified shell.

# Usage Example

```sh
submod add my-lib libs/my-lib https://github.com/example/my-lib.git --sparse-paths "src/,include/" --settings "ignore=all"
submod change my-lib --branch "main" --sparse-paths "src/,include/" --fetch "always" --update "checkout"
submod check
submod init
submod update
submod reset --all
submod sync
```

# Configuration

Use the `--config` option to specify a custom config file location.

See the [README.md](../README.md) for full usage and configuration details.
"#]

use clap::{Parser, Subcommand};
use std::{ffi::OsString, path::PathBuf};
use crate::options::{SerializableBranch, SerializableFetchRecurse, SerializableUpdate, SerializableIgnore};
use clap_complete::aot::{generate, Generator, Shell};
use clap_complete_nushell::Nushell;


/// Top-level CLI parser for the `submod` tool.
///
/// Accepts a subcommand and an optional config file path.
#[derive(Parser, Debug)]
#[command(name = clap::crate_name!(), version = clap::crate_version!(), propagate_version = true, author(clap::crate_authors!()), about = clap::crate_description!(), infer_subcommands = true, long_about(None))]
pub struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the configuration file (default: submod.toml).
    #[arg(long = "config", global = true, default_value = "submod.toml", value_parser = clap::value_parser!(PathBuf), value_hint = clap::ValueHint::FilePath, about = "Optionally provide a different configuration file path. Defaults to 'submod.toml' in the current directory.")]
    pub config: PathBuf,
}

/// Supported subcommands for the `submod` tool.
#[derive(Subcommand)]
pub enum Commands {
    #[subcommand(name = "add", visible_alias = "a", help_heading = "Add a Submodule", about = "Add and initialize a new submodule.")]
    Add {
        #[arg(required = true, action = clap::ArgAction::Set, value_parser = clap::value_parser!(String), about = "The URL or local path of the submodule's git repository.")]
        url: String,

        #[arg(short = "n", long = "name", value_parser = clap::value_parser!(String), about = "Optional *nickname* for the submodule to use in your config and `submod` commands. Otherwise we'll use the relative path, which is what git uses.")]
        name: Option<String>,

        #[arg(short = "p", long = "path", value_parser = clap::value_parser!(OsString), value_hint = clap::ValueHint::DirPath, about = "Local path where you want to put the submodule.")]
        path: Option<OsString>,

        #[arg(short = "b", long = "branch", value_parser(SerializableBranch::from_str), about = "Branch to use for the submodule. If not provided, defaults to the submodule's default branch.")]
        branch: Option<String>,

        #[arg(short = "i", long = "ignore", value_parser = clap::value_parser!(SerializableIgnore), default_value(SerializableIgnore::default().to_string()), about = "What changes in the submodule git should ignore.")]
        ignore: SerializableIgnore,

        #[arg(short = "x", long = "sparse-paths", value_delimiter = ',', about = "Sparse checkout paths (comma-separated). Can be globs or paths")]
        sparse_paths: Option<Vec<String>>,

        #[arg(short = "f", long = "fetch", value_parser = clap::value_parser!(SerializableFetchRecurse), default_value(SerializableFetchRecurse::default().to_string()), about = "Sets the recursive fetch behavior for the submodule (like, if we should fetch its submodules).")]
        fetch: SerializableFetchRecurse,

        #[arg(short = "u", long = "update", value_parser = clap::value_parser!(SerializableUpdate), default_value(SerializableUpdate::default().to_string()), about = "How git should update the submodule when you run `git submodule update`.")]
        update: SerializableUpdate,

        // TODO: Implement this arg
        #[arg(short = "s", long = "shallow", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", value_hint = clap::ValueHint::CommandName, about = "If given, sets the submodule as a shallow clone. It will only fetch the last commit of the branch, not the full history.")]
        shallow: bool,

        // TODO: Implement this arg
        #[arg(long = "no-init", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", value_hint = clap::ValueHint::CommandName, about = "If given, we'll add the submodule to your submod.toml but not initialize it.")]
        no_init: bool,
    },
    // TODO: Implement this subcommand
    #[subcommand(name = "change", help_heading = "Change a Submodule's Settings", about = "Change the configuration of an existing submodule. Any field you provide will overwrite an existing value (unless both are defaults). If you change the path, it will nuke the submodule from orbit (delete it and re-clone it).")]
    Change {
        #[arg(required = true, value_parser = clap::value_parser!(String), value_hint = clap::ValueHint::CommandName, about = "The name of the submodule to change. Must match an existing submodule.", long_about = "The name of the submodule to change. Must match an existing submodule in your submod.toml. Because we use this value to lookup your config, you cannot change the name from the CLI. You must manually change it in your submod.toml. All other options can be changed here.")]
        name: String,

        #[arg(short = "p", long = "path", value_parser = clap::value_parser!(OsString), value_hint = clap::ValueHint::DirPath, about = "New local path for the submodule. Implies `nuke-it-from-orbit` (no-kill) if the path changes.")]
        path: Option<OsString>,

        #[arg(short = "b", long = "branch", value_parser(SerializableBranch::from_str), about = "Change the submodule's branch.")]
        branch: Option<String>,

        #[arg(short = "x", long = "sparse-paths", value_delimiter = ',', value_parser = clap::value_parser!(OsString), about = "Replace the sparse checkout paths (comma-separated), or add if not set. Use `--append` to append to existing sparse paths.", default_missing_value = "none")]
        sparse_paths: Option<Vec<OsString>>,

        #[arg(requires("sparse_paths"), short = "a", long = "append", value_parser = clap::value_parser!(bool), default_value = "false", default_missing_value = "true", about = "If given, appends the new sparse paths to the existing ones.")]
        append: bool,

        #[arg(short = "i", long = "ignore", value_parser = clap::value_parser!(SerializableIgnore), about = "Change the ignore settings for the submodule.")]
        ignore: Option<SerializableIgnore>,

        #[arg(short = "f", long = "fetch", value_parser = clap::value_parser!(SerializableFetchRecurse), about = "Change the fetch settings for the submodule.")]
        fetch: Option<SerializableFetchRecurse>,

        #[arg(short = "u", long = "update", value_parser = clap::value_parser!(SerializableUpdate), about = "Change the update settings for the submodule.")]
        update: Option<SerializableUpdate>,

        #[arg(short = "s", long = "shallow", default_value = "false", default_missing_value = "true", about = "If true, sets the submodule as a shallow clone. Set false to disable shallow cloning.")]
        shallow: bool,
        #[arg(short = "u", long = "url", value_parser = clap::value_parser!(String), about = "Change the URL of the submodule. The submodule name from the url must match an existing submodule.")]
        url: Option<String>,

        #[arg(long = "active", default_value = "true", value_parser = clap::value_parser!(bool), default_missing_value = "true", about = "If true, the submodule will be considered active and included in operations. If false, will disable the submodule. For a shorter version of this command, use `submod disable <name>` instead.")]
        active: bool,
    },
    // TODO: Implement this subcommand
    #[subcommand(name = "change-global", visible_aliases = ["cg", "chgl", "global"], help_heading = "Change Global Settings", about = "Add or change the global settings for submodules, affecting all submodules in the current repository. Any individual submodule settings will override these global settings.")]
    ChangeGlobal {

        #[arg(short = "i", long = "ignore", value_parser = clap::value_parser!(SerializableIgnore), about = "Sets the default ignore behavior for all submodules in this repository. This will override any individual submodule settings.")]

        ignore: Option<SerializableIgnore>,
        #[arg(short = "f", long = "fetch", value_parser = clap::value_parser!(SerializableFetchRecurse), about = "Sets the default fetch behavior for all submodules in this repository. This will override any individual submodule settings.")]

        fetch: Option<SerializableFetchRecurse>,
        #[arg(short = "u", long = "update", value_parser = clap::value_parser!(SerializableUpdate), about = "Sets the default update behavior for all submodules in this repository. This will override any individual submodule settings.")]
        update: Option<SerializableUpdate>,
    },

    #[subcommand(name = "check", visible_alias = "c", help_heading = "Check Submodules", about = "Checks the status of submodules, ensuring they are initialized and up-to-date.")]
    Check,

    // TODO: Implement this subcommand
    #[subcommand(name = "list", visible_aliases = ["ls", "l"], help_heading = "List Submodules", about = "Lists all submodules, optionally recursively.")]
    List {
        /// Recursively list all submodules for the current repository.
        #[arg(short = "r", long = "recursive", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", value_hint = clap::ValueHint::CommandName, about = "If given, lists all submodules recursively (like, the submodules of the submodules).")]
        recursive: bool,
    },

    #[subcommand(name = "init", visible_alias = "i", help_heading = "Initialize Submodules", about = "Initializes missing submodules based on the configuration file.")]
    Init,

    // TODO: Implement this subcommand (use git2 + fs to delete files)
    #[subcommand(name = "delete", visible_alias = "del", help_heading = "Delete a Submodule", about = "Deletes a submodule by name; removes it from the configuration and the filesystem.")]
    Delete,

    // TODO: Implement this subcommand (use git2). Functionally this changes a module to `active = false` in our config and `.gitmodules`, but does not delete the submodule from the filesystem.
    #[subcommand(name = "disable", visible_alias = "d", help_heading = "Disable a Submodule", about = "Disables a submodule by name; sets its active status to false. Does not remove settings or files.")]
    Disable,

    #[subcommand(name = "update", visible_alias = "u", help_heading = "Update Submodules", about = "Updates all submodules to their configured state.")]
    Update,

    #[subcommand(name = "reset", visible_alias = "r", help_heading = "Reset Submodules", about = "Hard resets submodules, stashing changes, resetting to the configured state, and cleaning untracked files.")]
    Reset {

        #[arg(short = "a", long = "all", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", value_hint = clap::ValueHint::CommandName, about = "If given, resets all submodules. If not given, you must specify specific submodules to reset.")]
        all: bool,

        #[arg(required_unless_present = "all", value_delimiter = ',', about = "Names of specific submodules to reset. If `--all` is not given, you must specify at least one submodule name.")]
        names: Vec<String>,
    },

    #[subcommand(name = "sync", visible_alias = "s", help_heading = "Sync Submodules", about = "Runs a full sync: check, init, update. Ensures all submodules are in sync with the configuration.")]
    Sync,

    // TODO: Implement this subcommand
    #[subcommand(name = "generate-config", visible_aliases = ["gc", "genconf"], help_heading = "Generate a Config File", about = "Generates a new configuration file.")]
    GenerateConfig {
        /// Path to the new configuration file to generate.
        #[arg(short('o'), long = "output", value_parser = clap::value_parser!(PathBuf), value_hint = clap::ValueHint::FilePath, default_value = "submod.toml", about = "Path to the output configuration file. Defaults to 'submod.toml' in the current directory.")]
        output: PathBuf,

        #[arg(short = "s", long = "from-setup", about = "Generates the config from your current repository's submodule settings.")]
        from_setup: String,

        #[arg(short = "f", long = "force", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", value_hint = clap::ValueHint::CommandName, about = "If given, overwrites the existing configuration file without prompting.")]
        force: bool,

        #[arg(short = "t", long = "template", about = "Generates a template configuration file with default values.", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", value_hint = clap::ValueHint::CommandName)]
        template: bool,
    },

    // TODO: Implement this subcommand (use git2) (not we can leverage this logic for `delete` because the `kill` option is the same.)
    #[subcommand(name = "nuke-it-from-orbit", visible_aliases = ["nuke-em", "nuke-it", "nuke-them"], help_heading = "Nuke It From Orbit", about = "Deletes all submodules or specific ones, removing them from the configuration and the filesystem. Optionally leaves them dead. 🚀💥👾💥💀.")]
    NukeItFromOrbit {
        #[arg(long = "all", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", value_hint = clap::ValueHint::CommandName, about = "Nuke 'em all? 🤓")]
        all: bool,
        #[arg(required_unless_present = "all", value_delimiter = ',', about = "... or only specific ones? 😔 (comma-separated list of names")]
        names: Option<Vec<String>>,

        #[arg(short = "k", long = "kill", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", value_hint = clap::ValueHint::CommandName, about = "If given, DOES NOT reinitialize the submodules and DOES NOT add them back to the config. They will be truly dead. 💀")]
        kill: bool,
    },

    // TODO: Implement this subcommand (super simple with clap_complete/clap_complete_nushell. The latter is just another enum variant that implements the `Generator` trait like all of the other clap_complete shells.)
    #[subcommand(name = "completions", visible_aliases = ["comp", "complete"], help_heading = "Generate Shell Completions", about = "Generates shell completions for the specified shell.", action= clap::ArgAction::Set, value_parser = clap::value_parser!(Shell))]
    Completions,
}

