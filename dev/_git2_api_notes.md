# git2 API notes

## git2.SubmoduleStatus returns submodule status

```rust
bitflags! {
    /// Return codes for submodule status.
    ///
    /// A combination of these flags will be returned to describe the status of a
    /// submodule.  Depending on the "ignore" property of the submodule, some of
    /// the flags may never be returned because they indicate changes that are
    /// supposed to be ignored.
    ///
    /// Submodule info is contained in 4 places: the HEAD tree, the index, config
    /// files (both .git/config and .gitmodules), and the working directory.  Any
    /// or all of those places might be missing information about the submodule
    /// depending on what state the repo is in.  We consider all four places to
    /// build the combination of status flags.
    ///
    /// There are four values that are not really status, but give basic info
    /// about what sources of submodule data are available.  These will be
    /// returned even if ignore is set to "ALL".
    ///
    /// * IN_HEAD   - superproject head contains submodule
    /// * IN_INDEX  - superproject index contains submodule
    /// * IN_CONFIG - superproject gitmodules has submodule
    /// * IN_WD     - superproject workdir has submodule
    ///
    /// The following values will be returned so long as ignore is not "ALL".
    ///
    /// * INDEX_ADDED       - in index, not in head
    /// * INDEX_DELETED     - in head, not in index
    /// * INDEX_MODIFIED    - index and head don't match
    /// * WD_UNINITIALIZED  - workdir contains empty directory
    /// * WD_ADDED          - in workdir, not index
    /// * WD_DELETED        - in index, not workdir
    /// * WD_MODIFIED       - index and workdir head don't match
    ///
    /// The following can only be returned if ignore is "NONE" or "UNTRACKED".
    ///
    /// * WD_INDEX_MODIFIED - submodule workdir index is dirty
    /// * WD_WD_MODIFIED    - submodule workdir has modified files
    ///
    /// Lastly, the following will only be returned for ignore "NONE".
    ///
    /// * WD_UNTRACKED      - workdir contains untracked files
    #[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
    pub struct SubmoduleStatus: u32 {
        #[allow(missing_docs)]
        const IN_HEAD = raw::GIT_SUBMODULE_STATUS_IN_HEAD as u32;
        #[allow(missing_docs)]
        const IN_INDEX = raw::GIT_SUBMODULE_STATUS_IN_INDEX as u32;
        #[allow(missing_docs)]
        const IN_CONFIG = raw::GIT_SUBMODULE_STATUS_IN_CONFIG as u32;
        #[allow(missing_docs)]
        const IN_WD = raw::GIT_SUBMODULE_STATUS_IN_WD as u32;
        #[allow(missing_docs)]
        const INDEX_ADDED = raw::GIT_SUBMODULE_STATUS_INDEX_ADDED as u32;
        #[allow(missing_docs)]
        const INDEX_DELETED = raw::GIT_SUBMODULE_STATUS_INDEX_DELETED as u32;
        #[allow(missing_docs)]
        const INDEX_MODIFIED = raw::GIT_SUBMODULE_STATUS_INDEX_MODIFIED as u32;
        #[allow(missing_docs)]
        const WD_UNINITIALIZED =
                raw::GIT_SUBMODULE_STATUS_WD_UNINITIALIZED as u32;
        #[allow(missing_docs)]
        const WD_ADDED = raw::GIT_SUBMODULE_STATUS_WD_ADDED as u32;
        #[allow(missing_docs)]
        const WD_DELETED = raw::GIT_SUBMODULE_STATUS_WD_DELETED as u32;
        #[allow(missing_docs)]
        const WD_MODIFIED = raw::GIT_SUBMODULE_STATUS_WD_MODIFIED as u32;
        #[allow(missing_docs)]
        const WD_INDEX_MODIFIED =
                raw::GIT_SUBMODULE_STATUS_WD_INDEX_MODIFIED as u32;
        #[allow(missing_docs)]
        const WD_WD_MODIFIED = raw::GIT_SUBMODULE_STATUS_WD_WD_MODIFIED as u32;
        #[allow(missing_docs)]
        const WD_UNTRACKED = raw::GIT_SUBMODULE_STATUS_WD_UNTRACKED as u32;
    }
}
```

## git2.ConfigLevel defines where the config is

```rust
impl ConfigLevel {
    /// Converts a raw configuration level to a ConfigLevel
    pub fn from_raw(raw: raw::git_config_level_t) -> ConfigLevel {
        match raw {
            raw::GIT_CONFIG_LEVEL_PROGRAMDATA => ConfigLevel::ProgramData,
            raw::GIT_CONFIG_LEVEL_SYSTEM => ConfigLevel::System,
            raw::GIT_CONFIG_LEVEL_XDG => ConfigLevel::XDG,
            raw::GIT_CONFIG_LEVEL_GLOBAL => ConfigLevel::Global,
            raw::GIT_CONFIG_LEVEL_LOCAL => ConfigLevel::Local,
            raw::GIT_CONFIG_LEVEL_WORKTREE => ConfigLevel::Worktree,
            raw::GIT_CONFIG_LEVEL_APP => ConfigLevel::App,
            raw::GIT_CONFIG_HIGHEST_LEVEL => ConfigLevel::Highest,
            n => panic!("unknown config level: {}", n),
        }
    }
}
```

## Default FetchOptions

Can use with `submodule.fetch()`

````rust
impl<'cb> Default for FetchOptions<'cb> {
fn default() -> Self {
    Self::new()
}

impl<'cb> FetchOptions<'cb> {
/// Creates a new blank set of fetch options
pub fn new() -> FetchOptions<'cb> {
    FetchOptions {
        callbacks: None,
        proxy: None,
        prune: FetchPrune::Unspecified,
        update_flags: RemoteUpdateFlags::UPDATE_FETCHHEAD,
        download_tags: AutotagOption::Unspecified,
        follow_redirects: RemoteRedirect::Initial,
        custom_headers: Vec::new(),
        custom_headers_ptrs: Vec::new(),
        depth: 0, // Not limited depth
        }
    }
  }
}
```rust

### Submodule API Notes

- git2 `Submodule` is the main interface for submodule management
- like with `gitoxide`, the `Submodule` struct is basically a `Repository` with submodule-specific methods
- git2 `Submodule.clone()` creates a new submodule
- git2 `Submodule.branch()` returns the branch of a submodule
- git2 `Submodule.branch_bytes()` returns the branch of a submodule as bytes [u8]
- git2 `Submodule.url()` returns the URL of a submodule

### Config API Notes
- git2 `Config` is the main interface for reading and writing configuration
- git2 `Config.entries()` [also for_each and next methods] returns all config entries as an iterator
- git2 `Config.open_global()` opens the global config
- git2 `Config.open_level()` opens the config at a specific level (see configlevel)
- git2 `Config.set_bool()`, `Config.set_i32()`, `Config.set_i64()`, `Config.set_str()`, `Config.set_multivar()` set config entries for the highest level config (usually local)
- there are corresponding `Config.parse_type()` (like `Config.parse_bool()`) methods to parse config entries
- each entry is a `ConfigEntry` which has a name() and value() method
- git2 `Config.snapshot()` creates a snapshot of the config, used to create a view of the config at a specific point in time, particularly for complex values like submodules and remotes
````
