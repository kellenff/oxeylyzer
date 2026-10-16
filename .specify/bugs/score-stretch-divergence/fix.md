# Bug Fix: displayed ranking now uses the search objective

- **Slug**: score-stretch-divergence
- **Fixed**: 2026-10-13T04:40:00Z
- **Assessment**: ./assessment.md
- **Status**: applied

## Summary

`Oxeylyzer::score()` — the function three ranking surfaces sort by — previously summed a stale formula (no stretch term, all ~57,680 trigrams) that disagreed with the objective the search optimized. It now returns exactly `score_with_precision(layout, self.trigram_precision)`, the standard `LayoutCache::total_score()` already pins. No call sites changed; the display surfaces became correct by construction. A regression test pins the identity on a fixed layout and across a swap sequence.

## Changes

| File | Change | Notes |
|------|--------|-------|
| `oxeylyzer-core/src/generate.rs` (`score()`, ~:498-523) | modified | Body is a one-line forward to `score_with_precision(self.trigram_precision)`; the old inline sum (usage+fspeed+pinky_ring+un-truncated trigrams, no stretch) deleted. `#[cfg(test)] ANALYZED_COUNT` increment preserved at the same site. Docstring replaced with one naming the standard. |
| `oxeylyzer-core/src/generate.rs` (`mod tests`, after `cached_totals`) | added test | `score_matches_search_objective` — asserts `score() == initialize_cache().total_score()` and `== score_with_precision(layout, trigram_precision)` on QWERTY, then re-asserts after each of 200 `accept_swap` steps. |

No other files touched. The three ranking callers (`display.rs:58`, `repl.rs:348`, Tauri `lib.rs:1000`), `get_layout_stats`/`analyze` (`usize::MAX` at `generate.rs:371`), bench, engines, and weights were left alone per the assessment's out-of-scope list.

## Diff Highlights

```rust
  pub fn score(&self, layout: &FastLayout) -> i64 {
      #[cfg(test)]
      ANALYZED_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

-     let fspeed_usage = Finger::FINGERS
-         .into_iter()
-         .map(|f| self.finger_usage(layout, f) + self.finger_fspeed(layout, f))
-         .sum::<i64>();
-     let pinky_ring = self.pinky_ring_score(layout);
-     let trigram_score = self.trigram_score_iter(layout, self.data.gen_trigrams());
-     trigram_score + fspeed_usage + pinky_ring
+     self.score_with_precision(layout, self.trigram_precision)
  }
```

The assessment offered this forwarding variant explicitly ("prefer it if it falls out that way") — it does, so the duplicate formula no longer exists anywhere outside `score_with_precision`. Four distinct total-score formulas in the product code are now three (`score` ≡ `score_with_precision(p=1000)` ≡ `cache.total_score()` at the configured precision; `analyze` remains the fourth, scoped out).

## Tests Added or Updated

- `generate::tests::score_matches_search_objective` — new. Pins `score() == cache.total_score()` on QWERTY and after every swap in a 200-swap `accept_swap` sequence, so neither standard can drift without a failing test. This was the assessment's "pinned invariant" metric (baseline 0 such tests → 1).
- No existing test needed updating: `cached_totals`, `optimize_qwerty`, `optimize_random_layouts` and the engine tests all assert against the cache/`score_with_precision` standard, which was not altered.

## Local Verification

- `cargo test -p oxeylyzer-core score_matches_search_objective` → ok (0.41 s)
- `cargo test-all` → all green: oxeylyzer-core 51 lib tests (was 50), repl 39, bench 9, 114 doctests, 0 failures
- `cargo clippy --all-targets --all-features` → 2 warnings, both pre-existing at HEAD and outside the changed code (verified by stash: `repl.rs:759` redundant reference; `fast_layout.rs:1367` no-op multiply, inside the dead commented-scaffolding region). No new warnings from `generate.rs`.
- Cost direction: the corrected path scores 1000 trigrams instead of 57,680 and adds a stretch loop — net cheaper for the sort path, as the assessment predicted. No throughput measurement taken (bench remains un-runnable in-workspace; scoped out).

## Deviations from Assessment

- **Forwarding over duplication.** The assessment's primary wording said "add `stretch_score` and truncate"; it then directed the fixer to prefer a forward to `score_with_precision` if clean. It was clean; taken.
- **Swap sequence length.** `cached_totals` uses 25,000 swap-permutations; the new test uses 200 sequential `accept_swap`s (first 200 of `possible_swaps`). The identity is checked after every step, so drift detection is equivalent for this purpose; a 25k variant would just re-run the existing `cached_totals` shape.

## Follow-ups

- Open questions carried from the assessment remain open and untouched: `analyze`'s `usize::MAX` total (`generate.rs:371`) now visibly differs from the ranking number by trigram count only; `benches/generation.rs` still cannot run in-workspace; README's "hardcoded to 1000" claim vs. config reality.
- Absolute displayed scores changed (stretch added, tail trigrams dropped) — expected consequence, recorded in the assessment's Risks.
- Suggested next step per pipeline: `/speckit.bug.test slug=score-stretch-divergence`.
