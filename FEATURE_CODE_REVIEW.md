<!--
SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>

SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
-->

# CLI Migration Mapping -- Assistant Summary of Unresolved CLI and Implementation Issues

This document maps all git CLI calls in `gitoxide_manager.rs` to their equivalent `GitOperations` trait methods.

## CLI Calls Found (17 total)

### 1. Submodule Cleanup Operations (lines 317-335)

```rust
// Current CLI calls:
Command::new("git").args(["submodule", "deinit", "-f", path])
Command::new("git").args(["rm", "--cached", "-f", path])
Command::new("git").args(["clean", "-fd", path])
```

**Maps to:** `deinit_submodule(path, force=true)` + `clean_submodule(path, force=true, remove_directories=true)`

### 2. Git Config Operations (line 400)

```rust
// Current CLI call:
Command::new("git").args(["config", "protocol.file.allow", "always"])
```

**Maps to:** `set_config_value("protocol.file.allow", "always", ConfigLevel::Local)`

### 3. Submodule Cleanup in Add (lines 413-422)

```rust
// Current CLI calls:
Command::new("git").args(["submodule", "deinit", "-f", path])
Command::new("git").args(["rm", "-f", path])
```

**Maps to:** `deinit_submodule(path, force=true)` + manual file removal

### 4. Submodule Add Operation (line 444)

```rust
// Current CLI call:
Command::new("git").args(["submodule", "add", "--force", "--branch", "main", url, path])
```

**Maps to:** `add_submodule(SubmoduleAddOptions { ... })`

### 5. Submodule Init Operation (line 458)

```rust
// Current CLI call:
Command::new("git").args(["submodule", "init", path])
```

**Maps to:** `init_submodule(path)`

### 6. Submodule Update Operation (line 471)

```rust
// Current CLI call:
Command::new("git").args(["submodule", "update", path])
```

**Maps to:** `update_submodule(path, SubmoduleUpdateOptions { ... })`

### 7. Sparse Checkout Config (line 540)

```rust
// Current CLI call:
Command::new("git").args(["config", "core.sparseCheckout", "true"])
```

**Maps to:** `enable_sparse_checkout(path)`

### 8. Sparse Checkout Apply (line 635)

```rust
// Current CLI call:
Command::new("git").args(["read-tree", "-m", "-u", "HEAD"])
```

**Maps to:** `apply_sparse_checkout(path)`

### 9. Submodule Update via Pull (line 663)

```rust
// Current CLI call:
Command::new("git").args(["pull", "origin", "HEAD"])
```

**Maps to:** `update_submodule(path, SubmoduleUpdateOptions { strategy: SerializableUpdate::Checkout, ... })`

### 10. Stash Operation (line 697)

```rust
// Current CLI call:
Command::new("git").args(["stash", "push", "--include-untracked", "-m", "Submod reset stash"])
```

**Maps to:** `stash_submodule(path, include_untracked=true)`

### 11. Reset Operation (line 717)

```rust
// Current CLI call:
Command::new("git").args(["reset", "--hard", "HEAD"])
```

**Maps to:** `reset_submodule(path, hard=true)`

### 12. Clean Operation (line 731)

```rust
// Current CLI call:
Command::new("git").args(["clean", "-fdx"])
```

**Maps to:** `clean_submodule(path, force=true, remove_directories=true)`

### 13. Init in init_submodule (line 800)

```rust
// Current CLI call:
Command::new("git").args(["submodule", "init", path_str])
```

**Maps to:** `init_submodule(path)`

### 14. Update in init_submodule (line 812)

```rust
// Current CLI call:
Command::new("git").args(["submodule", "update", path_str])
```

**Maps to:** `update_submodule(path, SubmoduleUpdateOptions { ... })`

## "For Now" Comments to Address

### 1. `src/gitoxide_manager.rs:539`

```rust
// Enable sparse checkout in git config (using CLI for now since config mutation is complex)
```

**Action:** Replace with `enable_sparse_checkout(path)` from git_ops

### 2. `src/gitoxide_manager.rs:188`

```rust
// For now, use a simple approach - check if there are any uncommitted changes
```

**Action:** Review if this aligns with project goals for comprehensive status checking

### 3. `src/gitoxide_manager.rs:207`

```rust
// For now, consider all submodules active if they exist in config
```

**Action:** Implement proper active status checking using git_ops

### 4. `src/gitoxide_manager.rs:348`

```rust
// For now, return an error to trigger fallback
```

**Action:** This is in a gix operation, acceptable as fallback trigger

## Implementation Priority

1. **High Priority** (Core submodule operations):
   - Submodule add, init, update operations
   - Cleanup and deinit operations
   - Config operations

2. **Medium Priority** (Repository operations):
   - Reset, stash, clean operations
   - Update via pull operations

3. **Low Priority** (Sparse checkout):
   - Sparse checkout enable/apply operations
   - These are less critical for basic functionality

## Integration Strategy

