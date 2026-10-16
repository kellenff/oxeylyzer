# Implementation Plan: Layout Auto-Naming Accuracy

**Branch**: `feature/time-machine-layout-auto-naming` | **Date**: 2026-10-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification `specs/007-layout-auto-naming/spec.md`

## Summary

Replace the fixed "skip ten keys, take four" naming offset with a rule derived from
the layout's own row shape, and put that rule in one shared place.

**Technical approach**: `auto_name(keys, shape, mapping)` in `fast_layout.rs`, using
the first row's declared width to locate the home row. All three call sites use it.

## Technical Context

**Language/Version**: Rust, edition 2024
**Primary Dependencies**: none added
**Testing**: `cargo test` — unit tests in `fast_layout.rs`
**Scale/Scope**: one new function, three call sites, seven tests

## Measurements

| Board | Old offset picks | Rule picks |
|---|---|---|
| 3×10 (all 815 shipped) | first 4 of row 1 | same ✓ |
| 4 rows of 10 | first 4 of **row 0** | first 4 of row 1 |
| Ragged rows | varies | first 4 of row 1 |
| Single row | skips every key → empty | first 4 of row 0 |

All 815 shipped layouts are 3×10, so current behaviour is correct for all of them and
must not change. That is verified, not assumed — see Verification.

## Design

```rust
pub fn auto_name(keys: &[u8], shape: &Shape, mapping: &CharMapping) -> String
```

`Shape` is already present on both `FastLayout` and `Layout`, so no new data has to
reach the naming sites — this is why the fix is small.

Locating the home row is the whole problem. Two of my attempts were wrong:

- `shape.inner().first()` skips the first row's width. Correct for the 3×10 case,
  but a single-row board has one width, so it skips past every key and returns "".
- Summing all rows after the first is only right when there are exactly two rows.

The rule that holds: **the first row's width, and only when there are at least two
rows.** Both degenerate cases are covered by tests rather than left to inspection.

## Call sites

| Site | Replaced |
|---|---|
| `layout.rs:262` `From<FastLayout> for Layout` | inline offset, carried the TODO |
| `repl.rs:402` `placeholder_name` | inline offset |
| `lib.rs:1060` `save_generated` | inline offset |

The REPL site also chains a counter and replaces `*` with `-`; that stays, applied to
the shared rule's output. Existing layouts with a name keep it — `From<FastLayout>`
still short-circuits on `Some(name)`.

## Verification

```bash
cargo test --workspace   # 212
cargo clippy --workspace
```

**SC-002 is the important one.** All 815 shipped layouts were enumerated from the
data archive and each one's name computed both ways: identical for all 815. The old
behaviour was correct for every layout that exists, so changing names that were
already right was the primary risk.

**New tests** cover 3×10, four-row, ragged rows, a short home row, a single row, an
empty board, and a shape wider than the keys present. The last two assert no panic
and that the keys are authoritative over the declared width.

## Complexity Tracking

| Choice | Simpler alternative rejected because |
|---|---|
| Shared `auto_name` in core | Three copies drifted already; the reasoning existed in one |
| First row's width, guarded on row count | Unguarded, it returns "" for single-row boards |
| Reuse `Shape` | No new plumbing needed; both types already carry it |
