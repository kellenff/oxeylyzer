---
title: Data Archive Pipeline
type: component
sources: [S001]
updated: 2026-10-06
---

`oxeylyzer-resources` owns the only path in the app where untrusted remote
data reaches the filesystem. The zip is downloaded on first run and nobody
verifies its contents, so this component is the whole blast radius of a
crafted archive (S001).

```text
download_and_extract          ureq 2 fetch → bytes in memory
  └─ extract_bytes(&[u8], &Path)   private; does the extraction loop
       ├─ ZipFile::enclosed_name()  containment check
       └─ .data-version sentinel    written only after the loop
seed.rs                       treats a failed download as fatal
```

Files: `download.rs` (extraction), `seed.rs` (fatal on failure), `dirs.rs`
(resolves the data root), `lib.rs` (`ResourceError`). Library crate, workspace
member; `zip` and `ureq` were both already declared (S001).

## The `extract_bytes` seam

Splitting extraction into a private `extract_bytes(&[u8], &Path)` that
`download_and_extract` delegates to was not in the original plan. Tests could
not call the download function without hitting the network, and without the
seam the containment rules are untestable at all (S001).

## Errors

`ResourceError::Extraction(String)` already existed in `lib.rs` and already
carried a `String`, so refusal errors name the offending entry without adding
a variant. `download.rs` propagates every failure with `?` (S001).

## The sentinel is the install's commit record

`.data-version` is written only after the extraction loop completes. Anything
that lets the loop finish early writes the sentinel and marks a partial
archive as a complete install — see
[Why Extraction Aborts Instead of Skipping](./why-extraction-aborts-instead-of-skipping.md) (S001).

## Streaming the download was deliberately skipped

The archive is ~4 MB (`static/text` 2.2M, `static/language_data` 2.0M).
Buffering it is not a practical failure; streaming would add temp-file
staging, its cleanup path, and its own failure mode. Deferred, not forgotten
(S001).

Related: [Archive Path Containment](./archive-path-containment.md) ·
[Testing Zip Extraction In-Crate](./testing-zip-extraction-in-crate.md)