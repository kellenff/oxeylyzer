# Bug Fix: with_dead_key is not inverted — no fix applied (verdict: invalid)

- **Slug**: with-dead-key-inverted
- **Fixed**: 2026-10-06
- **Assessment**: ./assessment.md
- **Status**: not-applied
- **Written against**: the corrected assessment (verdict `invalid`, 2026-10-06T23:57:32Z).
  This file previously described a not-applied fix for the *superseded* assessment's
  inversion proposal. That status is unchanged; the reasoning below now also explains
  why the code is correct.

## Summary

No code was changed. The corrected assessment returned verdict `invalid`, so there is
nothing to fix. `CorpusCleanerBuilder` maps corpus character → keystroke sequence, and
`with_dead_key` composing corpus `"á"` into keystrokes `"´a"` is correct — press the
dead key, then `a`. The only real defect is documentation: the docstring never states
its direction and the doctest asserts nothing.

## Changes

| File | Change | Notes |
|------|--------|-------|
| — | none | No source file modified |

## Diff Highlights

None. Nothing to diff.

## Tests Added or Updated

None. No behavior changed, so there is nothing to characterize. The baseline suite
(`cargo test -p oxeylyzer-core --lib corpus_cleaner`) passes 3/3 and was left untouched.

## Local Verification

- Commands run: `cargo test -p oxeylyzer-core --lib corpus_cleaner` → 3 passed, 0 failed.
- Manual checks: read `with_dead_key` / `with_mapping` / `build()` / `next()` in full;
  traced `qwerty_punctuation_mappings` through `include_as_uppercase` into `build()` to
  confirm corpus `!` becomes `⇑1`; read the passing expansion test at
  `corpus_cleaner.rs:540` (corpus `A` → `⇑a`); read `README.md:177,179` for the
  documented direction of `to_multiple` and its `["ç", "*c"]` example; confirmed the
  docstring line the original report quoted as its contract does not exist in the
  source, by grep and against commit `13327a7`.
- Confirmed after writing: `git status --short` shows only `.specify/bugs/` untracked;
  `git diff --stat` is empty.

## Deviations from Assessment

None. The assessment's verdict is `invalid` and its proposed remediation is
documentation-only, so per step 1 of this command I stopped instead of applying it. No
judgment call was needed on scope.

The documentation-only change the assessment proposes (state the direction in the
docstring, give the doctest a real assertion) is **deliberately not applied here**.
Applying it while the verdict is `invalid` would mean acting on a bug that does not
exist. It is a good change and is recorded in the assessment's Proposed Remediation
section, ready to pick up as ordinary work rather than as a bug fix.

## Follow-ups

- **Open decision, worth settling before the docstring is written**: should
  `with_dead_key` exist at all? `multiple` already expresses the same thing —
  `with_dead_key([('á','a')], '´')` is roughly `á = "´a"` in a `multiple` table.
  Consolidating would remove the ambiguity at its source rather than document around
  it. The assessment raises this as an open question.
- **Two stale documents still assert the inversion**, both outside this command's
  write scope:
  - `TODO.md` — the original finding, including the non-existent docstring quote.
  - `.specify/extensions/time-machine/features-queue.yml`, entry `dead-key-mapping` —
    which is additionally wrong about custom shift and repeat char being
    unimplemented, a second independent error in one entry.
  These should be corrected by whoever owns them. They are the reason this false
    report exists, so leaving them is how the next one gets written.
- `corpus_transposition.rs:225` (`// TODO: add proper dead key mapping`) plus the
  missing `CorpusConfig` field is a genuine feature gap. It is not a bug, not part of
  this assessment, and should not be bundled with a documentation change.
- If a fix artifact is required for bookkeeping, `/speckit.bug.test slug=with-dead-key-inverted`
  is the next command, but there is nothing to test — no behavior differs from intended.