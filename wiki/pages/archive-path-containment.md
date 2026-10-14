---
title: Archive Path Containment
type: concept
sources: [S001, S002, S003, S014]
updated: 2026-10-13
---

Never derive a filesystem write path from a zip entry's raw name. In this
project the containment check is `ZipFile::enclosed_name()`, which returns
`Option<PathBuf>` — `None` when the entry cannot be contained (S001).

It is public API in `zip` 2.4.2, already a dependency of `oxeylyzer-resources`
(`default-features = false, deflate`). Its implementation walks components with a
depth counter: `Prefix` and `RootDir` return `None`, `ParentDir` returns `None`
at depth 0, `Normal` increments depth (S001, S003).

## The prefix must be stripped BEFORE a name is judged contained

`enclosed_name()` counts the prefix's own component as one level of depth. So
checking `oxeylyzer-data-files/../escaped.txt` whole sees depth 1, decrements
to 0, and **accepts** it. Strip the prefix first and the remainder `../escaped.txt`
starts at depth 0, so `ParentDir` hits `depth.checked_sub(1)` on 0 and
returns `None` (S001, S003).

> ✔ resolved: `specs/001-archive-safety/plan.md` prescribed
> strip-then-check, which is correct. `specs/001-archive-safety/tasks.md` T002
> prescribed the opposite ("strip `ZIP_INNER_PREFIX` from the *enclosed* path,
> after the containment check"), and the shipped code followed T002 — which is
> why the breach below existed (S001, S002). Both claims came from S001; the
> ordering was settled against the `zip` 2.4.2 implementation (S003) and a
> failing test (S002). Settled in code by `3d0d66d`, verified again against the
> working tree on 2026-10-13 (S014).

Measured, using `zip` 2.4.2 directly (S003):

| Entry | Check whole, then strip | Strip, then check |
|---|---|---|
| `oxeylyzer-data-files/../X.txt` | **accepted** → `dest/../X.txt` | rejected |
| `oxeylyzer-data-files/../../X.txt` | rejected | rejected |
| `oxeylyzer-data-files/../../../X.txt` | rejected | rejected |
| `../X.txt` (no prefix) | rejected | rejected |
| `/etc/passwd` | rejected | rejected |
| `oxeylyzer-data-files/static/text/en.txt` | accepted → `static/text/en.txt` | accepted → `static/text/en.txt` |

Only the single-`..`-after-prefix case separates the two orders (S001).

## The single-`..` breach — closed, by a second check rather than a reorder

`download.rs` used to check containment first and strip second, the order that
accepts `oxeylyzer-data-files/../escaped.txt` and then writes it one level above
the destination. The suite missed it because `traversal_escaping_root_is_rejected`
uses three `..`, which both orders reject; the comment above the code described
the rule correctly while the code did the opposite (S002).

The shipped shape now keeps `enclosed_name()` on the whole name **and** adds a
second, in-repo check of the *stripped remainder* via `is_contained()` — an
inline component walk, since `enclosed_name()` takes no subpath argument and
cannot be re-applied to the remainder (S002, S003). `single_parent_after_prefix_is_rejected`
pins the previously-escaping shape, and the doubled-separator variant pins the
rooted-remainder case where `dest.join("/abs")` discards `dest` (S014).

## Symlinks are not a vector here

The loop never calls `is_symlink()`, and nothing in the workspace creates
symlinks. A symlink entry's *content* is the target path, written as ordinary
file bytes — no link is ever created, so nothing later can follow it (S001).

Related: [Data Archive Pipeline](./data-archive-pipeline.md) ·
[Zip 9 Normalizes Paths That Zip 2 Refused](./zip9-normalizes-paths-that-zip2-refused.md) ·
[Why Extraction Aborts Instead of Skipping](./why-extraction-aborts-instead-of-skipping.md) ·
[Testing Zip Extraction In-Crate](./testing-zip-extraction-in-crate.md)
