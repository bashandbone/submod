// SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
//
// SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

#![doc = r#"
Command-line argument definitions for the `submod` tool.

Defines the CLI structure and commands using [`clap`] for managing git submodules with sparse checkout support.

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
submod add https://github.com/example/my-lib.git --name my-lib --path libs/my-lib --sparse-paths "src/,include/"
submod change my-lib --branch "main" --sparse-paths "src/,include/" --fetch "always" --update "checkout"
submod check
submod init
submod update
submod reset --all
submod sync
submod completeme bash
```

# Configuration

Use the `--config` option to specify a custom config file location.

See the [README.md](../README.md) for full usage and configuration details.
"#]

use clap::{CommandFactory, Parser, Subcommand, ValueEnum, error::ErrorKind};

use crate::long_abouts::COMPLETE_ME;
use std::{ffi::OsString, path::PathBuf};
use submod::options::{
    SerializableFetchRecurse as FetchRecurse, SerializableIgnore as Ignore,
    SerializableUpdate as Update,
};
use submod::shells::Shell;

/// Top-level CLI parser for the `submod` tool.
///
/// Accepts a command and an optional config file path.
#[derive(Parser, Debug)]
#[command(name = clap::crate_name!(), version = clap::crate_version!(), propagate_version = true, author = clap::crate_authors!(), about = clap::crate_description!(), infer_subcommands = true)]
pub struct Cli {
    /// command to execute.
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the configuration file (default: submod.toml).
    #[arg(long = "config", global = true, default_value = "submod.toml", value_parser = clap::value_parser!(PathBuf), value_hint = clap::ValueHint::FilePath, help = "Use this configuration file. Without --config, submod discovers the repository root and uses its submod.toml.")]
    pub config: PathBuf,

    /// Preview a mutating command after full local validation, without locks, writes, staging, or remote access.
    #[arg(long, global = true, action = clap::ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Enable verbose output with detailed status information.
    #[arg(long, short, global = true)]
    pub verbose: bool,
}

/// Optional settings that can be removed to restore inherited behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum UnsetSetting {
    Branch,
    Ignore,
    Fetch,
    Update,
    Shallow,
    Active,
    UseGitDefaultSparseCheckout,
}

impl UnsetSetting {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Branch => "branch",
            Self::Ignore => "ignore",
            Self::Fetch => "fetch",
            Self::Update => "update",
            Self::Shallow => "shallow",
            Self::Active => "active",
            Self::UseGitDefaultSparseCheckout => "use-git-default-sparse-checkout",
        }
    }
}

/// Supported commands for the `submod` tool.
#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(
        name = "add",
        visible_alias = "a",
        next_help_heading = "Add a Submodule",
        about = "Add and initialize a new submodule."
    )]
    Add {
        #[arg(required = true, action = clap::ArgAction::Set, value_parser = clap::value_parser!(String), help = "The URL or local path of the submodule's git repository.")]
        url: String,

        #[arg(short = 'n', long = "name", value_parser = clap::value_parser!(String), help = "Optional *nickname* for the submodule to use in your config and `submod` commands. Otherwise we'll use the relative path, which is what git uses.")]
        name: Option<String>,

        #[arg(short = 'p', long = "path", value_parser = clap::value_parser!(OsString), value_hint = clap::ValueHint::DirPath, help = "Local path where you want to put the submodule.")]
        path: Option<OsString>,

        #[arg(
            short = 'b',
            long = "branch",
            help = "Branch to use for the submodule. If not provided, defaults to the submodule's default branch."
        )]
        branch: Option<String>,

        #[arg(
            short = 'i',
            long = "ignore",
            help = "What changes in the submodule git should ignore."
        )]
        ignore: Option<Ignore>,

        #[arg(
            short = 'x',
            long = "sparse-paths",
            value_delimiter = ',',
            help = "Sparse checkout paths (comma-separated). Can be globs or paths"
        )]
        sparse_paths: Option<Vec<String>>,

        #[arg(
            long = "use-git-default-sparse-checkout",
            num_args = 0..=1,
            value_parser = clap::value_parser!(bool),
            default_missing_value = "true",
            help = "Opt out of submod's deny-all-by-default sparse-checkout model and use git's built-in behavior instead. When set, the `!/*` prefix is NOT prepended automatically."
        )]
        use_git_default_sparse_checkout: Option<bool>,

        #[arg(
            short = 'f',
            long = "fetch",
            help = "Sets the recursive fetch behavior for the submodule (like, if we should fetch its submodules)."
        )]
        fetch: Option<FetchRecurse>,

        #[arg(
            short = 'u',
            long = "update",
            help = "How git should update the submodule when you run `git submodule update`."
        )]
        update: Option<Update>,

        #[arg(short = 's', long = "shallow", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", help = "If given, sets the submodule as a shallow clone. It will only fetch the last commit of the branch, not the full history.")]
        shallow: bool,

        #[arg(long = "no-init", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", help = "If given, we'll add the submodule to your submod.toml but not initialize it.")]
        no_init: bool,
    },
    #[command(
        name = "change",
        group(clap::ArgGroup::new("settings").required(true).multiple(true).args(["path", "branch", "sparse_paths", "clear_sparse_paths", "use_git_default_sparse_checkout", "ignore", "fetch", "update", "shallow", "url", "active", "unset"])),
        next_help_heading = "Change a Submodule's Settings",
        about = "Change the configuration of an existing submodule. Only provided fields are changed. A path change uses a safe Git-aware move for clean initialized modules; dirty, conflicted, or unsupported moves are refused without mutation."
    )]
    Change {
        #[arg(required = true, value_parser = clap::value_parser!(String), value_hint = clap::ValueHint::CommandName, help = "The name of the submodule to change. Must match an existing submodule.", long_help = "The name of the submodule to change. Must match an existing submodule in your submod.toml. Because we use this value to lookup your config, you cannot change the name from the CLI. You must manually change it in your submod.toml. All other options can be changed here.")]
        name: String,

        #[arg(short = 'p', long = "path", value_parser = clap::value_parser!(OsString), value_hint = clap::ValueHint::DirPath, help = "New local path for the submodule. Implies `nuke-it-from-orbit` (no-kill) if the path changes.")]
        path: Option<OsString>,

        #[arg(
            short = 'b',
            long = "branch",
            help = "Branch to use for the submodule. If not provided, defaults to the submodule's default branch."
        )]
        branch: Option<String>,

        #[arg(short = 'x', long = "sparse-paths", value_delimiter = ',', value_parser = clap::value_parser!(OsString), help = "Replace the sparse checkout paths (comma-separated), or add if not set. Use `--append` to append to existing sparse paths.")]
        sparse_paths: Option<Vec<OsString>>,

        #[arg(requires("sparse_paths"), short = 'a', long = "append", value_parser = clap::value_parser!(bool), default_value = "false", default_missing_value = "true", help = "If given, appends the new sparse paths to the existing ones.")]
        append: bool,

        #[arg(long, conflicts_with_all = ["sparse_paths", "append"], help = "Clear all sparse checkout paths.")]
        clear_sparse_paths: bool,

        #[arg(
            long,
            value_enum,
            value_delimiter = ',',
            help = "Remove optional overrides to restore inherited behavior. May be repeated."
        )]
        unset: Vec<UnsetSetting>,

        #[arg(
            long = "use-git-default-sparse-checkout",
            num_args = 0..=1,
            value_parser = clap::value_parser!(bool),
            default_missing_value = "true",
            help = "Opt out of submod's deny-all-by-default sparse-checkout model and use git's built-in behavior instead."
        )]
        use_git_default_sparse_checkout: Option<bool>,

        #[arg(
            short = 'i',
            long = "ignore",
            help = "Change the ignore settings for the submodule."
        )]
        ignore: Option<Ignore>,

        #[arg(
            short = 'f',
            long = "fetch",
            help = "Change the fetch settings for the submodule."
        )]
        fetch: Option<FetchRecurse>,

        #[arg(
            short = 'u',
            long = "update",
            help = "Change the update settings for the submodule."
        )]
        update: Option<Update>,

        #[arg(
            short = 's',
            long = "shallow",
            num_args = 0..=1,
            default_missing_value = "true",
            help = "If true, sets the submodule as a shallow clone. Set false to disable shallow cloning."
        )]
        shallow: Option<bool>,

        #[arg(short = 'U', long = "url", value_parser = clap::value_parser!(String), help = "Change the URL of the submodule. The submodule name from the url must match an existing submodule.")]
        url: Option<String>,

        #[arg(long = "active", num_args = 0..=1, value_parser = clap::value_parser!(bool), default_missing_value = "true", help = "Set to true/false to enable or disable the submodule. Omit to leave unchanged. For a quick disable, use `submod disable <name>` instead.")]
        active: Option<bool>,
    },
    #[command(name = "change-global", visible_aliases = ["cg", "chgl", "global"], next_help_heading = "Change Global Settings", about = "Patch inherited defaults. Explicit per-submodule settings take precedence.")]
    #[command(group(clap::ArgGroup::new("settings").required(true).multiple(true).args(["branch", "ignore", "fetch", "update", "use_git_default_sparse_checkout", "unset"])))]
    ChangeGlobal {
        #[arg(
            long,
            value_enum,
            value_delimiter = ',',
            help = "Remove a global default to restore the built-in behavior."
        )]
        unset: Vec<UnsetSetting>,
        #[arg(
            short = 'b',
            long = "branch",
            help = "Set the inherited tracking branch. Use --unset branch to restore each remote's default branch."
        )]
        branch: Option<String>,
        #[arg(
            short = 'i',
            long = "ignore",
            help = "Set the inherited ignore behavior. An explicit per-submodule value takes precedence."
        )]
        ignore: Option<Ignore>,

        #[arg(
            short = 'f',
            long = "fetch",
            help = "Set the inherited fetch behavior. An explicit per-submodule value takes precedence."
        )]
        fetch: Option<FetchRecurse>,

        #[arg(
            short = 'u',
            long = "update",
            help = "Set the inherited update behavior. An explicit per-submodule value takes precedence."
        )]
        update: Option<Update>,

        #[arg(
            long = "use-git-default-sparse-checkout",
            num_args = 0..=1,
            value_parser = clap::value_parser!(bool),
            default_missing_value = "true",
            help = "Set the global default for sparse-checkout mode. When true, all submodules use git's built-in behavior instead of submod's deny-all-by-default model (unless overridden per-submodule)."
        )]
        use_git_default_sparse_checkout: Option<bool>,
    },

    #[command(
        name = "check",
        visible_alias = "c",
        next_help_heading = "Check Submodules",
        about = "Checks the status of submodules, ensuring they are initialized and up-to-date."
    )]
    Check,

    #[command(name = "list", visible_aliases = ["ls", "l"], next_help_heading = "List Submodules", about = "Lists all submodules, optionally recursively.")]
    List {
        /// Recursively list all submodules for the current repository.
        #[arg(short = 'r', long = "recursive", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", help = "If given, lists all submodules recursively (like, the submodules of the submodules).")]
        recursive: bool,
    },

    #[command(
        name = "init",
        visible_alias = "i",
        next_help_heading = "Initialize Submodules",
        about = "Initializes missing submodules based on the configuration file."
    )]
    Init {
        /// Also initialize nested submodules selected by each managed submodule.
        #[arg(short = 'r', long = "recursive", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true")]
        recursive: bool,
    },

    #[command(
        name = "delete",
        visible_alias = "del",
        next_help_heading = "Delete a Submodule",
        about = "Deletes a submodule by name; removes it from the configuration and the filesystem."
    )]
    Delete {
        /// Name of the submodule to delete.
        #[arg(help = "Name of the submodule to delete.")]
        name: String,

        /// Discard tracked, untracked, and ignored content inside the verified checkout.
        #[arg(long, action = clap::ArgAction::SetTrue, help = "Discard local content inside the verified submodule checkout. Never removes an unrelated path or repository.")]
        force: bool,
    },

    #[command(
        name = "disable",
        visible_alias = "d",
        next_help_heading = "Disable a Submodule",
        about = "Disables a submodule by name; sets its active status to false. Does not remove settings or files."
    )]
    Disable {
        /// Name of the submodule to disable.
        #[arg(help = "Name of the submodule to disable.")]
        name: String,
    },

    #[command(
        name = "update",
        visible_alias = "u",
        next_help_heading = "Update Submodules",
        about = "Updates all submodules to their configured state."
    )]
    Update {
        /// Advance each submodule to its configured remote-tracking branch.
        #[arg(long = "remote", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true")]
        remote: bool,

        /// Also update nested submodules selected by each managed submodule.
        #[arg(short = 'r', long = "recursive", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true")]
        recursive: bool,
    },

    #[command(
        name = "reset",
        visible_alias = "r",
        next_help_heading = "Reset Submodules",
        about = "Preserve local changes in a named stash, then reset selected submodules to their parent gitlinks."
    )]
    Reset {
        #[arg(short = 'a', long = "all", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", help = "If given, resets all submodules. If not given, you must specify specific submodules to reset.")]
        all: bool,

        #[arg(
            required_unless_present = "all",
            conflicts_with = "all",
            value_delimiter = ',',
            help = "Names of specific submodules to reset. If `--all` is not given, you must specify at least one submodule name."
        )]
        names: Vec<String>,
    },

    #[command(
        name = "sync",
        visible_alias = "s",
        next_help_heading = "Sync Submodules",
        about = "Runs a full sync: check, init, update. Ensures all submodules are in sync with the configuration."
    )]
    Sync {
        /// Also initialize and update nested submodules selected by each managed submodule.
        #[arg(short = 'r', long = "recursive", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true")]
        recursive: bool,
    },

    #[command(name = "generate-config", visible_aliases = ["gc", "genconf"], next_help_heading = "Generate a Config File", about = "Generates a new configuration file.")]
    GenerateConfig {
        /// Path to the new configuration file to generate.
        #[arg(short = 'o', long = "output", value_parser = clap::value_parser!(PathBuf), value_hint = clap::ValueHint::FilePath, default_value = "submod.toml", help = "Path to the output configuration file. Defaults to submod.toml in the current directory.")]
        output: PathBuf,

        #[arg(
            short = 's',
            long = "from-setup",
            action = clap::ArgAction::SetTrue,
            conflicts_with = "template",
            help = "Generates the config from your current repository's submodule settings."
        )]
        from_setup: bool,

        #[arg(short = 'f', long = "force", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", help = "If given, overwrites the existing configuration file without prompting.")]
        force: bool,

        #[arg(short = 't', long = "template", conflicts_with = "from_setup", help = "Generates a template configuration file with placeholder URLs.", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true")]
        template: bool,
    },

    #[command(name = "nuke-it-from-orbit", visible_aliases = ["nu", "nuke-em", "nuke-it", "nuke-them"], next_help_heading = "Nuke It From Orbit", about = "Repair selected submodules by rebuilding their checkouts, or remove them with --kill.")]
    NukeItFromOrbit {
        #[arg(long = "all", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", help = "Nuke 'em all? 🤓")]
        all: bool,
        #[arg(
            required_unless_present = "all",
            conflicts_with = "all",
            value_delimiter = ',',
            help = "... or only specific ones? 😔 (comma-separated list of names"
        )]
        names: Option<Vec<String>>,

        #[arg(short = 'k', long = "kill", default_value = "false", action = clap::ArgAction::SetTrue, default_missing_value = "true", help = "If given, DOES NOT reinitialize the submodules and DOES NOT add them back to the config. They will be truly dead. 💀")]
        kill: bool,

        #[arg(long, action = clap::ArgAction::SetTrue, help = "Discard local content inside each verified submodule checkout. Never removes unrelated paths or repositories.")]
        force: bool,
    },

    // Shell completions are implemented using clap_complete/clap_complete_nushell
    #[command(name = "completeme", visible_aliases = ["comp", "complete", "comp-me", "complete-me"], next_help_heading = "Generate Shell Completions", about = "Generates shell completions for the specified shell. Completions generated to stdout.", long_about = COMPLETE_ME)]
    CompleteMe {
        #[arg(value_enum, action = clap::ArgAction::Set, help = "The shell to generate completions for. Supported shells: `bash`, `zsh`, `fish`, `powershell`, `elvish`, `nushell`.")]
        shell: Shell,
    },
}

impl Cli {
    /// Validate combinations whose legality depends on argument values.
    pub fn validate(&self) -> Result<(), clap::Error> {
        if self.dry_run
            && matches!(
                self.command,
                Commands::Check | Commands::List { .. } | Commands::CompleteMe { .. }
            )
        {
            return Err(Self::command().error(
                ErrorKind::ArgumentConflict,
                "--dry-run is only valid for mutating commands",
            ));
        }
        let (unset, supplied): (&[UnsetSetting], Vec<(UnsetSetting, bool)>) = match &self.command {
            Commands::Change {
                branch,
                ignore,
                fetch,
                update,
                shallow,
                active,
                use_git_default_sparse_checkout,
                unset,
                ..
            } => (
                unset,
                vec![
                    (UnsetSetting::Branch, branch.is_some()),
                    (UnsetSetting::Ignore, ignore.is_some()),
                    (UnsetSetting::Fetch, fetch.is_some()),
                    (UnsetSetting::Update, update.is_some()),
                    (UnsetSetting::Shallow, shallow.is_some()),
                    (UnsetSetting::Active, active.is_some()),
                    (
                        UnsetSetting::UseGitDefaultSparseCheckout,
                        use_git_default_sparse_checkout.is_some(),
                    ),
                ],
            ),
            Commands::ChangeGlobal {
                branch,
                ignore,
                fetch,
                update,
                use_git_default_sparse_checkout,
                unset,
            } => {
                if !unset.iter().all(|setting| {
                    matches!(
                        setting,
                        UnsetSetting::Branch
                            | UnsetSetting::Ignore
                            | UnsetSetting::Fetch
                            | UnsetSetting::Update
                            | UnsetSetting::UseGitDefaultSparseCheckout
                    )
                }) {
                    return Err(Self::command().error(
                        ErrorKind::InvalidValue,
                        "Unsupported global setting in --unset",
                    ));
                }
                (
                    unset,
                    vec![
                        (UnsetSetting::Branch, branch.is_some()),
                        (UnsetSetting::Ignore, ignore.is_some()),
                        (UnsetSetting::Fetch, fetch.is_some()),
                        (UnsetSetting::Update, update.is_some()),
                        (
                            UnsetSetting::UseGitDefaultSparseCheckout,
                            use_git_default_sparse_checkout.is_some(),
                        ),
                    ],
                )
            }
            Commands::Reset { names, .. }
            | Commands::NukeItFromOrbit {
                names: Some(names), ..
            } => {
                let mut seen = std::collections::HashSet::new();
                for name in names {
                    if name.trim().is_empty() {
                        return Err(Self::command()
                            .error(ErrorKind::InvalidValue, "Submodule target cannot be empty"));
                    }
                    if !seen.insert(name) {
                        return Err(Self::command().error(
                            ErrorKind::ArgumentConflict,
                            format!("Duplicate submodule target: {name}"),
                        ));
                    }
                }
                return Ok(());
            }
            _ => return Ok(()),
        };
        for (index, setting) in unset.iter().enumerate() {
            if unset[..index].contains(setting) {
                return Err(Self::command().error(
                    ErrorKind::ArgumentConflict,
                    format!("Duplicate --unset setting: {}", setting.as_str()),
                ));
            }
            if supplied
                .iter()
                .any(|(field, present)| field == setting && *present)
            {
                return Err(Self::command().error(
                    ErrorKind::ArgumentConflict,
                    format!("Cannot set and unset {} together", setting.as_str()),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn change_contract() {
        for args in [
            vec!["submod", "change", "module"],
            vec!["submod", "cg"],
            vec!["submod", "change", "module", "--append"],
            vec![
                "submod",
                "change",
                "module",
                "--clear-sparse-paths",
                "--sparse-paths",
                "src",
            ],
            vec![
                "submod",
                "change",
                "module",
                "--clear-sparse-paths",
                "--append",
                "--sparse-paths",
                "src",
            ],
            vec!["submod", "reset", "--all", "module"],
            vec!["submod", "nuke-it", "--all", "module"],
        ] {
            assert!(Cli::try_parse_from(&args).is_err(), "{args:?}");
        }
        for args in [
            vec![
                "submod",
                "change",
                "module",
                "--shallow=false",
                "--unset",
                "shallow",
            ],
            vec!["submod", "reset", "module,module"],
            vec!["submod", "nuke-it", "module", "module"],
            vec!["submod", "change", "module", "--unset", "ignore,ignore"],
        ] {
            assert!(
                Cli::try_parse_from(&args).unwrap().validate().is_err(),
                "{args:?}"
            );
        }
        for (flags, expected) in [
            (vec!["--branch", "main"], None),
            (vec!["--shallow"], Some(true)),
            (vec!["--shallow=false"], Some(false)),
        ] {
            let cli =
                Cli::try_parse_from([vec!["submod", "change", "module"], flags].concat()).unwrap();
            cli.validate().unwrap();
            let Commands::Change { shallow, .. } = cli.command else {
                panic!("wrong command")
            };
            assert_eq!(shallow, expected);
        }
        for args in [
            vec!["submod", "change", "module", "--clear-sparse-paths"],
            vec!["submod", "change", "module", "--unset", "shallow,branch"],
            vec!["submod", "cg", "--unset", "branch"],
            vec!["submod", "cg", "--use-git-default-sparse-checkout=false"],
            vec![
                "submod",
                "global",
                "--unset",
                "use-git-default-sparse-checkout",
            ],
            vec![
                "submod",
                "change",
                "module",
                "--append",
                "--sparse-paths",
                "src",
            ],
        ] {
            Cli::try_parse_from(&args).unwrap().validate().unwrap();
        }
    }
}
