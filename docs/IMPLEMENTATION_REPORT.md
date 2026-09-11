<!--
SPDX-FileCopyrightText: 2026 Adam Poulemanos and contributors
SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT
-->

# Improvement plan implementation report

Implementation base: `31c5e373bfd29162f86675fa60ed4af0adf8df97` on
`codex/improvement-plan`. Early delegated harness RED/retained runs used the
pre-existing `/private/tmp/submod-audit-target`, and early config and R11–R15
RED runs resolved to the worktree default `target/` despite their requested
target environment. Phase 2 gate checks and all later checks use the explicit
Cargo option
`--target-dir /private/tmp/submod-improvement-target`. This is required because
login-shell activation reads `mise.toml`, whose `CARGO_TARGET_DIR = "target"`
overrides a requested environment value. Test fixtures set
their own Git identity, disable signing and prompts, and allow local file
transport only in the fixture environment.

## Baseline

Before application edits:

```text
cargo test --locked --offline --all-features -- --test-threads=1 \
  --skip test_invalid_git_url --skip test_network_timeout_simulation
exit 0: 567 executions passed; two external-network tests filtered
rustc 1.98.1; cargo 1.98.1
```

The baseline reproduced the audit. It also reproduced duplicate compilation
of 168 unit tests in both the library and binary targets.

## Phase reports

### Phase 1 — regression harness

Gate ready for independent review. The RED runs established these audited discrepancies before
production changes:

- R11: omitted path is rejected instead of defaulting to the TOML nickname.
- R12: a successful fresh-clone init leaves no checkout.
- R13: explicit add with `--branch develop` checks out the default branch and
  omits the branch from `.gitmodules`; explicit add with `update=none` already
  creates the requested initial checkout.
- R14: sync does not propagate an edited managed URL to `.gitmodules`.
- R15: removing sparse paths leaves sparse checkout enabled and excluded files
  absent.

The preservation slice additionally reproduced R01–R05 and R16 with checked
Git commands and actual file, ref, index mode/OID, config, and gitdir
snapshots. The configuration slice reproduced R06–R10 and R18. Three specified
behaviors already worked: absent schema metadata loads, append without sparse
patterns is rejected, and an occupied output ancestor is preserved. These are
smoke reproductions for each family; untested variants remain outstanding in
the table. No production file was changed before these runs.

Harness/tooling validation:

```text
87 retained tests in security/error/git-ops: PASS
cargo nextest ... --profile ci -E test(regression_r04_delete_preserves):
  command/profile accepted and test RED for the intended sibling-index loss
cargo fmt --all -- --check: PASS after formatting
git diff --check: PASS
```

Exact logs: `/private/tmp/submod-phase1-harness-report.md` links the harness
logs; config RED output is `/private/tmp/submod-phase1-config-final.log`;
lead lifecycle output is `/private/tmp/submod-r11-r14-red.log` and
`/private/tmp/submod-r15-red.log`. The first complete retained nextest run found
one old test that relied on ignored Git failure status; it was corrected to
assert the expected missing-object failure explicitly. Three old config tests
whose names promised inheritance/roundtrip/required-field behavior without
asserting it were removed because the new RED fixtures cover those contracts.

### Phase 2 — repository context and native Git mutation boundary

The implementation now discovers the invocation directory, worktree root,
per-worktree Git directory, shared Git common directory, and resolved config
path once. Git-reported paths are retained as `PathBuf` values; on Unix their
raw bytes are preserved. Lifecycle calls use the worktree root even when the
CLI starts in a nested directory or a linked worktree.

All lifecycle mutations now pass through one native Git command boundary in
the operations module. Paths and logical registration names are validated
before mutation; destination ancestors cannot be symlinks; root, parent,
absolute, administrative, overlapping, and case-normalized overlapping paths
are rejected. Existing files, directories, symlinks, independent repositories,
and dirty registered checkouts are refused without cleanup. Exact
`.gitmodules` lookup supports ordinary path-named sections such as
`submodule.vendor/lib` and alternate formatting while rejecting duplicate path
registrations and custom update commands.

Native add, move, delete, update, reset, stash, and sparse calls use argument
arrays, rooted working directories, checked statuses, and option terminators.
There is no mutation retry through gix or git2 after an ordinary failure.
Before metadata changes the implementation refuses native Git lock files,
symlink metadata targets, and mixed staged/unstaged `.gitmodules` layers. It
verifies the resulting registration, gitlink mode, checkout identity, and
deletion postconditions instead of trusting exit status alone.

Deletion has separate verified paths for committed and newly staged gitlinks.
Both remove only the exact registration and parent local section, leave the
stored module repository and refs available, and leave no checkout placeholder.
A later add can reactivate that repository only when the requested URL exactly
matches its retained origin; occupied checkout paths still refuse. Disable and
activation changes write local `submodule.<git-name>.active` while keeping the
app-only `active` field out of portable `.gitmodules`, and never deinitialize a
checkout. Reset refuses held stash locks and target-tree collisions with
ignored content or nested repositories; it no longer runs `clean`.

Mutating manager calls acquire the canonical shared-common-dir app lock and
the canonical sibling config lock in lexical order, reload and revalidate
configuration under the locks, and hold both through postcondition checks.
Config generation acquires only its output sibling lock. Locks are never
stolen; timeout errors explain stale-lock recovery. Full simultaneous-writer,
external-edit comparison, and no-op-write acceptance remains in R10 for Phase
3, where the direct config writer is replaced atomically.

Phase 2 focused gate evidence:

```text
cargo +1.98.1 nextest run \
  --target-dir /private/tmp/submod-improvement-target \
  --locked --offline --all-features --profile ci \
  --no-fail-fast \
  -E 'test(/^(regression_r0[1-5]|regression_r16|phase2_|test_delete_committed_gitlink_retains_history_and_readds)/)'
exit 0: 44/44 passed; 608 skipped

cargo +1.98.1 nextest run \
  --target-dir /private/tmp/submod-improvement-target \
  --locked --offline --all-features --profile ci \
  --no-fail-fast -E 'not test(/phase1_config|phase1_commands|r1[1-5]_/)'
exit 0: 614/614 retained tests passed; 38 later RED tests skipped

reviewed debug artifact SHA-256:
73e399b7bd56946d631816c6783a2e50c4f8f751193d878df314665e1f10af78
```

