<!--
SPDX-FileCopyrightText: 2026 Adam Poulemanos and contributors
SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
-->

# Submod reliability audit and implementation plan

Audited 2026-09-10 at commit `31c5e373bfd29162f86675fa60ed4af0adf8df97`, version 0.4.0. This document is an implementation handoff, not a claim that the fixes have been made. Application source was unchanged during the audit. Pre-existing `.serena/` work was left alone.

## Assessment

The reported problems are real and arise from shared lifecycle and configuration defects. There are several implementations of what a submodule is, when it is initialized, and which configuration wins. Commands update different pieces of state, sometimes return success after incomplete work, and sometimes treat failure as permission to delete existing files. Repairing individual command symptoms will leave sibling paths broken.

The highest priority is data preservation. In disposable local repositories, a failed add erased a pre-existing file; disable removed the submodule object database containing local history and a stash; deleting `lib` removed unrelated prefix siblings from the index. The next priority is reliable reconciliation between declared configuration and observed Git state. Performance improvements should primarily remove repeated work and duplicate implementations after those contracts are established.

Keep the Rust CLI, current commands, flat TOML configuration, sparse pattern support, and integration-test approach. Use native Git for repository lifecycle mutations, retain useful native read operations only where they remain simpler, and replace the handwritten TOML editor with a real document editor. Do not complete three different mutation backends, build a plugin system, add a daemon/database, or introduce a general transaction engine.

### Evidence and limits

Four independent code audits covered configuration/lifecycle, Git safety, option/schema consistency, and tests/CI/performance. Runtime reproductions used only disposable local repositories. The environment was macOS aarch64, Rust 1.98.1, and Apple Git 2.50.1. Linux/Windows behavior and live private-remote authentication were not exercised.

