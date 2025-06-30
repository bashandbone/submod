# Gix API Notes

gix::File is the main high level interface for interacting with gix configuration files across the gix crates.

gix::File can be constructed from a gix::State object, which can itself be constructed from a Path (to a config file). There are many other ways to get a File object, including from a byte slice.

The State object is the primary means of interacting with configuration data and is us. A File object can be derefed directly into a State object, allowing for easy manipulation of the underlying configuration data.

## Gix::State

The full API:

Struct State
pub struct State { /* private fields */ }
An in-memory cache of a fully parsed git index file.

As opposed to a snapshot, it’s meant to be altered and eventually be written back to disk or converted into a tree. We treat index and its state synonymous.

§A note on safety
An index (i.e. State) created by hand is not guaranteed to have valid entry paths as they are entirely controlled by the caller, without applying any level of validation.

This means that before using these paths to recreate files on disk, they must be validated.

It’s notable that it’s possible to manufacture tree objects which contain names like .git/hooks/pre-commit which then will look like .git/hooks/pre-commit in the index, which doesn’t care that the name came from a single tree instead of from trees named .git, hooks and a blob named pre-commit. The effect is still the same - an invalid path is presented in the index and its consumer must validate each path component before usage.

It’s recommended to do that using gix_worktree::Stack which has it built-in if it’s created for_checkout(). Alternatively one can validate component names with gix_validate::path::component().

Implementations
Source
impl State
General information and entries

Source
pub fn version(&self) -> Version
Return the version used to store this state’s information on disk.

Source
pub fn timestamp(&self) -> FileTime
Returns time at which the state was created, indicating its freshness compared to other files on disk.

Source
pub fn set_timestamp(&mut self, timestamp: FileTime)
Updates the timestamp of this state, indicating its freshness compared to other files on disk.

Be careful about using this as setting a timestamp without correctly updating the index will cause (file system) race conditions see racy-git.txt in the git documentation for more details.

Source
pub fn object_hash(&self) -> Kind
Return the kind of hashes used in this instance.

Source
pub fn entries(&self) -> &[Entry]
Return our entries

Source
pub fn path_backing(&self) -> &PathStorageRef
Return our path backing, the place which keeps all paths one after another, with entries storing only the range to access them.

Source
pub fn entries_with_paths_by_filter_map<'a, T>(
    &'a self,
    filter_map: impl FnMut(&'a BStr, &Entry) -> Option<T> + 'a,
) -> impl Iterator<Item = (&'a BStr, T)> + 'a
Runs filter_map on all entries, returning an iterator over all paths along with the result of filter_map.

Source
pub fn entries_mut_with_paths_in<'state, 'backing>(
    &'state mut self,
    backing: &'backing PathStorageRef,
) -> impl Iterator<Item = (&'state mut Entry, &'backing BStr)>
Return mutable entries along with their path, as obtained from backing.

Source
pub fn entry_index_by_path_and_stage(
    &self,
    path: &BStr,
    stage: Stage,
) -> Option<usize>
Find the entry index in entries() matching the given repository-relative path and stage, or None.

Use the index for accessing multiple stages if they exists, but at least the single matching entry.

Source
pub fn prepare_icase_backing(&self) -> AccelerateLookup<'_>
Return a data structure to help with case-insensitive lookups.

It’s required perform any case-insensitive lookup. TODO: needs multi-threaded insertion, raw-table to have multiple locks depending on bucket.

Source
pub fn entry_by_path_icase<'a>(
    &'a self,
    path: &BStr,
    ignore_case: bool,
    lookup: &AccelerateLookup<'a>,
) -> Option<&'a Entry>
Return the entry at path that is at the lowest available stage, using lookup for acceleration. It must have been created from this instance, and was ideally kept up-to-date with it.

If ignore_case is true, a case-insensitive (ASCII-folding only) search will be performed.

Source
pub fn entry_closest_to_directory_icase<'a>(
    &'a self,
    directory: &BStr,
    ignore_case: bool,
    lookup: &AccelerateLookup<'a>,
) -> Option<&'a Entry>
Return the entry (at any stage) that is inside of directory, or None, using lookup for acceleration. Note that submodules are not detected as directories and the user should make another call to entry_by_path_icase() to check for this possibility. Doing so might also reveal a sparse directory.

If ignore_case is set

Source
pub fn entry_closest_to_directory(&self, directory: &BStr) -> Option<&Entry>
Return the entry (at any stage) that is inside of directory, or None. Note that submodules are not detected as directories and the user should make another call to entry_by_path_icase() to check for this possibility. Doing so might also reveal a sparse directory.

Note that this is a case-sensitive search.

Source
pub fn entry_index_by_path_and_stage_bounded(
    &self,
    path: &BStr,
    stage: Stage,
    upper_bound: usize,
) -> Option<usize>
Find the entry index in entries()[..upper_bound] matching the given repository-relative path and stage, or None.

Use the index for accessing multiple stages if they exists, but at least the single matching entry.

Panics
If upper_bound is out of bounds of our entries array.

Source
pub fn entry_by_path_and_stage(
    &self,
    path: &BStr,
    stage: Stage,
) -> Option<&Entry>
Like entry_index_by_path_and_stage(), but returns the entry instead of the index.

Source
pub fn entry_by_path(&self, path: &BStr) -> Option<&Entry>
Return the entry at path that is either at stage 0, or at stage 2 (ours) in case of a merge conflict.

Using this method is more efficient in comparison to doing two searches, one for stage 0 and one for stage 2.

Source
pub fn entry_index_by_path(&self, path: &BStr) -> Result<usize, usize>
Return the index at Ok(index) where the entry matching path (in any stage) can be found, or return Err(index) to indicate the insertion position at which an entry with path would fit in.

Source
pub fn prefixed_entries(&self, prefix: &BStr) -> Option<&[Entry]>
Return the slice of entries which all share the same prefix, or None if there isn’t a single such entry.

If prefix is empty, all entries are returned.

Source
pub fn prefixed_entries_range(&self, prefix: &BStr) -> Option<Range<usize>>
Return the range of entries which all share the same prefix, or None if there isn’t a single such entry.

If prefix is empty, the range will include all entries.

Source
pub fn entry(&self, idx: usize) -> &Entry
Return the entry at idx or panic if the index is out of bounds.

The idx is typically returned by entry_by_path_and_stage().

Source
pub fn is_sparse(&self) -> bool
Returns a boolean value indicating whether the index is sparse or not.

An index is sparse if it contains at least one Mode::DIR entry.

Source
pub fn entry_range(&self, path: &BStr) -> Option<Range<usize>>
Return the range of entries that exactly match the given path, in all available stages, or None if no entry with such path exists.