The first retained run after routing mutations through native Git produced 35
failures. Independent triage classified 14 as obsolete retry/purge/path-name
contracts, 13 as fake or unisolated fixtures, seven as real integration
blockers, and one as a later sparse import contract. The real blockers were
fixed: unspecified Git policy values are no longer emitted, both delete paths
support safe retained reattachment, alias-aware git2 status resolves the
portable name, and initialized sparse imports propagate real inspection
errors while never-initialized registrations import without fabricating sparse
state. Obsolete tests were retargeted to preservation and actual Git state;
fake sparse fixtures now use contained repositories.

Independent Phase 2 review found three additional containment defects. A
configured checkout named `lib*` was reaching `lib-extra` because `--` does
not disable Git pathspec expansion; an existing child gitfile could redirect
update/init into an unrelated worktree; and a symlink within
`<gitdir>/modules/<logical-name>` could redirect retained-repository reuse
outside the parent Git directory. The fixes pass explicit `:(literal)`
pathspecs only to Git interfaces that consume selectors, keep filesystem
operands such as `git mv` raw, verify an existing materialization target's
top-level identity, and validate every derived module-storage ancestor beneath
the canonical per-worktree gitdir. Positive coverage retains a legitimate
external child gitdir whose worktree identity is the intended checkout.
Dangling Git lock symlinks are detected with `symlink_metadata`, and sparse Git
paths use the same lossless native decoder as repository context.

Logs: `/private/tmp/submod-phase2-gate-final-family-explicit-target.log`,
`/private/tmp/submod-phase2-retained-explicit-target.log`,
`/private/tmp/submod-phase2-delete-context-final.log`, and
`/private/tmp/submod-phase2-contract-retarget-report.md`. Focused R03/R05,
storage/external-gitdir, and literal-interface evidence is in
`/private/tmp/submod-phase2-r03-r05-acceptance.md`,
`/private/tmp/submod-phase2-storage-boundary.log`, and
`/private/tmp/submod-phase2-literal-move-magic.log`. The linked-worktree,
native lock, staged-layer, reset-collision, and gitfile-containment probes used
as implementation inputs are recorded under `/private/tmp/submod-supervision/`;
they are review evidence, not counted as Submod acceptance tests.

### Phase 3 — raw configuration, effective defaults, and atomic persistence

Configuration now parses and retains raw declarations. A single
`effective_entry` resolver supplies inherited and built-in behavior to planning
without adding those values to the module's TOML table. This includes global
branch inheritance while preserving an explicit legacy `branch = "HEAD"`
override. TOML alias escaping is distinct from native Git spelling, so a literal
branch named `current` round-trips through TOML as `refs/heads/current` while Git
receives `current`. Sparse settings are derived from module entries rather than
kept in a second mutable map.

The parser validates schema versions, aliases, unknown keys, types, required
URLs, effective paths, option enums, and sparse boundaries before actions are
constructed. Semantic errors carry the original TOML line. The checked-in
sample, generated template, absent schema, and supported 1.0/1.1 schemas use the
same parser. CLI booleans are tri-state, supported optional values can be
removed with `--unset`, sparse patterns can be cleared explicitly, and dynamic
argument conflicts return command-line usage status 2 before mutation.

The config editor uses `toml_edit` on the exact bytes loaded while mutation
locks are held. It patches only touched managed fields, preserves comments,
ordering, table spelling, unrelated sections, explicit false values, and raw
inheritance, then reparses rendered bytes before replacement. A byte-identical
edit skips the write. Otherwise it writes and syncs a temporary sibling and
atomically replaces the destination while preserving existing Unix mode bits.
The final comparison distinguishes an absent file from an empty file and
detects changed, created, or deleted destinations. Symlink outputs and
non-files are refused. Deterministic injected failure tests preserve the exact
old parseable bytes and remove the temporary file.

All cooperating writers acquire canonical common-repository and output locks
in lexical order, reload the same bytes used for document parsing and the final
snapshot comparison, and write at most once per command. This includes nuke
rebuilds and `generate-config --from-setup`; ordinary relative, relative
subdirectory, and absolute output paths share the same lock-path normalization.
Nuke invokes Git-only lifecycle primitives while the command lock is held and
keeps declarations through rebuild. Simultaneous same/different output files,
linked worktrees, repositories sharing one config, and stale preloaded managers
are covered with deterministic lock-arrival barriers rather than timing sleeps.

Effective policy reconciliation validates the exact child checkout and rejects
repository-supplied custom update commands before any metadata write. It
changes only explicitly declared or inherited settings, compares the exact
managed worktree key with its index value before editing, preserves unrelated
staged/unstaged content, and leaves existing-module policy edits unstaged for
review. Automatic init/update no longer manufactures built-in policy keys.

Phase 3 gate evidence:

```text
cargo +1.98.1 nextest run \
  --target-dir /private/tmp/submod-improvement-target \
  --locked --offline --all-features --profile ci --no-fail-fast \
  -E 'test(phase3_acceptance)'
exit 0: 51/51 passed; 645 skipped

cargo +1.98.1 nextest run \
  --target-dir /private/tmp/submod-improvement-target \
  --locked --offline --all-features --profile ci --no-fail-fast \
  -E 'not test(/phase1_config|phase1_commands|r1[1-5]_/)'
exit 0: 690/690 retained tests passed; 6 later RED tests skipped

reviewed debug artifact SHA-256:
0a4ed386090f6c3a53c6e2f40b475012fe3fd3739f7e043d16bbb8cae781ac94
```

Exact final logs are `/private/tmp/submod-phase3-acceptance-gate-fix1.log`
and `/private/tmp/submod-phase3-retained-fix1.log`. Focused atomic/stale-manager,
nuke, concurrency, relative-output, model, and CLI evidence is linked from
`/private/tmp/submod-phase3-acceptance-report.md`,
`/private/tmp/submod-phase3-config-model-report.md`, and
`/private/tmp/submod-phase3-cli-report.md`. The first retained gate run exposed
eight integration regressions and three obsolete assertions; all were corrected
before the final 690-test run. Independent model review passed. Independent
persistence review accepted nuke and concurrency, then identified the relative
default output regression that the final tests now cover.