| Verification | Observed result |
| --- | --- |
| `cargo test --locked --offline --all-features -- --test-threads=1 --skip test_invalid_git_url --skip test_network_timeout_simulation` | Passed: 567 executions, zero failures, two network tests excluded. Includes all ten performance tests. |
| Independent test count | 168 unit tests run in both library and binary, so 567 executions represent 399 distinct tests passing; two further tests were excluded. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo clippy --locked --offline --all-targets --all-features` | Passed with six distinct warnings; this was not a warning-free run. |
| `cargo +1.89 check --locked --offline --all-features` | Passed; the declared MSRV builds locally. |
| Existing hk CI command: `cargo nextest run --offline --all-features --fail-fast -p ci` | Failed with exit 101: `ci` is interpreted as a package. |
| `cargo audit --db /private/tmp/submod-audit/advisory-db --json` with freshly fetched database | Exit 0, 274 dependencies, zero non-ignored vulnerabilities; `RUSTSEC-2024-0364` is ignored and `bisync 0.3.0` is reported yanked. Database commit `b50980aad8b8f14f77e25a97b32dd94bf008b0af`, updated 2026-09-09. |

Build/test artifacts were isolated using `CARGO_TARGET_DIR=/private/tmp/submod-audit-target`; MSRV used a separate temporary target. The dependency audit result does not establish application safety or excuse the ignored advisory. The current RustSec entry describes terminal-output manipulation in `gitoxide-core` and lists no patched version. Review the actual remaining use of that dependency after mutation consolidation. [RustSec advisory](https://rustsec.org/advisories/RUSTSEC-2024-0364.html)

Machine-readable observations are preserved in [audit-results.json](audit-results.json). Original scripts/logs for this session are under `/private/tmp/submod-audit/`: `repro_config.py`, `repro_preservation.py`, `state_lifecycle.py`, `cargo-test.log`, `cargo-clippy.log`, `cargo-msrv.log`, and `cargo-audit-live.json`. These temporary files are supporting evidence; implementation must not depend on their continued existence. The regression matrix below specifies the fixtures to retain in the repository's test suite.

### Prioritized findings

Severity is repair priority for this application, not a CVSS score. **P0** means prevent data loss or writes outside the intended module before further feature work; **P1** means core lifecycle/configuration correctness; **P2** means usability, assurance, or avoidable cost. “Reproduced” means observed with the compiled CLI; “source” means traced without a dedicated runtime experiment.

| ID | Priority / evidence | Finding, impact, and source |
| --- | --- | --- |
| F01 | P0 / source | Mutation validation is incomplete and inconsistently called. Empty/root-equivalent paths and `.git` pass the existing validator; config-driven init/delete bypass it; unchecked names enter module-directory joins. `src/utilities.rs:230-292`, `src/git_manager.rs:413,845,1519-1522,1548-1557`, `src/git_ops/mod.rs:451-464`. |
| F02 | P0 / reproduced | Add performs replacement cleanup before establishing whether it can succeed, and backend fallback removes the destination after an error. Adding a nonexistent local remote over an ordinary directory deleted its sentinel file, then exited 1. `src/git_manager.rs:443,505-514`, `src/git_ops/mod.rs:364-508`. |
| F03 | P0 / reproduced | Disable can erase repository history. With a clean checkout containing a local-only commit and stash, `disable` exited 0 and removed the object database. A remaining worktree directory is not preservation of the repository. `src/git_manager.rs:1466-1493`, `src/git_ops/gix_ops.rs:609-684`. |
| F04 | P0 / reproduced | Gix deletion removes index entries by string prefix. Deleting `lib` also removed `library.txt` and `lib-extra/file` from the index; their files survived, so this is unintended staged deletion. Index writes also bypass normal lock handling. `src/git_ops/gix_ops.rs:537-552`. |
| F05 | P0 / source | Reset warns on every stash error and continues into hard reset and clean. A failed preservation step is treated as optional. `src/git_manager.rs:733-752`, `src/git_ops/git2_ops.rs:658-662`. |
| F06 | P1 / reproduced | `sync` never calls the existing configuration synchronization API. Changing a URL then running sync left both `.gitmodules` and child `origin` unchanged while announcing success. Existing init short-circuits on `.git` presence. `src/main.rs:191-205`, `src/config.rs:1054-1077`, `src/git_manager.rs:805-814,1659-1724`. |
| F07 | P1 / reproduced | A fresh clone with `.gitmodules` and an old pinned gitlink reported successful init but produced no checkout. Native Git initialized the same fixture correctly. Gix drops a prepared checkout instead of completing it. `src/git_ops/gix_ops.rs:378-393,415-462`. This differs from TOML-only creation, which succeeded in the audit. |
| F08 | P1 / reproduced + source | Repository roots and module identity are inconsistent. Invoking even list/check from a nested directory failed repository discovery; omitted `path` failed despite the documented name-based default. Init searches `.gitmodules` by substring, allowing prefix/comment false matches and formatting false misses. Nicknames, Git section names, and paths are interchanged. `src/git_manager.rs:243-250,766-827`, `src/git_ops/git2_ops.rs:22-33,408-421`. |
| F09 | P1 / reproduced | Durable disabled state is ignored. `active=false` still cloned during init. Conversely, `add --no-init` persists `active=false`, conflating a one-invocation option with persistent disabling. `src/main.rs:106-135,173-205`, `src/git_manager.rs:418-435,688-705,761-800`. |
| F10 | P1 / reproduced | Loading materializes inherited defaults into stored entries. Changing global ignore from dirty to all wrote `ignore="dirty"` into the formerly inheriting module. `src/config.rs:1000-1017,1125`, `src/git_manager.rs:1053-1069,1439-1451`. |
| F11 | P1 / reproduced | The line-based TOML editor corrupts valid multiline arrays and duplicates tables with trailing comments. Commands return success and the next load fails. Sparse mode is omitted from serialization; unrelated change clears `shallow=true` because the CLI always passes false. Writes truncate the live file. `src/git_manager.rs:1030-1370`, `src/main.rs:238`, `src/commands.rs:208-215`. |
| F12 | P1 / reproduced + source | Branch/update behavior differs by backend. Add with feature branch checked out main and omitted branch from `.gitmodules`. Manager update always uses default options. Gix updates fetch without the required checkout transition; git2 normally follows the parent gitlink. Remaining at a parent pin is valid under the chosen contract below, but contradicts the README's unconditional “latest” promise. `src/git_ops/git2_ops.rs:344-403`, `src/git_ops/gix_ops.rs:398-479`, `src/git_manager.rs:701-705,854-857`. |
| F13 | P1 / reproduced + source | Removing sparse paths does not disable sparse checkout or restore excluded files. Status uses a subset comparison, accepts extra patterns and ignores order, so it cannot establish equivalent sparse behavior. `src/git_manager.rs:328-378,805-814`; low-level application also uses a cwd-relative path at `src/git_ops/mod.rs:600-609`. |
| F14 | P1 / reproduced + source | Generated template cannot load: `schema_version` is interpreted as a submodule. Version 1.1 schema has invalid JSON, schema symlinks are broken, schema `fetch` differs from runtime `fetchRecurse`, and current fields are missing. Unknown runtime settings can be silently ignored. `sample_config/submod.toml:4-5`, `src/config.rs:961-968`, `schemas/v1.1.0/submod_config_v1.1.0.json:19-21,67,76`. |
| F15 | P1 / source | Wiring the existing sync function directly would be unsafe/incomplete: gix replaces all `.gitmodules`, git2 only upserts present values, and only local branch receives explicit synchronization. Removed fields and unmanaged Git modules have no consistent contract. Every backend error triggers fallback, including errors after partial mutation. `src/config.rs:1054-1077`, `src/git_ops/gix_ops.rs:140-145`, `src/git_ops/git2_ops.rs:213-294`, `src/git_ops/mod.rs:288-326`. |
| F16 | P2 / reproduced + source | `check` reports missing checkout with exit 0; an explicitly misspelled config path behaves like an empty config; recursive list is only a top-level supplement. README `generate-config --from-setup .` and `completeme nu` both fail. Normal sync emitted terminal control output even when captured. `src/git_manager.rs:885-1000,1369-1420`, `src/commands.rs:344-351`, `src/shells.rs:43-56`. |
| F17 | P1 assurance / source + reproduced | Green tests include heading-only “inheritance”/roundtrip checks, a fake disk-exhaustion test, a sequential “concurrency” test, and helpers that ignore Git exit status. The hk CI entry point fails; test-only edits do not trigger its test hook. `tests/config_tests.rs:18-100,298-318`, `tests/error_handling_tests.rs:325-405,587-622`, `tests/common/mod.rs:352-359`, `hk.pkl:37,67`. |
| F18 | P2 / source | Performance benchmarks copy production algorithms; allocation ceilings omit native and child-process memory; test code changes global cwd. Main compiles library modules again. Linux-only functional CI does not establish macOS/Windows release correctness; tag builds skip the manual linkage checks and have no direct test prerequisite. `benches/benchmark.rs:24-58`, `tests/performance_tests.rs:18-55,430-474`, `src/main.rs:25-32`, `.github/workflows/ci.yml:19`, `.github/workflows/release.yml:99,131-169`. |

## Required behavior: settle these contracts before editing

The following are recommended product decisions for this implementation. They intentionally remove ambiguity so the implementing agent can proceed. They are not claims about current behavior. Record their compatibility impact in the changelog.

### Configuration and state ownership

1. `submod.toml` declares desired settings for its managed modules. `.gitmodules` remains compatible with ordinary Git and is an import source, not a second source that silently overwrites declared TOML. Import is explicit through `generate-config --from-setup`.
2. An entry absent from TOML but present in Git is **unmanaged**, not scheduled for deletion. Preserve and report it. Only explicit delete/nuke removes a module. Do not add implicit pruning.
3. Retain raw declarations, including whether a field was absent. Compute effective settings separately: explicit command override, then per-module declaration, then supported global default, then documented built-in/Git behavior. Never save inherited values as declarations.
4. Discover the repository from invocation cwd once. Use its worktree root for all module paths, its discovered Git directories for metadata, and Git's actual gitdir/common-dir resolution for linked worktrees. Do not infer these from `<cwd>/.git`.
5. The default config is `<repo-root>/submod.toml`. An explicit relative `--config` is relative to invocation cwd; module paths inside any config are still relative to repo root. A missing explicit config is an error. Without a default file, `add` may create one; configuration-dependent commands explain how to import/generate it. Help, completions, and template generation work outside repositories.
6. Omitted module path means its validated name relative to root. URL is required for managed entries. Reject duplicate/overlapping managed paths before mutation. Normalize harmless `./` spelling; reject `..` components rather than interpreting them during writes.
7. Keep TOML nickname, Git logical section name, and checkout path distinct. Match existing Git registrations by exact normalized path and preserve their section names. New registrations use the TOML nickname as Git logical name. A nickname rename at an unchanged path must not create another module. Never assume old path-named registrations must be renamed. Ambiguous matches are errors. Manual path edits with no unambiguous old identity are reported for explicit `change --path`, not guessed as permission to delete old state.
8. `active=false` means no clone/fetch/checkout during init/update/sync. Disable changes activation only and preserves checkout, object database, refs, and stashes. `--no-init` skips materialization for this add only; it does not disable future init.

### Managed fields

| Field | TOML behavior | Git/application effect |
| --- | --- | --- |
| `path`, `url` | Required effective path; required declared URL | Exact `.gitmodules` entry; synchronize managed URL into parent local config and initialized child remote using Git's relative-URL semantics. An explicit sync of a managed URL replaces a divergent local URL; say so in preview. |
| `branch` | Optional globally and per entry; absence retains Git remote-default semantics | `.gitmodules` tracking branch and any corresponding managed local override. Named branch is selected for initial add; existing pinned checkout moves to tracking tip only with `update --remote`. Preserve `.` semantics and fail clearly when resolution needs a branch but the parent is detached. |
| `ignore` | Canonical all/dirty/untracked/none | `.gitmodules` plus appropriate local value; safety checks must still detect local work even when display/status ignore settings hide it. |
| `update` | checkout/merge/rebase/none | `.gitmodules` and local policy; effective strategy actually controls update. `none` skips update and fetching. Reject `!command` from TOML/CLI; do not promote executable configuration from a repository file into trusted local config. |
| `fetchRecurse` | always/on-demand/never | Git key is `fetchRecurseSubmodules`, encoded true/on-demand/false. This is fetch behavior, not an implicit instruction to initialize all nested checkouts. |
| `active` | Default true | Managed local activation and app selection; keep it in TOML, not as a portable `.gitmodules` feature. |
| `shallow` | Default false; cloning policy | Advertise supported Git shallow recommendation and pass depth when creating a clone. It is not a promise to retroactively truncate/unshallow an existing repository. Report an existing-history mismatch; document that limitation explicitly. |
| `sparse_paths` | Ordered patterns; absent/empty means full checkout | Per-child sparse configuration and real checked-out contents. Removing patterns disables sparse checkout safely. |
| `use_git_default_sparse_checkout` | Per-entry > global > false | Preserve the existing opt-in/default pattern semantics, including explicit false; do not write this app-only setting to `.gitmodules`. |
| `schema_version` | Optional; accept absent legacy form and supported 1.0.0/1.1.0 metadata | Explicit metadata, never interpreted as a module. Unsupported future version fails before writes. |

Clear obsolete managed Git keys when their declaration/default disappears; otherwise an old local override continues winning. Preserve unrelated keys and sections. Validate aliases explicitly: accept historical `fetch` and `fetch_recurse` with a migration warning, write canonical `fetchRecurse`, and reject conflicting duplicate spellings. Unknown keys must produce a contextual error rather than silently changing behavior. Preserve the original document on validation failure.

Compatibility also includes values produced by older submod commands: treat serialized `branch="HEAD"` as the legacy remote-default sentinel, and legacy fetch strings `"true"`/`"false"` as always/never, with contextual migration warnings. Do not reject every older generated file when introducing stricter validation. Do not rewrite these files during read-only commands; canonicalize the touched fields on an explicit edit/import.

Native Git distinguishes the parent-recorded commit from tracking a remote tip, and its URL sync is narrower than full configuration reconciliation. The implementation must handle both distinctions explicitly. [Git submodule documentation](https://git-scm.com/docs/git-submodule)

### Command contract

| Command | Required behavior |
| --- | --- |
| `check` | Read-only, no fetch. Report missing, uninitialized, disabled, unmanaged, dirty, conflicted, configuration drift, gitlink difference, and sparse drift distinctly. Exit 0 if managed active modules match their strategy-specific required state, 1 for drift/operation failure, 2 for argument/config validation errors. Unmanaged/disabled modules alone do not fail. Dirty managed checkouts are reported and yield 1. |
| `list [--recursive]` | Read-only deterministic listing. Recursive means actual descendants, including registered but uninitialized entries where discoverable; distinguish “not inspected” from empty. Exit nonzero on inspection failure. |
| `add` | Validate then register/materialize a new module and save its explicit settings. Refuse conflicting existing content. Never silently replace a module. Report that Git registration stages `.gitmodules`/gitlink; never commit. |
| `add --no-init` | Save a valid enabled declaration only; later init/sync can materialize it. |
| `init` | Reconcile managed settings, create missing registration/checkout as appropriate, honor existing gitlinks, and apply sparse policy. Already-correct state is a no-op. |
| `sync` | Inspect/preflight, reconcile settings, initialize eligible active missing modules, apply the selected strategy against parent-recorded commits, reconcile sparse checkout, then verify. No preliminary status error may prevent expected missing-state repair. No automatic advancement to remote tips. |
| `update [--remote]` | Default updates to parent gitlink using the effective strategy; `--remote` explicitly advances to the configured remote branch/default and leaves its changed gitlink for user review. Initialize missing active checkouts safely. Report skipped `update=none`. Do not silently stage remote advancement. |
| `change` / `change-global` | Patch only supplied fields, then run metadata reconciliation for existing affected modules and safe sparse reconciliation only when sparse policy changed. These commands do not fetch, move HEAD, or materialize a missing checkout. Missing/config-only modules remain declared until init. Unrelated omitted flags do not reset values. `--unset FIELD` removes an override; `--clear-sparse-paths` requests a full checkout. |
| `change --path` | Validate the destination and use a safe Git-aware move for an initialized, unambiguous module; do not delete/reclone. Decline dirty/conflicted or unsupported layout moves without mutation. For never-materialized declarations, change only desired path. |
| `disable` / `change --active false` | Persist inactive state/local activation; preserve files and history. `change --active true` makes the module eligible for later init/sync, without an implicit destructive reset. |
| `reset` | Preserve tracked/untracked changes first, identify the resulting stash, abort on preservation error, reset to the parent gitlink, and verify. No `clean -x`; ignored data is preserved. |
| `delete` | Explicit removal of selected registration/checkout/config; refuse local changes without explicit force. Retain Git's stored module repository so local history remains recoverable. Config-only deletion never deletes an unrelated occupied directory. |
| `nuke-it-from-orbit` | Explicit repair/remove workflow with selected targets, preview, and protection for local data. Default rebuild retains declarations until successful reinit. `--kill` removes declarations after successful removal. Neither mode silently purges stored local history; do not add an implicit `.git/modules` garbage collector. |
| `generate-config` | `--from-setup` is a boolean flag; reads registration even without checkout. `--template` conflicts with it. Existing output needs `--force`. Output is atomically written and must load through the ordinary parser. |

Provide `--dry-run` for mutating commands using the same computed actions as execution. It must not write files, lock metadata, stage the index, or access remotes. Make `--all` conflict with explicit names for reset/nuke, reject duplicate targets, and validate all selected targets before changing any. Keep existing aliases and add documented `nu`; do not add more aliases or a TUI.

Verification is strategy-specific. Checkout requires child HEAD equal to the target gitlink. Merge/rebase require the native operation to succeed without conflicts and the target commit to be an ancestor of resulting HEAD; preserved local commits can make HEAD differ legitimately. If that relationship already holds, a repeated sync skips the operation. Check reports that expected difference as informational. `update=none` skips automatic fetch/materialization/checkout, including missing checkout, and reports it as intentionally skipped; metadata still reconciles where possible. Explicit `add` is a request to create the initial checkout even if later update policy is none. For ordinary init/update/sync and sparse reconciliation, dirty work that would be touched by a planned action causes refusal; untouched dirty work is preserved and reported as unresolved, so sync returns 1 rather than claiming every check is clean. Reset/delete/nuke follow their explicit preservation/force contracts instead. After `update --remote`, check can correctly report a checkout-policy difference until the user records the new parent gitlink.

Deletion/deinitialization must preserve repository history according to the chosen command contract; Git distinguishes submodule worktrees from their stored repositories. Linked worktrees can also use a `.git` file and separate administrative directories. [Git repository layout](https://git-scm.com/docs/gitrepository-layout)

## Implementation sequence

Follow these phases in order. Each phase has a completion gate. Keep changes reviewable, but finish the whole plan rather than stopping after the first passing subset. Tests should be added beside the affected existing integration tests, not as a second test framework.

### Phase 1 — Make the regression harness trustworthy

**Own:** `tests/common/mod.rs`, relevant existing test files, `hk.pkl`, `.config/nextest.toml`, `scripts/run-tests.sh`.

1. Fix Git helpers to return/check exit status, stderr, and output. A missing config key is a specifically handled Git exit status; arbitrary Git failure must not become “absent.” Capture index mode/OID, worktree HEAD/content, parent/child config, and retained module gitdir through successful Git queries.
2. Use local remotes with at least two commits and two distinct branches. Clone the parent without submodules to test the onboarding state; do not use only `submod add` to build every fixture.
3. Keep Git identity/signing/transport settings isolated to each fixture or child process. Allow local file transport only in tests. Avoid process-global cwd/environment changes; use child `current_dir` and explicit repository paths.
4. Add failing reproductions for R01–R16 below before fixes. Snapshot unrelated files, refs, TOML, `.gitmodules`, and index state where preservation matters. A test that expects rejection must also prove nothing unintended changed.
5. Fix hk `-p ci` to `--profile ci`; include tests, Cargo files, schemas, and test tooling in relevant hook inputs. Distinguish nextest configuration profile from Cargo build profile. For performance execution, pass the actual intended Cargo profile to nextest.
6. Replace fake failure tests with deterministic failure conditions: missing local remote, held Git lock, unwritable fixture where supported, or a narrowly scoped test seam at the actual file-write/stash boundary. Delete tests whose name promises behavior they never exercise.

**Gate:** Original baseline remains reproducible, new tests fail for the intended state discrepancies, and the advertised hk/nextest command runs the real suite. No fixture operation reaches the user's repository or global Git configuration.

### Phase 2 — Centralize repository context and stop destructive fallback

**Own:** `src/utilities.rs`, `src/git_ops/mod.rs`, `src/git_ops/{gix_ops,git2_ops}.rs`, shared `src/git_manager.rs` call sites.

1. Discover once from cwd; store invocation directory, worktree root, gitdir/common-dir, and resolved config path in a small repository context. Reuse an existing repository object where possible. Bare repositories produce a specific unsupported-worktree error. Pass rooted paths everywhere; remove literal `Path::new(".")` and manual `.git/modules` assumptions from lifecycle code.
2. At the common mutation boundary validate every effective path and any administrative identity before touching disk. Require a strict descendant, forbid root-equivalent/empty paths, `..`, absolute paths, Git administrative locations and their aliases. Check existing ancestors without requiring a nonexistent destination to canonicalize. Reject symlink components in mutation destinations rather than trying to safely mutate through them. Reject overlapping managed paths and relevant case-insensitive collisions. Retain `PathBuf`/`OsStr` through filesystem and process operations.
3. Inspect occupied destinations. Treat an unrelated directory, file, symlink, or independent repository as a conflict. An existing registered submodule can be adopted only by the reconciliation rules; add does not replace it. Do not erase partial-looking directories simply because a command failed.
4. Route add/init/update/move/deinit/delete/reset/stash/clean/sparse work to **one native Git mutation path** inside the existing operations module. Use `std::process::Command` with argument arrays, rooted cwd, option terminators where supported, literal pathspec handling, checked statuses, and contextual errors. Read-only/preview Git calls use `--no-optional-locks` where applicable so status cannot silently refresh the index. Keep credential helpers/SSH agent behavior available. Do not run shell command strings assembled from config.
5. Remove unconditional cleanup in `cleanup_existing_submodule` and CLI fallback. Select the supported implementation before mutation; real permission/authentication/lock/conflict errors are terminal. If temporary cleanup is necessary, record the exact artifacts created by this invocation and clean only those; otherwise leave recoverable partial state with a useful error.
6. Remove manual index rewriting and prefix deletion. Use exact validated Git paths and ordinary Git lock handling. For mutation, lock both `<resolved-common-dir>/submod.lock` and a sibling `<config-filename>.submod.lock` for each output config. For template generation outside a repo, only the output-file lock applies. Acquire all lock paths in canonical lexical order using exclusive creation, then load/revalidate mutable state; hold them through postcondition verification and release on normal error/success. This serializes linked worktrees, different configs targeting one repo, and one config shared by different repos. Dry-run takes no locks. Stale locks produce recovery guidance, not automatic stealing. External Git commands do not honor these app locks, so retain native Git lock handling and revalidation around each mutation; do not promise a transaction across repositories.
7. Before native Git updates, reject repository-supplied custom update commands and avoid importing arbitrary Git config. Apply the allowed effective update strategy explicitly so a latent local custom command cannot become a surprise execution path. Preserve user-controlled Git protocol restrictions; do not set `protocol.file.allow=always` in production.

**Gate:** R01–R05 and R16 preservation assertions pass, plus nested cwd and worktree fixtures. A failed add over existing data cannot delete it. No lifecycle mutation retries through another backend after an ordinary failure. Read-only native backends can remain while useful; no effort goes into filling their mutation stubs.

### Phase 3 — Fix parsing, defaults, and atomic persistence

**Own:** `src/config.rs`, `src/options.rs`, config editing portions of `src/git_manager.rs`, `src/commands.rs`, `src/main.rs`, `Cargo.toml`/`Cargo.lock`.

1. Parse raw declarations without `apply_defaults()` mutating them. Add a single effective-entry resolver, used by command planning and status. Reduce `SubmoduleEntries`' duplicated sparse map to one authoritative representation, or make derived accessors read entries directly.
2. Explicitly parse/validate schema metadata, required URL, effective path, booleans, option enums, sparse pattern boundaries, and unknown fields before constructing actions. Because top-level modules are flattened, do not blindly add serde `deny_unknown_fields` where flattening makes it unsuitable; validate the parsed TOML table and preserve field/line context.
3. Unify branch parsing for serde and CLI. Keep supported historical aliases consistent, distinguish absence from `.` and a named branch, reject blank/invalid refs, and accept full `refs/heads/...` spelling for a literal branch whose name otherwise matches an alias. Do not manufacture explicit `branch="HEAD"` on an omitted flag.
4. Promote the already-transitive `toml_edit` crate to a direct dependency compatible with the project's MSRV. Use its document model to patch explicit fields; remove `section_name_from_header`, `entry_to_kv_lines`, `line_key`, `merge_section_body`, and the handwritten comment scanner once replaced. Preserve untouched comments/unknown text on rejected input; for valid edits preserve unrelated sections, comments, and ordering to the editor's documented capabilities. [toml_edit documentation](https://docs.rs/toml_edit/latest/toml_edit/)
5. Serialize every supported setting, including explicit false and sparse mode at both scopes. CLI change booleans use `Option<bool>` so omitted/true/false are distinct. Add `--unset` for supported optional overrides and `--clear-sparse-paths` with clear conflicts against replacement/append. Changing defaults never pins inheriting entries.
6. Validate the rendered document by parsing it before replacing the original. Write to a temporary sibling, preserve appropriate permissions, flush, and atomically replace; use platform-correct replacement behavior. Refuse symlink config destinations. Compare current file bytes with the locked load snapshot immediately before committing a write. This detects most external edits, but an editor ignoring the lock can still race between comparison and replacement: document that remaining limitation rather than promising race-free protection against arbitrary writers. Cooperating submod processes must serialize without lost updates. Atomic replacement protects one file; it is not a transaction across TOML, Git config and index. Write once per command and skip byte-identical output.
7. Fix generated templates and all serialization paths to use the same representation. No separate sample-only or generation-only parser behavior.

**Gate:** R06–R10 and R18 pass. Every supported edit can be loaded in a fresh CLI process. Failed writes leave the original file valid and recoverable. A no-op command leaves configuration bytes unchanged.

### Phase 4 — Implement one reconciliation flow

**Own:** `src/git_manager.rs`, `src/config.rs`, `src/git_ops/mod.rs`, thin CLI dispatch in `src/main.rs`.

Represent observed state with the smallest useful records: declared/effective entry, matched Git registration, index gitlink, checkout/gitdir presence, activation, and drift. A small concrete action enum/list is sufficient to share preview and application; do not build a generic workflow engine. Move init/update/sync loops out of `main.rs` so selection, ordering, failure policy, and summaries are shared.

| Observed state | Required action |
| --- | --- |
| TOML entry only; absent destination | For an eligible module, add registration/checkout using effective options; verify a mode-160000 gitlink before reporting successful creation. Disabled/update-none automatic materialization remains intentionally skipped. |
| `.gitmodules` + gitlink, checkout missing or empty | Initialize/update to the recorded commit using Git; do not add again or select remote HEAD. |
| `.gitmodules` present, gitlink missing | Treat as incomplete registration. If there is no conflicting content, complete through the controlled add path and report the new gitlink. If existing content/history makes intent ambiguous, fail with exact repair guidance. |
| Gitlink present, `.gitmodules` absent | Reconstruct only the uniquely matched managed registration from validated TOML, retaining the gitlink OID; then initialize. No matching declaration means report unsupported/unmanaged incomplete state. |
| Initialized registered module, correct metadata | Skip clone/init work; apply only detected configuration/checkout/sparse drift. |
| Initialized checkout with divergent URL/options | Update owned settings before any fetch; synchronize URL resolution and then apply the relevant operation. |
| Existing `.git` but no registered identity/gitlink | Do not equate `.git` presence with success. Report conflict; leave the independent repository intact. |
| Retained module gitdir, missing checkout | Let Git reattach/reinitialize it; preserve local refs/stashes. Do not remove it to make clone succeed. |
| Broken gitdir pointer, unmerged index, conflicting path, corrupt metadata | Fail before mutation with the location and a targeted recovery step. Do not swallow parse/open errors as absence. |
| Inactive or update-none declared entry | Synchronize safe declaration/activation metadata where applicable; skip automatic network/materialization/checkout work, including unreachable URLs. Explicit add with update-none still creates the initial checkout. Do not require an intentionally skipped automatic checkout to exist. |
| Git module absent from TOML | Preserve and report unmanaged. |

1. Parse `.gitmodules` through Git config APIs/commands, not substring matching. Build an exact identity map once; detect duplicate names/paths and missing required fields. Keep app-only sparse/active data out of this representation so equality measures only relevant fields.
2. Build a managed-key diff. Patch existing registrations without rebuilding unrelated sections. Implement setting and unsetting, including removal of stale local overrides. For URL changes, first update portable registration, then synchronize initialized parent/child URL state through native Git. Do not mistake `git submodule sync` for handling every managed option.
3. Validate the entire batch before its first mutation: selections, names, paths, local conflicts, executable update policies, existing metadata, output writability where knowable. Network availability is not knowable without doing work; handle later failure explicitly. Dry-run names the intended remote/branch action but labels unknown remote commit IDs unresolved until execution; it must not invent a target or fetch to fill the preview.
4. Apply modules in deterministic order. For newly created modules, preserve the intended TOML declaration on partial failure so a later init/sync can resume. For removal/rebuild, keep declarations until the corresponding operation has succeeded. On failure, stop later mutations and report completed/failed/pending modules; never print “sync complete.” Do not roll back completed unrelated modules by deleting their data.
5. Separate structural inspection from command status policy. Check may return drift, but sync must treat missing eligible registered checkouts as normal repair input. Pass an explicit command scope into the shared planner: metadata-only for ordinary change/defaults, metadata plus selected sparse edits when requested, and lifecycle/checkout for init/update/sync. After applying actions, query actual Git state again and require the intended strategy-specific postconditions before reporting success.
6. Reconcile removals of managed values as first-class changes. Do not initialize a disabled module just to set local child configuration. Apply those settings when it is explicitly re-enabled and initialized.
7. Record Git staging behavior in action output. Registration/add/move/delete can stage `.gitmodules` and exact gitlinks through Git; an existing-module policy sync should leave config edits reviewable without staging unrelated files. Refuse commands that would clobber conflicting staged/unstaged `.gitmodules` changes. Never commit, stash the entire superproject, or reset its index as cleanup.

**Gate:** R11–R17, R19, and R31 pass. TOML-only setup, ordinary fresh clone, retained-gitdir recovery, and an existing correctly initialized repo all converge according to selected policy. Running sync a second time with unchanged local/remote inputs produces no metadata/index/content changes, no unnecessary fetch, and no misleading errors.

### Phase 5 — Complete checkout, sparse, and recovery semantics

**Own:** the same manager/Git operation boundaries, plus `src/commands.rs`; focused integration tests.

1. For default update/init/sync, use the parent gitlink as the target. For `--remote`, obtain the configured tracking branch/default. Apply checkout/merge/rebase as selected; `none` does no update. Explicit `--branch` at add affects checkout before the gitlink is finalized. Do not turn a fetch into an “updated” message without checking the resulting target.
2. Test relative URLs with a parent remote, non-main default branches, branch `.`, detached parent HEAD, shallow initialization of an older recorded commit, and nested submodules. If a shallow remote cannot provide the required pin, report the exact missing commit; do not silently choose its tip. Add explicit `--recursive` selection for init/update/sync if recursive materialization is exposed; keep it separate from `fetchRecurse`.
3. Apply sparse patterns with native non-cone behavior for arbitrary globs, passing patterns through stdin rather than command-line options. Keep order and negation semantics; reject embedded NUL/newline boundaries in individual input patterns. Preserve the existing automatic deny-all prefix only when that mode is selected.
4. Compare the full normalized ordered effective pattern sequence and actual enabled/mode settings. Equal sequences skip writes and checkout reapplication. Absent/empty patterns use Git's sparse disable operation and restore the full clean checkout. Handle dirty paths safely: preserve edits or report refusal rather than overwriting them. [Git sparse-checkout documentation](https://git-scm.com/docs/git-sparse-checkout)
5. Reset must distinguish “nothing to stash” from an actual stash failure. On preservation success, retain/report stash identity and a recovery command; on failure, do not reset/clean. Reset target is the parent pin, not whatever child HEAD happened to be. Before stashing, detect ignored files/directories and nested repositories that obstruct tracked paths at the target commit; refuse those collisions without mutation. Avoiding `clean -x` alone does not protect them from hard reset. Preserve ignored files and nested repository content. Test recovery by applying the saved stash and comparing original bytes.
6. Disable must not call the current gix deinit path. It updates the raw active flag and managed local activation without removing checkout/history. Re-enabling then init/sync must not lose stashes or local refs.
7. Delete uses Git-aware removal and retains the module object database. For legacy embedded `.git` layouts, move/retain the repository safely before removing the checkout, using Git's supported layout migration. Protect dirty/untracked/ignored data; require an explicit `--force` if the user intends to discard worktree data and show that scope in dry-run. A config-only deletion removes only the declaration.
8. Nuke rebuilds one preflighted module at a time, preserves recoverable history, and leaves intended declarations present if reinit fails. Force does not authorize deletion of unrelated files or an arbitrary gitdir. Do not purge retained repositories automatically. Path change uses Git-aware move, updating registration/local metadata and preserving retained repository identity.

**Gate:** R03–R05, R12–R15, R20–R24 pass. Verify content, OIDs, refs/stashes, parent index, and sparse state—not just command output. Offline reruns and failed remotes leave a repairable repository.

### Phase 6 — Make the CLI explain what happened

**Own:** `src/main.rs`, `src/commands.rs`, `src/git_manager.rs` output, `src/shells.rs`, `src/long_abouts.rs`, `README.md`, `sample_config/`, `schemas/`.

1. Return structured per-module outcomes internally and format them at the CLI boundary. Normal output should identify changed, unchanged, skipped-disabled, and failed modules with a concise summary; verbose output adds effective settings, target commits, and operation context.
2. Separate human status on stdout from progress/errors on stderr. Use `std::io::IsTerminal` to suppress progress renderer control sequences and decorative output when redirected. Sanitize control characters in repository-derived names/paths/messages. Redact URL credentials in app-generated errors and captured child diagnostics; do not print raw full command arguments containing credential-bearing URLs.
3. Return documented exit codes, including check drift and incomplete batches. Preserve underlying Git causes instead of reducing everything to “Repository not found” or calling git2/CLI errors “GitoxideError.” Error context should name the module, phase, affected path, and actionable next step.
4. Make command help match the contract table. Reject conflicting flags at clap parsing. Fix `--shallow` change semantics, `nu`, the from-setup example, disable alias, reset target description, `--all` selection conflicts, and global-default precedence descriptions. Implement actual recursive listing or do not claim it works.
5. Add global `--branch` editing support to `change-global` if documenting `[defaults].branch`; keep schema, types, CLI, and effective resolution aligned. Keep migration/resetting an override discoverable through `--unset`.
6. Repair versioned schema JSON and field names. Make advertised current/latest paths actual JSON assets or verified publication outputs, not broken repository symlinks. Check local alias content and test HTTP-delivered JSON during release verification. Preserve old schemas as historical contracts while documenting canonical current fields/aliases.
7. Make the exact sample and generated template parse. Clearly mark template placeholder URLs; successful parsing does not mean example remotes exist. Execute copy-paste command examples with local fixture URLs. Explain import, default root discovery, pinned versus remote update, unmanaged modules, local URL overwrite during explicit sync, sparse versus partial clone, and recoverable removal.

**Gate:** R08, R17–R19, R25–R27 pass. A new contributor can import an existing repository or start from TOML, initialize from a nested directory, see configuration drift, repair it, and recover from a failed operation without reading implementation details.

### Phase 7 — Remove avoidable cost and misleading measurements

**Own:** `src/main.rs`, `src/lib.rs`, config storage, surviving Git backends, `Cargo.toml`, `benches/benchmark.rs`, `tests/performance_tests.rs`, test script.

1. Import the library from `main.rs` instead of redeclaring config/manager/options/Git modules. Keep CLI-only modules where appropriate. This removes duplicated compilation and test execution without changing functionality.
2. After lifecycle parity passes, delete unreachable mutation implementations and helpers. Remove `gitoxide-core`/`simple_gix`/progress dependencies if the live call graph no longer needs them. Retain only Git libraries used for supported reads; reduce their unused transport/features. Do not upgrade every dependency or rewrite enums solely to erase a crate in this phase.
3. Eliminate whole-map cloning in `Config::add_submodule` (`src/config.rs:1026-1028`) and duplicated sparse storage. Sort module names once per command. Load relevant metadata once, then refresh only after mutation. Skip byte-identical TOML/Git config and unchanged sparse application.
4. Replace benchmarks of private copies with real config load/edit, inspection, and no-op sync paths. Use release/bench artifacts intentionally. Include 1/10/100-module metadata fixtures and small materialized Git fixtures. Keep fixture creation outside timed loops.
5. Record wall time, child process counts, and parent/child memory where available. Label the existing tracking allocator as Rust allocations only if retained; do not call it process peak memory. Use process isolation for allocator tests and host-appropriate RSS reporting. Compare before/after on the same machine and dataset.
6. Performance acceptance is initially structural: zero duplicate module compilation, no-op sync avoids clone/fetch/sparse reapply/config writes, metadata is not reparsed inside every module loop, and existing meaningful ceilings do not regress. Do not invent a universal millisecond/RSS limit from this audit's green synthetic tests.
7. Defer custom threading, caching, and async runtime. If realistic clone/fetch measurement still shows a material bottleneck, use Git's native bounded `--jobs` for compatible batches before writing a scheduler. Do not parallelize shared index/config mutations.

**Gate:** R28 passes, measurements name their actual scope, and the retained dependency/backend surface is smaller. Explain any measured regression; do not describe a source-level simplification as a measured speedup.

### Phase 8 — Gate the shipped behavior

**Own:** `.github/workflows/{ci,release,docs}.yml`, `hk.pkl`, `mise.toml`, `audit.toml`, `.cargo/audit.toml`, `deny.toml`, contributor documentation.

1. Run required stable functional tests on Linux, macOS, and Windows; platform-guard only genuinely unavailable fixture features such as symlink creation. Keep a Rust 1.89 build gate while that is the declared MSRV. Beta/nightly can remain informational; prioritize platform behavior over repeating identical stable coverage jobs.
2. Use locked dependency resolution in CI and release. Ensure the actual tagged commit passes required tests/lint/audit before publication, using an explicit workflow dependency/reusable verification job. Check the artifact intended for release, including native linkage and a CLI smoke test, for tag builds as well as manual dry runs. Do not infer release correctness from an earlier branch run.
3. Consolidate advisory exception policy: the root `audit.toml` still lists a stale extra ignore, while `.cargo/audit.toml`, deny, and workflow settings differ. Every retained exception needs a current reachable-dependency rationale. Determine why `bisync 0.3.0` is yanked and update/remove through its owning dependency; do not add a blanket yanked-package allowance. Rerun audit after dependency pruning.
4. Remove unnecessary workflow write permissions (for example coverage `contents: write`) and replace mutable third-party action refs with reviewed immutable refs under the project's update policy. No runtime or workflow should print credentials. Preserve existing release authorization/environment gates.
5. Validate schema JSON, sample/parser agreement, help/examples, and package contents in CI. Run `cargo package --locked` and test the packaged source so development-only paths are not required. Run docs checks without implying that generated Rust API docs prove end-user command behavior.
6. Update README/CLAUDE/CONTRIBUTING descriptions of backends, defaults, test counts, Git requirement, supported platforms/MSRV, sync semantics, and recovery. Remove stale “TODO: implement” comments on completed commands and unsupported claims that sparse checkout alone reduces downloaded history.

**Gate:** R29–R30 pass and the full acceptance checklist is satisfied. Prepare a reviewable code/documentation change; do not publish or tag a release as part of implementing this plan unless separately authorized.

## Regression matrix and completion checklist

Each row is a fixture family, not a request for one test per helper function. Reuse the existing harness and put closely related cases into table-driven integration tests. All setup Git commands must succeed. Use local remotes and unique temporary directories. For data-preservation rows assert the before/after repository state in addition to exit status.

| Test ID | Fixture and action | Required assertions / suggested test file |
| --- | --- | --- |
| R01 | Config and CLI paths: empty, `.`, `.git`, `..`, absolute path, escaped/symlinked ancestors; names reaching administrative paths | Every mutation rejects unsafe targets before writes; root/outside sentinels and index/config snapshots unchanged. `security_tests.rs`. |
| R02 | Occupied ordinary directory, unrelated repo, dirty module; add with unavailable local remote | Failure preserves files, refs, index and metadata; no unconditional cleanup. `error_handling_tests.rs`. |
| R03 | Clean module with local-only commit and stash; disable, enable, init | Files and object DB retained; original commit/stash remain accessible and recoverable. `integration_tests.rs`. |
| R04 | `lib` module alongside tracked `library.txt` and `lib-extra/file`; delete | Only exact gitlink/registration removed; sibling mode/OIDs unchanged. Held `index.lock` causes refusal with preserved index. `git_ops_tests.rs`. |
| R05 | Dirty tracked/untracked/ignored files; force real stash write/lock failure, then reset; ignored file/directory becomes tracked at target pin | Failed stash or target collision refuses without reset/clean and preserves all bytes. Successful stash+reset can restore original work and resets to parent pin. `error_handling_tests.rs`. |
| R06 | Default ignore dirty, one inheriting module, one explicit none; change default to all and unrelated entry field | Inheriting raw entry stays absent; effective all applied, explicit none preserved; second process sees same values. `config_tests.rs`. |
| R07 | Sparse mode true/false at both scopes; shallow true followed by unrelated change | All booleans round-trip; absent CLI options preserve existing settings; false is not treated as absent. `command_contract_tests.rs`. |
| R08 | Exact checked-in sample, generated template, absent/supported/future schema versions, typo/legacy aliases, older generated `branch="HEAD"` and fetch true/false strings | Canonical examples load; future version/unknown field/conflicting aliases fail contextually without writes; legacy generated values retain documented intent and warn. `config_tests.rs`. |
| R09 | Multiline arrays/strings, commented table headers, quoted/dotted/literal names, escaped strings, non-ASCII names, inline comments | Edit then parse succeeds; requested value changed; unrelated document content preserved within editor contract. `config_tests.rs`. |
| R10 | Write failure, external edit before final comparison, simultaneous submod writers using same/different configs and linked worktrees, shared config across repos, symlink output, no-op edit | Original stays valid; external edit detected at comparison; cooperating writers serialize without lost updates/deadlocks; identical edit avoids rewrite; locks released on normal errors. The external check/replace race is an explicitly documented limit. `error_handling_tests.rs`. |
| R11 | TOML-only parent with no `.gitmodules`, no module directories and optional omitted path | Init creates correct registration, mode-160000 gitlink and files at effective root-relative path. `integration_tests.rs`. |
| R12 | Parent records old child commit; child remote advances; clone parent without recursion, init/sync | Correct old HEAD/content materialized, never empty successful checkout or remote tip. Repeat is no-op. `integration_tests.rs`. |
| R13 | Distinct main/feature/default branches; add branch feature; default update and update --remote; update=none/merge/rebase; clean divergent local commits | HEAD/content, gitlink and branch configuration satisfy strategy-specific contract; explicit add with none creates the initial checkout, while later automatic init/update/sync do not fetch/materialize. Successful merge/rebase followed by check accepts preserved descendants; repeated sync is no-op. Metadata-only change must not move a divergent HEAD. Include `.` and detached parent. `integration_tests.rs`. |
| R14 | Existing module: edit URL/branch/ignore/update/fetch, change defaults, remove previously managed keys | TOML, `.gitmodules`, parent local config and child URL/checkout effects agree; stale local overrides removed; unrelated keys retained. `integration_tests.rs`. |
| R15 | Ordered positive/negative sparse patterns, extra/reordered existing patterns, changed mode, removed/empty patterns | Exact effective sequence and enabled/mode drift detected; full files restored on disable; unchanged sync no reapply; dirty excluded paths protected. `sparse_checkout_tests.rs`. |
| R16 | Alias != path; Git name != path; quoted/alternate-formatted `.gitmodules`; prefix paths; duplicate/overlapping paths | Exact unique matching, no duplicate registration, no substring match; ambiguity rejected before mutation. `git_ops_tests.rs`. |
| R17 | Same command from repo root and nested cwd; custom config; linked worktree; missing explicit/default config | Same managed repository/paths; intended file selected; helpful missing-file errors; no accidental discovery of sibling repo. `command_contract_tests.rs`. |
| R18 | Change omits fields, explicit false, unset override, clear sparse, append conflicts, all+names, no settings | CLI tri-state/clear/conflict semantics enforced; no silent defaults; conflict before manager mutation. `command_contract_tests.rs`. |
| R19 | Disabled entry with unreachable URL; no-init add; unmanaged Git module; actual nested module hierarchy | Disabled not touched; no-init later initializes; unmanaged preserved; recursive list reports real descendants. `integration_tests.rs`. |
| R20 | Relative child URL with parent origin; non-main default branch; shallow older pin; requested recursive init | Git-consistent URL and commit resolution; shallow failure explicit; recursive selection distinct from fetch settings. `integration_tests.rs`. |
| R21 | Missing `.gitmodules` with valid managed gitlink; missing gitlink with valid declaration; retained gitdir; broken pointer | Recover only unambiguous states; preserve pin/history; conflicts fail with targeted diagnosis. `error_handling_tests.rs`. |
| R22 | Never-initialized/config-only delete, initialized delete, legacy embedded gitdir, failed nuke reinit | Removal never deletes unrelated directory; recoverable object DB retained; failed rebuild keeps declaration and clear partial outcome. `integration_tests.rs`. |
| R23 | Initialized path move with clean tree, dirty tree, occupied destination, and inactive declaration | Safe move preserves identity/history and registration; rejected moves preserve all state; inactive not cloned. `integration_tests.rs`. |
| R24 | Three-module batch with invalid target, later network failure, and interruption/retry | Preflight error changes none; runtime failure reports completed/failed/pending accurately; retry converges without erasing work. `error_handling_tests.rs`. |
| R25 | Read-only check/list and every dry-run; fingerprint files/index/refs before and after | No writes/network for inspection; correct exit codes; planned actions match later mutation; no dry-run lock file. `command_contract_tests.rs`. |
| R26 | Redirected output; fake credential-bearing URL; control characters in repository metadata; real Git error | No unintended terminal control sequences or credential leakage; module/phase/cause retained; no false success. `security_tests.rs`. |
| R27 | Copy-paste README workflow with local URLs, from-setup, template, help and all supported completion shells | Examples parse and accomplish stated operations; templates reload; `nu` works; generation works outside repo where appropriate. `command_contract_tests.rs`. |
| R28 | Production config/edit/status/no-op sync at 1/10/100 modules with bench profile | Real production code measured; no duplicate test compilation or unnecessary writes/fetch; allocator scope accurately labeled. `performance_tests.rs`, benchmark. |
| R29 | Linux/macOS/Windows stable jobs, MSRV build, required release verification and linkage/smoke tests | Failures gate the exact artifact/tag; no release path skips functional prerequisites; locked package builds. CI workflows. |
| R30 | All schemas/aliases, sample agreement, fresh advisory/deny check, packaged source test | Published assets are real valid JSON; current dependency exceptions deliberate; no stale/broad suppression; package is self-contained. CI/tooling. |
| R31 | Unrelated staged `.gitmodules` edit plus a separate unstaged edit before add/move/delete and metadata sync | Refuse before mutation if both layers cannot be preserved; otherwise assert index bytes/content and worktree diff independently. No accidental staging of unrelated edits. `integration_tests.rs`. |

After native mutation consolidation, retarget `fallback_tests.rs` to the surviving backend's operation/error contract or remove obsolete fallback-only tests. Do not keep 100% coverage of removed code as a goal. Any backend still selectable for mutations must pass the same behavior matrix; the preferred end state has only one.

Final local verification should include:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features
cargo nextest run --locked --all-features --profile ci --no-fail-fast
cargo test --locked --all-features --doc
cargo +1.89 check --locked --all-features
cargo audit
cargo deny check
cargo package --locked
```

