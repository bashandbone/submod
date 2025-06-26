<!--
SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>

SPDX-License-Identifier: MIT
Licensed under the [Plain MIT License](LICENSE.md)
-->

# What we need to do

## Big Picture

1. We need to ensure that we are actually modifying `git` configs/.gitmodules when we actually change submod.toml or CLI options. Our config exists to simplify the user experience, but we need to ensure that the underlying git configuration is also updated accordingly.

2. I'd like to **eliminate** all use of `git` cli calls.

    - `Gitoxide` has good coverage of _most_ of what we need, and can fully handle:

        - Reading and writing git configs (`gix_config`)
        - **Reading** `.gitmodules` files (`gix_submodule`)
        - Reporting repository and submodule status
        - Most fetching (it can't, however `init` a submodule yet)

    - `git2` has all of the remaining functionality we need:

        - It can completely create and initialize submodules, including cloning them.
        - Any config ops not covered by gix_config can be handled by `git2`.

    - Absolute worst case, we can use `gix_command` to run `git` commands, which at least gives us a consistent interface and pushes validation and error handling down to the `gitoxide` layer.
    - **Bottom line: If you see a `git` command in the code; it's wrong.**

3. Besides triggering creation and updates, we should generally avoid trying to emulate `git` tasks. We update and tell git to do the work through `gitoxide` and `git2`, and keep its configs in sync with our own.

    - We don't need to handle fetching behavior, for example. We just need to make sure that git is configured to fetch how the user wants it to, and then let git handle the fetching. We can trigger it with `git2` or `gix` but we don't do it ourselves.

4. Minimize direct file ops. Again, let `gitoxide` and `git2` handle the file ops. We should not be reading or writing `.gitmodules` or `.git/config` directly, except through the git libraries. The exception _might_ be `sparse-checkout` files, but even those I think worst case we can use `gix_command` to handle them since git clearly can do it.

    - **Possible Exception**: Our `nuke-it-from-orbit` and `delete` commands -- using internal `git submodule deinit` like functionality _should_ be fine, but if it doesn't work we have to be able to do direct deletion of the submodule directory and files. `git2` should have this functionality of basically the library equivalent of `git submodule deinit`, and worst case we can use `gix_command` to run the command and then delete the submodule directory and files ourselves. We also need to look at exactly what git does in a deinit -- we may need to add `git rm --cached` like behavior (possibly with an optional flag like `--it-never-existed`). We should also remove the submodule from the `.gitmodules` file and the `.git/config` file, but we can do that through `git2` or `gix_config`.

5. The longterm goal is to solely use `gix` when it has the full functionality we need, so we need to architect for easy transition between `gix` and `git2` as needed. Basically we need our types to be interchangeable between `gix` and `git2` where possible, so that we can swap them out as needed without having to rewrite large portions of the code.

6. A possible issue we should think about. We don't currently have a way to differentiate configuration at the user, repository, submodule, superproject, and global levels. We should consider how we can handle that in the future, but for now we can just assume that all configuration is at the repository level. For example, if we allowed user configs (i.e. `~/.config/submod.toml` or in a repo `.dev.submod.toml` (added to the `.gitignore`)), we would write the user `.git/config` instead of the `.gitmodules`. That would let users add/change submodule behavior for a repository to suit their needs without impacting the repo behavior. This is a future consideration, but something we should keep in mind as we design our configuration system. I think _it mostly_ matters for user and repo level. Most folks don't want global submodule configs, and we can leave it on the user to differentiate between sub-repo and superproject configs.

    - A related abstraction I think would be useful is to differentiate `developer` and `user` configs. Kind of like `developer` dependencies -- i.e. anyone working on the project itself can pull the developer config and get those submodules, but a user who clones the project to build it doesn't need to pull those submodules. You could use this to basically vendor documentation for dependencies, for example (very helpful for MCP context). This is a future consideration, but I think it would be useful to keep in mind as we design our configuration system. That is, they _would_ have the submodules in the _.gitignore_ for the project, but the `submod.toml` could be used to pull them. As I write this, I realize all we would need to do is add a `developer` flag and write the submodule path to the `.gitignore` file. The developer flag would tell us to write submodule settings to `.git/config` instead of `.gitmodules`. (I'm going to go ahead and pencil this in in the commands.rs file, but we can revisit it later.)
    - A really killer version of this would be to automatically find the project docs and make them sparse. We could look at how tools like `context7` handle very similar behavior (I'm guessing look for a docs dir or _.md/_.mdx/*.rst files.) Like those, you'd want it to be able to handle pulling from public git*and where that doesn't work well\*, snapshotting webpages to markdown. Throwing a little bit of search on top of that would be even better... (maybe that's out of scope or more suitable for a dedicated tool, but it's a neat idea.)

**Update**: After some research, I went ahead and implemented the foundations for `figment` for our config/toml handling. It enormously simplified things, pushing all of the config loading, merging, and validation to `figment`, which is a great fit for our needs. It also gives us good provenance for our config, so we can easily see where settings came from (i.e. CLI options, submod.toml, etc.). That will allow us to just write the logic for how we push that information to git's configuration, and let `figment` handle the rest. This also means we can easily add new config sources in the future (like user configs) without having to rewrite a lot of code. (I actually penciled in user and developer configs in a comment in the config.rs file; but we can revisit that later.)

Summary of the flow:

**gix** -> **git2** -> **gix_command** (if needed)

## Missing Core Functionality to Support All Features

### Setting and Updating Global Defaults

Git, of course, does not have a concept of "global defaults" for submodules (as far as I'm aware). That's an abstraction _we introduced_ to simplify the user experience. However, we need to ensure that these defaults are reflected in the git configuration. It means that where a user has not explicitly set a value in their `submod.toml` for a specific submodule, we should use the global defaults to fill in the gaps when we update `.gitmodules` (through git2).

- We _should already_ have a method that sets global defaults because the behavior should be there for handling global defaults in the config
  - **I couldn't find it in the codebase**, so we need to verify it exists and is working as expected. This is something we also need to make sure is tested.

**The behavior ^^should^^ be**:

- Any default setting for `ignore`, `fetch`, `update` _are not_ written to the `.gitmodules` file _or the submod.toml_. We leave those blank.
  - If a user sets a specific value for `ignore`, `fetch`, or `update` in their `submod.toml` or through CLI options, we write that value to the `submod.toml` file but not the `.gitmodules` file (because there are no global default overrides in git -- we reconcile it).
  - We treat those as overrides of the global defaults (i.e. if the user sets `ignore = "all"` as a global default, but then sets `ignore = "none"` for a specific submodule, we change all other submodules to `ignore = "all"` in `.gitmodules` but leave the specific submodule as `ignore = "none"` in the `.gitmodules` file), using our config as the source of truth.
- **Non-default values for ignore, fetch, update**. For non-default values, we always write them to both `submod.toml` and `.gitmodules`. We just need to deconflict them with the global defaults (submodule config wins) before updating `.gitmodules`.
- (sidenote: consider adding `shallow` as a global default, but I think we can leave that for later)

### Setting and Updating All Other Settings

Like with the global defaults, we need to ensure that our configuration is getting accurately reflected in the `.gitmodules` file and the `.git/config` file. This means that when we update settings in `submod.toml` or through CLI options, we need to ensure that those changes are also reflected in the git configuration.

**The behavior ^^should^^ be**:

- **All other settings get written to both submod.toml and .gitmodules if explicitly set** `submod.toml` and `.gitmodules` files. There are no global defaults for these settings, so we always write them to both files _if explicitly set_ in the `submod.toml` or CLI options.
  - Note that some of our "defaults" are actually more like _inferred settings_, those should always get written to both. The biggest example is `name`, which is actually required for both our ops and git's submodule handling, but we infer it from the submodule path (it's hidden on the `add` command).

### Toml/Config Handling

A lot of what we need to reconcile and work on is keeping our config properly updated and aligned with the git configuration. New commands and features require granular control over how we read and write the configuration, so we need to ensure that our config handling is robust and flexible.

After some investigation, I found that `figment` is a good choice for our toml handling. We can replace a lot of our manual parsing and validation with `figment`'s built-in functionality, which will simplify our code and make it more maintainable. We already have serialize/deserialize traits for our types, so it's literally just a few lines of code. Figment handles layer config loading, merging, and validation for us and keeps good provenance, so we can focus on the actual logic of our commands.

### CLI Parsing and Validation

- **Leveraging Clap**. I added `clap`'s `ValueEnum` trait to all of our config enums. This integrates validation and parsing directly with clap. We need to update all the handling logic from the parsed arguments to reflect this, and I suspect remove a lot of code.
- I also specified clap value parsers for _everything_ that needs to be parsed, so we can remove a lot of the manual parsing and validation code.
- **Type changes**. I also narrowed some of the existing default types to more specific types. Specifically, `path` now uses `OsString` instead of `String` (because it can handle non-UTF8 paths). I didn't make this PathBuf because we're not actually using it as a path, but I added a conversion function in `utilities.rs` to convert it to a `PathBuf` when needed.

#### Add config generation

We need to add config generation for submod.toml. This should be pretty trivial since we already have the logic to add/update it.

- I did a lot of work beefing up the example config in [`sample_config/submod.toml`](sample_config/submod.toml). I also added placeholders in `commands.rs` for a config generation command with options.
  - The command has two options in terms of content:
    - `--from-setup` to generate a config from the current git submodule setup (i.e. the current `.gitmodules` and `.git/config` files). This is just reversing the process we do in `add` and `update` commands.
    - `--template` to generate a config from a template. This would basically just copy the `sample_config/submod.toml` file to the current directory, and then the user can edit it as needed (how do we make sure it gets packaged in the binary? Or I guess we could fetch it from the repo... but that assumes internet connectivity - new territory for me with rust).

## Implement the New Commands and Make Sure Existing Commands Do What They Should

We need to implement the new commands and ensure that existing commands do what they should. This includes:

1. `submod add` (see above on making sure it does what it should)

    - Once question is how to handle a user trying to add an existing submodule. I'd lean towards erroring out and telling them to use `submod change` instead. We should spit out their exact command as a `submod change` command in our error message so they can easily copy/paste it.
    - We need to ensure that it updates the `.gitmodules` file and the `.git/config` file as needed.
    - Add `shallow` and `no-init` options to the command.
    - We also need to make sure submod add _is_ _initiating_ the submodule; I can't tell if it does that now, but it should. For us we just use the `submod init` logic to do that, so we need to ensure that it works as expected.

2. `submod change` (new) to update or change all options for a specific submodule.

    - We just need the deconfliction logic on how to handle updates/changes.
    - Here if a user tries to change a submodule that _doesn't exist_ I'd lean towards a prompt asking if they want to create it, which would redirect it to `submod add`.

3. `submod change-global` (new) to update or change global defaults for all submodules in the repo.

    - As discussed above, we should already have this logic, and if we don't, it is something we have to do anyway.

4. `check` We need to make sure this is differentiated from `submod list`.

    - `submod check` should check and report the current status of all submodules.
    - I had considered adding a `submod status` command, but I think that functionality can go here.
    - `gix` already has a fair amount of statistics and status summary features for its own cli; we should be able to directly add those without writing our own. The gix crate implementing all of its CLI capabilities is `gitoxide-core` (library part), actual CLI handling is in the main `gitoxide` crate.

5. `submod list` (new) We need to make sure this is properly listing all submodules with their current settings.

6. `submod init` We need to make sure this is properly initializing submodules.

    - Remove any existing reliance on `git` commands for this.

7. `submod delete` (new) to remove a submodule from the repository and the submod.toml file, and all of its files.

    - This should be a hard delete, removing the submodule directory and files, and deleting all references to it in git's configuration.
    - We need to ensure that it updates the `.gitmodules` file and the `.git/config` file as needed and actually clears the submodule's entry from the configuration.
    - We will also use this logic for the `nuke-it-from-orbit --kill` command, so we need to ensure that it works as expected.

8. `submod disable` (new) to set a submodule to inactive nondestructively, allowing it to be re-enabled later.

    - Very simple -- set the submodule's `active` flag to `false` in the `submod.toml` and `.gitmodules` files.

9. `submod update` -- just need to move any `git` calls to `git2` or `gix_command` as needed.

10. `submod reset` -- just need to move any `git` calls to `git2` or `gix_command` as needed.

11. `submod sync` -- this should be fine, as it's just a wrapper for `check, init, update` commands. If we make `check` more of a status command, we would need to separate the `check` logic for this command (check would do both).

12. `submod generate-config` (new) to generate a submod.toml file from the current git submodule setup or a template.

    - This should be pretty trivial since we already have the logic to add/update it.
    - We can use the `--from-setup` and `--template` options as discussed above.
    - the optional `--output` option is a PathBuf to write the generated config to, defaulting to `submod.toml` in the current directory.

13. `submod nuke-it-from-orbit` (new).

    - Most of the underlying logic is used or will be used in other parts of the codebase.
    - By default it
    - By default this is an **extra hard reset**, removing all submodules and files, deleting all references to them in git's configuration, resyncing with git, before _adding them all back to the gitconfig from your submod.toml settings_. You can optionally pass the `--kill` flag to _completely_ remove them without adding them back to the gitconfig, and deleting them from your submod.toml (this is the same as `submod delete` but for multiple submodules).

14. `submod completions` (new) This is super simple. It's just a parse operation with a `generate` command passing the shell type as an argument. I already added `Shell` and `generate` to the imports and types for the command, and added the dependencies to `Cargo.toml`. We just need that quick function to trigger them.

Other: - **global `--config` option**. I made this truly global so it can be used with any command. It is also now a PathBuf instead of a String, so it can be used with clap's value parsers. It defaults to `submod.toml` in the current directory, but can be overridden with the `--config` option.

- **We need to make sure we are actually using this option in all commands**. Be on the lookout for any commands that assume a config location or don't use the global config option. We should be using the `config` variable in `commands.rs` to get the config location and read/write it as needed.

## Tests

Once we get all of this implemented, we need to comb through the tests to ensure that they are still valid and cover all of the new functionality. We should also add integration tests for any new commands and features we implement.

On the plus side, we can actually use the `nuke-it-from-orbit` command for test tear down.

## Documentation

We're in a good place but we'll need to go through the README to make it reflect the new commands and features.

## API Notes

While we need more research on the APIs for `gix` and `git2`, I've found quite a few useful resources:

- Data checks:
  - The `gix` [`repository`](https://docs.rs/gix/latest/gix/struct.Repository.html) struct has a lot of useful functionality for introspecting the repository and submodules, including:
    - `modules()` returns a shared _live_ view of the `.gitmodules` file, so you can use it to read the current submodule configuration and validate changes.
    - `submodules()` returns an iterator over the submodules in the repository, each submodule object is essentially a Repository object with its own methods for introspection and manipulation.
    - `workdir()` returns the worktree path containing all checkout items (if it exists)
    - `workdir_path()` is a convenience method that normalizes relative paths with the worktree path, so you can use it to get the full path anything in the worktree. (takes asRef `Bstr` and returns `Path`)
    - `kind()` returns the kind of repository (bare, worktree, etc.), which can be useful for determining how to handle submodules. (gix::repository::Kind with variants Bare, Worktree and Submodule). The enum itself has method `is_bare()` to check for a bare repo.
    - `head_id()` returns the current HEAD ID of the submodule or repository.
    - `head_tree()` returns the current HEAD [Tree](https://docs.rs/gix/latest/gix/struct.Tree.html) of the submodule or repository, which can be useful for checking the current state of the submodule.
    - `head_name()` returns the name of the symbolic ref for HEAD; note that it may not exist yet -- it can have a name and no reference.
    - `pathspec()` not really for now, but if we ever wanted to get real fancy with using pathspecs for live filtering of submodules (i.e. we could show exactly what a sparse checkout would look like), this is the method for it.
    - `try_find_remote()` returns a `Result<Option<Remote>>` for the remote with the given name, which can be useful for checking if a submodule or repo has a remote set up. Similarly:
      - `find_fetch_remote()` mirrors `git fetch` in how to finds and retrieves a remote.
      - `find_default_remote()` to find the default remote for the repo
    - `open_modules_file()` returns a `Result<gix::File>` for the `.gitmodules` file, which can be used to read the current submodule configuration. Unlike `.modules` this view is stale.
    - `current_dir()` returns the current working directory of the repository, which can be useful for relative paths.
    - `path()` returns the path to the repository .git directory itself; we can use to construct sparse-checkout index paths
    - `main_repo()` returns the superproject repo object