Atomic replacement protects one config file, not a transaction spanning TOML,
Git config, and the parent index. An external editor that ignores the app lock
can still race after the final byte comparison and before replacement. Windows
replacement uses `tempfile`'s platform operation; this report does not claim
preservation of Windows ACLs or read-only attributes until hosted Windows
acceptance runs.

### Phase 4 — shared reconciliation

Init, update, and sync now use one reconciliation flow. It observes each
selected module's raw and effective TOML declaration, exact portable
registration, stage-0 gitlink, retained repository, materialized checkout,
managed parent/child config, selected update strategy, and sparse state before
choosing actions. Whole-batch preflight rejects unsafe identities, unmerged
gitlinks, known native locks, mixed `.gitmodules` layers, dirty checkout
transitions, and incomplete registrations before the first mutation. Structural
registration actions run before metadata-only actions, so changes created by
the command are not mistaken for pre-existing user edits and retries converge.

The same flow covers TOML-only setup, fresh-clone materialization, incomplete
registration repair, retained-gitdir reattachment, and initialized checkout
drift. A manual TOML path edit cannot silently reuse a logical registration at
its old path. Missing `.gitmodules` can be reconstructed only from an
unambiguous managed gitlink; a declaration without a gitlink completes only at
an empty destination. Unmerged index stages and ambiguous occupied content are
reported without changing files, refs, config, or the parent index. Successful
reconciliation verifies the stage-0 pin, registration, checkout identity, and
strategy-specific ancestry before reporting success.

Managed metadata is patched key by key in portable, local, worktree, and child
config. Removed declarations unset stale managed overrides while unrelated
module keys remain. Native Git resolves relative URLs against the parent remote
and selects the child's configured default remote, including cases where the
parent and child URL spellings legitimately differ. The no-op predicate checks
those native results before invoking `git submodule sync`; a second unchanged
sync preserves config bytes, inode, and modification time and performs no
fetch. Known parent worktree and selected child config locks are preflighted
before TOML or Git metadata changes.

Checkout policy now distinguishes the recorded parent pin from explicit
`--remote`. Default init/update/sync materializes the recorded stage-0 pin
without fetching when it is already available; `--remote` selects the
configured/default branch without staging a new parent gitlink. Checkout,
merge, and rebase verify their respective target or ancestry postconditions.
`update = "none"` and `active = false` skip automatic fetch/materialization,
while explicit add with `update = "none"` still creates its initial checkout.
Branch `.` uses the parent's symbolic branch and refuses a detached parent
before batch mutation. Metadata-only edits never move a divergent child HEAD.

Sparse reconciliation compares the exact ordered effective pattern sequence,
enablement, and cone mode. It restores a full checkout when patterns are
removed, protects dirty included and excluded paths, and skips sparse commands
when state already matches. Recursive init, sync, and update traverse real
nested module registrations independently of `fetchRecurseSubmodules`.
Unmanaged Git modules are preserved and reported. `--no-init` remains a
one-command selection; a later init materializes normally.

The installed mise runner was cargo-nextest 0.9.29, which predates test groups
and silently ran the integration and performance binaries concurrently. Those
failed performance logs are diagnostic only. The repository now pins 0.9.128,
declares 0.9.55 as the enforceable minimum, and makes `scripts/run-tests.sh`
reject older versions before running tests. `show-config test-groups` with
cargo-nextest 0.9.128 confirmed the integration, sparse, reconciliation, and
performance binaries in `serial-integration` with one thread.

Phase 4 combined gate evidence:

```text
/Users/adampoulemanos/.cargo/bin/cargo-nextest nextest run \
  --config-file /private/tmp/submod-improvement-plan/.config/nextest.toml \
  --manifest-path /private/tmp/submod-improvement-plan/Cargo.toml \
  --locked --offline --all-features --profile ci \
  --target-dir /private/tmp/submod-improvement-target --no-fail-fast
exit 0: 757/757 passed; 0 skipped; 399.247s

cargo-nextest 0.9.128 (96bc6d4f4 2026-02-19)
debug artifact SHA-256 before and after the gate:
6b06a819931a4167e34e03b38dc8da7e80f03d461c3f5a70d48c3dbc871e5e3f

cargo +1.98.1 check --locked --all-features \
  --target-dir /private/tmp/submod-improvement-target: PASS
cargo +1.98.1 fmt --all -- --check: PASS
bash -n scripts/run-tests.sh: PASS
git diff --check: PASS
```

The combined log is
`/private/tmp/submod-phase4-retained-supported-nextest.log`; effective group
membership is `/private/tmp/submod-phase4-nextest-groups.log`. The unchanged
performance ceilings passed in the serialized run. Separate uncontended runs
also passed, supporting contention as the cause of the old-runner failures;
they are not a before/after optimization measurement.

Independent metadata review verified relative URLs, selected non-origin child
remotes, parent-relative URL contexts, known lock refusal, exact key removal,
and second-sync file identity. Independent planner review reproduced and then
closed dirty later-module partial mutation, initialized init drift, unmerged
gitlink acceptance, manual path identity reuse, and mixed metadata/registration
layer hazards. Reports are
`/private/tmp/submod-supervision/phase4-metadata-review.md` and
`/private/tmp/submod-supervision/phase4-planner-review.md`. The supervisor's
original lifecycle replay is
`/private/tmp/submod-supervision/phase4-original-repro.json`. These independent
probes supplement the retained regression suite and are not counted as its
757 tests.

### Phase 5 — checkout, removal, reset, and recovery completion

Reset now plans a sorted, duplicate-free selection under one command lock and
preflights every stage-0 parent gitlink, child identity, stash lock, and target
collision before creating the first stash. It records the exact stash object,
resets to the parent gitlink rather than the current child HEAD, and verifies
the resulting OID and content. The recovery guidance is an in-child
`git stash branch` command with an unused generated branch name; executing only
that printed command from the post-reset pinned checkout restores the stash
base, index, staged and unstaged bytes, and untracked files. A stash failure or
unmerged parent gitlink aborts before reset. Ignored files, directories, and
nested repositories that collide with the target tree refuse without mutation.