Run profile-specific performance checks through the corrected script and record the command/profile/dataset. Remove the two external-network dependencies from ordinary test execution by replacing them with deterministic local failures; retain any live-authentication smoke test as explicit, optional evidence. Verify platform/release jobs from their actual results when available, and name any environment limitation rather than declaring unrun checks green.

Completion requires all of the following: safe preservation tests pass; root/nested/fresh-clone onboarding works; TOML edits persist with inheritance intact; Git configuration and checkout state converge; disabled and sparse removal behavior is correct; repeated sync is a no-op; output/exit codes describe reality; regression tests check real Git state; package/platform/security gates pass; and docs describe the implemented contracts. Do not stop at passing the old suite.

## Agent execution guidance

The implementing agent should first read this plan, inspect current changes since the audited commit, and preserve unrelated work. Reproduce the baseline before editing. Use independent agents for bounded work as requested by repository instructions, with explicit file ownership; do not let config and lifecycle workers edit `git_manager.rs` concurrently without dividing its responsibilities.

A useful sequence is: one harness/CI worker during the root-context and safety work; then config/options/schema work independently of the Git operation layer; integrate both before reconciliation; then parallel documentation/CI and performance measurement while the main agent verifies lifecycle acceptance. The primary agent owns contract consistency and final integration. Each worker must report which regression IDs pass and any changed assumptions.

The plan deliberately defers custom parallel schedulers, persistent caches, new UI modes, automatic pruning of undeclared modules, purging retained history, and general transactions across repositories. Add those only for a demonstrated requirement. This repair is complete when the existing product's promised workflows work safely and predictably.
