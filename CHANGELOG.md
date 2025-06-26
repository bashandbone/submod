<!--
SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>

SPDX-License-Identifier: MIT

Licensed under the [Plain MIT License](LICENSE.md)
-->

# Changelog

## v0.2.0 - 2025-06-23

### **Git2 Now Required**

- `git2` is now a required dependency. `gix_submodule` (gitoxide) lacks needed features for now, so we rely on `git2` until it matures.
- The implementation already depended on `git2`; this formalizes that requirement.
- We'll revisit `gitoxide` as it adds features.
- Minor version bump due to this core change and other big changes...

### **New Features**

- Submodule CLI options for `submod add` are now flattened for easier use (no more grouping under `settings`).
- Submodule creation now correctly applies options.
- Added `--shallow` option to `submod add` to create shallow submodules and `--no-init` option to add the submodule to your submod.toml without initializing it.
- **Many new commands**:
  - `submod change` to update or change all options for a specific submodule.
  - `submod change-global` to update or change global defaults for all submodules in the repo.
  - `submod list` to list all submodules with their current settings.
  - `submod delete` to remove a submodule from the repository and the submod.toml file, and all of its files.
  - `submod disable` to set a submodule to inactive nondestructively, allowing it to be re-enabled later.
  - `submod generate-config` to generate a submod.toml file from the current git submodule setup or a template.
  - `submod nuke-it-from-orbit`. By default this is an **extra hard reset**, removing all submodules and files, deleting all references to them in git's configuration, resyncing with git, before *adding them all back to the gitconfig from your submod.toml settings*. You can optionally pass the `--kill` flag to *completely* remove them without adding them back to the gitconfig, and deleting them from your submod.toml (this is the same as `submod delete` but for multiple submodules).
  - `submod completions` to generate shell completions for the submod CLI in bash, zsh, fish, elvish, nushell, and powershell.
- Improved the `help` text for all commands, making it more informative and user-friendly.

### **Backend Changes**

- Added conversions between `git2` submodule option types and our own `SubmoduleOptions` types, enabling easier backend switching and serialization.
- Removed nearly all use of `git` directly in the codebase, relying on `gix` and `git2` for all git operations.
- Much better use of `clap` value parsing and validation.

## v0.1.2 - 2025-06-22

- **First Release**: Initial release of the submodule management tool.
- **Core Features**:
  - Basic submodule management using gitoxide.
  - Support for TOML configuration files.
  - Command-line interface with basic commands.
  - Integration with git2 for fallback operations.
- **Documentation**:
  - Initial documentation for core modules.
  - Solid README with setup instructions.
- **Testing**:
  - Good integration test coverage; all passing.
- **Linting**:
  - Strict linting configuration with clippy.
  - Hk/Mise integration for git hooks, tool management, task running
