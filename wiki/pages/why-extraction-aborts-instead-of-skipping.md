---
title: Why Extraction Aborts Instead of Skipping
type: decision
sources: [S001, S002, S014]
updated: 2026-10-13
---

An archive entry the containment check **refuses** aborts extraction. It is
not skipped, and extraction does not report success (S001).

**Rejected alternative:** skip the offending entry and keep extracting (S001).

**Why that is wrong here:** `.data-version` is written only after the
extraction loop completes. Skip-and-continue reaches that write, so a
partially applied hostile archive gets recorded as a complete install, and the
next startup does not retry. Aborting is what keeps the requirement that a
failed extraction never leaves a state mistaken for a finished install (S001).

Aborting is also the existing style, not a new policy: `download.rs`
propagates every failure with `?`, and `seed.rs` already treats a failed
download as fatal. There is no partially-ok recovery path to preserve (S001).

The refusal error names the offending entry, so a mixed valid-and-hostile
archive tells the user which item was rejected rather than silently producing
a partial data directory (S001).

## Entries the check accepts bypass this entirely

The abort path only engages for entries the containment check rejects. An entry
it accepts is extracted and reported as success like any other — no abort, no
sentinel. Until `3d0d66d` one accepted shape escaped the destination instead of
being refused; that breach is closed (see
[Archive Path Containment](./archive-path-containment.md)) (S002, S014).

Under `zip` ≥ 9 this section matters more, not less: `enclosed_name()` stopped
rejecting and started *normalizing*, so a hostile name becomes an accepted,
in-bounds write the archive never claimed — with the sentinel written behind
it. Feature 009's answer is to keep abort-on-refusal and move the decision to
the raw name: [Zip 9 Normalizes Paths That Zip 2 Refused](./zip9-normalizes-paths-that-zip2-refused.md)
(S014).

Related: [Archive Path Containment](./archive-path-containment.md) ·
[Zip 9 Normalizes Paths That Zip 2 Refused](./zip9-normalizes-paths-that-zip2-refused.md) ·
[Data Archive Pipeline](./data-archive-pipeline.md)