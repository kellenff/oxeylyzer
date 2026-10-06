---
title: Swap Moves Two Characters
type: concept
sources: [S010, S011]
updated: 2026-10-06
---

A key swap exchanges two characters, and finger-per-position is fixed by the
board. So after a swap **only the two fingers holding the swapped keys can have
changed usage** — every other finger's key set is untouched (S010).

That makes each affected finger's post-swap usage derivable from its cached
value in O(1):

```
new_usage(f) = cached_usage(f) - freq(char leaving f) + freq(char arriving f)
```

Same-finger swap → one subtraction. Two-finger swap → two (S010).

`finger_usage_after_swap_matches_recompute` establishes the precondition rather
than the optimization: it initializes the cache, asserts it agreed with a full
recompute beforehand, swaps three pairs (`(5,7)`, `(20,25)`, `(12,13)`), and
asserts no finger outside the two swapped ones changed usage (S010).

**Not yet exploited.** `score_swap_cached` still recomputes `finger_usage` for
both affected fingers by walking each finger's keys (S011) — see
[Why Scoring Does Not Mutate The Cache](./why-scoring-does-not-mutate-the-cache.md)
for why that rewrite is deferred.

One caveat the spec names: a stale read would silently corrupt scores, so the
cache must be updated whenever a swap is accepted — `accept_swap` does that
(S010).

Related: [Scoring Cache And LayoutCache](./scoring-cache-and-layout-cache.md) ·
[Finger Usage Is Summation Plus Penalty](./finger-usage-is-summation-plus-penalty.md)