The range can be used to access the respective entries via entries() or `entries_mut().

Source
impl State
Mutation

Source
pub fn return_path_backing(&mut self, backing: PathStorage)
After usage of the storage obtained by take_path_backing(), return it here. Note that it must not be empty.

Source
pub fn entries_mut(&mut self) -> &mut [Entry]
Return mutable entries in a slice.

Source
pub fn entries_mut_and_pathbacking(&mut self) -> (&mut [Entry], &PathStorageRef)
Return a writable slice to entries and read-access to their path storage at the same time.

Source
pub fn entries_mut_with_paths(
    &mut self,
) -> impl Iterator<Item = (&mut Entry, &BStr)>
Return mutable entries along with their paths in an iterator.

Source
pub fn into_entries(self) -> (Vec<Entry>, PathStorage)
Return all parts that relate to entries, which includes path storage.

This can be useful for obtaining a standalone, boxable iterator

Source
pub fn take_path_backing(&mut self) -> PathStorage
Sometimes it’s needed to remove the path backing to allow certain mutation to happen in the state while supporting reading the entry’s path.

Source
pub fn entry_mut_by_path_and_stage(
    &mut self,
    path: &BStr,
    stage: Stage,
) -> Option<&mut Entry>
Like entry_index_by_path_and_stage(), but returns the mutable entry instead of the index.

Source
pub fn dangerously_push_entry(
    &mut self,
    stat: Stat,
    id: ObjectId,
    flags: Flags,
    mode: Mode,
    path: &BStr,
)
Push a new entry containing stat, id, flags and mode and path to the end of our storage, without performing any sanity checks. This means it’s possible to push a new entry to the same path on the same stage and even after sorting the entries lookups may still return the wrong one of them unless the correct binary search criteria is chosen.

Note that this is likely to break invariants that will prevent further lookups by path unless entry_index_by_path_and_stage_bounded() is used with the upper_bound being the amount of entries before the first call to this method.

Alternatively, make sure to call sort_entries() before entry lookup by path to restore the invariant.

Source
pub fn sort_entries(&mut self)
Unconditionally sort entries as needed to perform lookups quickly.

Source
pub fn sort_entries_by(
    &mut self,
    compare: impl FnMut(&Entry, &Entry) -> Ordering,
)
Similar to sort_entries(), but applies compare after comparing by path and stage as a third criteria.

Source
pub fn remove_entries(
    &mut self,
    should_remove: impl FnMut(usize, &BStr, &mut Entry) -> bool,
)
Physically remove all entries for which should_remove(idx, path, entry) returns true, traversing them from first to last.

Note that the memory used for the removed entries paths is not freed, as it’s append-only, and that some extensions might refer to paths which are now deleted.

Performance
To implement this operation typically, one would rather add entry::Flags::REMOVE to each entry to remove them when writing the index.

Source
pub fn remove_entry_at_index(&mut self, index: usize) -> Entry
Physically remove the entry at index, or panic if the entry didn’t exist.

This call is typically made after looking up index, so it’s clear that it will not panic.

Note that the memory used for the removed entries paths is not freed, as it’s append-only, and that some extensions might refer to paths which are now deleted.

Source
impl State
Extensions

Source
pub fn tree(&self) -> Option<&Tree>
Access the tree extension.

Source
pub fn remove_tree(&mut self) -> Option<Tree>
Remove the tree extension.

Source
pub fn link(&self) -> Option<&Link>
Access the link extension.

Source
pub fn resolve_undo(&self) -> Option<&Vec<ResolvePath>>
Obtain the resolve-undo extension.

Source
pub fn remove_resolve_undo(&mut self) -> Option<Vec<ResolvePath>>
Remove the resolve-undo extension.

Source
pub fn untracked(&self) -> Option<&UntrackedCache>
Obtain the untracked extension.

Source
pub fn fs_monitor(&self) -> Option<&FsMonitor>
Obtain the fsmonitor extension.

Source
pub fn had_end_of_index_marker(&self) -> bool
Return true if the end-of-index extension was present when decoding this index.

Source
pub fn had_offset_table(&self) -> bool
Return true if the offset-table extension was present when decoding this index.

Source
impl State
Initialization

Source
pub fn new(object_hash: Kind) -> Self
Return a new and empty in-memory index assuming the given object_hash.

Source
pub fn from_tree<Find>(
    tree: &oid,
    objects: Find,
    validate: Options,
) -> Result<Self, Error>
where
    Find: Find,
Create an index State by traversing tree recursively, accessing sub-trees with objects. validate is used to determine which validations to perform on every path component we see.

No extension data is currently produced.

Source
impl State
Source
pub fn from_bytes(
    data: &[u8],
    timestamp: FileTime,
    object_hash: Kind,
    _options: Options,
) -> Result<(Self, Option<ObjectId>), Error>
Decode an index state from data and store timestamp in the resulting instance for pass-through, assuming object_hash to be used through the file. Also return the stored hash over all bytes in data or None if none was written due to index.skipHash.

Source
impl State
Source
pub fn verify_entries(&self) -> Result<(), Error>
Assure our entries are consistent.

Source
pub fn verify_extensions(
    &self,
    use_find: bool,
    objects: impl Find,
) -> Result<(), Error>
Note: objects cannot be Option<F> as we can’t call it with a closure then due to the indirection through Some.

Source
impl State
Source
pub fn write_to(&self, out: impl Write, _: Options) -> Result<Version, Error>
Serialize this instance to out with options.

Trait Implementations
Source
impl Clone for State
Source
fn clone(&self) -> State
Returns a copy of the value. Read more
1.0.0 · Source
fn clone_from(&mut self, source: &Self)
Performs copy-assignment from source. Read more
Source
impl Debug for State
Source
fn fmt(&self, f: &mut Formatter<'_>) -> Result
Formats the value using the given formatter. Read more
Source
impl From<File> for State
Source
fn from(f: File) -> Self
Converts to this type from the input type.


## Gix::File

Struct FileCopy item path
Settings
Help

Summary
Source
pub struct File<'event> { /* private fields */ }
High level git-config reader and writer.

This is the full-featured implementation that can deserialize, serialize, and edit git-config files without loss of whitespace or comments.

‘multivar’ behavior
git is flexible enough to allow users to set a key multiple times in any number of identically named sections. When this is the case, the key is known as a “multivar”. In this case, raw_value() follows the “last one wins”.

Concretely, the following config has a multivar, a, with the values of b, c, and d, while e is a single variable with the value f g h.

[core]
    a = b
    a = c
[core]
    a = d
    e = f g h
Calling methods that fetch or set only one value (such as raw_value()) key a with the above config will fetch d or replace d, since the last valid config key/value pair is a = d:

Filtering
All methods exist in a *_filter(…, filter) version to allow skipping sections by their metadata. That way it’s possible to select values based on their gix_sec::Trust for example, or by their location.

Note that the filter may be executed even on sections that don’t contain the key in question, even though the section will have matched the name and subsection_name respectively.

assert_eq!(git_config.raw_value("core.a").unwrap().as_ref(), "d");
Consider the multi variants of the methods instead, if you want to work with all values.

Equality
In order to make it useful, equality will ignore all non-value bearing information, hence compare only sections and their names, as well as all of their values. The ordering matters, of course.

Implementations
Source
impl File<'static>
Easy-instantiation of typical non-repository git configuration files with all configuration defaulting to typical values.

Limitations
Note that includeIf conditions in global files will cause failure as the required information to resolve them isn’t present without a repository.

Also note that relevant information to interpolate paths will be obtained from the environment or other source on unix.

Source
pub fn from_globals() -> Result<File<'static>, Error>
Open all global configuration files which involves the following sources:

git-installation
system
globals
which excludes repository local configuration, as well as override-configuration from environment variables.

Note that the file might be empty in case no configuration file was found.

Source
pub fn from_environment_overrides() -> Result<File<'static>, Error>
Generates a config from GIT_CONFIG_* environment variables and return a possibly empty File. A typical use of this is to append this configuration to another one with lower precedence to obtain overrides.

See git-config’s documentation for more information on the environment variables in question.

Source
impl File<'static>
An easy way to provide complete configuration for a repository.

Source
pub fn from_git_dir(dir: PathBuf) -> Result<File<'static>, Error>
This configuration type includes the following sources, in order of precedence:

globals
repository-local by loading dir/config
worktree by loading dir/config.worktree
environment
Note that dir is the .git dir to load the configuration from, not the configuration file.

Includes will be resolved within limits as some information like the git installation directory is missing to interpolate paths with as well as git repository information like the branch name.

Source
impl File<'static>
Instantiation from environment variables

Source
pub fn from_env(options: Options<'_>) -> Result<Option<File<'static>>, Error>
Generates a config from GIT_CONFIG_* environment variables or returns Ok(None) if no configuration was found. See git-config’s documentation for more information on the environment variables in question.

With options configured, it’s possible to resolve include.path or includeIf.<condition>.path directives as well.

Source
impl File<'static>
Instantiation from one or more paths

Source
pub fn from_path_no_includes(
    path: PathBuf,
    source: Source,
) -> Result<Self, Error>
Load the single file at path with source without following include directives.

Note that the path will be checked for ownership to derive trust.

Source
pub fn from_paths_metadata(
    path_meta: impl IntoIterator<Item = impl Into<Metadata>>,
    options: Options<'_>,
) -> Result<Option<Self>, Error>
Constructs a git-config file from the provided metadata, which must include a path to read from or be ignored. Returns Ok(None) if there was not a single input path provided, which is a possibility due to Metadata::path being an Option. If an input path doesn’t exist, the entire operation will abort. See from_paths_metadata_buf() for a more powerful version of this method.

Source
pub fn from_paths_metadata_buf(
    path_meta: &mut dyn Iterator<Item = Metadata>,
    buf: &mut Vec<u8>,
    err_on_non_existing_paths: bool,
    options: Options<'_>,
) -> Result<Option<Self>, Error>
Like from_paths_metadata(), but will use buf to temporarily store the config file contents for parsing instead of allocating an own buffer.

If err_on_nonexisting_paths is false, instead of aborting with error, we will continue to the next path instead.

Source
impl<'a> File<'a>
Source
pub fn new(meta: impl Into<OwnShared<Metadata>>) -> Self
Return an empty File with the given meta-data to be attached to all new sections.

Source
pub fn from_bytes_no_includes(
    input: &'a [u8],
    meta: impl Into<OwnShared<Metadata>>,
    options: Options<'_>,
) -> Result<Self, Error>
Instantiate a new File from given input, associating each section and their values with meta-data, while respecting options.

Source
pub fn from_parse_events_no_includes(
    _: Events<'a>,
    meta: impl Into<OwnShared<Metadata>>,
) -> Self
Instantiate a new File from given events, associating each section and their values with meta-data.

Source
impl File<'static>
Source
pub fn from_bytes_owned(
    input_and_buf: &mut Vec<u8>,
    meta: impl Into<OwnShared<Metadata>>,
    options: Options<'_>,
) -> Result<Self, Error>
Instantiate a new fully-owned File from given input (later reused as buffer when resolving includes), associating each section and their values with meta-data, while respecting options, and following includes as configured there.

Source
impl File<'_>
Comfortable API for accessing values

Source
pub fn string(&self, key: impl AsKey) -> Option<Cow<'_, BStr>>
Like string_by(), but suitable for statically known keys like remote.origin.url.

Source
pub fn string_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
) -> Option<Cow<'_, BStr>>
Like value(), but returning None if the string wasn’t found.

As strings perform no conversions, this will never fail.

Source
pub fn string_filter(
    &self,
    key: impl AsKey,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Cow<'_, BStr>>
Like string_filter_by(), but suitable for statically known keys like remote.origin.url.

Source
pub fn string_filter_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Cow<'_, BStr>>
Like string(), but the section containing the returned value must pass filter as well.

Source
pub fn path(&self, key: impl AsKey) -> Option<Path<'_>>
Like path_by(), but suitable for statically known keys like remote.origin.url.

Source
pub fn path_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
) -> Option<Path<'_>>
Like value(), but returning None if the path wasn’t found.

Note that this path is not vetted and should only point to resources which can’t be used to pose a security risk. Prefer using path_filter() instead.

As paths perform no conversions, this will never fail.

Source
pub fn path_filter(
    &self,
    key: impl AsKey,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Path<'_>>
Like path_filter_by(), but suitable for statically known keys like remote.origin.url.

Source
pub fn path_filter_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Path<'_>>
Like path(), but the section containing the returned value must pass filter as well.

This should be the preferred way of accessing paths as those from untrusted locations can be

As paths perform no conversions, this will never fail.

Source
pub fn boolean(&self, key: impl AsKey) -> Option<Result<bool, Error>>
Like boolean_by(), but suitable for statically known keys like remote.origin.url.

Source
pub fn boolean_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
) -> Option<Result<bool, Error>>
Like value(), but returning None if the boolean value wasn’t found.

Source
pub fn boolean_filter(
    &self,
    key: impl AsKey,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Result<bool, Error>>
Like boolean_filter_by(), but suitable for statically known keys like remote.origin.url.

Source
pub fn boolean_filter_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Result<bool, Error>>
Like boolean_by(), but the section containing the returned value must pass filter as well.

Source
pub fn integer(&self, key: impl AsKey) -> Option<Result<i64, Error>>
Like integer_by(), but suitable for statically known keys like remote.origin.url.

Source
pub fn integer_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
) -> Option<Result<i64, Error>>
Like value(), but returning an Option if the integer wasn’t found.

Source
pub fn integer_filter(
    &self,
    key: impl AsKey,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Result<i64, Error>>
Like integer_filter_by(), but suitable for statically known keys like remote.origin.url.

Source
pub fn integer_filter_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Result<i64, Error>>
Like integer_by(), but the section containing the returned value must pass filter as well.

Source
pub fn strings(&self, key: impl AsKey) -> Option<Vec<Cow<'_, BStr>>>
Like strings_by(), but suitable for statically known keys like remote.origin.url.

Source
pub fn strings_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
) -> Option<Vec<Cow<'_, BStr>>>
Similar to values_by(…) but returning strings if at least one of them was found.

Source
pub fn strings_filter(
    &self,
    key: impl AsKey,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Vec<Cow<'_, BStr>>>
Like strings_filter_by(), but suitable for statically known keys like remote.origin.url.

Source
pub fn strings_filter_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Vec<Cow<'_, BStr>>>
Similar to strings_by(…), but all values are in sections that passed filter.

Source
pub fn integers(&self, key: impl AsKey) -> Option<Result<Vec<i64>, Error>>
Like integers(), but suitable for statically known keys like remote.origin.url.

Source
pub fn integers_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
) -> Option<Result<Vec<i64>, Error>>
Similar to values_by(…) but returning integers if at least one of them was found and if none of them overflows.

Source
pub fn integers_filter(
    &self,
    key: impl AsKey,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Result<Vec<i64>, Error>>
Like integers_filter_by(), but suitable for statically known keys like remote.origin.url.

Source
pub fn integers_filter_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Result<Vec<i64>, Error>>
Similar to integers_by(…) but all integers are in sections that passed filter and that are not overflowing.

Source
impl<'event> File<'event>
Mutating low-level access methods.

Source
pub fn section_mut<'a>(
    &'a mut self,
    name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
) -> Result<SectionMut<'a, 'event>, Error>
Returns the last mutable section with a given name and optional subsection_name, if it exists.

Source
pub fn section_mut_by_key<'a, 'b>(
    &'a mut self,
    key: impl Into<&'b BStr>,
) -> Result<SectionMut<'a, 'event>, Error>
Returns the last found mutable section with a given key, identifying the name and subsection name like core or remote.origin.

Source
pub fn section_mut_by_id<'a>(
    &'a mut self,
    id: SectionId,
) -> Option<SectionMut<'a, 'event>>
Return the mutable section identified by id, or None if it didn’t exist.

Note that id is stable across deletions and insertions.

Source
pub fn section_mut_or_create_new<'a>(
    &'a mut self,
    name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
) -> Result<SectionMut<'a, 'event>, Error>
Returns the last mutable section with a given name and optional subsection_name, if it exists, or create a new section.

Source
pub fn section_mut_or_create_new_filter<'a>(
    &'a mut self,
    name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<SectionMut<'a, 'event>, Error>
Returns an mutable section with a given name and optional subsection_name, if it exists and passes filter, or create a new section.

Source
pub fn section_mut_filter<'a>(
    &'a mut self,
    name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<Option<SectionMut<'a, 'event>>, Error>
Returns the last found mutable section with a given name and optional subsection_name, that matches filter, if it exists.

If there are sections matching section_name and subsection_name but the filter rejects all of them, Ok(None) is returned.

Source
pub fn section_mut_filter_by_key<'a, 'b>(
    &'a mut self,
    key: impl Into<&'b BStr>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<Option<SectionMut<'a, 'event>>, Error>
Like section_mut_filter(), but identifies the with a given key, like core or remote.origin.

Source
pub fn new_section(
    &mut self,
    name: impl Into<Cow<'event, str>>,
    subsection: impl Into<Option<Cow<'event, BStr>>>,
) -> Result<SectionMut<'_, 'event>, Error>
Adds a new section. If a subsection name was provided, then the generated header will use the modern subsection syntax. Returns a reference to the new section for immediate editing.

Examples
Creating a new empty section:

let mut git_config = gix_config::File::default();
let section = git_config.new_section("hello", Some(Cow::Borrowed("world".into())))?;
let nl = section.newline().to_owned();
assert_eq!(git_config.to_string(), format!("[hello \"world\"]{nl}"));
Creating a new empty section and adding values to it:

let mut git_config = gix_config::File::default();
let mut section = git_config.new_section("hello", Some(Cow::Borrowed("world".into())))?;
section.push(section::ValueName::try_from("a")?, Some("b".into()));
let nl = section.newline().to_owned();
assert_eq!(git_config.to_string(), format!("[hello \"world\"]{nl}\ta = b{nl}"));
let _section = git_config.new_section("core", None);
assert_eq!(git_config.to_string(), format!("[hello \"world\"]{nl}\ta = b{nl}[core]{nl}"));
Source
pub fn remove_section<'a>(
    &mut self,
    name: impl AsRef<str>,
    subsection_name: impl Into<Option<&'a BStr>>,
) -> Option<Section<'event>>
Removes the section with name and subsection_name , returning it if there was a matching section. If multiple sections have the same name, then the last one is returned. Note that later sections with the same name have precedent over earlier ones.

Examples
Creating and removing a section:

let mut git_config = gix_config::File::try_from(
r#"[hello "world"]
    some-value = 4
"#)?;

let section = git_config.remove_section("hello", Some("world".into()));
assert_eq!(git_config.to_string(), "");
Precedence example for removing sections with the same name:

let mut git_config = gix_config::File::try_from(
r#"[hello "world"]
    some-value = 4
[hello "world"]
    some-value = 5
"#)?;

let section = git_config.remove_section("hello", Some("world".into()));
assert_eq!(git_config.to_string(), "[hello \"world\"]\n    some-value = 4\n");
Source
pub fn remove_section_by_id(&mut self, id: SectionId) -> Option<Section<'event>>
Remove the section identified by id if it exists and return it, or return None if no such section was present.

Note that section ids are unambiguous even in the face of removals and additions of sections.

Source
pub fn remove_section_filter<'a>(
    &mut self,
    name: impl AsRef<str>,
    subsection_name: impl Into<Option<&'a BStr>>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Option<Section<'event>>
Removes the section with name and subsection_name that passed filter, returning the removed section if at least one section matched the filter. If multiple sections have the same name, then the last one is returned. Note that later sections with the same name have precedent over earlier ones.

Source
pub fn push_section(
    &mut self,
    section: Section<'event>,
) -> SectionMut<'_, 'event>
Adds the provided section to the config, returning a mutable reference to it for immediate editing. Note that its meta-data will remain as is.

Source
pub fn rename_section<'a>(
    &mut self,
    name: impl AsRef<str>,
    subsection_name: impl Into<Option<&'a BStr>>,
    new_name: impl Into<Cow<'event, str>>,
    new_subsection_name: impl Into<Option<Cow<'event, BStr>>>,
) -> Result<(), Error>
Renames the section with name and subsection_name, modifying the last matching section to use new_name and new_subsection_name.

Source
pub fn rename_section_filter<'a>(
    &mut self,
    name: impl AsRef<str>,
    subsection_name: impl Into<Option<&'a BStr>>,
    new_name: impl Into<Cow<'event, str>>,
    new_subsection_name: impl Into<Option<Cow<'event, BStr>>>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<(), Error>
Renames the section with name and subsection_name, modifying the last matching section that also passes filter to use new_name and new_subsection_name.

Note that the otherwise unused lookup::existing::Error::KeyMissing variant is used to indicate that the filter rejected all candidates, leading to no section being renamed after all.

Source
pub fn append(&mut self, other: Self) -> &mut Self
Append another File to the end of ourselves, without losing any information.

Source
impl<'event> File<'event>
Raw value API
These functions are the raw value API, returning normalized byte strings.

Source
pub fn raw_value(&self, key: impl AsKey) -> Result<Cow<'_, BStr>, Error>
Returns an uninterpreted value given a key.

Consider Self::raw_values() if you want to get all values of a multivar instead.

Source
pub fn raw_value_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
) -> Result<Cow<'_, BStr>, Error>
Returns an uninterpreted value given a section, an optional subsection and value name.

Consider Self::raw_values() if you want to get all values of a multivar instead.

Source
pub fn raw_value_filter(
    &self,
    key: impl AsKey,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<Cow<'_, BStr>, Error>
Returns an uninterpreted value given a key, if it passes the filter.

Consider Self::raw_values() if you want to get all values of a multivar instead.

Source
pub fn raw_value_filter_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<Cow<'_, BStr>, Error>
Returns an uninterpreted value given a section, an optional subsection and value name, if it passes the filter.

Consider Self::raw_values() if you want to get all values of a multivar instead.

Source
pub fn raw_value_mut<'lookup>(
    &mut self,
    key: &'lookup impl AsKey,
) -> Result<ValueMut<'_, 'lookup, 'event>, Error>
Returns a mutable reference to an uninterpreted value given a section, an optional subsection and value name.

Consider Self::raw_values_mut if you want to get mutable references to all values of a multivar instead.

Source
pub fn raw_value_mut_by<'lookup>(
    &mut self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&'lookup BStr>,
    value_name: &'lookup str,
) -> Result<ValueMut<'_, 'lookup, 'event>, Error>
Returns a mutable reference to an uninterpreted value given a section, an optional subsection and value name.

Consider Self::raw_values_mut_by if you want to get mutable references to all values of a multivar instead.

Source
pub fn raw_value_mut_filter<'lookup>(
    &mut self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&'lookup BStr>,
    value_name: &'lookup str,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<ValueMut<'_, 'lookup, 'event>, Error>
Returns a mutable reference to an uninterpreted value given a section, an optional subsection and value name, and if it passes filter.

Consider Self::raw_values_mut_by if you want to get mutable references to all values of a multivar instead.

Source
pub fn raw_values(&self, key: impl AsKey) -> Result<Vec<Cow<'_, BStr>>, Error>
Returns all uninterpreted values given a key.

The ordering means that the last of the returned values is the one that would be the value used in the single-value case.

Examples
If you have the following config:

[core]
    a = b
[core]
    a = c
    a = d
Attempting to get all values of a yields the following:

assert_eq!(
    git_config.raw_values("core.a").unwrap(),
    vec![
        Cow::<BStr>::Borrowed("b".into()),
        Cow::<BStr>::Borrowed("c".into()),
        Cow::<BStr>::Borrowed("d".into()),
    ],
);
Consider Self::raw_value if you want to get the resolved single value for a given key, if your value does not support multi-valued values.

Source
pub fn raw_values_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
) -> Result<Vec<Cow<'_, BStr>>, Error>
Returns all uninterpreted values given a section, an optional subsection and value name in order of occurrence.

The ordering means that the last of the returned values is the one that would be the value used in the single-value case.

Examples
If you have the following config:

[core]
    a = b
[core]
    a = c
    a = d
Attempting to get all values of a yields the following:

assert_eq!(
    git_config.raw_values_by("core", None, "a").unwrap(),
    vec![
        Cow::<BStr>::Borrowed("b".into()),
        Cow::<BStr>::Borrowed("c".into()),
        Cow::<BStr>::Borrowed("d".into()),
    ],
);
Consider Self::raw_value if you want to get the resolved single value for a given value name, if your value does not support multi-valued values.

Source
pub fn raw_values_filter(
    &self,
    key: impl AsKey,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<Vec<Cow<'_, BStr>>, Error>
Returns all uninterpreted values given a key, if the value passes filter, in order of occurrence.

The ordering means that the last of the returned values is the one that would be the value used in the single-value case.

Source
pub fn raw_values_filter_by(
    &self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<Vec<Cow<'_, BStr>>, Error>
Returns all uninterpreted values given a section, an optional subsection and value name, if the value passes filter, in order of occurrence.

The ordering means that the last of the returned values is the one that would be the value used in the single-value case.

Source
pub fn raw_values_mut<'lookup>(
    &mut self,
    key: &'lookup impl AsKey,
) -> Result<MultiValueMut<'_, 'lookup, 'event>, Error>
Returns mutable references to all uninterpreted values given a key.

Examples
If you have the following config:

[core]
    a = b
[core]
    a = c
    a = d
Attempting to get all values of a yields the following:

assert_eq!(
    git_config.raw_values("core.a")?,
    vec![
        Cow::<BStr>::Borrowed("b".into()),
        Cow::<BStr>::Borrowed("c".into()),
        Cow::<BStr>::Borrowed("d".into())
    ]
);

git_config.raw_values_mut(&"core.a")?.set_all("g");

assert_eq!(
    git_config.raw_values("core.a")?,
    vec![
        Cow::<BStr>::Borrowed("g".into()),
        Cow::<BStr>::Borrowed("g".into()),
        Cow::<BStr>::Borrowed("g".into())
    ],
);
Consider Self::raw_value if you want to get the resolved single value for a given value name, if your value does not support multi-valued values.

Note that this operation is relatively expensive, requiring a full traversal of the config.

Source
pub fn raw_values_mut_by<'lookup>(
    &mut self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&'lookup BStr>,
    value_name: &'lookup str,
) -> Result<MultiValueMut<'_, 'lookup, 'event>, Error>
Returns mutable references to all uninterpreted values given a section, an optional subsection and value name.

Examples
If you have the following config:

[core]
    a = b
[core]
    a = c
    a = d
Attempting to get all values of a yields the following:

assert_eq!(
    git_config.raw_values("core.a")?,
    vec![
        Cow::<BStr>::Borrowed("b".into()),
        Cow::<BStr>::Borrowed("c".into()),
        Cow::<BStr>::Borrowed("d".into())
    ]
);

git_config.raw_values_mut_by("core", None, "a")?.set_all("g");

assert_eq!(
    git_config.raw_values("core.a")?,
    vec![
        Cow::<BStr>::Borrowed("g".into()),
        Cow::<BStr>::Borrowed("g".into()),
        Cow::<BStr>::Borrowed("g".into())
    ],
);
Consider Self::raw_value if you want to get the resolved single value for a given value name, if your value does not support multi-valued values.

Note that this operation is relatively expensive, requiring a full traversal of the config.

Source
pub fn raw_values_mut_filter<'lookup>(
    &mut self,
    key: &'lookup impl AsKey,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<MultiValueMut<'_, 'lookup, 'event>, Error>
Returns mutable references to all uninterpreted values given a key, if their sections pass filter.

Source
pub fn raw_values_mut_filter_by<'lookup>(
    &mut self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&'lookup BStr>,
    value_name: &'lookup str,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<MultiValueMut<'_, 'lookup, 'event>, Error>
Returns mutable references to all uninterpreted values given a section, an optional subsection and value name, if their sections pass filter.

Source
pub fn set_existing_raw_value<'b>(
    &mut self,
    key: &'b impl AsKey,
    new_value: impl Into<&'b BStr>,
) -> Result<(), Error>
Sets a value in a given key. Note that the parts leading to the value name must exist for this method to work, i.e. the section and the subsection, if present.

Examples
Given the config,

[core]
    a = b
[core]
    a = c
    a = d
Setting a new value to the key core.a will yield the following:

git_config.set_existing_raw_value(&"core.a", "e")?;
assert_eq!(git_config.raw_value("core.a")?, Cow::<BStr>::Borrowed("e".into()));
assert_eq!(
    git_config.raw_values("core.a")?,
    vec![
        Cow::<BStr>::Borrowed("b".into()),
        Cow::<BStr>::Borrowed("c".into()),
        Cow::<BStr>::Borrowed("e".into())
    ],
);
Source
pub fn set_existing_raw_value_by<'b>(
    &mut self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
    new_value: impl Into<&'b BStr>,
) -> Result<(), Error>
Sets a value in a given section_name, optional subsection_name, and value_name. Note sections named section_name and subsection_name (if not None) must exist for this method to work.

Examples
Given the config,

[core]
    a = b
[core]
    a = c
    a = d
Setting a new value to the key core.a will yield the following:

git_config.set_existing_raw_value_by("core", None, "a", "e")?;
assert_eq!(git_config.raw_value("core.a")?, Cow::<BStr>::Borrowed("e".into()));
assert_eq!(
    git_config.raw_values("core.a")?,
    vec![
        Cow::<BStr>::Borrowed("b".into()),
        Cow::<BStr>::Borrowed("c".into()),
        Cow::<BStr>::Borrowed("e".into())
    ],
);
Source
pub fn set_raw_value<'b>(
    &mut self,
    key: &'event impl AsKey,
    new_value: impl Into<&'b BStr>,
) -> Result<Option<Cow<'event, BStr>>, Error>
Sets a value in a given key. Creates the section if necessary and the value as well, or overwrites the last existing value otherwise.

Examples
Given the config,

[core]
    a = b
Setting a new value to the key core.a will yield the following:

let prev = git_config.set_raw_value(&"core.a", "e")?;
git_config.set_raw_value(&"core.b", "f")?;
assert_eq!(prev.expect("present").as_ref(), "b");
assert_eq!(git_config.raw_value("core.a")?, Cow::<BStr>::Borrowed("e".into()));
assert_eq!(git_config.raw_value("core.b")?, Cow::<BStr>::Borrowed("f".into()));
Source
pub fn set_raw_value_by<'b, Key, E>(
    &mut self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: Key,
    new_value: impl Into<&'b BStr>,
) -> Result<Option<Cow<'event, BStr>>, Error>
where
    Key: TryInto<ValueName<'event>, Error = E>,
    Error: From<E>,
Sets a value in a given section_name, optional subsection_name, and value_name. Creates the section if necessary and the value as well, or overwrites the last existing value otherwise.

Examples
Given the config,

[core]
    a = b
Setting a new value to the key core.a will yield the following:

let prev = git_config.set_raw_value_by("core", None, "a", "e")?;
git_config.set_raw_value_by("core", None, "b", "f")?;
assert_eq!(prev.expect("present").as_ref(), "b");
assert_eq!(git_config.raw_value("core.a")?, Cow::<BStr>::Borrowed("e".into()));
assert_eq!(git_config.raw_value("core.b")?, Cow::<BStr>::Borrowed("f".into()));
Source
pub fn set_raw_value_filter<'b>(
    &mut self,
    key: &'event impl AsKey,
    new_value: impl Into<&'b BStr>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<Option<Cow<'event, BStr>>, Error>
Similar to set_raw_value(), but only sets existing values in sections matching filter, creating a new section otherwise.

Source
pub fn set_raw_value_filter_by<'b, Key, E>(
    &mut self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    key: Key,
    new_value: impl Into<&'b BStr>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<Option<Cow<'event, BStr>>, Error>
where
    Key: TryInto<ValueName<'event>, Error = E>,
    Error: From<E>,
Similar to set_raw_value_by(), but only sets existing values in sections matching filter, creating a new section otherwise.

Source
pub fn set_existing_raw_multi_value<'a, Iter, Item>(
    &mut self,
    key: &'a impl AsKey,
    new_values: Iter,
) -> Result<(), Error>
where
    Iter: IntoIterator<Item = Item>,
    Item: Into<&'a BStr>,
Sets a multivar in a given key.

This internally zips together the new values and the existing values. As a result, if more new values are provided than the current amount of multivars, then the latter values are not applied. If there are less new values than old ones then the remaining old values are unmodified.

Note: Mutation order is not guaranteed and is non-deterministic. If you need finer control over which values of the multivar are set, consider using raw_values_mut(), which will let you iterate and check over the values instead. This is best used as a convenience function for setting multivars whose values should be treated as an unordered set.

Examples
Let us use the follow config for all examples:

[core]
    a = b
[core]
    a = c
    a = d
Setting an equal number of values:

let new_values = vec![
    "x",
    "y",
    "z",
];
git_config.set_existing_raw_multi_value(&"core.a", new_values.into_iter())?;
let fetched_config = git_config.raw_values("core.a")?;
assert!(fetched_config.contains(&Cow::<BStr>::Borrowed("x".into())));
assert!(fetched_config.contains(&Cow::<BStr>::Borrowed("y".into())));
assert!(fetched_config.contains(&Cow::<BStr>::Borrowed("z".into())));
Setting less than the number of present values sets the first ones found:

let new_values = vec![
    "x",
    "y",
];
git_config.set_existing_raw_multi_value(&"core.a", new_values.into_iter())?;
let fetched_config = git_config.raw_values("core.a")?;
assert!(fetched_config.contains(&Cow::<BStr>::Borrowed("x".into())));
assert!(fetched_config.contains(&Cow::<BStr>::Borrowed("y".into())));
Setting more than the number of present values discards the rest:

let new_values = vec![
    "x",
    "y",
    "z",
    "discarded",
];
git_config.set_existing_raw_multi_value(&"core.a", new_values)?;
assert!(!git_config.raw_values("core.a")?.contains(&Cow::<BStr>::Borrowed("discarded".into())));
Source
pub fn set_existing_raw_multi_value_by<'a, Iter, Item>(
    &mut self,
    section_name: impl AsRef<str>,
    subsection_name: Option<&BStr>,
    value_name: impl AsRef<str>,
    new_values: Iter,
) -> Result<(), Error>
where
    Iter: IntoIterator<Item = Item>,
    Item: Into<&'a BStr>,
Sets a multivar in a given section, optional subsection, and key value.

This internally zips together the new values and the existing values. As a result, if more new values are provided than the current amount of multivars, then the latter values are not applied. If there are less new values than old ones then the remaining old values are unmodified.

Note: Mutation order is not guaranteed and is non-deterministic. If you need finer control over which values of the multivar are set, consider using raw_values_mut(), which will let you iterate and check over the values instead. This is best used as a convenience function for setting multivars whose values should be treated as an unordered set.

Examples
Let us use the follow config for all examples:

[core]
    a = b
[core]
    a = c
    a = d
Setting an equal number of values:

let new_values = vec![
    "x",
    "y",
    "z",
];
git_config.set_existing_raw_multi_value_by("core", None, "a", new_values.into_iter())?;
let fetched_config = git_config.raw_values("core.a")?;
assert!(fetched_config.contains(&Cow::<BStr>::Borrowed("x".into())));
assert!(fetched_config.contains(&Cow::<BStr>::Borrowed("y".into())));
assert!(fetched_config.contains(&Cow::<BStr>::Borrowed("z".into())));
Setting less than the number of present values sets the first ones found:

let new_values = vec![
    "x",
    "y",
];
git_config.set_existing_raw_multi_value_by("core", None, "a", new_values.into_iter())?;
let fetched_config = git_config.raw_values("core.a")?;
assert!(fetched_config.contains(&Cow::<BStr>::Borrowed("x".into())));
assert!(fetched_config.contains(&Cow::<BStr>::Borrowed("y".into())));
Setting more than the number of present values discards the rest:

let new_values = vec![
    "x",
    "y",
    "z",
    "discarded",
];
git_config.set_existing_raw_multi_value_by("core", None, "a", new_values)?;
assert!(!git_config.raw_values("core.a")?.contains(&Cow::<BStr>::Borrowed("discarded".into())));
Source
impl<'event> File<'event>
Read-only low-level access methods, as it requires generics for converting into custom values defined in this crate like Integer and Color.

Source
pub fn value<'a, T: TryFrom<Cow<'a, BStr>>>(
    &'a self,
    key: impl AsKey,
) -> Result<T, Error<T::Error>>
Returns an interpreted value given a key.

It’s recommended to use one of the value types provide dby this crate as they implement the conversion, but this function is flexible and will accept any type that implements TryFrom<&BStr>.

Consider Self::values if you want to get all values of a multivar instead.

If a string is desired, use the string() method instead.

Examples
let config = r#"
    [core]
        a = 10k
        c = false
"#;
let git_config = gix_config::File::try_from(config)?;
// You can either use the turbofish to determine the type...
let a_value = git_config.value::<Integer>("core.a")?;
// ... or explicitly declare the type to avoid the turbofish
let c_value: Boolean = git_config.value("core.c")?;
Source
pub fn value_by<'a, T: TryFrom<Cow<'a, BStr>>>(
    &'a self,
    section_name: &str,
    subsection_name: Option<&BStr>,
    value_name: &str,
) -> Result<T, Error<T::Error>>
Returns an interpreted value given a section, an optional subsection and value name.

It’s recommended to use one of the value types provide dby this crate as they implement the conversion, but this function is flexible and will accept any type that implements TryFrom<&BStr>.

Consider Self::values if you want to get all values of a multivar instead.

If a string is desired, use the string() method instead.

Examples
let config = r#"
    [core]
        a = 10k
        c = false
"#;
let git_config = gix_config::File::try_from(config)?;
// You can either use the turbofish to determine the type...
let a_value = git_config.value_by::<Integer>("core", None, "a")?;
// ... or explicitly declare the type to avoid the turbofish
let c_value: Boolean = git_config.value_by("core", None, "c")?;
Source
pub fn try_value<'a, T: TryFrom<Cow<'a, BStr>>>(
    &'a self,
    key: impl AsKey,
) -> Option<Result<T, T::Error>>
Like value(), but returning an None if the value wasn’t found at section[.subsection].value_name

Source
pub fn try_value_by<'a, T: TryFrom<Cow<'a, BStr>>>(
    &'a self,
    section_name: &str,
    subsection_name: Option<&BStr>,
    value_name: &str,
) -> Option<Result<T, T::Error>>
Like value_by(), but returning an None if the value wasn’t found at section[.subsection].value_name

Source
pub fn values<'a, T: TryFrom<Cow<'a, BStr>>>(
    &'a self,
    key: impl AsKey,
) -> Result<Vec<T>, Error<T::Error>>
Returns all interpreted values given a section, an optional subsection and value name.

It’s recommended to use one of the value types provide dby this crate as they implement the conversion, but this function is flexible and will accept any type that implements TryFrom<&BStr>.

Consider Self::value if you want to get a single value (following last-one-wins resolution) instead.

To access plain strings, use the strings() method instead.

Examples
let config = r#"
    [core]
        a = true
        c
    [core]
        a
        a = false
"#;
let git_config = gix_config::File::try_from(config).unwrap();
// You can either use the turbofish to determine the type...
let a_value = git_config.values::<Boolean>("core.a")?;
assert_eq!(
    a_value,
    vec![
        Boolean(true),
        Boolean(false),
        Boolean(false),
    ]
);
// ... or explicitly declare the type to avoid the turbofish
let c_value: Vec<Boolean> = git_config.values("core.c").unwrap();
assert_eq!(c_value, vec![Boolean(false)]);
Source
pub fn values_by<'a, T: TryFrom<Cow<'a, BStr>>>(
    &'a self,
    section_name: &str,
    subsection_name: Option<&BStr>,
    value_name: &str,
) -> Result<Vec<T>, Error<T::Error>>
Returns all interpreted values given a section, an optional subsection and value name.

It’s recommended to use one of the value types provide dby this crate as they implement the conversion, but this function is flexible and will accept any type that implements TryFrom<&BStr>.

Consider Self::value if you want to get a single value (following last-one-wins resolution) instead.

To access plain strings, use the strings() method instead.

Examples
let config = r#"
    [core]
        a = true
        c
    [core]
        a
        a = false
"#;
let git_config = gix_config::File::try_from(config).unwrap();
// You can either use the turbofish to determine the type...
let a_value = git_config.values_by::<Boolean>("core", None, "a")?;
assert_eq!(
    a_value,
    vec![
        Boolean(true),
        Boolean(false),
        Boolean(false),
    ]
);
// ... or explicitly declare the type to avoid the turbofish
let c_value: Vec<Boolean> = git_config.values_by("core", None, "c").unwrap();
assert_eq!(c_value, vec![Boolean(false)]);
Source
pub fn section(
    &self,
    name: &str,
    subsection_name: Option<&BStr>,
) -> Result<&Section<'event>, Error>
Returns the last found immutable section with a given name and optional subsection_name.

Source
pub fn section_by_key(
    &self,
    section_key: &BStr,
) -> Result<&Section<'event>, Error>
Returns the last found immutable section with a given section_key, identifying the name and subsection name like core or remote.origin.

Source
pub fn section_filter<'a>(
    &'a self,
    name: &str,
    subsection_name: Option<&BStr>,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<Option<&'a Section<'event>>, Error>
Returns the last found immutable section with a given name and optional subsection_name, that matches filter.

If there are sections matching section_name and subsection_name but the filter rejects all of them, Ok(None) is returned.

Source
pub fn section_filter_by_key<'a>(
    &'a self,
    section_key: &BStr,
    filter: impl FnMut(&Metadata) -> bool,
) -> Result<Option<&'a Section<'event>>, Error>
Like section_filter(), but identifies the section with section_key like core or remote.origin.

Source
pub fn sections_by_name<'a>(
    &'a self,
    name: &'a str,
) -> Option<impl Iterator<Item = &'a Section<'event>> + 'a>
Gets all sections that match the provided name, ignoring any subsections.

Examples
Provided the following config:

[core]
    a = b
[core ""]
    c = d
[core "apple"]
    e = f
Calling this method will yield all sections:

let config = r#"
    [core]
        a = b
    [core ""]
        c = d
    [core "apple"]
        e = f
"#;
let git_config = gix_config::File::try_from(config)?;
assert_eq!(git_config.sections_by_name("core").map_or(0, |s|s.count()), 3);
Source
pub fn sections_and_ids_by_name<'a>(
    &'a self,
    name: &'a str,
) -> Option<impl Iterator<Item = (&'a Section<'event>, SectionId)> + 'a>
Similar to sections_by_name(), but returns an identifier for this section as well to allow referring to it unambiguously even in the light of deletions.

Source
pub fn sections_by_name_and_filter<'a>(
    &'a self,
    name: &'a str,
    filter: impl FnMut(&Metadata) -> bool + 'a,
) -> Option<impl Iterator<Item = &'a Section<'event>> + 'a>
Gets all sections that match the provided name, ignoring any subsections, and pass the filter.

Source
pub fn num_values(&self) -> usize
Returns the number of values in the config, no matter in which section.

For example, a config with multiple empty sections will return 0. This ignores any comments.

Source
pub fn is_void(&self) -> bool
Returns if there are no entries in the config. This will return true if there are only empty sections, with whitespace and comments not being considered void.

Source
pub fn meta(&self) -> &Metadata
Return this file’s metadata, typically set when it was first created to indicate its origins.

It will be used in all newly created sections to identify them. Change it with File::set_meta().

Source
pub fn set_meta(&mut self, meta: impl Into<OwnShared<Metadata>>) -> &mut Self
Change the origin of this instance to be the given metadata.

This is useful to control what origin about-to-be-added sections receive.

Source
pub fn meta_owned(&self) -> OwnShared<Metadata>
Similar to meta(), but with shared ownership.

Source
pub fn sections(&self) -> impl Iterator<Item = &Section<'event>> + '_
Return an iterator over all sections, in order of occurrence in the file itself.

Source
pub fn sections_and_ids(
    &self,
) -> impl Iterator<Item = (&Section<'event>, SectionId)> + '_
Return an iterator over all sections and their ids, in order of occurrence in the file itself.

Source
pub fn section_ids(&mut self) -> impl Iterator<Item = SectionId> + '_
Return an iterator over all section ids, in order of occurrence in the file itself.

Source
pub fn sections_and_postmatter(
    &self,
) -> impl Iterator<Item = (&Section<'event>, Vec<&Event<'event>>)>
Return an iterator over all sections along with non-section events that are placed right after them, in order of occurrence in the file itself.

This allows to reproduce the look of sections perfectly when serializing them with write_to().

Source
pub fn frontmatter(&self) -> Option<impl Iterator<Item = &Event<'event>>>
Return all events which are in front of the first of our sections, or None if there are none.

Source
pub fn detect_newline_style(&self) -> &BStr
Return the newline characters that have been detected in this config file or the default ones for the current platform.

Note that the first found newline is the one we use in the assumption of consistency.

Source
impl File<'static>
Source
pub fn resolve_includes(&mut self, options: Options<'_>) -> Result<(), Error>
Traverse all include and includeIf directives found in this instance and follow them, loading the referenced files from their location and adding their content right past the value that included them.

Limitations
Note that this method is not idempotent and calling it multiple times will resolve includes multiple times. It’s recommended use is as part of a multi-step bootstrapping which needs fine-grained control, and unless that’s given one should prefer one of the other ways of initialization that resolve includes at the right time.
Deviation
included values are added after the section that included them, not directly after the value. This is a deviation from how git does it, as it technically adds new value right after the include path itself, technically ‘splitting’ the section. This can only make a difference if the include section also has values which later overwrite portions of the included file, which seems unusual as these would be related to includes. We can fix this by ‘splitting’ the include section if needed so the included sections are put into the right place.
hasconfig:remote.*.url will not prevent itself to include files with [remote "name"]\nurl = x values, but it also won’t match them, i.e. one cannot include something that will cause the condition to match or to always be true.
Source
impl File<'_>
Source
pub fn to_bstring(&self) -> BString
Serialize this type into a BString for convenience.

Note that to_string() can also be used, but might not be lossless.

Source
pub fn write_to_filter(
    &self,
    out: &mut dyn Write,
    filter: impl FnMut(&Section<'_>) -> bool,
) -> Result<()>
Stream ourselves to the given out in order to reproduce this file mostly losslessly as it was parsed, while writing only sections for which filter returns true.

Source
pub fn write_to(&self, out: &mut dyn Write) -> Result<()>
Stream ourselves to the given out, in order to reproduce this file mostly losslessly as it was parsed.

Trait Implementations
Source
impl<'event> Clone for File<'event>
Source
fn clone(&self) -> File<'event>
Returns a duplicate of the value. Read more
1.0.0 · Source
const fn clone_from(&mut self, source: &Self)
Performs copy-assignment from source. Read more
Source
impl<'event> Debug for File<'event>
Source
fn fmt(&self, f: &mut Formatter<'_>) -> Result
Formats the value using the given formatter. Read more
Source
impl<'event> Default for File<'event>
Source
fn default() -> File<'event>
Returns the “default value” for a type. Read more
Source
impl Display for File<'_>
Source
fn fmt(&self, f: &mut Formatter<'_>) -> Result
Formats the value using the given formatter. Read more
Source
impl From<File<'_>> for BString
Source
fn from(c: File<'_>) -> Self
Converts to this type from the input type.
Source
impl FromStr for File<'static>
Source
type Err = Error
The associated error which can be returned from parsing.
Source
fn from_str(s: &str) -> Result<Self, Self::Err>
Parses a string s to return a value of this type. Read more
Source
impl PartialEq for File<'_>
Source
fn eq(&self, other: &Self) -> bool
Tests for self and other values to be equal, and is used by ==.
1.0.0 · Source
const fn ne(&self, other: &Rhs) -> bool
Tests for !=. The default implementation is almost always sufficient, and should not be overridden without very good reason.
Source
impl<'a> TryFrom<&'a BStr> for File<'a>
Source
fn try_from(value: &'a BStr) -> Result<File<'a>, Self::Error>
Convenience constructor. Attempts to parse the provided byte string into a File. See Events::from_bytes() for more information.

Source
type Error = Error
The type returned in the event of a conversion error.
Source
impl<'a> TryFrom<&'a str> for File<'a>
Source
fn try_from(s: &'a str) -> Result<File<'a>, Self::Error>
Convenience constructor. Attempts to parse the provided string into a File. See Events::from_str() for more information.

Source
type Error = Error
The type returned in the event of a conversion error.
Source
impl<'event> Eq for File<'event>
Auto Trait Implementations
impl<'event> Freeze for File<'event>
impl<'event> RefUnwindSafe for File<'event>
impl<'event> !Send for File<'event>
impl<'event> !Sync for File<'event>
impl<'event> Unpin for File<'event>
impl<'event> UnwindSafe for File<'event>
Blanket Implementations
Source
impl<T> Any for T
where
    T: 'static + ?Sized,
Source
impl<T> Borrow<T> for T
where
    T: ?Sized,
Source
impl<T> BorrowMut<T> for T
where
    T: ?Sized,
Source
impl<T> CloneToUninit for T
where
    T: Clone,
Source
impl<T> From<T> for T
Source
impl<T, U> Into<U> for T
where
    U: From<T>,
Source
impl<T> Same for T
Source
impl<T> ToOwned for T
where
    T: Clone,
Source
impl<T> ToString for T
where
    T: Display + ?Sized,
Source
impl<T, U> TryFrom<U> for T
where
    U: Into<T>,
Source
impl<T, U> TryInto<U> for T
where
    U: TryFrom<T>,
