---
title: Data Archive Pipeline
type: component
sources: [S001, S004, S006]
updated: 2026-10-06
---

`oxeylyzer-resources` owns the only path in the app where untrusted remote
data reaches the filesystem. The zip is downloaded when the installation has no
recorded data version or one that is behind the current build's, and nobody
verifies its contents — so this component is the whole blast radius of a
crafted archive (S001, S004).

```text
seed.rs::ensure_data          branches on dirs.needs_data()
  └─ download_and_extract     ureq 2 fetch → bytes in memory
       └─ extract_bytes(&[u8], &Path)   private; does the extraction loop
            ├─ ZipFile::enclosed_name()  containment check
            └─ .data-version sentinel    written only after the loop
```

`ensure_data` then always calls `ensure_config`, which is idempotent and safe on
every startup (S004). Both `oxeylyzer-repl` and `oxeylyzer-tauri` call it
unconditionally; only the Tauri app also gates on `needs_data()` for its startup
screen (S004). See
[Data Version Currency](./data-version-currency.md).

Files: `download.rs` (extraction), `seed.rs` (fatal on failure), `dirs.rs`
(resolves the data root), `lib.rs` (`ResourceError`). Library crate, workspace
member; `zip` and `ureq` were both already declared (S001).

`ensure_config` writes the default config on first run; it was only ever asserted
to exist and to not be overwritten, so a default config no loader could parse
would have passed the whole suite (S006). See
[Generated Config Round-Trip](./generated-config-round-trip.md) and
[The Config Loader Is The Contract](./config-loader-is-the-contract.md).

## The `extract_bytes` seam

Splitting extraction into a private `extract_bytes(&[u8], &Path)` that
`download_and_extract` delegates to was not in the original plan. Tests could
not call the download function without hitting the network, and without the
seam the containment rules are untestable at all (S001).

## Errors

`ResourceError::Extraction(String)` already existed in `lib.rs` and already
carried a `String`, so refusal errors name the offending entry without adding
a variant. `download.rs` propagates every failure with `?` (S001).

## The sentinel is both the commit record and the version gate

`.data-version` is written only after the extraction loop completes. Anything
that lets the loop finish early writes the sentinel and marks a partial archive
as a complete install — see
[Why Extraction Aborts Instead of Skipping](./why-extraction-aborts-instead-of-skipping.md) (S001).

It is now also read on every startup to decide whether data must be downloaded at
all, so the same file that marks an install complete is the one whose staleness
triggers an upgrade (S004). One property, two jobs: a failed refresh leaves the
old version recorded, so the next startup retries (FR-009, FR-010) — see
[Refresh Replaces Shipped Paths](./refresh-replaces-shipped-paths.md).

## Streaming the download was deliberately skipped

The archive is ~4 MB (`static/text` 2.2M, `static/language_data` 2.0M).
Buffering it is not a practical failure; streaming would add temp-file
staging, its cleanup path, and its own failure mode. Deferred, not forgotten
(S001).

Related: [Archive Path Containment](./archive-path-containment.md) ·
[Data Version Currency](./data-version-currency.md) ·
[Testing Zip Extraction In-Crate](./testing-zip-extraction-in-crate.md) ·
[Refresh Replaces Shipped Paths](./refresh-replaces-shipped-paths.md) ·
[Generated Config Round-Trip](./generated-config-round-trip.md) ·
[Platform Root Resolution](./platform-root-resolution.md) ·
[Shipped Data Files Layout](./shipped-data-files-layout.md)