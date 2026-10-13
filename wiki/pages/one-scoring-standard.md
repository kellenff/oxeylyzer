---
title: One Scoring Standard
type: concept
sources: [S013]
updated: 2026-10-13
---

The app has one number that says which layout is best: the **search objective**,
`initialize_cache(layout).total_score()` — stretch included, trigrams truncated
at `trigram_precision` (S013). The optimizer maximizes it, and every ranking
surface must sort by it (S013).

Three surfaces rank generated layouts: REPL `generate`
(`oxeylyzer-repl/src/display.rs`), REPL `rank` (`oxeylyzer-repl/src/repl.rs`),
and the Tauri layout list (`oxeylyzer-tauri/src-tauri/src/lib.rs`) — all through
`Oxeylyzer::score()` (S013).

## The divergence

Before the fix, `score()` summed a stale formula — no `stretch_score` term and
every trigram (~57,680) instead of the configured precision — so the displayed
ranking disagreed with the objective the hill-climb maximized (S013). On a
freshly generated 2,500-layout pool the top-10s shared 2 layouts and the
displayed #1 ranked 91st by the objective (S013). The exact figures are
pool-dependent — the assessment recorded 0/10 and #254 — but the identity of the
standards is not (S013).

## The fix

`score()` is now a one-line forward to
`score_with_precision(layout, self.trigram_precision)`, so

`score(l) == score_with_precision(l, p) == initialize_cache(l).total_score()`

holds by construction, comparing exact `i64` totals (S013). The three ranking
call sites were not edited (S013). Four total-score formulas became three (S013).

## The standard that stays different

`analyze` / `get_layout_stats` still reports `score_with_precision(layout,
usize::MAX)` — all ~57,680 trigrams, not the configured precision (S013). Rank
and analyze therefore differ by trigram count only; this was deliberately left
out of the fix (S013).

The regression test `score_matches_search_objective` pins the identity on QWERTY
and after each of 200 accepted swaps (S013).

Related: [Scoring Cache And LayoutCache](./scoring-cache-and-layout-cache.md) ·
[Exact-Integer Scoring Is An Invariant](./exact-integer-scoring-is-an-invariant.md) ·
[Why obsolete.rs Stays](./why-obsolete-rs-stays.md)