Fresh materialization follows native relative-URL resolution and preserves an
older recorded pin even after the remote advances. Shallow initialization
checks the exact requested object: an available older pin succeeds, while a
remote that cannot supply it reports that OID and never substitutes the remote
tip. Malformed and nonexistent child gitfile targets refuse before touching the
referenced or intended worktree. The Phase 4 branch, update-strategy, recursive,
and sparse tests remain part of this gate and continue to verify exact target or
ancestry and ordered sparse content.

Delete has an explicit `--force` contract. The default protects tracked,
untracked, ignored, nested-repository, and deleted-worktree content. Force may
discard content only inside the selected validated checkout; it still retains
the module object database, refs, stash, registration identity needed for later
reuse, and unrelated parent/sibling state. Native Git's supported removal
migrates a legacy embedded `.git` directory into retained module storage before
the checkout is removed. A config-only delete changes only the TOML
declaration and leaves an occupied unrelated destination untouched.

Default nuke rebuild now retains the Git registration, parent gitlink, logical
section name, and per-worktree retained storage, then uses native deinit/update
rather than delete/re-add. All selected modules are preflighted before the first
rebuild. Native deinit's internal force flag is used only after the app proves
the selected checkout has no tracked changes, untracked or ignored files, or
nested gitlinks; an explicit user `--force` is required to authorize discarding
such data.
Because outer status can hide ignored content inside a nested checkout, default
rebuild conservatively refuses any nested mode-160000 index entry, including an
incomplete nested registration with no `.gitmodules` declaration.

Rebuild snapshots ordered duplicate local and worktree Git configuration,
reconciles a TOML-only URL change to the selected child remote before a needed
fetch, and restores all unrelated values afterward. Managed active, update,
ignore, branch, recurse, shallow, URL, and worktree override state reaches the
declaration after materialization; `active=false` remains durable and an
explicit nuke still creates the requested initial checkout when update is
`none`. New structural registrations deliberately stage only their requested
`.gitmodules` settings so multiple config-only rebuilds do not reject deltas
created by an earlier module. Existing-registration metadata edits remain
unstaged and preserve the prior staged blob and parent index. A later runtime
failure reports completed, failed, and pending modules, keeps declarations,
gitlinks, storage, refs, and stashes repairable, and converges on retry or an
explicit force retry when native deinit left a deletion-only checkout.

Path changes use native move for initialized modules and metadata-only movement
for inactive declarations. Dirty sources, occupied destinations, and identity
conflicts refuse with exact parent, child, config, ref, and file snapshots
unchanged. An actual interrupted child process leaves owned partial state and
locks diagnosable; after the injected cause is repaired, retry converges without
erasing prior work.

Phase 5 combined gate evidence:

```text
/Users/adampoulemanos/.cargo/bin/cargo-nextest nextest run \
  --config-file /private/tmp/submod-improvement-plan/.config/nextest.toml \
  --manifest-path /private/tmp/submod-improvement-plan/Cargo.toml \
  --locked --offline --all-features --profile ci \
  --target-dir /private/tmp/submod-improvement-target --no-fail-fast
full retained run: 784 run; 778 passed; 6 test assertions failed; 0 skipped;
670.867s

same command and artifact, filtered to the six corrected assertions:
6/6 passed; 778 skipped; 31.533s

final executable reset-recovery assertion on the same artifact:
1/1 passed; 3.029s

cargo-nextest 0.9.128 (96bc6d4f4 2026-02-19)
debug artifact SHA-256 before and after both runs:
68be84a59b877a7cb5526145008b27711be2b0800677c757ec0d1eed1334ec64

RUSTUP_TOOLCHAIN=1.98.1 cargo check --locked --offline --all-features \
  --target-dir /private/tmp/submod-improvement-target: PASS
cargo +1.98.1 fmt --all -- --check: PASS
git diff --check: PASS
```

The six full-run failures were stale test expectations on the same successful
production artifact. Three expected old progress wording. Their replacements
require verified parent pins, child HEAD/content, parent index, exact stash OID,
stash contents, and executable recovery output. Three fixtures assumed the old
add path omitted `shallow=false`: one redundantly tried to commit an already
staged value, and two compared a pre-declaration portable value that correct
reconciliation removed. The corrected tests require the exact intended
working-tree removal while preserving the raw TOML, staged `.gitmodules` blob,
parent index, history, and unrelated settings. No production source or binary
changed between the 778-test result and the 6-test correction run.

Logs are `/private/tmp/submod-phase5-retained-final.log`,
`/private/tmp/submod-phase5-retained-corrections.log`,
`/private/tmp/submod-phase5-reset-recovery-final.log`,
`/private/tmp/submod-phase5-nextest-groups-final2.log`, and
`/private/tmp/submod-phase5-cargo-check.log`. Focused recovery evidence is in
`/private/tmp/submod-phase5-checkout-recovery-final.log` and the Phase 5 review
and native-oracle files under `/private/tmp/submod-supervision/`. Independent
review replayed unmerged-reset preservation, the printed stash recovery command
with paths containing spaces, logical-name nuke identity, TOML-only changed-URL
fetch, ordered local/worktree config preservation, missing nested declarations,
and two `shallow=false` config-only rebuilds on frozen artifacts. Those probes
supplement the repository suite and are not included in its 784-test count.

### Phase 6 — command contract, read-only inspection, and operator output

Every mutating command now has a read-only planning path. `--dry-run` uses the
same selection, validation, observed-state, and preflight logic as execution,
then prints the concrete per-module action without acquiring application or Git
locks, changing config/index/worktree metadata, or starting clone/fetch
transport. Plans distinguish unchanged modules, metadata-only reconciliation,
structural registration and staging, checkout materialization, known parent
pins, unresolved remote targets, branch/remote selection, skipped checkout
policy, and completed/failed/pending batch outcomes. Git inspection commands
disable `diff.autoRefreshIndex`; this is necessary because Git 2.50.1 refreshes
the index stat cache even with optional locks disabled. Stale-stat-cache tests
fingerprint file contents, inode/mode/mtime, index bytes, directory metadata,
and transfer traces before and after every structural preview.

Read-only `check` and `list` use the same repository context and validated
child identity boundary as lifecycle execution. They report portable/local/
worktree/child metadata drift, recorded-pin or strategy ancestry drift, dirty
managed checkouts including `update=none`, unmanaged registrations, inactive
and uninitialized states, and inspection failures. Recursive inspection walks
actual nested registrations, marks uninitialized descendants as not inspected,
sorts output deterministically, and rejects redirected child worktrees before
descent. Implicit config discovery is distinct from an explicit path even when
the spelling is `submod.toml`; default, relative, absolute, nested-cwd, linked
worktree, missing-config, and outside-repository template/completion contexts
have command-specific behavior.

