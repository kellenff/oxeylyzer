---
title: Scoring Cache And LayoutCache
type: component
sources: [S010, S011]
updated: 2026-10-06
---

`LayoutCache` holds the seven totals scoring needs, all `i64`: `usage[10]` and
`usage_total`, `fspeed[10]` and `fspeed_total`, `stretch_total`,
`trigrams_total`, `pinky_ring`. `total_score()` is the sum of the five totals
(S011).

`initialize_cache` walks all ten fingers once and fills it — that single pass is
why `usage[finger]` is available for the rest of a search without recomputation
(S010).

## `score_swap_cached` computes, never updates

`score_swap_cached(layout, swap, cache) -> Option<i64>` answers "what would this
swap score" without touching the cache (S011). It:

1. rejects early — `None` for same position, same character, or both characters
   having zero corpus frequency;
2. swaps in memory, reads the resulting fingers `f1`/`f2`;
3. recomputes the two affected fingers and adjusts the cached total:
   same-finger is `- cache.fspeed[f1] + recompute(f1)`, two-finger is
   `- cache.fspeed[f1] - cache.fspeed[f2] + recompute(f1) + recompute(f2)`;
   `usage` follows the same shape against `usage[10]`/`usage_total`;
4. recomputes `pinky_ring` **only** when `pinky_ring_indices.affects_pinky_ring(swap)`,
   otherwise uses the cached value;
5. for stretches and trigrams, computes each before and after the swap and
   differences against the cached total, then swaps back.

The cached pre-swap value is used as a subtrahend. The post-swap value is
recomputed by walking every key assigned to the finger — `finger_usage` sums
`data.get_char_u` over `layout.usage_indices` — which is the redundant work this
feature set out to remove (S010).

> ⚠ conflict: T005 and SC-006 claim the misleading `TODO: optimize` was replaced
> by documenting where the scoring cost actually sits (S010). The marker was
> indeed removed, and the commit's prose explains it — but no replacement
> comment exists at the cost site. `score_swap_cached`'s doc comment still reads
> only "Scores a potential key swap using the cache", and the redundant
> recompute at the `usage_score` binding is uncommented. The cost is documented in
> the spec and plan, not in the code, so the next reader of `generate.rs` finds
> nothing (S010, S011).

## `accept_swap` is the only mutator

`accept_swap(layout, swap, cache)` runs the same recomputation and writes the
values back (S011). The split is what keeps scoring pure: `score_swaps_no_accept`
asserts the layout is byte-identical to its pre-swap clone after a full sweep of
`possible_swaps` (S011). See
[Why Scoring Does Not Mutate The Cache](./why-scoring-does-not-mutate-the-cache.md).

## Same structural pattern everywhere

`score_swap_cached` is the only caller of the swap-scoring entry point in
production: `ils.rs`, `lahc.rs`, and `depth.rs` all call it (S011).

Related: [Swap Moves Two Characters](./swap-moves-two-characters.md) ·
[Why obsolete.rs Stays](./why-obsolete-rs-stays.md)