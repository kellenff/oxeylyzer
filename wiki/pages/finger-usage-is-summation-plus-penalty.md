---
title: Finger Usage Is Summation Plus Penalty
type: component
sources: [S010, S011]
updated: 2026-10-06
---

Per-finger usage score is two things fused into one function. As of `410324f` they
are split in `oxeylyzer-core/src/generate.rs` (S010):

| Function | Does |
|---|---|
| `max_finger_use_threshold(finger) -> i64` | the finger's share of the corpus total, from `weights.max_finger_use`; maps the ten fingers onto five thresholds — `LP`/`RP` pinky, `LR`/`RR` ring, `LM`/`RM` middle, `LI`/`RI` index, `LT`/`RT` thumb (S011) |
| `finger_usage_score(finger, usage) -> i64` | `penalty * (usage - max_finger_use_threshold(finger) * data.char_total / 100)` (S011) |
| `finger_usage(layout, finger) -> i64` | sums `data.get_char_u` over `layout.usage_indices` for the finger, then applies the penalty (S011) |

**Why the split:** with the two fused, the cached path cannot use a cached usage
number without duplicating the penalty arithmetic to work out what the cached
number means. Splitting makes the relationship explicit and testable (S010).

**Rejected: `f64` for the threshold.** `char_total`, the usage sum, and the
scaled weights are all `i64`; a float in the middle risks changing a score by
rounding, and a one-unit score change changes which layouts users are shown and
saved. An earlier draft did route through `f64` and had to be corrected (S010,
S011).

The unscaled `Weights` in `oxeylyzer-core/src/weights.rs` *are* `f64` — thresholds
cast with `as i64` and the penalty scaled by 100 — but conversion happens at the
`Weights` → `AnalyzerWeights` boundary, so scoring only ever sees integers
(S011). Same boundary discipline as
[Generated Config Round-Trip](./generated-config-round-trip.md).

`finger_usage_score_matches_inline_expression` re-inlines the original
expression for all ten fingers and asserts exact equality against the extracted
function (S010).

Related: [Scoring Cache And LayoutCache](./scoring-cache-and-layout-cache.md) ·
[Swap Moves Two Characters](./swap-moves-two-characters.md) ·
[Why Scoring Does Not Mutate The Cache](./why-scoring-does-not-mutate-the-cache.md)