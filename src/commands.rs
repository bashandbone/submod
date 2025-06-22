use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "submod")]
#[command(about = "Manage git submodules with sparse checkout support")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    #[arg(short, long, default_value = "submod.toml")]
    pub config: PathBuf,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a new submodule configuration
    Add {
        /// Submodule name
        name: String,
        /// Local path for the submodule
        path: String,
        /// Git repository URL
        url: String,
        /// Sparse checkout paths (comma-separated)
        #[arg(short, long)]
        sparse_paths: Option<String>,
        /// Git settings like "ignore=all"
        #[arg(short = 'S', long)]
        settings: Option<String>,
    },
    /// Check submodule status and configuration
    Check,
    /// Initialize missing submodules
    Init,
    /// Update all submodules
    Update,
    /// Hard reset submodules (stash, reset --hard, clean)
    Reset {
        /// Reset all submodules
        #[arg(short, long)]
        all: bool,
        /// Specific submodule names to reset
        #[arg(required_unless_present = "all")]
        names: Vec<String>,
    },
    /// Run full sync: check, init, update
    Sync,
}
