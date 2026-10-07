---
title: Why obsolete.rs Stays
type: decision
sources: [S010, S011]
updated: 2026-10-06
---

`oxeylyzer-core/src/generate/obsolete.rs` is named for what it replaced, not for
being unreachable. `mod obsolete;` sits in `generate.rs` **unconditionally** — no
`#[cfg(test)]` on the module declaration — and it holds the uncached reference
scoring path: `best_swap` (`pub`) and `score_swap` (`pub(crate)`), where
`score_swap` swaps the pair in, calls `score_with_precision` at
`trigram_precision`, and swaps back (S010).

Four call sites depend on it (S010):

| Caller | What it needs from `obsolete.rs` |
|---|---|
| `best_found_swap` (generate.rs) | `best_swap` against `best_swap_cached` |
| `score_swaps_no_accept` (generate.rs) | `score_swap` against `score_swap_cached` |
| `benches/generation.rs` | registers `"best swap"` beside `"best swap cached"` |
| `#[cfg(test)] impl Oxeylyzer` in the same file | `optimize_normal_no_cols` drives `best_swap` |

The two tests are what make the first two rows load-bearing rather than
incidental: without them, the file is reachable but unverified (S011).

**Rejected: deleting it as dead code.** The queued entry for this feature asked
for exactly that. Removing it deletes the only independent way to compute the
same answer, which is the only way to check that the cached path stayed faithful
and the only way to measure what caching bought (S010).

> ✔ resolved: S010's account of the check is now accurate. It previously was
> not — `best_found_swap` asserted the chosen swaps match only *when the two
> scores were already equal*, and on a mismatch printed `"scores not the same"`
> and passed. A score regression could not fail it. Both weak spots are fixed:
> the nested `if let`s are gone so neither `None` outcome skips silently, the
> scores are asserted before the swaps, and `score_swaps_no_accept` no longer
> tolerates `SMALLEST_SCORE + 1000` (S011). Both tests passed unchanged against
> current code, so the fix cost no score — it removed the ability to hide one.
> **A guard that tolerates its own failure mode is not a guard.** The sentinel was
> `i64::MIN + 1000`, unreachable from `score_with_precision` short of overflow,
> so it never fired — but it would have masked exactly the drift it existed to
> permit.

The asymmetry to remember: the module is called *obsolete* because the cached
path is the production path, not because the file is retired — `mod obsolete;`
(`generate.rs:1119`) is unconditional, and production reaches `score_swap_cached`
and `best_swap_cached` while `obsolete.rs` holds the uncached twins (S010, S011).
Deleting it also requires renaming whatever "obsolete" was meant to say.

Related: [Scoring Cache And LayoutCache](./scoring-cache-and-layout-cache.md) ·
[Exact-Integer Scoring Is An Invariant](./exact-integer-scoring-is-an-invariant.md) ·
[Finger Usage Is Summation Plus Penalty](./finger-usage-is-summation-plus-penalty.md)