# Bug Verification: displayed ranking equals the search objective

- **Slug**: score-stretch-divergence
- **Tested**: 2026-10-13T05:47:13Z
- **Assessment**: ./assessment.md
- **Fix**: ./fix.md
- **Result**: verified

## Summary

The original symptom reproduces at HEAD and is gone after the fix, demonstrated by an
independent A/B run rather than by re-reading the fix. On a freshly generated pool,
pre-fix (HEAD `ae333d8`) top-10 overlap between the display standard and the search
objective was 2/10 with the displayed #1 ranking 91st by the objective; post-fix it is
10/10 with the displayed #1 also the objective #1 and zero score gap on every layout.
The assessment's own regression test fails at HEAD and passes with the fix. All suites
green, no regressions, no new lint or format findings.

## Checks Performed

| Check | Command / Action | Result | Notes |
|-------|------------------|--------|-------|
| Reproduction (pre-fix, HEAD) | `rank_divergence <head-tree> 2500` in a detached worktree at `ae333d8` | pass (symptom present) | overlap 2/10; displayed #1 is objective #91; max abs gap 27,101,683,955 |
| Reproduction (post-fix) | same binary built from the patched tree, `rank_divergence <post-tree> 2500` | pass (symptom gone) | overlap 10/10; displayed #1 is objective #1; max abs gap 0 |
| Assessment's regression test at HEAD | `cargo test -p oxeylyzer-core --lib score_matches_search_objective` in HEAD worktree with the new test spliced in | **fail** | `left: -19378971018 right: -31833064819` — test is genuinely pinned to the fix |
| New / updated tests | `cargo test -p oxeylyzer-core score_matches_search_objective` | pass | 1 passed, 0 failed |
| Regression suite | `cargo test-all` | pass | core 51, repl 9, resources 39, bench 7, doctests 114; 0 failures |
| Per-crate suites | `cargo test -p <crate>` for all five | pass | counts as above; tauri 0 tests |
| Lint / type-check | `cargo clippy --all-targets --all-features -- -D warnings` | fail (pre-existing) | 1 warning (`repl.rs:759` redundant reference), identical at HEAD; not from `generate.rs` |
| Format check | `cargo fmt --all -- --check` | skipped | repo-wide drift pre-existing at HEAD; not a regression signal |
| REPL reproduction step 1 | `printf 'generate qwerty 30\nexit\n' \| cargo run -q --release` | pass | top-10 sort strictly descending under the corrected standard |
| Bench | `cargo bench --bench generation` | not-run | panics at `benches/generation.rs:126` in both trees — missing `.../oxeylyzer/static/text/bokmal`; scoped out by the assessment |

## Output Excerpts

Independent reproducer (`rank_divergence`, built in two throwaway worktrees — HEAD vs
HEAD+fix diff; no repo file modified):

```unknown
pre-fix (HEAD ae333d8)                  post-fix
pool_size                        2500   pool_size                        2500
top10_overlap                    2/10   top10_overlap                    10/10
display_1_objective_rank         91     display_1_objective_rank         1
objective_1_display_rank         8      objective_1_display_rank         1
max_abs_gap_score_vs_objective   27101683955   max_abs_gap_score_vs_objective   0
```

The bottom line is the load-bearing one: the pre-fix gap is ~27.1e9 integer points
between the two standards; post-fix it is exactly 0 across all 2500 layouts.

Assessment's regression test against pre-fix HEAD (test spliced into the HEAD worktree,
body otherwise untouched):

```unknown
assertion `left == right` failed: score() must equal the search objective
  left: -19378971018
 right: -31833064819
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 50 filtered out
```

Post-fix:

```unknown
running 1 test
test generate::tests::score_matches_search_objective ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 50 filtered out
```

Full suite (post-fix): `oxeylyzer-core` 51 + 114 doctests, `oxeylyzer-repl` 9,
`oxeylyzer-resources` 39, `oxeylyzer-bench` 7 — 0 failures.

## Residual Risks

- **The measurement is on a fresh pool, and pre-fix numbers differ from the assessment's.**
  The assessment recorded 0/10 overlap and objective-rank #254 over its 2500-layout pool;
  the same binary shape over a new 2500-layout pool gives 2/10 and #91. The qualitative
  conclusion is unchanged and, if anything, sharper (the top-10s are still not the same
  set), but the exact figures are pool-dependent and the assessment's specific #254 is
  not reproduced. The post-fix identity is exact (gap 0) and pool-independent.
- **No display-surface test pins the sort.** The fix relies on the three ranking callers
  (`display.rs:58`, `repl.rs:348`, Tauri `lib.rs:1000`) continuing to call `score()`.
  Coverage is at the `score()` level, not the sort level; a future caller that re-sorts
  by `score_with_precision(l, usize::MAX)` would reopen the divergence without failing
  the new test. This matches the fix's stated scope.
- **`analyze` still reports a different total** (`get_layout_stats` uses
  `score_with_precision(l, usize::MAX)`, `generate.rs:371`). Now that `score()` is the
  configured-precision standard, rank and analyze visibly differ by trigram count only.
  Deliberately out of scope; carried in the assessment's open questions.
- **REPL reproduction was exercised, not the GUI.** The Tauri list path shares the same
  `score()` call and could not be launched headlessly here; its correctness follows from
  the response path but was not clicked through.
- **Format and one lint warning are pre-existing.** `cargo fmt --check` is red at HEAD
  (repo-wide drift, including a region of `generate.rs` the fix did not touch; the fix's
  own added lines are fmt-clean), and clippy's `repl.rs:759` warning exists at HEAD. The
  CI `-D warnings` gate would fail at HEAD too; not a regression from this fix.

## Recommendation

Close the bug — verified end-to-end by execution, not by inspection. The fix removes the
divergence by construction (the same function participates in both the display and search
paths), the added test fails without the fix and passes with it, and no existing suite,
lint, or format signal regressed. Before closing, note that no test pins the *sort sites*
themselves, and that `analyze`'s reported total remains intentionally on the
`usize::MAX` standard; both are recorded above and in the assessment's open questions,
not blockers.