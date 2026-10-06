---
title: Testing Zip Extraction In-Crate
type: howto
sources: [S001, S002, S004]
updated: 2026-10-06
---

Hostile archives are built in-crate: a helper takes `(name, contents)` pairs
and writes them through `zip::ZipWriter` into an in-memory `Cursor`. No
fixtures, no dev-dependencies. Each test extracts into a temp directory
(S001).

Temp directory: `std::env::temp_dir()` plus a unique name. Adding `tempfile`
was deliberately avoided — it appears in `Cargo.lock` only as a transitive
dependency of `uds_windows`, not as anything this crate can rely on, and the
tests need one directory, not a general facility (S001).

## Assert on the filesystem, not the return value

The security property is that no file exists outside the destination. A test
that only checks the `Result` proves nothing about where bytes landed — walk
the parent directory and confirm nothing new appeared (S001).

## Do not test the absolute-path case with `/etc/passwd`

The first version of `absolute_path_entry_is_rejected` targeted
`/etc/passwd` and passed even against vulnerable code. The write failed on
permissions, not containment, so the test proved nothing. It now targets a
writable absolute path under the temp dir and asserts the file is absent
(S001).

## Mutation-check the guard

Reverting the guard to the raw `file.name()` makes the three security tests
(`traversal_escaping_root_is_rejected`,
`absolute_path_entry_is_rejected`,
`mixed_valid_and_hostile_fails_without_partial_files`) fail while the four
positive-path tests keep passing (S001).

**But that check does not cover the ordering bug.** Every one of those tests
uses two or three `..` segments, which both the correct and the incorrect
check order reject. The order-sensitive case is a *single* `..` (S001, S002):

```text
oxeylyzer-data-files/../escaped.txt
```

Adding that case to the real crate as a temporary test failed on `assert!(!ok)`
— the shipped code accepts it and writes outside the destination. See
[Archive Path Containment](./archive-path-containment.md). A mutation check
proves a guard is load-bearing only for the shapes it exercises; the traversal
test's `..` count is itself an untested choice (S002).

Tests call `extract_bytes` directly — see the seam in
[Data Archive Pipeline](./data-archive-pipeline.md) (S001).

One planned helper was written and then deleted: the directory-tree walker
became unused once assertions targeted specific paths (S001).

## Proving a download did *not* happen: pass a callback that panics

A gate that must avoid the network cannot be tested by asserting on a return
value — absence of a request is not visible in a `Result`. Pass a progress
callback that panics: `|_| panic!("download must not run for current data")`.
If the version gate ever fails to short-circuit, the test panics instead of
quietly reaching the network (S004).

This replaced a task written to call `ensure_data` with a stale or absent
sentinel and assert the download failed. It did not fail: those tests hit the
real network, downloaded the actual `data-files.zip`, and extracted the real
corpus into a temp directory, so they were really asserting "did GitHub
respond" (S004).

The accidental live run was still useful evidence — a `0.0.1` sentinel advanced
to `1.0.0` and the real archive extracted cleanly through the containment check
(S004). It is *not* evidence about the ordering bug above: a clean run against
the genuine archive says nothing about whether a hostile one would be refused.

Split decision from effect when testing a gate like this — the version
comparison is tested purely against a temp sentinel, and post-refresh state by
writing the sentinel and an unshipped file directly (S004).

Related: [Archive Path Containment](./archive-path-containment.md) ·
[Data Version Currency](./data-version-currency.md) ·
[Generated Config Round-Trip](./generated-config-round-trip.md) ·
[The Config Loader Is The Contract](./config-loader-is-the-contract.md)