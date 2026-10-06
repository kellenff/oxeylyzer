---
title: Testing Zip Extraction In-Crate
type: howto
sources: [S001, S002]
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

Result at time of writing: 7 new tests, 178 in the workspace (171
pre-existing), clippy and fmt clean (S001).

Related: [Archive Path Containment](./archive-path-containment.md)