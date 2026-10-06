---
title: Why Extraction Aborts Instead of Skipping
type: decision
sources: [S001, S002]
updated: 2026-10-06
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

The abort path only engages for entries `enclosed_name()` rejects. An entry it
accepts is extracted and reported as success like any other — no abort, no
sentinel. One such shape currently escapes the destination instead of being
refused: see [Archive Path Containment](./archive-path-containment.md) (S002).

Related: [Archive Path Containment](./archive-path-containment.md) ·
[Data Archive Pipeline](./data-archive-pipeline.md)