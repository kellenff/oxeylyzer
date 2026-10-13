# Bug Assessment: displayed ranking uses a different objective than the search

- **Slug**: score-stretch-divergence (auto-generated; no user slug provided)
- **Created**: 2026-10-13T04:10:00Z
- **Source**: pasted text (`/speckit.bug.assess` argument — condensed from `.specify/assessments/performance-bottlenecks-parallelism/`, whose `decision.md` verdict is **go**)
- **Verdict**: valid
- **Severity**: high

## Report (summarized)

The project computes four total-score formulas across eight call sites. Three display surfaces rank generated layouts by `Oxeylyzer::score()`, which omits the stretch term and scores *all* trigrams, while the search optimizer ranks by `LayoutCache::total_score()` (stretch included, trigrams truncated at `trigram_precision`). Measured over 2500 generated layouts: 0/10 overlap between the two top-10s; the layout displayed as #1 sits at #254 by the search objective. The owner confirmed the divergence is a defect, not a deliberate display/search split (resolves `problem.md` Blocking Question 1). Chosen remediation is concept Option A: make `score()` the search objective and delete the second standard.

## Symptom

`generate` in the REPL and the GUI's layout list show a "top 10" that the optimizer did not produce: sorting is done by a scoring function that disagrees with the one the hill-climb maximized. The layout presented as best is, by the objective actually searched, ranked hundreds of places lower. Expected: what is displayed as best is best by the objective that generated it.

## Reproduction

1. `cargo run` → `generate n` (or open the Tauri GUI and generate).
2. Note the top-10 list and scores (`oxeylyzer-repl/src/display.rs:58` sorts by `layout_gen.score(&l)`).
3. Re-rank the same 2500-layout pool by `initialize_cache(&l).total_score()` — the objective `climb`/`best_swap_cached` used (`generate.rs:984`, `annealing.rs:36`, `ils.rs:33`, `lahc.rs:33`, `depth.rs:77`).
4. Observed: top-10s share 0 layouts; displayed #1 is #254 by the search objective. (Recorded in research.md "The score/objective discrepancy"; reproduction steps otherwise `[NEEDS CLARIFICATION: is a checked-in repro script wanted, or is the recorded measurement sufficient?]` — non-blocking, the divergence is provable by inspection.)

## Suspected Code Paths

Verified against source at HEAD `ae333d8`:

- `oxeylyzer-core/src/generate.rs:515` `score()` — **root cause**. Sums `trigram_score + fspeed_usage + pinky_ring` (line 527): no `stretch_score` term, and the trigram iterator is `self.data.gen_trigrams()` un-truncated (all ~57,680 trigrams), unlike every other standard.
- `oxeylyzer-core/src/generate.rs:547` `score_with_precision()` — the correct formula shape: `trigrams.take(p) + stretch + fspeed_usage + pinky_ring` (line 559).
- `oxeylyzer-core/src/generate.rs:187` `LayoutCache::total_score()` — the search objective: `trigrams_total + pinky_ring + stretch_total + usage_total + fspeed_total`, with trigrams baked at `self.trigram_precision` in `initialize_cache` (line ~826).
- `oxeylyzer-core/src/generate/obsolete.rs:37` — `score_swap` oracle independently uses `score_with_precision(layout, self.trigram_precision)`, confirming the search standard.
- Ranking callers (fixed for free, no edits): `oxeylyzer-repl/src/display.rs:58`, `oxeylyzer-repl/src/repl.rs:348` (`rank`), `oxeylyzer-tauri/src-tauri/src/lib.rs:1000`.
- `benches/generation.rs:75` — also calls `score()`; bench already cannot run in this workspace (out of scope, see below).
- `oxeylyzer-core/src/generate.rs:371` — `get_layout_stats` reports `score_with_precision(layout, usize::MAX)`, a third standard (stretch included, all trigrams). Carried as an open question, not part of this fix.

## Root Cause Hypothesis

**High confidence.** `score()` predates the stretch term / precision-truncation standardization and was never reconciled; the search hot path moved to the cached incremental formula (`total_score()`) while display callers kept calling the stale standalone sum. No test ever related the two standards — `cached_totals` (`generate.rs:1219-1248`) pins `cache.total_score() == score_with_precision(…, trigram_precision)` but never touches `score()`, so drift was structurally undetectable. Matches the wiki's own stated stakes (`wiki/pages/exact-integer-scoring-is-an-invariant.md`: an ordering shift changes what "best" means with no panic and no failing test).

## Proposed Remediation

**Preferred (Option A, owner-chosen):** change `Oxeylyzer::score` (`generate.rs:515`) to be exactly the search objective — add `let stretch_score = self.stretch_score(layout);` to the sum and truncate the trigram iterator with `.iter().take(self.trigram_precision)`, making `score(layout) == score_with_precision(layout, self.trigram_precision) == initialize_cache(layout).total_score()` hold by construction. The three ranking call sites need no source edit. Replace the generic docstring ("Calculates total score for a given layout") with one naming the standard: the search objective at configured trigram precision. The naive body duplication of `score_with_precision` is acceptable; deleting `score()` in favor of forwarding to `score_with_precision(layout, self.trigram_precision)` is the smaller variant of the same fix and the fixer should prefer it if it falls out that way.

