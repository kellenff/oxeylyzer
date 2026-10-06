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