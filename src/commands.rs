#![doc = r#"
Command-line argument definitions for the `submod` tool.

Defines the CLI structure and subcommands using [`clap`] for managing git submodules with sparse checkout support.

# Overview

- Parses CLI arguments for the `submod` binary.
- Supports commands for adding, checking, initializing, updating, resetting, and syncing submodules.
- Allows specifying a custom configuration file (default: `submod.toml`).

# Commands

- [`Commands::Add`](src/commands.rs): Adds a new submodule configuration.
- [`Commands::Check`](src/commands.rs): Checks submodule status and configuration.
- [`Commands::Init`](src/commands.rs): Initializes missing submodules.
- [`Commands::Update`](src/commands.rs): Updates all submodules.
- [`Commands::Reset`](src/commands.rs): Hard resets submodules (stash, reset --hard, clean).
- [`Commands::Sync`](src/commands.rs): Runs a full sync (check, init, update).

# Usage Example

```sh
submod add my-lib libs/my-lib https://github.com/example/my-lib.git --sparse-paths "src/,include/" --settings "ignore=all"
submod check
submod init
submod update
submod reset --all
submod sync
```

# Configuration

Use the `--config` option to specify a custom config file.

See the [README.md](../README.md) for full usage and configuration details.
"#]

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Top-level CLI parser for the `submod` tool.
///
/// Accepts a subcommand and an optional config file path.
#[derive(Parser)]
#[command(name = "submod")]
#[command(about = "Manage git submodules with sparse checkout support")]
pub struct Cli {
    /// Subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,

    /// Path to the configuration file (default: submod.toml).
    #[arg(short, long, default_value = "submod.toml")]
    pub config: PathBuf,
}

/// Supported subcommands for the `submod` tool.
#[derive(Subcommand)]
pub enum Commands {
    /// Adds a new submodule configuration.
    Add {
        /// Submodule name.
        name: String,
        /// Local path for the submodule.
        path: String,
        /// Git repository URL.
        url: String,
        /// Sparse checkout paths (comma-separated).
        #[arg(short, long)]
        sparse_paths: Option<String>,
        /// Additional git settings (e.g., "ignore=all").
        #[arg(short = 'S', long)]
        settings: Option<String>,
    },
    /// Checks submodule status and configuration.
    Check,
    /// Initializes missing submodules.
    Init,
    /// Updates all submodules.
    Update,
    /// Hard resets submodules (stash, reset --hard, clean).
    Reset {
        /// Reset all submodules.
        #[arg(short, long)]
        all: bool,
        /// Specific submodule names to reset.
        #[arg(required_unless_present = "all")]
        names: Vec<String>,
    },
    /// Runs a full sync: check, init, update.
    Sync,
}