Errors retain their origin. Argument and configuration validation exits 2;
native Git, repository discovery, drift, filesystem permission, and other
operational failures exit 1. Filesystem validation preserves the affected path,
the underlying `io::ErrorKind`, and its cause instead of converting every path
failure to invalid input. Native Git output is parsed from its original bytes;
only human display passes through the output sanitizer.

Human results are written to stdout, progress and warnings to stderr, and
machine completion output remains unmodified. Per-module summaries are
finalized from observed postconditions, so a remote advance reports the actual
new OID, a no-op remote or recursive request reports unchanged, and a metadata
repair under disabled/none policy reports changed plus the checkout skip.
Partial failures name completed, failed, and pending modules and do not print a
success summary. Human text escapes control characters and terminal sequences,
redacts apparent URL userinfo before escaping, preserves ordinary Unicode and
credential-free host:port prose, and covers native, configuration, fallback,
captured gix, generated-path, sparse-diagnostic, and final error paths.

The README, long help, sample, built-in template, current schema aliases,
versioned schema, shell completions including nushell, and test runner now
describe the canonical command and configuration contract. `generate-config
--from-setup` is a boolean repository import, handles registered but
uninitialized children, validates the destination equally in preview and
execution, and conflicts with template mode. Global branch changes use the
same raw/effective inheritance rules as module settings.

The first full Phase 6 diagnostic run exposed 42 retained assertions tied to
old wording or fixtures, one real permission-error category defect, and a
repeatable 10-module inspection cost. The retained assertions were rewritten
around exact named outcomes, targets, streams, Git state, bytes, OIDs, index,
and recovery behavior. The identity path was then reduced from repeated full
repository discovery to lossless gitfile inspection plus one native
`--show-toplevel` containment check. A controlled replay on the preserved test
artifact changed check from 6.510s to 3.637s and no-op update from 25.275s to
8.566s; the original 5s and 20s limits were retained. This is bounded evidence
for the corrected inspection path, not the Phase 7 production benchmark or a
general before/after speed claim.

Phase 6 final gate evidence:

```text
cwd: /private/tmp/submod-improvement-plan
RUSTUP_TOOLCHAIN=1.98.1 \
/Users/adampoulemanos/.cargo/bin/cargo-nextest nextest run \
  --locked --offline --all-features --profile ci --no-fail-fast \
  --target-dir /private/tmp/submod-improvement-target
exit 0: 834/834 passed; 0 skipped; 756.762s

cargo-nextest 0.9.128 (96bc6d4f4 2026-02-19)
test-selected debug CLI SHA-256 before and after the gate:
387eb3f79be8cb514ea22dd66acdb868bd2d776c4a50b1bf19bf5de8e4de474c

ordinary cargo-build CLI independently reviewed by the supervisor:
37529dd21de7aa18325b884d927060f588fbbfcc8804c1b4053104860bfa63b5

cargo +1.98.1 check --locked --offline --all-features \
  --target-dir /private/tmp/submod-improvement-target: PASS
cargo +1.98.1 fmt --all -- --check: PASS
git diff --check: PASS
```

The final log is `/private/tmp/submod-phase6-retained-fix3.log`. Supported
Nextest group inspection confirmed that all integration, reconciliation,
performance, Phase 5, and Phase 6 binaries use the serial group with one
thread. Cargo supplies a distinct integration-test executable artifact through
`CARGO_BIN_EXE_submod`; its hash and the ordinary debug-build hash are recorded
separately and evidence is not transferred between them.

Independent read-only and summary replays passed 9/9 and 4/4 on the frozen
Phase 6 candidate. Independent identity replay passed relative gitfiles,
embedded gitdirs, a valid external gitdir reached through a symlink, explicit
foreign `core.worktree` refusal, and owned-storage ancestor escape refusal with
exact state snapshots. Evidence is in
`/private/tmp/submod-supervision/phase6-readonly-fix2.json`,
`phase6-summary-fix2.json`, and `phase6-identity-optimization-fix3.json`.
Sanitizer, schema, execution-summary, retained-correction, and identity reviews
are recorded in the corresponding `phase6-*-review.md` files under
`/private/tmp/submod-supervision/`. Those probes supplement the 834 retained
tests and are not included in that count.

### Phase 7 — backend retirement, duplicate compilation, and measurement

`src/main.rs` imports the library instead of redeclaring
config/manager/options/Git/shell/utility modules, removing the duplicate
compilation (and duplicate unit-test execution) of the library in the binary
target. The unreachable backend mutation bodies, their `GitOperations`
implementations, `try_with_fallback_mut`, the `forcing_cli_add` seam, and
`simple_gix.rs` (with the direct `gitoxide-core` and `prodash` dependencies
and the gix `blocking-http-transport-curl-openssl` / `worktree-mutation`
features) were deleted. `gitoxide-core` is no longer in the dependency graph
at all. gix remains for supported reads (`status`, plus `sha1` and
`max-performance-safe` tuning); git2 remains for reads, repository handles,
and option conversions. The yanked `bisync 0.3.0` stays reachable via
gix `status` → `gix-protocol`; upstream withdrew it without a released
replacement, so it is documented in `deny.toml`, not blanket-allowed.

`Config::add_submodule` and `remove_submodule` mutate in place (no whole-map
clone); sparse patterns are borrowed (`&[String]`) through the read APIs.
`fallback_tests.rs` and `git_ops_tests.rs` were retargeted to the surviving
contract: native manager mutations checked against real Git state plus the
retained backend readers; obsolete stub-error assertions were removed, and
the `.gitmodules` `active` handling now asserts the app-only contract
(`active` never leaks into portable fields). `benches/benchmark.rs` measures
real config parse/load/edit at 1/10/100 modules, and
`scripts/measure-performance.py` drives before/after comparisons (alternating
samples, warm-up, Trace2 child-launch counts, direct-Git invocation counts,
no-op byte/mtime/index identity) against immutable bench-profile artifacts.
The performance memory test labels the tracking allocator as Rust allocations
only and restores the process working directory via a drop guard.

