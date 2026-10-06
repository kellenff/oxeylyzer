---
title: Why Coverage Is Scoped By Measurement
type: decision
sources: [S006]
updated: 2026-10-06
---

`oxeylyzer-resources` originally had no tests. Two shipped features added 25
between them — 7 for archive containment, 18 for data version currency — and the
crate went from 0 tests to 25 without anyone specifying the gap (S006).

The queue entry for more coverage asked for the whole crate again: path
resolution under platform override roots, sentinel-gated download, default
config generation, extraction results. Measurement showed most of it already
existed, so the feature was rescoped down to the two behaviors still uncovered
rather than re-specified on top of work already done (S006).

**Rejected: re-specifying crate-wide coverage.** The already-covered behaviors
were left alone, and the rescope was recorded so a later change does not
re-investigate what the existing tests protect (S006).

## What was already protected when the work started

| Behavior | File |
|---|---|
| entry containment, hostile-entry refusal, prefix stripping, root-entry handling, no-prune-on-failure | `download.rs` |
| version ordering, data-currency state | `dirs.rs` |
| paths resolving under an override root | `dirs.rs` |
| refresh decision short-circuiting current data | `dirs.rs`, `seed.rs` |
| default config written but not overwritten | `seed.rs` |

Sentinel-gated download was on the original queue list and was dropped as doubly
covered by the version-currency tests (S006).

The remaining two gaps were the generated default config round-tripping through
the app's loader, and containment against platform-reported directories — the
second having passed before because every test supplied its own root (S006).

The work shipped without new dependencies and with no build-time cost; the one
obstruction was that `cargo fmt` does not run on this machine, so formatting went
through the IDE reformat action — see
[Formatting Rust Without cargo fmt](./formatting-rust-without-cargo-fmt.md) (S006).

Related: [Platform Root Resolution](./platform-root-resolution.md) ·
[Testing Zip Extraction In-Crate](./testing-zip-extraction-in-crate.md) ·
[The Config Loader Is The Contract](./config-loader-is-the-contract.md) ·
[Data Archive Pipeline](./data-archive-pipeline.md) ·
[Formatting Rust Without cargo fmt](./formatting-rust-without-cargo-fmt.md)