1. **Phase 1**: Add GitOpsManager to GitoxideSubmoduleManager
2. **Phase 2**: Replace CLI calls one by one with git_ops equivalents
3. **Phase 3**: Test each replacement for equivalent behavior
4. **Phase 4**: Remove CLI dependencies entirely

---

## Adam's observations of Issues with gix_ops, git2_ops, and GitoxideManager

### GitoxideManager

#### We probably should rename it

As it's really our interface with git_ops/gix_ops/git2_ops, and not really a manager of gitoxide itself.

#### Not really using git_ops (and its submodules) fully (or at all???)

Generally, it shouldn't be directly implementing any operations. At most it pipelines operations from the ops modules, and maybe does some basic validation.

1. `check_all_submodules`:
   - Doesn't even use the ops modules

2. `apply_sparse_checkout_cli`:
   - Why are we doing CLI commands here, or at all? The whole point of the git_ops modules is to provide a Rust interface to git operations and eliminate direct shell calls.

- I could go on, but actually, I also noticed that the ops modules AREN'T EVEN IMPORTED in the `gitoxide_manager.rs` file. Clearly, this is wrong.


### gix_ops

1. convert_gix_submodule_to_entry - line 39-65

    - Problem: There's a TODO on line 47 that says "gix doesn't expose submodule config directly yet"
      - That's not true. `gix_submodule/lib.rs` exposes `File::from_bytes()` which can be used to read .gitmodules files.
      - Info can also be processed from the File to a `gix::Config` with `File::into_config()`, returning the parsed .gitmodules
      - Our `Serialized{Branch,Ignore,Update,FetchRecurse}` (options.rs) types can convert from the types in a gix submodule config already.

2. `convert_gix_status_to_flags` - line 67 - 76

    - Doesn't actually implement status, the comment is lazy -- "this is a simplified mapping as gix status structure may differ"
    - Put another way: I made this up because I didn't want to write something that would work.

3. `write_gitmodules` - line 96-100

    - Problem: The `write_gitmodules` function is not implemented, and the comment says gix doesn't have the capability. I'm ~90% sure it can. The `File` object returned by `gix_submodule` can *read and write*

4. `read_git_config`, `write_git_config`, `set_config_value` - lines 102-119

    - Problem: Also not implemented and also says gix doesn't have the capability. I'll just include the main comment from `gix_config/lib.rs`:

      > This crate is a high performance `git-config` file reader and writer. It
      > exposes a high level API to parse, read, and write [`git-config` files].
      >
      > This crate has a few primary offerings and various accessory functions. The
      > table below gives a brief explanation of all offerings, loosely in order
      > from the highest to lowest abstraction.
      >
      > | Offering      | Description                    | Zero-copy?        |
      > | ------------- | ------------------------------ | ----------------- |
      > | [`File`]      | Accelerated wrapper for reading and writing values. | On some reads[^1] |
      > | [`parse::State`] | Syntactic events for `git-config` files. | Yes |
      > | value wrappers | Wrappers for `git-config` value types. | Yes |

5. `add_submodule` and `update_submodule` and `delete_submodule` - lines 121-143

    - Problem: These functions are not implemented, and the comments say gix doesn't have the capability. Again, I think it does.
    - If you can get a `File` from `.gitmodules`, then you should be able to do all three of these things.

6. `list_submodules` - lines 155-166

    - While implemented, it only returns the submodule paths. It should return the full entry, including the URL and branch and any optional settings (ignore, fetchRecurseSubmodules, update, shallow, active).

### git2_ops

1. General observation:
   - Nearly all of the methods use string lookups like `config.get("submodule.<name>.url")` or `config.get("submodule.<name>.branch")`.
   - That *might* be okay, but it does also have an *entries()* iterator that could be used to construct a `SubmoduleEntry` directly.

2. `write_gitmodules`:
   - A comment says that there's not direct .gitmodules writing, but I think that's because `git2` treats it as a config file.
   - Since `.gitmodules` is just a config file (with a subset of allowed values), it should be possible to write to it using the same methods as for other config files.

3. `{add,update}_submodule`:
   - It seems to try to use the converted `git2` `Update` and `Ignore` equivalents directly, but in these operations they should be strings. Those are more useful for reading the config, not writing it.

4. `{add,update,delete,deinit}_submodule`:
   - We seem to only be handling a subset of settings/options. We're missing `shallow`, and `active`.

5. `list_submodules`:
   - Like with gix_ops, only returns the paths; it should return the full entry.

6. `{set,get}_sparse_patterns`:
    - Set and get both use direct file ops, which we *really* want to avoid.
    - Here again, I think git2 has the capability. I haven't had a chance to really dig into it, but I think it would handle sparse indexes just like it does for git/index files. Heck, because of that, gix very likely has the capability too.
    - Worst case we should use `gix_command`
7. `apply_sparse_checkout`:
   - Not implemented. I'm not sure... this may just be a language difference -- what are we calling 'apply_sparse_checkout'? Isn't that just the same as setting the patterns/setting? ... and then maybe running a checkout or re-init?
   - If that's the case, we should be able to implement it using the existing `set_sparse_patterns` and `git2` checkout methods.
