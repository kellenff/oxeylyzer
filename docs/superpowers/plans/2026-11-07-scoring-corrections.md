# Scoring Corrections Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Resolve issues #5–#8 without changing established nonzero-SFB scoring semantics.

**Architecture:** Correct corpus preprocessing in `AnalyzerData` and share the complete score and unweighted Pinky-Ring count in `Oxeylyzer`. Existing generation and display callers keep their interfaces.

**Tech Stack:** Rust, Cargo, existing core test fixtures.

**Spec:** `docs/superpowers/specs/2026-11-07-scoring-corrections-design.md`

## Global Constraints

- Preserve optimizer precision and cached score consistency.
- Preserve existing nonzero-SFB arithmetic, including rounding behavior.
- Add no dependencies or unrelated scoring changes.

## Review Focus

- Tied trigram counts with differently ordered character maps must produce the same character order (Task 1).
- A precision of zero and a precision exceeding the trigram count must select zero and all entries respectively (Task 1).
- Stretch-sensitive layouts must produce the same full score through both public paths (Task 2).
- Zero Pinky-Ring weight must retain a finite, unchanged statistic for a corpus with bigrams (Task 2).
- Zero SFB with zero and nonzero skipgram weights must handle all three distances and stretch without non-finite intermediates (Task 3).

### Task 1: Deterministic trigram selection

**Files:** Modify and test `oxeylyzer-core/src/analyzer_data.rs`.

**Interfaces:** Preserve `AnalyzerData::new(Data, &Weights)` and `gen_trigrams()`.

- [ ] Add a fixture with unequal frequencies and equal-frequency trigrams in different insertion orders; assert descending counts, character-lexicographic tie order, and correct zero, limited, and full prefixes after decoding IDs.
- [ ] Run the focused test with `RUSTC_WRAPPER= cargo test -p oxeylyzer-core <test_name>` and confirm it fails on current ordering.
- [ ] Sort by frequency descending and original character sequence ascending before conversion to mapping IDs; retain existing count conversion.
- [ ] Rerun the focused test and confirm it passes.

### Task 2: Full score and descriptive Pinky-Ring statistic

**Files:** Modify and test `oxeylyzer-core/src/generate.rs`.

**Interfaces:** Preserve `score(&FastLayout) -> i64`, `score_with_precision(&FastLayout, usize) -> i64`, and `get_layout_stats(&FastLayout) -> LayoutStats`.

- [ ] Add a stretch-sensitive layout regression asserting `score(layout) == score_with_precision(layout, usize::MAX) == get_layout_stats(layout).score`; assert the stretch contribution is nonzero. Add a Pinky-Ring regression comparing zero and nonzero weights on identical corpus/layout data and asserting a finite, equal, nonzero statistic, including JSON serialization.
- [ ] Run the focused tests with `RUSTC_WRAPPER= cargo test -p oxeylyzer-core <test_name>` and confirm the relevant assertions fail.
- [ ] Delegate full scoring to the precision-aware implementation, retaining test instrumentation; extract the unweighted pair count and use it for the statistic and weighted score.
- [ ] Rerun the focused tests and existing cached-score tests; confirm they pass.

### Task 3: Zero SFB weighting

**Files:** Modify and test `oxeylyzer-core/src/analyzer_data.rs`.

**Interfaces:** Preserve the existing same-finger and stretch weighted-bigram accessors.

- [ ] Add a small corpus fixture with distinct direct and three skip-distance counts. Assert exact symmetric weighted values for `sfbs = 0`, `sfs = -1`, `stretches = -0.3`: same-finger is minus the sum of skip counts; stretch is minus 0.3 times the sum of direct and skip counts, with existing final integer conversion. Assert zero `sfs` excludes all skip counts while retaining direct stretch. Also assert representative nonzero-SFB results against the existing formula, including fractional rounding.
- [ ] Run the focused test with `RUSTC_WRAPPER= cargo test -p oxeylyzer-core <test_name>` and confirm the zero-SFB assertions fail.
- [ ] Add only the zero-SFB branch described in the spec; leave the nonzero branch's arithmetic unchanged.
- [ ] Rerun the focused tests and confirm they pass.

### Task 4: Integrated verification

**Files:** All changed files; no new interfaces.

- [ ] Trace GUI and CLI ranking callers and confirm they use the corrected full score; verify the optimizer still uses configured precision and cache tests cover it.
- [ ] Run `cargo fmt --all -- --check` and `RUSTC_WRAPPER= cargo test -p oxeylyzer-core`; report any environment or runtime limitations.
- [ ] Inspect the complete diff for unintended changes and document the changed zero-SFB and ranking behavior in the final report.
