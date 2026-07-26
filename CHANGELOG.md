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
## [0.4.0] - 2026-07-26
### Bug Fixes

- Gix update causing incompatibility with gix modules([`f8164a0`](https://github.com/bashandbone/submod/commit/f8164a0f28ab0e4bcb9742b67fdd2e8967cf2364))

- (**config**) Round-trip the fetchRecurse config key correctly (#63)([`441f41b`](https://github.com/bashandbone/submod/commit/441f41bacce25921b3acf9c6fef0e3e5c29ba782))

- (**update**) Checkout submodule to recorded gitlink commit (#62 P1) (#67)([`dcf20fd`](https://github.com/bashandbone/submod/commit/dcf20fd01106bac6f5d79a2cd762660f7d94b2de))

- (**check**) Report submodules with uncommitted changes as dirty (#62 P1) (#68)([`0440da4`](https://github.com/bashandbone/submod/commit/0440da45edf0d0b94b302a294e9785473cb21412))

- (**config**) Round-trip submod.toml — preserve [defaults] on load, persist edits on save (#62 P1) (#70)([`af455ae`](https://github.com/bashandbone/submod/commit/af455ae2939d1bc07a51935b45f2c7c68406fa9c))

- (**config**) Merge_from carries other.use_git_default_sparse_checkout (#62 P2) (#72)([`786b0c3`](https://github.com/bashandbone/submod/commit/786b0c3644bd39cea3a24876690c4abc646c8484))

- (**git_ops**) Clean up path-keyed .gitmodules after a failed add + idempotency tests (#62 P2) (#75)([`598747b`](https://github.com/bashandbone/submod/commit/598747b6cda68a9ad9ee2f3919df6a565d39d7c5))

- Failing tests and lints([`1c22370`](https://github.com/bashandbone/submod/commit/1c2237080ae87182410658336a8b24bbc6d796c7))

- Lint tests and fix formatting in a few modules([`e1d8569`](https://github.com/bashandbone/submod/commit/e1d85692d6c2450236f05a9b88e83827a4b6f15f))

- (**deps**) Update gix modules to address a transient vulnerability([`fb4d5ac`](https://github.com/bashandbone/submod/commit/fb4d5ace90ce504c82443485fa291e3c0c233d9f))

- CI failing on missing checkout action([`7f5448c`](https://github.com/bashandbone/submod/commit/7f5448cb63f029018ea0e9673472a6e0c1366bcd))

- (**ci**) Repair the lint job, make the test job actually run tests (#76)([`3f25280`](https://github.com/bashandbone/submod/commit/3f25280786d1c4fa539677d102b5f2a8a13d21fd))

- (**update**) Keep gitoxide's fetch report off stdout, name each submodule (#78)([`9bac978`](https://github.com/bashandbone/submod/commit/9bac978c3fdd56702705007c2ac52719e69ed775))

### CI/Build

- (**ci.yml**) Fix tool install in ci action([`d54e521`](https://github.com/bashandbone/submod/commit/d54e521aaa021475c0a621f38e17bf33bf16e795))

- (**release**) Measure static linkage on dry runs instead of inferring it (#80)([`f751eea`](https://github.com/bashandbone/submod/commit/f751eeafc8d4b5d2097510de49f8c64fc4b4d337))

- Run all four cargo-deny checks, not just advisories (#82)([`debc885`](https://github.com/bashandbone/submod/commit/debc885cf798405b0fec962ccd5c453152ebd3e2))

### Features

- (**submodule**) Disable command updates .gitmodules active status (#50)([`5c2c8c5`](https://github.com/bashandbone/submod/commit/5c2c8c55949cef60786fe46696a137fd5b5bbfc3))

### Miscellaneous

- (**deps**) Bump rustls-webpki in the cargo group across 1 directory (#58)([`5c754c3`](https://github.com/bashandbone/submod/commit/5c754c392f0d55029ac764857f708c29b326bf69))

- (**deps**) Bump gix from 0.81.0 to 0.83.0 in the cargo group across 1 directory (#59)([`4e0419b`](https://github.com/bashandbone/submod/commit/4e0419b119d61f43dc9240d60c7b742787926e70))

- (**cleanup**) Remove old files, add license.toml for licet (#61)([`efb606e`](https://github.com/bashandbone/submod/commit/efb606e512ff6c39448896112fe8640a544547ab))

- Remove unused import in tests/security_tests.rs([`a59068f`](https://github.com/bashandbone/submod/commit/a59068f491ec68b77187e0514b7e2c0782512859))

- Consolidate on one TLS stack, ship static musl binaries (#79)([`1256c3e`](https://github.com/bashandbone/submod/commit/1256c3eae4b6e065af5d4a6e043d969ec6af1f26))

- (**deps**) Update deps and deny.toml([`135353e`](https://github.com/bashandbone/submod/commit/135353efbf4b93585623356658a0ba6999c219a4))

### Refactoring

- (**tests**) Make sparse checkout unit tests hermetic and complete (#52)([`a50023b`](https://github.com/bashandbone/submod/commit/a50023b2527c0fc284711ffac958fdcd851791ae))

- Complete p3/p4 of test improvement plan([`ffe76d8`](https://github.com/bashandbone/submod/commit/ffe76d8c98c59a9cdbbdff5f86252aac4b88a181))

### Testing

- (**integration**) Assert real git state for add/delete/nuke (#62 P0-2) (#64)([`c033a80`](https://github.com/bashandbone/submod/commit/c033a800b7321eb4d0b097fa8f9a8d20236e8f7d))

- (**fallback**) Git2 failure-injection seam for fallback architecture (#62 P0-1) (#65)([`31b72ea`](https://github.com/bashandbone/submod/commit/31b72ea5337eac1ac3254f764f73cf8c9c58e613))

- (**fallback**) Exercise CLI last-resort add + assert fallback warnings (#62 P0-1) (#66)([`d32c9eb`](https://github.com/bashandbone/submod/commit/d32c9eb343a90fe714e9284378f191721da9790f))

- (**config/git_ops**) Cover git2 option bridges + get_submodule_status OIDs/flags (#62 P1) (#71)([`f17a043`](https://github.com/bashandbone/submod/commit/f17a043a5519009d2d8a90040eb41ae317f32ae0))

- (**p2**) Make cannot-fail error tests meaningful + un-fake null-byte (#62 P2) (#73)([`fda1880`](https://github.com/bashandbone/submod/commit/fda1880188797e4c5d0399009d7f26b6ef3d26bc))

- (**p2**) Command/flag-injection containment for name/url/.gitmodules (#62 P2) (#74)([`50bd136`](https://github.com/bashandbone/submod/commit/50bd136a6564f647c945c74f0338888f927299a9))

- (**p2**) Pin loose error/output assertions to exact strings (#62 P2) (#77)([`ed6fd35`](https://github.com/bashandbone/submod/commit/ed6fd3526a10f5642841db1fa571958b815b0789))

### Deps

- Bump git2 to 0.21, clearing RUSTSEC-2026-0184 (#81)([`45de49a`](https://github.com/bashandbone/submod/commit/45de49a9877edf20d8fe504ee0a60eb0ba8dd238))

### Style

- Format code to spec([`e66b258`](https://github.com/bashandbone/submod/commit/e66b2587f7ed6deecae22405b928375a9aad2406))

- Lint and format([`15cd804`](https://github.com/bashandbone/submod/commit/15cd8048525aab087432789100d519219d4dc9ed))


## [0.3.0] - 2026-04-08
### Bug Fixes

- (**ci**) Resolve security audit job failure (#44)([`eed6371`](https://github.com/bashandbone/submod/commit/eed63718718fd003623f44c601a89d0b7dca7b08))

- Formatting and linting fixes([`eac9410`](https://github.com/bashandbone/submod/commit/eac94101785396a481de35919901aab9da2c4c77))

- Update schema version to 1.1.0 and improve code formatting([`43d9722`](https://github.com/bashandbone/submod/commit/43d97228c61c72e1745f756aebdb580835457983))

- Remove invalid `#[serde(flatten)]` from branch deserialization test (#49)([`6791c3c`](https://github.com/bashandbone/submod/commit/6791c3c2fff842ae95905b0d573f5181e07186ce))

### Features

- Sparse checkout deny-all-by-default (modified cone pattern) with opt-out support (#48)([`216c28e`](https://github.com/bashandbone/submod/commit/216c28e283b697b29fa4cc903e930fec3586dc06))

### Miscellaneous

- (**schemas**) Update config schemas to reflect new sparse path handling and the `use_git_default_sparse_checkout` setting([`d0d6022`](https://github.com/bashandbone/submod/commit/d0d6022f51605fa3b605b56b85b3e6cbf9c28a05))

- Update Cargo.toml to v0.3.0([`16bdfbe`](https://github.com/bashandbone/submod/commit/16bdfbea29d70ca1f9987421a770a0afca585b43))

- Update CHANGELOG.md for v0.3.0([`14f37cc`](https://github.com/bashandbone/submod/commit/14f37ccf405d0673e15ff2b2992f24e0e559d06f))


## [0.2.7] - 2026-03-22
### Bug Fixes

- Propagate fetch errors from gix to enable git2 fallback([`3162a2c`](https://github.com/bashandbone/submod/commit/3162a2cc02577d0cfd04f5c23d5b12f663fe9cbf))

### Miscellaneous

- (**deps**) Bump rustls-webpki in the cargo group across 1 directory (#41)([`8ddb5cd`](https://github.com/bashandbone/submod/commit/8ddb5cd6b8b8f0e25a859215212a1053b3064137))

### Performance

- Hoist format! call out of loop in src/git_ops/mod.rs (#43)([`f1994e2`](https://github.com/bashandbone/submod/commit/f1994e2ff8285f0f32ce7d6667c7003ff8aebb6a))


## [0.2.6] - 2026-03-22
### Bug Fixes

- Missing https transport causes fetch to fail with gix([`cbd01d2`](https://github.com/bashandbone/submod/commit/cbd01d23fe111a3aa743e2bf1d4ebfcc17de3afb))


## [0.2.4] - 2026-03-22
### Bug Fixes

- Fix issue with https not compiled into gix([`9e5b20d`](https://github.com/bashandbone/submod/commit/9e5b20df2cf09be2f59ac67d579ede5e6d9c8a25))

- Update dependencies for http transport([`5918721`](https://github.com/bashandbone/submod/commit/5918721b23abf807f9ef05f8b344a9f05ffafd2e))

- Update dependencies for http transport([`a868953`](https://github.com/bashandbone/submod/commit/a86895313bf25983001d824ce5dce9a9b46157e2))

- Update dependencies for http transport([`ae5d6cd`](https://github.com/bashandbone/submod/commit/ae5d6cd1ca99d6a9153b07f0880bdfa3fd5bb501))


## [0.2.3] - 2026-03-20
### Fix

- Gix submodule resolution bug, verbose output (added --verbose flag)([`17a4877`](https://github.com/bashandbone/submod/commit/17a4877a17ada5aaf51a86ba898ef6c85ba10cae))

### Miscellaneous

- Bump to v0.2.3([`bec2d46`](https://github.com/bashandbone/submod/commit/bec2d46cdd1d071ba682743a9ef63be3b3e71f28))

- Update changelog([`fd3c0bb`](https://github.com/bashandbone/submod/commit/fd3c0bb07d15bc2bf8f137feb5be94eb36c00c09))


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
