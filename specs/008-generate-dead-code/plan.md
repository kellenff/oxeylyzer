# Implementation Plan: Scoring Cache Consistency

**Branch**: `feature/time-machine-generate-dead-code` | **Date**: 2026-10-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification `specs/008-generate-dead-code/spec.md`

## Summary

Extract the over-use penalty from the per-finger usage summation so the two are
separately addressable, and establish the exact relationship between a swap and a
finger's usage that the cached path depends on.

**Technical approach**: split `finger_usage` into a summation and a penalty, and
prove by test that the penalty arithmetic is unchanged. The cache-consistency work
this enables is deliberately not done in this change — see Scope.

## Technical Context

**Language/Version**: Rust, edition 2024
**Primary Dependencies**: none added
**Testing**: `cargo test` — two tests added to `generate.rs`
**Scale/Scope**: one file, ~20 lines moved

## What the code showed

- `LayoutCache.usage[finger]` already holds the per-finger usage, populated once at
  `initialize_cache`.
- `score_swap_cached` reads `cache.usage[f1]` to subtract the pre-swap contribution,
  then recomputes the post-swap value from scratch — consulting and discarding the
  cached number in one expression.
- `finger_usage` mixes two concerns: summing character frequencies across a finger's
  keys, and applying that finger's over-use penalty.
- The `TODO: optimize` marker sits on one of six match arms that all do identical
  work. It points at the wrong line.

## Design

```rust
fn max_finger_use_threshold(&self, finger: Finger) -> i64   // which threshold
fn finger_usage_score(&self, finger: Finger, usage: i64) -> i64  // the penalty
fn finger_usage(&self, layout, finger) -> i64               // sum, then penalty
```

Integer arithmetic is preserved exactly. An earlier draft used `f64` for the
threshold and had to be corrected — the weights are `i64`, and any float conversion
would risk changing a score by rounding.

## Scope: why the cache rewrite is not in this change

The cached path recomputes per-finger usage while holding `cache: &LayoutCache`.
Reading the cached value instead requires either making scoring mutate the cache, or
restructuring what scoring owns.

Both were considered and deferred:

- Mutating the cache during scoring means a scored-but-rejected swap leaves its
  values behind, so the next candidate's subtraction starts from a base that never
  existed. That is silent score corruption.
- Keeping scoring pure means the recomputation stays.

The alternative worth doing instead is exact and needs neither: **a swap moves two
characters, so each affected finger's usage changes by a two-term delta.** Since a
swap exchanges two positions and finger-per-position never changes, the new usage of
an affected finger is its cached usage minus one character's frequency plus the
other's. That is O(1), needs no mutation, and preserves the purity guarantee.

This change establishes the split that makes such a change expressible, and proves
the precondition it depends on.

## Verification

```bash
cargo test --workspace   # 214
cargo clippy -p oxeylyzer-core
cargo bench --no-run    # FR-006
```

**`finger_usage_score_matches_inline_expression`** compares the extracted penalty
against the original inline expression for all ten fingers. Exact integer equality —
a changed score would change which layouts users are shown and saved.

**`finger_usage_after_swap_matches_recompute`** establishes the delta precondition:
after a swap, only the two fingers holding the swapped keys have changed usage, and
the cache agreed with a full recompute beforehand.

## Complexity Tracking

| Choice | Simpler alternative rejected because |
|---|---|
| Split penalty from summation | Keeps the two concerns separable so the cached path can use a delta without duplicating the penalty |
| `i64` threshold | `f64` would risk a rounding change in an exact-integer score |
| No cache mutation in scoring | A scored-but-rejected swap would leave stale values behind |
| Deferred the cached-path rewrite | It needs a purity decision that belongs to scoring's contract, not to a refactor |