Compatibility impact: the `GitOperations` implementations for
`GixOperations`/`Git2Operations` are removed along with
`Config::sync_with_git_config`/`load_with_git_sync` and the backend mutation
methods. The library API is explicitly unstable (see `src/lib.rs`); in-tree
callers were migrated to `GitOpsManager`. No TOML, CLI, or Git-state contract
changes in this phase.

Phase 7 gate evidence (`scripts/measure-performance.py`, `--profile bench`,
10 alternating samples after validation + warm-up, same macOS host,
Git 2.50.1, raw records in `/private/tmp/submod-perf-work/final.jsonl`):

- Baseline: accepted Phase 6 snapshot production code with only the
  measurement harness file overlaid (verified: the build copy differs from
  the snapshot in `benches/benchmark.rs` alone); CLI
  `c12429684c60f5961e95c133ab92c559a08f1302a71bb4272c1300f1ca09d431`
  (3,924,672 bytes), bench
  `8c44e40b5c5f26a277fb9adf5428c2bdf0ca4bfe1d975ee0999b7ec8754f5d1a`.
- Candidate: current tree; CLI
  `12b0281a197ef0a4db75803ae409420c14c7522091a1316c3715d0c6d264010e`
  (3,888,016 bytes, ~36 KB smaller; doc-comment-only source edits after the
  freeze rebuild to this identical hash), bench
  `756abab96fdd5868da69899f92df5fb154b7b81e63bd4818041f11ea12f89348`.

| Workload (median of 10) | Baseline | Candidate |
| --- | --- | --- |
| config parse 1/10/100 (ns/iter) | 3846 / 25549 / 240781 | 3985 / 26123 / 242557 (parity) |
| config load 1/10/100 (ns/iter) | 12785 / 33861 / 248290 | 12502 / 34475 / 249881 (parity) |
| config add-one 1/10/100 (ns/iter) | 185 / 1135 / 9729 (linear) | 38 / 47 / 158 (flat; 4.9x/24x/61x) |
| check absent 1/10/100 (ms wall) | 53.3 / 46.3 / 48.0 | 53.3 / 43.4 / 45.1 (parity) |
| check materialized 1/10 (ms wall) | 441 / 4062 | 437 / 4023 (parity) |
| sync no-op 1/10 (ms wall) | 923 / 9263, state held | 916 / 9182, state held (parity) |

The add-one speedup is the in-place `update_entry` (no whole-map clone):
baseline cost grows with map size, candidate cost is flat. Parse/load and all
CLI paths are within overlapping sample ranges — no measured regression, and
no source-level simplification is described as a speedup beyond the add-one
case above. Instrumented samples show identical direct Git invocation counts
(8/50/101/428/938 across workloads) and identical Trace2 child-launch counts
on both artifacts, byte/mtime/index/porcelain identity held on every
inspection and no-op run, and lower direct-executable max RSS on the
candidate (~10.0–10.5 MB vs ~11.7–12.5 MB, `/usr/bin/time -l` boundary:
direct executable only, not a parent+descendant aggregate). This is
warm-cache local evidence only; no cold-cache or network claim is made.

### Phase 8 — shipping gates

CI runs the required stable suite on Linux, macOS, and Windows plus a
separate informational beta/nightly job, an explicit Rust 1.89 MSRV gate,
locked nextest with the `ci` profile, separate locked doctests, and a new
package job (`cargo package --locked`, build/test of the extracted packaged
source, schema JSON validation, template generation smoke). Coverage
permissions are read-only. Release resolves the tag to its commit and fails
mismatches; a `verify-tag` job (tests, doctests, MSRV, fmt/clippy, deny,
audit, package) gates the build; archives are built, then their exact bytes
undergo linkage inspection and native execution smoke (`--version`, `--help`,
disposable local submodule add/init/check) before upload; `cargo publish`
uses locked clean-source publication; the GitHub release attaches the checked
archives. All third-party actions are pinned to reviewed immutable SHAs (see
workflow headers for the update procedure). Advisory policy is consolidated:
the stale `RUSTSEC-2024-0436` ignore and the now-unreachable
`RUSTSEC-2024-0364` ignore were removed from `audit.toml`,
`.cargo/audit.toml`, `deny.toml`, and CI, so a re-entry fails loudly.
`cargo package` no longer names absent `LICENSE.md`/`submod-sbom.spdx`
assets; the real `LICENSE-*.md` files ship. Stale "TODO: implement" markers
on completed commands and the delete/reclone `change --path` help text were
corrected to the implemented safe-move contract.

Unrun and explicitly unverified: hosted Linux/macOS/Windows job results,
cross-architecture archive execution (aarch64 on x64 runners), live private
authentication, and HTTP schema delivery. No release was published or tagged
as part of this work.

## Regression coverage

Status values are `RED` (proved failing before its fix), `PASS`, `PARTIAL`, or
`PENDING`. Each row represents the family in `IMPROVEMENT_PLAN.md`, not a claim
that every platform variant has run locally.

