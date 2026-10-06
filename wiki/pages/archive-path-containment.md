---
title: Archive Path Containment
type: concept
sources: [S001, S002, S003]
updated: 2026-10-06
---

Never derive a filesystem write path from a zip entry's raw name. In this
project the containment check is `ZipFile::enclosed_name()`, which returns
`Option<PathBuf>` — `None` when the entry cannot be contained (S001).

It is public API in `zip` 2.4.2, already a dependency of `oxeylyzer-resources`
(`default-features = false, deflate`). Its implementation walks components with a
depth counter: `Prefix` and `RootDir` return `None`, `ParentDir` returns `None`
at depth 0, `Normal` increments depth (S001, S003).

## The prefix must be stripped BEFORE the check

`enclosed_name()` counts the prefix's own component as one level of depth. So
checking `oxeylyzer-data-files/../escaped.txt` whole sees depth 1, decrements
to 0, and **accepts** it. Strip the prefix first and the remainder `../escaped.txt`
starts at depth 0, so `ParentDir` hits `depth.checked_sub(1)` on 0 and
returns `None` (S001, S003).

> ✔ resolved: `specs/001-archive-safety/plan.md` prescribes
> strip-then-check, which is correct. `specs/001-archive-safety/tasks.md` T002
> prescribes the opposite ("strip `ZIP_INNER_PREFIX` from the *enclosed* path,
> after the containment check"), and the shipped code follows T002 — which is
> why the breach below exists (S001, S002). Both claims came from S001; the
> ordering was settled against the `zip` 2.4.2 implementation (S003) and a
> failing test (S002). T002 remains uncorrected in the source artifact, which
> the ingest guardrail forbids editing — tracked in `TODO.md`.

Measured, using `zip` 2.4.2 directly (S003):

| Entry | Check whole, then strip | Strip, then check |
|---|---|---|
| `oxeylyzer-data-files/../X.txt` | **accepted** → `dest/../X.txt` | rejected |
| `oxeylyzer-data-files/../../X.txt` | rejected | rejected |
| `oxeylyzer-data-files/../../../X.txt` | rejected | rejected |
| `../X.txt` (no prefix) | rejected | rejected |
| `/etc/passwd` | rejected | rejected |
| `oxeylyzer-data-files/static/text/en.txt` | accepted → `static/text/en.txt` | accepted → `static/text/en.txt` |
| `static/text/en.txt` (no prefix) | accepted → `static/text/en.txt` | accepted → `static/text/en.txt` |

Only the single-`..`-after-prefix case separates the two orders (S001).

## Verified breach in the shipped code

`oxeylyzer-resources/src/download.rs` (lines 90–103) checks first and strips
second — the order that accepts it (S002):

```rust
let enclosed = file.enclosed_name().ok_or_else(|| ...)?;   // accepts ../escaped.txt
let raw_name = enclosed.to_string_lossy();
let stripped = raw_name.strip_prefix(ZIP_INNER_PREFIX).unwrap_or(&raw_name);
let out_path = dest.join(stripped);                          // ../escaped.txt
```

A temporary test in the real crate (since reverted) confirmed the entry extracts
with `Ok(())` and writes one level above the destination. Being accepted, it is
then extracted like any other entry: the abort path never engages and the
sentinel is written as usual (S002). The suite still passes, because
`traversal_escaping_root_is_rejected` uses `../../../escaped.txt` — three `..`,
which both orders reject. That count is what leaves this shape undetected (S002).

The comment at lines 85–89 states this rule correctly while the code below it
does the opposite; its example uses two `..`, so it never demonstrates the
hazard it warns about (S002). Fix plan tracked in `TODO.md`; `tasks.md` T002
stays uncorrected in the artifact.

## Symlinks are not a vector here

The loop never calls `is_symlink()`, and nothing in the workspace creates
symlinks. A symlink entry's *content* is the target path, written as ordinary
file bytes — no link is ever created, so nothing later can follow it (S001).

## Why the library instead of hand-rolled normalization

Hand-rolled normalization would have to reimplement `..` depth tracking,
absolute-path detection, and NUL handling — every case the library already
covers (S001).

The fix is not "call the library twice": `enclosed_name()` takes no subpath
argument, so re-checking a stripped remainder needs a second `ZipFile` or an
inline component walk (S002, S003).

Related: [Data Archive Pipeline](./data-archive-pipeline.md) ·
[Why Extraction Aborts Instead of Skipping](./why-extraction-aborts-instead-of-skipping.md) ·
[Testing Zip Extraction In-Crate](./testing-zip-extraction-in-crate.md)