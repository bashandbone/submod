# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build & Run
```bash
cargo build                    # debug build
cargo build --release          # release build
mise run build                 # alias: mise run b
```

### Test
```bash
cargo nextest run --all-features --no-fail-fast -j 1   # run all tests (preferred)
cargo test --test integration_tests                     # integration tests only
cargo test --test command_contract_tests                # command contract tests only
./scripts/run-tests.sh --verbose                        # comprehensive test runner with reporting
./scripts/run-tests.sh --filter sparse_checkout         # filter to specific tests
mise run test                                           # alias: mise run t (runs via hk)
```

Tests must run single-threaded (`RUST_TEST_THREADS=1` is set in `mise.toml`). The test suite is integration-test-focused; unit tests are minimal by design.

### Lint & Format
```bash
cargo fmt                      # format code
cargo clippy --all-features    # lint
hk run check                   # run all linters (fmt, clippy, deny, typos, pkl)
hk fix                         # auto-fix where possible
mise run lint                  # alias for hk run check
```

### Full CI
```bash
mise run ci                    # build + lint + test
hk run ci                      # same via hk (uses --fail-fast for tests)
```

### Git Hooks
Managed by `hk` (configured in `hk.pkl`). Pre-commit runs: cargo fmt, clippy, check, nextest, typos, cargo-deny, pkl eval. Hooks install automatically with `mise install`.

## Architecture

`submod` is a CLI tool for managing git submodules with TOML configuration and sparse checkout support. It exposes the library as `src/lib.rs` primarily for integration testing (the public API is not stable).

### Layer Stack

```
CLI (commands.rs + main.rs)
    ↓ clap parsing
GitManager (git_manager.rs)          ← high-level submodule operations
    ↓ delegates to
GitOpsManager (git_ops/mod.rs)       ← unified backend with automatic fallback
    ├── GixOperations (git_ops/gix_ops.rs)     ← gitoxide (preferred)
    ├── Git2Operations (git_ops/git2_ops.rs)   ← libgit2 (fallback)
    └── Git CLI                               ← last resort (spawned via std::process)
Config (config.rs)                   ← figment-based TOML config loading/saving
```

### Fallback Architecture

The core design is a **gix-first, git2-fallback, CLI-last-resort** strategy, driven by the immaturity of gitoxide's submodule support. `GitOpsManager` wraps both backends behind the `GitOperations` trait and calls `try_with_fallback()` / `try_with_fallback_mut()` for every operation. When gix fails, it logs a warning and transparently retries with git2; `add_submodule` has an additional CLI fallback that also cleans up any partial state from the prior attempt.

After destructive operations (delete, nuke), `GitOpsManager::reopen()` must be called to refresh the in-memory repository state. git2 reopen errors are fatal; gix reopen errors are warnings only.

### Configuration

`Config` uses [figment](https://docs.rs/figment) to load `submod.toml`. The schema has:
- `[defaults]` → `SubmoduleDefaults` (global git options applied to all submodules)
- `[<name>]` sections → `SubmoduleEntry` (per-submodule config, overrides defaults)

`SubmoduleEntry` merges `SubmoduleGitOptions` (ignore, fetch_recurse, branch, update) with submodule-specific fields (path, url, sparse_paths, active, shallow). Options are serialized to/from git-config-compatible strings via the `options.rs` newtype wrappers (`SerializableIgnore`, `SerializableUpdate`, etc.).

### Key Conventions

- **Unsafe code is forbidden** (`unsafe_code = "forbid"` in `Cargo.toml`)
- Clippy is configured at `pedantic` + `nursery` warn level; `correctness` is deny
- `missing_docs` is warn — public items need doc comments
- `module_name_repetitions` and `too_many_lines` are allowed
- All error handling uses `anyhow` for propagation and `thiserror` for defining error types
- `simple_gix.rs` contains lightweight gix helpers used in `gix_ops.rs`

### Testing Approach

Integration tests in `tests/` use a `TestHarness` (in `tests/common/mod.rs`) that creates temporary git repos and invokes the compiled binary. Test files:
- `integration_tests.rs` — end-to-end CLI behavior
- `command_contract_tests.rs` — CLI command argument contracts
- `config_tests.rs` — configuration parsing/serialization
- `sparse_checkout_tests.rs` — sparse checkout behavior
- `error_handling_tests.rs` — error conditions and messages
- `performance_tests.rs` — benchmarks (uses criterion)