| ID | Status | Evidence / remaining condition |
| --- | --- | --- |
| R01 | PASS | Empty/root/admin/parent/absolute/escaped/symlinked paths and administrative names refuse before mutation; sentinels and index/config snapshots remain unchanged. |
| R02 | PASS | Occupied file/directory/symlink/unrelated repo, dirty registration, and unavailable remote preserve files, refs, index, config, and metadata. |
| R03 | PASS | Disable and re-enable preserve the exact divergent checkout, local branch/ref, local-only commit, stash, child index/config, object database, and parent state. A later init intentionally converges to the parent pin; the saved commit/ref and stash remain accessible and recover the original files/index. |
| R04 | PASS | Delete removes only the exact gitlink/registration; prefix siblings retain mode/OIDs and held index lock refuses without mutation. |
| R05 | PASS | Held stash-ref lock and ignored file/directory/nested-repo target collisions refuse before reset with all bytes/state preserved; successful stash/reset reaches the parent pin and retained stash recovers staged, unstaged, and untracked work. |
| R06 | PASS | Raw inheriting fields remain absent while explicit module overrides remain; changed global ignore is applied to portable Git metadata and survives a fresh process. |
| R07 | PASS | Omitted/true/false booleans and sparse mode at both scopes round-trip without resetting unrelated fields. |
| R08 | PASS | Sample/template, absent/1.0/1.1/future schema, aliases, legacy values, unknown fields, types, and source-line diagnostics use one validated model. |
| R09 | PASS | Multiline arrays/strings, comments, quoted/literal/dotted/Unicode names, escapes, dotted keys, and unrelated sections survive managed edits without duplicate tables. |
| R10 | PASS | No-op bytes/inode/mtime, atomic failure, absence/empty identity, symlink refusal, deterministic simultaneous writers, linked worktrees, shared configs, stale managers, and relative/absolute generated outputs pass. Arbitrary external-editor race remains documented. |
| R11 | PASS | TOML-only init/sync with omitted path creates the exact registration, mode-160000 gitlink, checkout files, and effective root-relative path; a repeat is byte/state identical. |
| R12 | PASS | Fresh-clone init/sync materializes the recorded old parent pin rather than the newer remote tip, and initialized checkout drift converges with verified content. |
| R13 | PASS | Named/default/`.` branches, detached-parent refusal, checkout/merge/rebase/none strategies, recorded-pin and explicit remote updates, divergent metadata-only HEAD preservation, ancestry postconditions, and repeat no-ops pass with exact OIDs/content/index evidence. |
| R14 | PASS | URL/branch/ignore/update/fetch and inherited defaults reconcile across portable, local, worktree, and selected child config; removed managed keys are unset, relative URLs follow native Git semantics, unrelated keys remain, and a repeat preserves file identity. |
| R15 | PASS | Ordered positive/negative patterns, extra/reordered patterns, mode/enable drift, removal/empty restoration, dirty included/excluded protection, exact content, and unchanged no-op behavior pass. |
| R16 | PASS | Alias/path splits, native nested logical names, alternate formatting, prefix paths, duplicate registrations, exact/case-normalized overlaps all use exact unique matching or refuse before mutation. |
| R17 | PASS | Root, nested cwd, linked-worktree, implicit/default-explicit/relative/absolute custom config, and missing-config policy select the intended repository/config; conflicting selectors fail before action. Check distinguishes config validation from Git/inspection failure and reports portable, local, child, pin, and strategy drift. |
| R18 | PASS | Every supported optional override can be unset, sparse paths can be cleared, boolean tri-state survives fresh processes, and no-settings/set+unset/clear+replace/append/all+names conflicts refuse with usage status before mutation. |
| R19 | PASS | Disabled/update-none entries with unreachable URLs are untouched while inspectable metadata/dirty drift is reported; no-init is transient; unmanaged modules are preserved/reported. Recursive init/sync/update/list traverses actual registrations, marks uninitialized descendants, sorts output, and reports identity/inspection failures. |
| R20 | PASS | Fresh relative-URL materialization resolves against the parent remote and preserves the parent pin; non-main branches and recursive selection pass; shallow available older pins materialize exactly and unavailable pins fail with the requested OID without substituting the tip. |
| R21 | PASS | Missing registration/gitlink and retained-gitdir states recover only when unambiguous, preserving exact pins/history; malformed, nonexistent, redirected, and conflicting child pointers refuse before mutation. |
| R22 | PASS | Config-only, initialized, forced, and legacy embedded-layout deletion preserve their exact boundaries and retained history. Rebuild preserves logical/storage identity, refs/stash, ordered unmanaged config, inactive/none policy, changed selected URLs, declarations/gitlinks, staged layers, and exact pins; failed/partial rebuilds retain usable retry state. Default destructive work conservatively refuses nested gitlinks because nested ignored data cannot be proven safe. |
| R23 | PASS | Clean initialized moves preserve registration, gitlink, checkout/storage identity, history, and unrelated state; dirty, occupied, and conflicting moves refuse exactly; inactive path changes do not clone or fetch. |
| R24 | PASS | Invalid/dirty later targets preflight before earlier reset/nuke mutation; runtime failures report completed/failed/pending and retry converges. A real killed child process, owned partial state, blocked retry, repair, and final convergence retain exact user work. |
| R25 | PASS | Every mutating command previews through shared validation/planning with concrete actions, staging/target/remote context, exact full-tree/index/metadata fingerprints unchanged, stale stat-cache coverage, and zero transfer attempts. Read-only check/list retain the same boundary. |
| R26 | PASS | Results/progress/warnings use the documented streams; named outcomes and actual targets/counts reflect postconditions; control and terminal text is escaped and apparent URL userinfo redacted across native, config, fallback, captured backend, generated path, sparse, and final-error output while useful Unicode/prose remains. |
| R27 | PASS | README, long help, template, sample, schema aliases/version, shell completions including nushell, test runner, and generate/import workflows agree with the canonical options and runtime behavior. |
| R28 | PASS | Real config parse/load/edit at 1/10/100 plus CLI inspection/no-op sync measured before/after on immutable bench-profile artifacts: add-one 4.9x/24x/61x (linear→flat), all other paths at parity, identical Git invocation counts, no-op byte/mtime/index identity held, allocator scope honestly labeled. Raw records `/private/tmp/submod-perf-work/final.jsonl`. |
| R29 | PARTIAL | `cargo +1.89 check --locked --all-features` passes locally (Rust 1.89.0). CI defines required stable Linux/macOS/Windows jobs, an informational beta/nightly job, an explicit MSRV gate, tag→commit resolution with mismatch failure, a verify-tag gate (tests, doctests, MSRV, fmt/clippy, deny, audit, package), build→linkage/smoke→upload ordering with native-execution smoke, locked clean-source `cargo publish`, and immutable action SHAs. Hosted job results, cross-arch archive execution, and any actual publication remain explicitly unrun/unverified; nothing was tagged or published. |
| R30 | PASS | All five schema JSON files parse; sample/template/completion agreement covered by passing R27 tests; fresh `cargo audit` (1243 advisories) reports zero vulnerabilities with one allowed yanked warning (bisync 0.3.0, documented in `deny.toml`); `cargo deny check` passes advisories/bans/licenses/sources with the retired ignores removed; `cargo package --locked --allow-dirty` builds `submod-0.4.0.crate` (23 files, real `LICENSE-*.md` shipped, absent-asset globs removed); the clean-tree `cargo package --locked` plus packaged-source build/test run in CI's package job. |
| R31 | PASS | Add, move, delete, metadata reconciliation, incomplete registration, and mixed structural/metadata batches preserve staged and unstaged `.gitmodules` layers independently or refuse before mutation; unrelated edits are never swept into the index. |

