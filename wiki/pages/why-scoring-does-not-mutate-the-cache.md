---
title: Why Scoring Does Not Mutate The Cache
type: decision
sources: [S010, S011]
updated: 2026-10-06
---

`score_swap_cached` takes `&LayoutCache` and `accept_swap` takes `&mut`. That
split is a contract, not a convenience (S011).

**Rejected: mutating the cache during scoring.** A scored-but-rejected swap would
leave its values behind, so the next candidate's subtraction would start from a
base that never existed. That is silent score corruption — no panic, no test
failure, just wrong rankings (S010).

The consequence is that `score_swap_cached` must recompute the post-swap value
for the two affected fingers rather than derive it from the cache (S010).

**Kept for later, not now:** the exact-delta approach. A swap moves two
characters and finger-per-position is fixed, so each affected finger's new usage
is its cached usage minus one character's frequency plus the other's — O(1), no
mutation, and the purity guarantee survives (S010). See
[Swap Moves Two Characters](./swap-moves-two-characters.md).

Deliberately deferred from the refactor: rewriting the cached path "needs the
purity decision, and belongs to scoring's contract rather than to a refactor
that happens to make it expressible" (S010).

Related: [Scoring Cache And LayoutCache](./scoring-cache-and-layout-cache.md) ·
[Why obsolete.rs Stays](./why-obsolete-rs-stays.md)