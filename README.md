# `submod` crate

A lightweight CLI tool for managing git submodules. It provides a simple interface to set up, update, and manage submodules in your git repositories. Built ontop of `gitoxide` and `git2` libraries, it aims to be fast and efficient.

## Features

- Toml-based configuration to define submodules, sparse-checkout paths, and other settings.
- Set global submodule settings with submodule-specific overrides.
- Support for adding, removing, and updating submodules with ease.
- Automatic handling of sparse-checkout paths and configuration.

## Motivation

I have multiple projects (at @knitli and @plainlicense) that use submodules for core features, and I found scripting initialization and management was tedious and error-prone. This tool aims to simplify the process of managing submodules, especially when dealing with sparse checkouts and multiple repositories. The documentation for `git submodule` and `git sparse-checkout` isn't always clear, and this tool aims to provide a more user-friendly interface.

I see the current state of submodule management as a barrier to contributions to projects that use submodules and for new developers generally. `submod` makes it easier for developers to work with submodules, especially in larger projects where submodules are used extensively.

## Installation

### Using Cargo

You can install the `submod` crate using Cargo:

```bash
cargo install submod
```

If you don't have Cargo installed, you can follow the instructions on the [Rust installation page](https://www.rust-lang.org/tools/install).

### Using Mise

[Mise (docs)][mise-docs] ([repo][mise-repo]) is a project management tool and package manager.

The workflow I use for my repos (including this one) is to use Mise to bootstrap developer setup, which includes installing `submod` and other tools.
You can install `submod` using Mise with the following command:

```bash
# the '-g' flag installs the package globally, leave it out for a project-specific installation
mise use -g cargo:submod@latest
```

## Usage

TODO: Add usage examples and documentation

[mise-docs]: https://mise.jdx.dev/ "Mise documentation"
[mise-repo]: https://github.com/jdx/mise "Mise GitHub repository"