Side effect (benign): the corrected path scores 1000 trigrams instead of 57,680, so the display/sort path gets *cheaper*; the climb is untouched.

**Alternatives (declined, recorded so a fixer doesn't drift into them):**
- Option B — keep two standards, document the split and pin it with a test. Declined: owner confirmed the split is unintended.
- Option C — unify the three "sum five terms" implementations into one typed objective value. Declined as over-scoped; renames of `score`/`score_with_precision`/`total_score` also out.

**Files likely to change:**
- `oxeylyzer-core/src/generate.rs` (`score()` body + docstring; test module)

**Tests to add/update:**
- New `#[test]` in `generate.rs` `mod tests`, shaped like `cached_totals`: assert `GEN.score(&layout) == GEN.initialize_cache(&layout).total_score()` on a fixed layout (QWERTY and/or `gust.dof`), *and* across a swap sequence (e.g. a loop of `accept_swap` over `possible_swaps`) so the invariant is pinned against future drift in either standard, not just on one snapshot. Equivalence form `score() == score_with_precision(layout, GEN.trigram_precision)` may be asserted alongside.
- Expect all existing suites green: `cargo test-all` is the CI gate (`.cargo/config.toml` alias, `.github/workflows/ci.yml:54`), plus `cargo clippy --all-targets --all-features -- -D warnings`.

## Risks & Considerations

- **Displayed numbers change.** Absolute scores (and the `fmt_score`-scaled values users compare run to run) shift for every layout; old notes/screenshots won't match. This is the intended correction, but users who bookmarked scores see a discontinuity. No score is persisted in layout files (checked: `LayoutStats.score` is computed, not stored), so no data migration.
- **Rankings visibly change** in REPL `generate`, REPL `rank`, and GUI list — again the point of the fix, but it is a user-visible behavior change on the tool's primary output.
- **`analyze` still disagrees with the ranking** after the fix: `get_layout_stats` keeps reporting the `usize::MAX` total (`generate.rs:371`). Deliberately out of scope; if left as-is, a user can rank by one standard and analyze the winner by another. Flagged below as the surviving open question.
- No weights, corpus, `trigram_precision` value, or search-path changes; no API signature changes; generation throughput unchanged.

## Out of Scope (guardrails for `/speckit.bug.fix`)

- `generate.rs:371` `usize::MAX` choice for `analyze` — decide only if explicitly escalated.
- Unifying `cache.total_score()` / `score_with_precision` / `LayoutCache` accumulation; renames; typed objective values.
- Any change to `trigram_precision`'s value, weights, corpora; performance work (no throughput target, no O(1) finger-usage retirement, no allocator/parallelism/profiler work).
- Making `benches/generation.rs` runnable (root-caused separately: `benches/util.rs:10` → `OxeylyzerDirs::resolve()` ignores `OXEYLYZER_DATA_DIR` unlike `repl.rs:214` / Tauri `lib.rs:1640`, and workspace `static/` lacks the `english`/`gust.dof` fixtures under `oxeylyzer-core/static/`).
- Tauri/TypeScript frontend, `convert_kbs.py`, UI/UX presentation, `LayoutStats` fields, heat-map percentages, `get_layout_stats`.
- Dead-code sweeps. The commented block at `fast_layout.rs:1383-1422` is dead test scaffolding referencing removed APIs (`LayoutAnalysis::score(&r, 5_000)`) — not a disabled optimization; at most correct the record wherever it was described as such.

## Open Questions

- [NEEDS CLARIFICATION: does `analyze`'s total (`generate.rs:371`, `score_with_precision(layout, usize::MAX)`) keep reporting all 57,680 trigrams, or follow the configured precision?] Non-blocking for this fix; the corrected `score()` matches the search objective either way. If changed later, state the consequence: single-layout reported scores shift again.
- [NEEDS CLARIFICATION: repair `benches/generation.rs` in this workspace, or is the measurement record in research.md enough?] Owner's Blocking Question 2 unanswered; scoped out here. Separate, small, separable.
- [NEEDS CLARIFICATION: is `trigram_precision` user-controllable end to end?] `README.md:77` claims it is hardcoded to 1000 everywhere; measurement shows the config value does change throughput (it is read at `generate.rs:248/251`). Non-blocking — the fix uses `self.trigram_precision` regardless.
- [NEEDS CLARIFICATION: Rust-only scope, or also Tauri/TS frontend and `convert_kbs.py`?] Assumed Rust-only, unchanged.
- Known limit, not a question: no line-level profile exists (`samply`/`perf` absent); nothing in this fix needs one.
