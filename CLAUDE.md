<!--
SPDX-FileCopyrightText: 2026 Adam Poulemanos and contributors

SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
-->
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
cargo nextest run --all-features --no-fail-fast         # run all tests (preferred)
cargo test --test integration_tests                     # integration tests only
cargo test --test command_contract_tests                # command contract tests only
./scripts/run-tests.sh --verbose                        # comprehensive test runner with reporting
./scripts/run-tests.sh --filter sparse_checkout         # filter to specific tests
mise run test                                           # alias: mise run t (runs via hk)
```

Integration tests that modify git repos are serialized via nextest test groups (`.config/nextest.toml`); other tests run in parallel. Each test gets an isolated git config via `GIT_CONFIG_GLOBAL` so tests never race on `~/.gitconfig`. The test suite is integration-test-focused; unit tests are minimal by design.

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
CLI (commands.rs + main.rs, imports the library — no duplicated modules)
    ↓ clap parsing
GitManager (git_manager.rs)          ← desired-state planning and reconciliation
    ↓ delegates to
GitOpsManager (git_ops/mod.rs)       ← native Git mutation boundary
    ├── Native Git CLI (spawned via std::process) ← all mutations
    ├── GixOperations (git_ops/gix_ops.rs)        ← gitoxide reads
    └── Git2Operations (git_ops/git2_ops.rs)      ← libgit2 reads
Config (config.rs)                   ← raw TOML declarations plus an effective-entry resolver
```

### Git operation design

All lifecycle mutations (add/init/update/move/deinit/delete/reset/stash/clean/sparse) run through **one native Git path** inside `GitOpsManager` (`std::process::Command` with argument arrays, rooted cwd, checked statuses). There are no competing backend mutation implementations and no cross-backend retry after a mutation failure; real errors are terminal with recoverable partial state left in place. Inspection reads use `try_with_fallback()` (gix first, git2 fallback); `GitOpsManager::without_gix` is the injection seam for exercising the git2 read path. `Config::add_submodule` and `remove_submodule` mutate in place (no whole-map clones); sparse patterns are borrowed (`&[String]`) through the read APIs.

`GitOpsManager::reopen()` refreshes the in-memory repository handles after external changes; it is used by tests and explicit refresh flows, not by every lifecycle call.

### Configuration

`Config` uses [figment](https://docs.rs/figment) to load `submod.toml`. The schema has:
- `[defaults]` → `SubmoduleDefaults` (global git options applied to all submodules)
- `[<name>]` sections → `SubmoduleEntry` (per-submodule config, overrides defaults)

`SubmoduleEntry` merges `SubmoduleGitOptions` (ignore, fetch_recurse, branch, update) with submodule-specific fields (path, url, sparse_paths, active, shallow). Options are serialized to/from git-config-compatible strings via the `options.rs` newtype wrappers (`SerializableIgnore`, `SerializableUpdate`, etc.).

### Key Conventions

- **Unsafe code is denied by default** (`unsafe_code = "deny"` in `Cargo.toml`)
- Clippy is configured at `pedantic` + `nursery` warn level; `correctness` is deny
- `missing_docs` is warn — public items need doc comments
- `module_name_repetitions` and `too_many_lines` are allowed
- All error handling uses `anyhow` for propagation and `thiserror` for defining error types
- Raw TOML declarations stay separate from effective settings: never persist inherited defaults into entries; `active`/`sparse` app-only state stays out of portable `.gitmodules` fields

### Testing Approach

Integration tests in `tests/` use a `TestHarness` (in `tests/common/mod.rs`) that creates temporary git repos and invokes the compiled binary. Beyond running the CLI, the harness inspects real git state — `index_gitlink_mode`, `gitmodules_entries`, `submodule_config_entries`, `git_modules_dir_exists` — and tests should assert on that rather than on printed output where both are available. Test files:
- `integration_tests.rs` — end-to-end CLI behavior
- `command_contract_tests.rs` — CLI command argument contracts
- `config_tests.rs` — configuration parsing/serialization
- `sparse_checkout_tests.rs` — sparse checkout behavior
- `error_handling_tests.rs` — error conditions and messages
- `fallback_tests.rs` — the retained gix→git2 read fallback and native CLI mutation state, via the `GitOpsManager::without_gix` injection seam
- `git_ops_tests.rs` — `GitOpsManager` (native mutations) and the retained backend read APIs directly
- `security_tests.rs` — path traversal, symlink escape, and command/flag injection containment
- `performance_tests.rs` — timing ceilings and Rust-allocator growth bounds (plain `#[test]`s, not criterion; the allocator measures test-process Rust allocations only, not process RSS)
- `reconciliation_*_tests.rs`, `phase5_*_tests.rs`, `phase6_*_tests.rs` — the R01–R31 regression families from `docs/IMPROVEMENT_PLAN.md`

Criterion benchmarks live separately in `benches/benchmark.rs` (`cargo bench`): real config parse/load/edit workloads at 1/10/100 modules, plus a `SUBMOD_MEASURE_CONFIG` mode driven by `scripts/measure-performance.py` for before/after comparisons with wall time, Git invocation counts, and no-op state identity.
