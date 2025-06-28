<!--
SPDX-FileCopyrightText: 2025 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>

SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
-->

# CLI Migration Mapping

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

