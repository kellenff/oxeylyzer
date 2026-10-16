---
title: Corpus Cleaner Direction
type: concept
sources: [S012]
updated: 2026-10-06
---

`CorpusCleaner` maps **corpus character → keystroke sequence**, never the reverse (S012).
The map is `HashMap<char, Vec<char>>` — one input `char` to the keys that produce it —
and every read in `CorpusCleanerIterator::next` looks up a single input char
(`oxeylyzer-core/src/corpus_cleaner.rs:9`, `:426`) (S012).

So corpus `A` becomes keystrokes `⇑a`, and corpus `!` becomes `⇑1` (S012). The shipped
test at `corpus_cleaner.rs:540` asserts this contract directly:
`"AABcd :dof:;"` → `"⇑aabcd⇑;d⇑;;"` (S012).

## Why this needed writing down

An ingest-worthy bug report claimed `with_dead_key` was inverted: that corpus `"´a"`
should collapse to `"á"` rather than `"á"` expanding to `"´a"` (S012). That reading
inverts the contract above. Executed against the crate, the report's *expected* behavior
does not exist — corpus `"´a"` yields two `U+FFFD` replacement characters, because `´`
and `a` are each absent from the map (S012). Unmapped input is replaced, not folded (S012).

Two gaps in the helper's own documentation made the misreading easy, and together they
are the real defect (S012):

- The docstring at `corpus_cleaner.rs:272` describes the mapping without naming which
  side is input — "Adds mappings that involve a dead key followed by another character"
  (S012). Read against a cleaner that folds keystrokes, the same sentence supports the
  opposite direction.
- The doctest under it (`:276-281`) calls `.build()` and asserts nothing about the
  result, so it passes whichever way the mapping goes (S012). An example with no
  assertion pins no behavior.

The report also quoted a docstring line — `// i.e. corpus "á" should become "a"` — that
does not exist in the source; confirmed by grep and against commit `13327a7`, which
added the bare example with no inline comment (S012).

## `with_dead_key` is a specialized `multiple`

`multiple` (README: `to_multiple`) takes pairs of "a single character, and the sequence
of keys this character should be converted into", with the worked example
`["ç", "*c"]` — corpus `ç` → keystrokes `*c` (`README.md:177,179`) (S012).
`with_dead_key([('á','a')], '´')` builds a cleaner whose output for corpus `"á"` is
identical to what `with_mappings([('á', vec!['´','a'])])` produces, confirmed by
execution (S012). The name denotes which character plays the dead-key role, not a
folding operation (S012).

Open and unresolved: since `multiple` already expresses the same mapping, whether
`with_dead_key` should exist at all, and whether dead keys should generate uppercase
forms given `multiple.uppercase_versions` already does exactly that
(`corpus_transposition.rs:34`) (S012).

## What is genuinely missing

Dead keys have **no config surface**: `CorpusConfig`
(`oxeylyzer-repl/src/corpus_transposition.rs`) has no dead-key field, and its
`From<CorpusConfig> for CorpusCleaner` impl leaves a `// TODO: add proper dead key
mapping` at `corpus_transposition.rs:225` (S012). So no published corpus config
can reach `with_dead_key` (S012). That is a feature gap, not a bug (S012).

Custom shift and repeat char are **not** missing, contrary to the same TODO's wording:
`.shift_char(...)` and `.repeat_key(...)` are wired at `corpus_transposition.rs:243-244`
and consumed at `corpus_cleaner.rs:396-397` (S012).

Related: [Why Coverage Is Scoped By Measurement](./why-coverage-is-scoped-by-measurement.md) ·
[The Config Loader Is The Contract](./config-loader-is-the-contract.md) ·
[Probing Crate Behavior From A Scratch Crate](./probing-crate-behavior-from-a-scratch-crate.md) ·
[Why Corpus Load Stays Blocking](./why-corpus-load-stays-blocking.md)