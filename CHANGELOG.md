<!--
SPDX-FileCopyrightText: 2026 Adam Poulemanos <89049923+bashandbone@users.noreply.github.com>
SPDX-License-Identifier: LicenseRef-PlainMIT OR MIT

Git-cliff generates this file from the git commit history. The configuration for how it generates this file is in `cliff.toml`. Please edit that file, not this one.

commit hashes cause false-positives for the spellchecker:
spellchecker:off
-->
# Changelog

We document all important changes below.

Submod follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) and uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.3] - 2026-03-20
### Fix

- Gix submodule resolution bug, verbose output (added --verbose flag)([`17a4877`](https://github.com/bashandbone/submod/commit/17a4877a17ada5aaf51a86ba898ef6c85ba10cae))

### Miscellaneous

- Bump to v0.2.3([`bec2d46`](https://github.com/bashandbone/submod/commit/bec2d46cdd1d071ba682743a9ef63be3b3e71f28))


## [0.2.2] - 2026-03-20
### Bug Fixes

- Correct name to cargo-nextest in ci.yml([`e9251c6`](https://github.com/bashandbone/submod/commit/e9251c6a4e089c23039847ace7268e6a1f4ca8fa))

- Incorrect key in ci.yml GH Action([`2d4d09e`](https://github.com/bashandbone/submod/commit/2d4d09e9050d082d2f63851b3ea7dd1f31feda11))

- (**release**) Repair release.yml — YAML syntax errors, wrong action versions, broken permissions and tokens (#40)([`d0f764a`](https://github.com/bashandbone/submod/commit/d0f764a57b6711fec83c551139c918dbc5c32a6e))

### Feat

- (**testing**) Add coverage macro attributes and streamline testing config for faster testing (#35)([`38da3a9`](https://github.com/bashandbone/submod/commit/38da3a974c7cd1af6fe93573daf4f095f8ae6479))

### Features

- Add config schema, schema URL to toml([`f5f8156`](https://github.com/bashandbone/submod/commit/f5f815624ce95d1082bb80859a016b21168ebb08))

- Add schema; delete old CLAUDE.md for regeneration.([`daa193e`](https://github.com/bashandbone/submod/commit/daa193e64f01b76c995b37ddafc18b61fedfd8e7))

- (**testing**) Add coverage macros across codebase where appropriate; Add/update nextest config to use test groups to prevent race conditions vice running all tests on a single thread serially.([`b6f09f2`](https://github.com/bashandbone/submod/commit/b6f09f2a24361278153ef12ffce12d9876f284d6))

- Add changelog configuration and update commit parsing rules in cliff.toml([`9b4e72d`](https://github.com/bashandbone/submod/commit/9b4e72dbea2824f091afbfaa0b6885a6b99bd07b))

### Fix

- Serialization/Deserialization bug; significantly expand testing in core areas. (#33)([`bce7bf8`](https://github.com/bashandbone/submod/commit/bce7bf850793df5d3e9392ed639dc47de26a8d8f))

### Miscellaneous

- Cleanup old/unused files([`a761af9`](https://github.com/bashandbone/submod/commit/a761af975441bce5a7a97fdb09659ceb996d76ff))

- Update CI workflow for coverage and install nextest; modify dependencies in Cargo.toml and Cargo.lock([`5fdfe24`](https://github.com/bashandbone/submod/commit/5fdfe249b8f6fd2e28992c17498a5e0349dd430f))

- Update Rust version to 1.89 in Cargo.toml and mise.toml([`b7739c1`](https://github.com/bashandbone/submod/commit/b7739c11f911cd182988d1f3967ecd58789a0542))

- Bump version to 0.2.2 in Cargo.toml and Cargo.lock([`f1bb973`](https://github.com/bashandbone/submod/commit/f1bb973c669614ca861df0ffb750d4250ed00974))

- Bump version to 0.2.2 in Cargo.toml and Cargo.lock([`a4f34ad`](https://github.com/bashandbone/submod/commit/a4f34ad9ed532264769472ab37334274bb43a43d))


## [0.2.1] - 2026-03-18
### Bug Fixes

- (**git_manager**) Improve success message for submodule updates([`3030474`](https://github.com/bashandbone/submod/commit/3030474c9fa4b4fdae42d23c7a2a1966a974bd53))

### CI/Build

- Ignore RUSTSEC-2024-0364 in cargo audit([`ee635aa`](https://github.com/bashandbone/submod/commit/ee635aa617b1138e77a0ff9b41466b1da2b18c02))

### Features

- Wire up no-init and shallow options for add command([`e315f76`](https://github.com/bashandbone/submod/commit/e315f7650e95f96bf2f30c7f18eebf45ccc06b9e))

### Fix

- Implement robust check for uncommitted changes using git2 fallback([`1aab1ed`](https://github.com/bashandbone/submod/commit/1aab1ed25c8e272d60a83bf1deab9036d104dc18))

### Miscellaneous

- (**docs**) Update README.md for v0.2.0 release([`9f6598f`](https://github.com/bashandbone/submod/commit/9f6598f45609957508ebb55a6c66926b0fc1ad4d))

- Fix cargo audit failure by ignoring RUSTSEC-2024-0364([`1cdb19a`](https://github.com/bashandbone/submod/commit/1cdb19aac55134c0841472dd0efefd122a082a7a))

- Update changelog for v2.1.1([`2d78b5d`](https://github.com/bashandbone/submod/commit/2d78b5d04898c2f56d32479f3d2d69cd3a51ca0f))

- Update Cargo.toml to 0.2.1([`c8ebfde`](https://github.com/bashandbone/submod/commit/c8ebfde3df328b7bcb0868084fdc3a0825ccd6c7))

### Performance

- Optimize line_key prefix checking([`bd36094`](https://github.com/bashandbone/submod/commit/bd3609448bdedc703c57b653e8b8c787b48c3a99))

- Optimize line_key prefix checking([`64a6396`](https://github.com/bashandbone/submod/commit/64a639635e6bbf26572e590047fe4567a5aef25f))

- Optimize line_key prefix checking([`88322b3`](https://github.com/bashandbone/submod/commit/88322b3369c07454c1c4d46ebff26b6985e841c4))

- Avoid Vec cloning when updating sparse paths([`211d963`](https://github.com/bashandbone/submod/commit/211d963dfb338ed0b280050d296a5c12a33547b2))

### Refactor

- Use gix is_dirty() for uncommitted changes check instead of git2([`b02bd3b`](https://github.com/bashandbone/submod/commit/b02bd3b2ace72821fe70f09fa21da312905a5e94))

### Testing

- Fix temporary value dropped while borrowed([`a50e1e3`](https://github.com/bashandbone/submod/commit/a50e1e31211b2e0dea66d247f738f9f15c026a43))

- Add missing tests for GitmodulesConvert on SerializableIgnore([`6bab038`](https://github.com/bashandbone/submod/commit/6bab03857d07794f66b66f862c1fad7e2f40f58e))

- Add missing tests for GitmodulesConvert on SerializableIgnore([`92b4598`](https://github.com/bashandbone/submod/commit/92b45980aabb3a878e567e9e1aa7bb7fcbf5bd0d))

- Add missing tests for GitmodulesConvert on SerializableIgnore([`8baf097`](https://github.com/bashandbone/submod/commit/8baf09777379cb97d8d550cd18ba35c8465bbb1d))

- Add missing tests for GitmodulesConvert on SerializableIgnore([`eea22cf`](https://github.com/bashandbone/submod/commit/eea22cf928f5606e139640a00fb1866fd255e285))

- Add tests for name_from_url([`1087738`](https://github.com/bashandbone/submod/commit/10877383ed2589a9b2107edc6e033450c105c0d7))

- Add tests for name_from_url([`c3c15a9`](https://github.com/bashandbone/submod/commit/c3c15a9c3ea5c43b1d7ef4c8bdf3ba112dbbcdbc))

- Add unit tests for Shell::from_path in src/shells.rs([`68e3cf6`](https://github.com/bashandbone/submod/commit/68e3cf62cd6ea6ba8a6007ea707fd33eaf0f524b))


## [0.2.0] - 2026-03-05
### Bug Fixes

- (**lints**) Fixed a series of lint warnings preventing release, and removed quite a bit of dead code in the process.([`dba5b8a`](https://github.com/bashandbone/submod/commit/dba5b8a551d9ae5b7207287db4f76bf15ce1bdaa))

- (**release**) Reuse compliance, cargo inclusions.([`cb71c01`](https://github.com/bashandbone/submod/commit/cb71c0119e24879868ac6a572ff36100469fe852))

- (**release**) Add sample config to release([`101f27a`](https://github.com/bashandbone/submod/commit/101f27af583a5297ec8266765e5709f2c020445d))


## [0.1.2] - 2025-06-23
### Documentation

- Update README and CONTRIBUTING to reflect hk and mise workflow([`2f841ae`](https://github.com/bashandbone/submod/commit/2f841ae16f1c25e5623bd5fff38649feb0e55e76))


<!-- spellchecker:on -->
