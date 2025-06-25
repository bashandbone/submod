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
- Minor version bump due to this core change.

### **New Features**

- Submodule CLI options are now flattened for easier use (no more grouping under `settings`).
- Submodule creation now correctly applies options.

### **Backend Changes**

- Added conversions between `git2` submodule option types and our own `SubmoduleOptions` types, enabling easier backend switching and serialization.

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
