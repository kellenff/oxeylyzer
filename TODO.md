# TODO

Findings that are real but not scheduled. Anything here was investigated and
understood — this is not a dumping ground for suspicions.

## `with_dead_key` is inverted and contradicts its own docs

**Status**: confirmed bug, not yet fixed
**Where**: `oxeylyzer-core/src/corpus_cleaner.rs:282` (definition), and the
unwired TODO at `oxeylyzer-repl/src/corpus_transposition.rs:225`

The builder stores mappings as `HashMap<char, Vec<char>>` — one input char mapped
to output chars. Dead keys need the opposite: a two-char *sequence* matched against
a window. The implementation cannot express that:

```rust
// with_dead_key([('á', 'a')], '´') currently does:
mappings['á'] = ['´', 'a']      // corpus "á" emits "´a"  (composes plain into dead key)

/// docstring above it says:
/// .with_dead_key([('á', 'a')], '´')   // i.e. corpus "á" should become "a"
```

The documented contract and the behavior are inverses of each other. Because the
map is keyed on a single char, a typed `´a` sequence can never match it.

### Why it matters

Corpus cleaning decides which characters get counted, and therefore every score
the analyzer reports. A corpus containing composed characters (`á`) is currently
emitted as the dead-key sequence (`´a`), which corrupts counts for any language
written with diacritics — Czech, French, Esperanto, Hungarian, Vietnamese, and so
on among the published corpus configs.

### Why it was never caught

`with_dead_key` has no callers and no test. It is reachable only if someone
configures it, which nothing currently does. The dead-key TODO in
`corpus_transposition.rs:225` is accurate: there is no config surface for dead keys
either, so the broken helper is unreachable today. It becomes a live bug the moment
that surface is added.

### The machinery already exists

`CorpusCleanerIterator` carries a `SlidingWindow<char>` (line 415) used for
`repeat_key` multi-character matching, enabled at line 514. Sequence matching for
dead keys needs the same window, which is why this is a wiring and direction fix
rather than new infrastructure.

### To fix

1. Decide the direction and make code match documentation: `with_dead_key` should
   map a dead-key sequence to the plain character, per its docstring.
2. Extend sequence matching so a two-char input can match, reusing the existing
   sliding window.
3. Add a `dead_key` section to `CorpusConfig` (`corpus_transposition.rs:112`) and
   wire it at the `From<CorpusConfig> for CorpusCleaner` impl (line 214), clearing
   the TODO at line 225.
4. Test the direction explicitly — a test that reads
   "input `´a` yields `a`" is what would have caught the inversion.

### Open design question

Whether dead keys should also generate uppercase forms (`Á`). The `multiple`
mapping table already does this via `uppercase_versions`, and two mechanisms
solving the same problem may not be worth it. Worth settling before writing the
spec.

### Correction to the original queue entry

The queue entry that produced this finding said dead keys, custom shift, and
repeat char were all missing. Only dead-key mapping is missing. Custom shift and
repeat char are implemented and wired (`.shift_char(config.shift_key.key())` and
`.repeat_key(config.repeat_key)` at lines 233–234, consumed at
`corpus_cleaner.rs:388` and `:397`). The original entry was written from the TODO
comment's wording rather than from the code.

## A single `..` after the archive prefix escapes the data directory

**Status**: confirmed bug, not yet fixed
**Where**: `oxeylyzer-resources/src/download.rs:90-103`

The extraction loop checks containment and strips the prefix in the wrong order.
`zip` 2.4.2's `enclosed_name()` counts components with a depth counter
(`types.rs:542-551`), so the `oxeylyzer-data-files/` prefix contributes one level
of depth. An entry of `oxeylyzer-data-files/../escaped.txt` validates at depth 1,
decrements to 0, and is accepted. The prefix is then stripped, leaving
`../escaped.txt`, and `dest.join()` writes it one level *above* the destination.

```rust
let enclosed = file.enclosed_name().ok_or_else(|| ...)?;   // accepts "../escaped.txt"
let raw_name = enclosed.to_string_lossy();
let stripped = raw_name.strip_prefix(ZIP_INNER_PREFIX).unwrap_or(&raw_name);
let out_path = dest.join(stripped);                          // <data_root>/../escaped.txt
```

Confirmed by adding the case as a temporary test to the real crate: it failed on
`assert!(!ok)`, and the file appeared outside the destination. Measured against
`zip` 2.4.2, only the single-`..` case differs between the two orders — two or more
`..` are rejected either way.

### Why it matters

This is the app's only path from untrusted remote data to the filesystem. The
archive is fetched from GitHub on first run and nothing verifies its contents, so
a crafted archive writes a file anywhere the user can write on the first launch.
That is arbitrary file write with the user's own privileges, from input the spec
treats as hostile (FR-001, FR-002, SC-001).

### Why it was never caught

Every traversal test uses two or three `..` segments, both of which the shipped
order rejects. `traversal_escaping_root_is_rejected` asserts on `../../../escaped.txt`
and `mixed_valid_and_hostile_fails_without_partial_files` on `../../escape.txt`. All
34 tests pass against the vulnerable code. The `..` count in those tests is itself
an untested choice — the order-sensitive shape is one `..`.

The in-code comment at lines 85-89 states the correct rule ("the prefix is stripped
only afterwards… validating the whole name first would let
`oxeylyzer-data-files/../../etc/passwd` through") while the code below it does the
opposite. The comment's own example is two `..`, which is rejected either way, so
it never demonstrates the hazard it warns about.

### The spec already had this right

`specs/001-archive-safety/plan.md` prescribes strip-then-check, with the depth
mechanism explained. `tasks.md` T002 prescribes the opposite ("strip
`ZIP_INNER_PREFIX` from the *enclosed* path, after the containment check"), and the
implementation followed T002. Worth noting before anyone re-reads the spec: T002 is
wrong, and the plan is the correct reading.

### To fix

1. Strip `ZIP_INNER_PREFIX` from the entry name *before* validating, then validate
   the remainder. `enclosed_name()` takes no subpath argument, so this means either
   a second `ZipFile` over the same entry or an inline component walk with the
   same depth rule — hand-rolling the walk is what `plan.md` rejected in favor of
   the library, but the library cannot express this particular check.
2. Correct the in-code comment to match whichever order lands, and fix the
   example — use a single `..` or the comment keeps not demonstrating anything.
3. Add the single-`..` case as a test asserting on the filesystem, not the return
   value. This is the test whose absence let the ordering bug through; the existing
   mutation check in `tasks.md` proves the guard is load-bearing only for the shapes
   it exercises.
4. Correct `tasks.md` T002 in a separate change — spec artifacts are immutable to
   the wiki ingest command, so it has to be done deliberately.

### Wiki record

`wiki/pages/archive-path-containment.md` (sources S002, S003). Re-ingesting that
source before the fix lands will re-record the breach as current.