## Final local verification (closing gates)

```text
cargo fmt --all -- --check: PASS
git diff --check: PASS
cargo clippy --locked --offline --all-targets --all-features: PASS (zero errors;
  warn-level pedantic/nursery only; touched files warning-free)
/private/tmp/submod-nextest-bin/cargo-nextest (0.9.128, 96bc6d4f4) nextest run
  --locked --offline --all-features --profile ci --no-fail-fast
  exit 0: 631/631 passed, 0 skipped, 874.418s
  log: /private/tmp/submod-final-retained.log
cargo test --locked --offline --all-features --doc: PASS (0 tests by design)
cargo test --locked --offline --all-features --lib: PASS (185/185)
cargo +1.89 check --locked --offline --all-features: PASS (MSRV)
cargo audit (fresh DB, 1243 advisories): 0 vulnerabilities, 1 allowed yanked
  warning (bisync 0.3.0)
cargo deny --all-features check: advisories/bans/licenses/sources ok
cargo package --locked --allow-dirty: builds submod-0.4.0.crate (local tree is
  uncommitted by design; clean-tree packaging runs in CI)
```

Test-selected debug CLI SHA-256 before and after the gate (no concurrent
builds during the run):
`a911ca4d9c5c626bfcf3a3487273bd0b2e2889b122498f0261bd3b078037acde`
(identical — no mixed-artifact replacement). Post-gate edits are limited to
this report, one comment word (`behaviour` → `behavior`), and the
`unparsable` spelling in the measurement script; no production code changed
after the gate.

## Review follow-up (adversarial pass)

Delegated agent review was requested but the runner failed every spawn
(4/4 infrastructure failures, no reviewer ever started), so a structured
self-review was performed instead — weaker independence, disclosed here.
Findings, all fixed and re-verified (fmt/clippy clean; fallback 30/30,
git_ops 73/73, lib 185/185 after the fixes):

- Stale strategy docs (minor): module/trait/struct docs still described the
  retired gix-first/git2-fallback mutation design and a "CLI fallback" update
  path. Rewritten to the native-mutation boundary.
- `reopen` silently upgraded `without_gix` managers back to gix (minor,
  real): `reopen` now preserves the backend policy and documents it; new test
  `reopen_preserves_without_gix_policy` was proven to FAIL on the old logic
  and pass on the new.
- Dead panicking API (minor): `From<GitOpsManager> for GixOperations` had
  zero callers and panicked on gix-less managers. Removed; the infallible
  git2 conversion stays.
- `verify-tag` release job could never pass (major, static): it invoked
  `cargo +1.89`, `cargo deny`, and `cargo audit` with only the stable
  toolchain and nextest installed. Added the 1.89 toolchain and the
  deny/audit install step (binstall fallback covers resolution).
- Misleading test names (minor): six fallback test names and one git_ops
  name still described the retired mutation-fallback design. Renamed to the
  native contract they actually assert.
- Checked and cleared: validation/preflight coverage on all native mutations
  (`child_git` validates internally), no whole-map clones in hot paths (the
  one `add`-time prospective clone is once per command, confirmed flat by
  measurement), no stub remnants, `without_gix`/`gix_enabled` docs accurate,
  install-action binstall fallback covers the deny/audit install step.

Count reconciliation against the accepted Phase 6 gate (834): the exact
test-set diff shows 207 removals and 4 additions (631 = 834 − 207 + 4).
Removals: 186 duplicated lib unit tests in the binary target (Phase 7 item 1;
every one retains its lib counterpart — spot-checked
`config::tests::test_config_toml_roundtrip`,
`options::tests::test_branch_set_branch_none_is_not_defaulted`, and
`utilities::tests::repository_context_nested_and_linked_worktrees` passing
under `--lib`), 1 `simple_gix` unit test with its deleted module, and 20
obsolete backend-mutation/stub assertions (3 fallback, 17 git_ops) replaced
by 4 retargeted native-contract tests. No unique coverage was lost.

Completion checklist: safe preservation tests pass; root/nested/fresh-clone
onboarding works; TOML edits persist with inheritance intact; Git
configuration and checkout state converge; disabled and sparse removal
behavior is correct; repeated sync is a no-op; output/exit codes describe
reality; regression tests check real Git state; package/security gates pass
locally; docs describe the implemented contracts. Hosted platform/release
execution is defined but unrun (R29 PARTIAL); everything runnable locally is
green.

## CI follow-up (Git 2.51+ submodule sync remote selection)

Hosted ubuntu CI (Git 2.55.0) failed
`r22_nuke_changed_relative_url_fetches_missing_pin_from_selected_remote`
with "Git did not synchronize the selected child URL", while the same suite
was green locally (Git 2.50.1). Root cause: Git 2.51 changed native
`git submodule sync` to look the child remote up by URL first
(`repo_remote_from_url`) instead of using the branch-default/`origin`
remote, so sync maintains `remote.selected.url` and never creates
`remote.origin.url`; `expected_synced_urls` still expected the old remote.
Reproduced authentically in an ubuntu container with Git 2.55.0 (byte-identical
failure), then fixed `expected_synced_urls` to mirror native selection —
URL-matching remote first, branch-default/`origin` fallback — in both the
relative and absolute URL branches. The existing r22 test is the relative-URL
regression test (proven red pre-fix, green post-fix on Git 2.55.0); new test
`r22_nuke_absolute_url_fetches_missing_pin_from_selected_remote` covers the
absolute-URL branch the same way. Verified green on Git 2.43.0, 2.50.1, and
2.55.0, full suite 633/633 locally, fmt clean, no new clippy warnings. The
sync-mismatch error message now reports expected vs actual values.

## Known environment limits

- Hosted Linux, Windows, and release authorization jobs cannot be asserted from
  this macOS worktree; workflow definitions and all locally available checks
  were validated, and unrun hosted results remain explicitly unverified.
- Live private-remote authentication is outside the deterministic regression
  suite. HTTP schema delivery was not verified.
- No release was published or tagged as part of this work.
