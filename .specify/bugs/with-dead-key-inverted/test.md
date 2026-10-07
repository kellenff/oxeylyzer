# Bug Verification: with_dead_key is not inverted — no fix applied, verdict upheld

- **Slug**: with-dead-key-inverted
- **Tested**: 2026-10-06
- **Assessment**: ./assessment.md
- **Fix**: ./fix.md
- **Result**: verified

## Summary

The fix is `not-applied` because there was nothing to fix, and this run confirms that:
the `invalid` verdict holds under direct execution, not just code reading. `with_dead_key`
composes corpus `"á"` into keystrokes `"´a"`, which is correct — the cleaner's contract
is corpus character → keystroke sequence. Critically, the report's *expected* behavior
(`"´a"` → `"á"`) produces replacement characters, so the behavior it wanted is the
behavior that does not exist. All 173 tests across both crates pass; no regressions.

## Checks Performed

| Check | Command / Action | Result | Notes |
|-------|------------------|--------|-------|
| Reproduction (post-fix) | scratch probe crate against `oxeylyzer-core` (outside repo) | pass | Report's symptom confirmed present and correct; report's expected behavior confirmed absent |
| New / updated tests | none exist | not-run | `fix.md` added no tests — no behavior changed |
| Regression suite (core) | `cargo test -p oxeylyzer-core` | pass | 50 unit + 114 doc tests, 0 failed |
| Regression suite (repl) | `cargo test -p oxeylyzer-repl` | pass | 9 tests incl. `existing_file_validity`, 0 failed |
| Lint / type-check | `cargo clippy -p oxeylyzer-core -p oxeylyzer-repl --all-targets` | pass | 2 pre-existing warnings in `fast_layout.rs:1367,1369`, unrelated to this slug |
| Format check | `cargo fmt -p oxeylyzer-core -p oxeylyzer-repl -- --check` | skipped | Pre-existing repo-wide drift in `generate.rs`; wiki has a page on formatting in this repo, so not treated as a regression signal here |
| Source integrity | `git status --short`, `git diff --stat` | pass | Only `.specify/bugs/` untracked; `git diff` empty |

## Output Excerpts

Probe results (scratch crate, `deadkey-probe`, built outside the repo against
`oxeylyzer-core` by path — no repo file created or modified):

```
with_dead_key([('á','a')],'´')  "á"   -> "´a"
with_dead_key([('á','a')],'´')  "´a"  -> "\u{FFFD}\u{FFFD}"
qwerty_punctuation_mappings    "A"   -> "⇑a"
qwerty_punctuation_mappings    "!"   -> "⇑1"
with_mappings('á' -> ['´','a'])  "á"   -> "´a"
shift_char(None) identity      "A"   -> "A"
```

Suite summaries:

```
running 50 tests
test result: ok. 50 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
running 114 tests
test result: ok. 114 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out

running 9 tests   (oxeylyzer-repl, incl. corpus_transposition::tests::*)
test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

The second probe line is the load-bearing result. The report claimed corpus `"´a"`
should collapse to `"á"`; it yields `U+FFFD U+FFFD` (two replacement characters),
because `´` and `a` are each absent from the map. The behavior the report asked for is
demonstrably not the behavior present.

## What the probe establishes

Three independent confirmations of corpus → keystrokes, all executed rather than read:

- `with_dead_key` maps `"á"` → `"´a"`, the report's own example, and the direction the
  verdict turns on.
- `qwerty_punctuation_mappings` maps `"A"` → `"⇑a"` and `"!"` → `"⇑1"`. A single corpus
  char becomes multiple keystrokes. This is the shipped, tested contract — it also
  matches the pre-existing test at `corpus_cleaner.rs:540` asserting
  `"AABcd :dof:;"` → `"⇑aabcd⇑;d⇑;;"`.
- `with_mappings([('á', vec!['´','a'])])` produces output identical to `with_dead_key`,
  confirming the helper is a specialized `multiple` rather than an outlier. This matches
  `README.md:179`'s documented `["ç", "*c"]` example.

The `shift_char(None)` probe is a control: with no shift key, the marker disappears and
identity holds, confirming the `⇑` prefix is shift encoding rather than part of any
dead-key inversion.

## Residual Risks

- The verification confirms the *direction* claim. It cannot confirm the report author's
  intent, since no working corpus config uses dead keys and none can be constructed that
  demonstrates the collapse behavior — the feature is genuinely unwired at
  `corpus_transposition.rs:225`. If dead-key *matching* (folding a typed sequence) is ever
  wanted as a second capability, that is a new feature, not this bug.
- The clippy warnings at `fast_layout.rs:1367,1369` and the `cargo fmt` drift in
  `generate.rs` are pre-existing and untouched by this slug. Neither is a regression, and
  neither was investigated further.
- Two stale documents still assert the inversion — `TODO.md` and the `dead-key-mapping`
  entry in `.specify/extensions/time-machine/features-queue.yml` (which is additionally
  wrong about custom shift and repeat char). This verification did not modify them, per
  the command's guardrails. They remain the highest residual risk on this slug: they are
  what generated the false report, so leaving them invites a re-run.
- No test pins the `with_dead_key` direction, which is the residual documentation defect
  the assessment identified. Nothing here prevents a future reader from re-deriving the
  same inversion.

## Recommendation

Close the bug. The report is invalid: no fix was applied because none was warranted, and
this run verifies that conclusion by execution rather than by re-reading the code. The
verdict is `verified` in the specific sense that the assessment's reasoning and the
no-fix decision hold — not in the sense that a fix was applied and survived, since there
was none. Guardrail note: the assessment listed no reproduction that could be exercised,
because none exists; I did not manufacture one, and the post-fix probe explicitly
confirms the report's expected behavior is absent rather than claiming the original
symptom was reproduced and cleared.

Before closing, correct `TODO.md` and the `dead-key-mapping` queue entry, both of which
still assert the inversion. Optionally settle whether `with_dead_key` should be folded
into `multiple` to remove the ambiguity at its source. If a docstring direction plus an
asserting doctest are wanted, treat that as ordinary work outside this slug.