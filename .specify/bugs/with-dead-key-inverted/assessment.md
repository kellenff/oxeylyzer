# Bug Assessment: `with_dead_key` is not inverted — report misreads the cleaner's direction

- **Slug**: with-dead-key-inverted
- **Created**: 2026-10-06T23:57:32Z
- **Supersedes**: the initial assessment of the same slug, written 2026-10-06 before
  the direction claim below was checked against the codebase. That assessment returned
  `valid` / `low` and proposed inverting `with_dead_key`; the proposed fix was then
  disproven during `/speckit.bug.fix` and no code was changed. See `./fix.md`.
- **Source**: pasted text (originating from `TODO.md`, itself seeded by the
  `dead-key-mapping` entry in `.specify/extensions/time-machine/features-queue.yml`)
- **Verdict**: invalid
- **Severity**: low

## Report (summarized)

The report claims `oxeylyzer-core`'s `CorpusCleanerBuilder::with_dead_key` is inverted
relative to its documented contract. Its argument: the implementation stores
`from -> vec![dead_key, to]`, so a corpus character is *expanded* into a dead-key
sequence, when dead keys should work the opposite way — a two-character typed sequence
collapsing to one character. It grounds the "documented contract" in a docstring line
`/// .with_dead_key([('á', 'a')], '´')   // i.e. corpus "á" should become "a"`.

Supporting claims: the `HashMap<char, Vec<char>>` key type cannot express a two-char
input sequence, so dead-key matching is impossible; the helper has zero callers and
zero tests; `CorpusConfig` has no dead-key field, making it unreachable; and
`CorpusCleanerIterator`'s `SlidingWindow<char>` (line 415) shows the sequence-matching
machinery exists.

Severity claimed: high, on the grounds that corpus cleaning feeds every score and
several published configs use diacritic languages.

## Symptom

As reported: `.with_dead_key([('á', 'a')], '´')` yields `map['á'] = ['´', 'a']`, so
corpus text `"á"` is emitted as `"´a"`, and the report expected corpus `"´a"` to
collapse to `"á"`.

As found: the reported behavior is the intended behavior. `CorpusCleaner` translates
corpus text into keystroke sequences, so corpus `"á"` must become keystrokes `"´a"` —
press the dead key, then `a`. There is no symptom. The expected behavior asserted by
the report is the one that would be the defect.

## Reproduction

None exists. The report's own precondition refutes it — it concedes the helper is
unreachable (no callers, no config field), so no configuration can exhibit the
reported "symptom". Reading the code settles the direction question:

1. `CorpusCleaner` holds `map: HashMap<char, Vec<char>>` (`oxeylyzer-core/src/corpus_cleaner.rs:9`)
   — input is one corpus `char`, output is a keystroke sequence. Every read of this
   map in `CorpusCleanerIterator::next` (`:426`) looks up a single input char.
2. The passing test at `corpus_cleaner.rs:540` asserts
   `corpus "AABcd :dof:;" -> "⇑aabcd⇑;d⇑;;"`: corpus `A` becomes keystrokes `⇑a`.
   Expansion is the established contract, not a bug.
3. `qwerty_punctuation_mappings([('1','!')])` (`:307`) → `include_as_uppercase` →
   `shifted_chars['!'] = '1'` → `build()` produces `map['!'] = ['⇑','1']` (`:388`).
   Corpus `!` becomes shift+1. Identical shape to `map['á'] = ['´','a']`.
4. `with_dead_key` (`:282`) does `self.with_mapping((from, vec![dead_key, to]))` (`:290`),
   giving `map['á'] = ['´','a']` for the report's example — same shape again.
5. `README.md:177` documents the same table (`to_multiple`, now `multiple`): "a single
   character, and the sequence of keys this character should be converted into... very
   useful for languages like Spanish or French where you can use an accent key
   (denoted by `*`) to collapse all kinds of accented keys into a single key + letter."
   Its worked example at `README.md:179` is `["ç", "*c"]` — corpus `ç` → keystrokes
   `*c`, precisely the behavior `with_dead_key` implements.
6. `with_dead_key` is therefore a specialized `multiple`: the pairs are
   `(corpus_char, base_letter)` and it prefixes the dead key. The name denotes which
   character plays the dead-key role, not "collapse sequences starting with this key."

[NEEDS CLARIFICATION: The docstring line the report quotes as its contract —
`// i.e. corpus "á" should become "a"` — does not exist in the source. Verified by
grep across the repo and against the commit that introduced the docstring
(`13327a7 add docs and doctests using gemini`, which added the bare example with no
inline comment). The real docstring at `corpus_cleaner.rs:272` is a single sentence
with no stated direction. See "Actual defect".]

## Suspected Code Paths

- `oxeylyzer-core/src/corpus_cleaner.rs:282` — `with_dead_key`, correctly implements
  corpus-char → dead-key + letter. **Not** a defect.
- `oxeylyzer-core/src/corpus_cleaner.rs:272` — docstring, the real finding: does not
  state direction, and is ambiguous enough to have produced this false report.
- `oxeylyzer-core/src/corpus_cleaner.rs:276-281` — doctest, the real finding: builds a
  cleaner and asserts nothing, so it passes under any behavior.
- `oxeylyzer-core/src/corpus_cleaner.rs:9` and `:540` — the `HashMap<char, Vec<char>>`
  key type and the expansion test that together establish the correct direction.
