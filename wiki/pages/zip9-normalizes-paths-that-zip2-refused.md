---
title: Zip 9 Normalizes Paths That Zip 2 Refused
type: decision
sources: [S014, S002, S003]
updated: 2026-10-13
---

`zip` ≥ 9 changes `ZipFile::enclosed_name()` from **rejecting** hostile entry
names to **normalizing** them in-bounds. `oxeylyzer-data-files/../escaped.txt`,
refused outright under zip 2, becomes `escaped.txt` — inside the destination —
and a leading `/` or drive prefix loses its root instead of being rejected (S014).

**Measured, not inferred.** A full `zip 2 → 9` bump was built and tested on a
throwaway copy: the six hostile-input tests in `download.rs` failed under zip 9,
while nothing escaped the destination. They failed on *policy*, not safety:
normalization lets the loop write to paths the archive never claimed, and
`.data-version` was still written in those cases (S014).

## Decision: validate the raw name, refuse (feature 009 clarification)

Containment validation moves to the **raw** entry name (`file.name()`), checked
in-repo after stripping `ZIP_INNER_PREFIX` and **before** any zip-side
normalization can launder it. `is_contained()` stays authoritative;
`enclosed_name()` is no longer used to decide containment. The six hostile-input
tests keep their current reject semantics (S014).

**Rejected alternative:** accept zip 9's normalization and rewrite the six tests
to expect in-bounds rewrites (S014). Rejected because the sentinel would then
record a normalized, partially-claimed install as complete, and because a
rewritten test suite hides the change from readers of the archive tests.

**Consequence for the known breach:** zip 9's sanitizer never sees the decision,
so the prefix-strip ordering breach in
[Archive Path Containment](./archive-path-containment.md) is closed by
construction once the raw-name check lands — the raw string is validated after
the strip, which is exactly what that page's "prefix must be stripped BEFORE the
check" rule demands (S014, S002).

## Two guards the replacement must carry over

- **NUL bytes.** `enclosed_name()` rejects names containing `\0` (zip 2.4.2
  implementation; S002, S003). `Path::components()` — what `is_contained()`
  walks — does not. A raw-name validator must reject `\0` explicitly or the
  check loses coverage silently (S014).
- **Sentinel integrity.** `.data-version` is written only when every entry was
  honored; an aborted archive leaves no sentinel (S014). This is the existing
  rule in
  [Why Extraction Aborts Instead of Skipping](./why-extraction-aborts-instead-of-skipping.md);
  zip 9 does not relax it.

Related: [Archive Path Containment](./archive-path-containment.md) ·
[Data Archive Pipeline](./data-archive-pipeline.md) ·
[Testing Zip Extraction In-Crate](./testing-zip-extraction-in-crate.md) ·
[Declare Current As A CI-Computed Window](./declare-current-as-a-ci-computed-window.md)