- `README.md:177,179` — documents the identical direction for `to_multiple` / `multiple`,
  with the `["ç", "*c"]` example.
- `oxeylyzer-repl/src/corpus_transposition.rs:225` — `// TODO: add proper dead key
  mapping`. A real feature gap, but out of scope for a bug verdict and not a regression.
- `.specify/extensions/time-machine/features-queue.yml` (`dead-key-mapping`) — the
  queue entry that seeded this report; carries the same inverted claim.

## Root Cause Hypothesis

The report inverts the cleaner's contract. `CorpusCleanerBuilder` maps **corpus
character → keystroke sequence**; the report reads it as **keystroke sequence → corpus
character** and therefore judges correct code to be backwards. Its structural argument
is sound reasoning about the wrong thing: it is true that `HashMap<char, Vec<char>>`
cannot match a two-char *input* sequence, but nothing in the cleaner consumes
multi-char input. The corpus is text; the output is keystrokes. The `SlidingWindow<char>`
at `:415` exists for `repeat_key`, where an input char is compared against the previous
input char — not for sequence matching — so its presence does not support the report.

The false premise originates in the undocumented direction. The docstring says only
"Adds mappings that involve a dead key followed by another character", which a reader
can take either way, and the doctest asserts nothing to settle it. A reader who assumes
the cleaner folds sequences gets an inversion that is entirely imaginary. Confidence:
high — the direction is confirmed by a passing test, by the qwerty mapping's own code
path, and by the README's worked example.

### Actual defect

Documentation-only, and minor: `with_dead_key`'s docstring should state its direction
explicitly (corpus `"á"` becomes keystrokes `"´a"`), and its doctest should assert that
output. Nothing else about the helper is wrong.

### Originating entry is unreliable

`.specify/extensions/time-machine/features-queue.yml` records that this entry was
previously wrong in a second, independent way: its comment claimed custom shift and
repeat char were unimplemented, which was false — `.shift_char(config.shift_key.key())`
and `.repeat_key(config.repeat_key)` are wired at `corpus_transposition.rs:243-244` and
consumed at `corpus_cleaner.rs:396-397`. That correction is the one part of the
original report that survives scrutiny. Two wrong claims in one entry is worth weighing
before trusting its third.

## Proposed Remediation

**Preferred**: documentation only — do not change behavior. Amend the docstring at
`oxeylyzer-core/src/corpus_cleaner.rs:272` to state the direction ("maps a corpus
character to the dead key followed by its base letter; e.g. corpus `á` becomes
keystrokes `´a`"), and give the doctest at `:276-281` a real assertion so the
direction is pinned. That is the whole fix. It closes the gap that produced the false
report without touching working code.

**Alternatives**:
- Leave the code untouched and only correct the queue entry and `TODO.md`, which both
  still assert the inversion. Cheapest, but the ambiguous docstring stays and the next
  reader can re-derive the same false report.
- Wire the helper to `CorpusConfig` as the report's step 3 proposes, since a dead-key
  section genuinely does not exist. That is a feature addition, not this bug's remedy,
  and it should not be bundled with a documentation fix. It also carries the real open
  design question about uppercase forms, which `multiple.uppercase_versions`
  (`corpus_transposition.rs:34`) may already solve — two mechanisms for one job.

**Files likely to change**:
- `oxeylyzer-core/src/corpus_cleaner.rs` (docstring + doctest only)
- `.specify/extensions/time-machine/features-queue.yml` (correct the entry's claim)
- `TODO.md` (retract or rewrite the finding)

**Tests to add or update**:
- An assertion in the `with_dead_key` doctest: corpus `"á"` → `"´a"`. The absence of
  any assertion is why the direction was never pinned, so this is the deliverable.
- Optionally a unit test in the existing `tests` module mirroring `repeat_key`
  (`:544`), to keep the direction explicit in test code and not only in a doc example.

## Risks & Considerations

- **The obvious "fix" is a regression.** Flipping `with_dead_key` to collapse `´a` → `á`
  would silently invert the mapping for every future caller and would contradict
  `multiple`, `qwerty_punctuation_mappings`, and the README. Anyone reading the original
  report without this assessment should be told explicitly.
- **Do not add sequence matching to satisfy the report.** Re-keying `map` to handle
  multi-char input would be a large change to the shift/repeat logic that currently
  works, in service of a capability nothing needs.
- **Scope discipline.** `corpus_transposition.rs:225` is a legitimate feature gap. Fixing
  it here would conflate a non-bug with a feature and expand review surface for a
  one-line doc change.
- **Blast radius of the real defect is low but nonzero.** No user-visible behavior is
  wrong today. The cost is misdirected engineering time: this report consumed an assess
  run, a fix run, and produced a proposed patch that would have broken working code.

## Open Questions

- [NEEDS CLARIFICATION: Should `with_dead_key` even exist, given `multiple` with a
  `dead_key`-prefixed string already expresses it? `with_dead_key([('á','a')], '´')` is
  roughly `multiple: á = "´a"`. Consolidating would remove the ambiguity at its source
  rather than document around it.]
- [NEEDS CLARIFICATION: Should `TODO.md` and the `dead-key-mapping` queue entry be
  corrected, and by whom? Both still assert the inversion. I did not edit them — the
  assess command writes only inside `.specify/bugs/<slug>/`.]