---
title: Refresh Replaces Shipped Paths
type: decision
sources: [S004]
updated: 2026-10-06
---

Built-in data files and user-created layouts share one directory, so a refresh
cannot update the corpus without also landing inside the layout directory. The
accepted approach is to re-extract the new release over the existing directory:
every file the release ships is replaced with the released version, and files it
does not ship are left alone (S004).

## The consequence, stated plainly

A saved layout whose path collides with a path the new release ships **is
overwritten by the released file**. A saved layout at any other path survives
untouched (S004).

**Rejected alternative:** keep user layouts in a separate directory from
built-in data (S004).

**Why rejected:** it relocates every existing user's saved layouts and splits the
data root's permissions and backup story. The project accepts the collision
risk instead because generated layouts get distinct names and rarely collide
with a shipped filename (S004).

The tradeoff is recorded deliberately so it reads as a choice rather than a bug
the user discovers when a layout disappears (S004).

## Never prune

The data directory is never pruned: files the archive does not ship stay
(FR-008, FR-012). A future release that drops a built-in file must not silently
delete it from existing installations (S004). This is why refresh is "extract
over" rather than "empty the directory, then extract" — and it is why refresh
has no rollback: there is nothing to roll back to, because nothing is removed.

## What this means for a failed refresh

Refresh failure during download or extraction leaves the previously installed
data usable and the recorded version unchanged, so the next startup retries
(FR-009, FR-010). No new machinery was needed for this — the sentinel is
written only after a successful extraction, so failure propagates before the
write (S004). That property is the same one that makes
[Why Extraction Aborts Instead of Skipping](./why-extraction-aborts-instead-of-skipping.md)
necessary, so the abort behavior is load-bearing for upgrade correctness too.

Note the boundary: "failed refresh leaves data intact" holds at the version
record, not as a transactional guarantee about file contents. Extraction
overwrites files as it goes, so a failure partway through leaves some files new
and some old while the sentinel still reads the old version — which is why the
next startup retries the whole thing rather than trusting a partial result
(S004).

Related: [Data Version Currency](./data-version-currency.md) ·
[Why the Data Version Is Hand-Bumped](./why-the-data-version-is-hand-bumped.md) ·
[Data Archive Pipeline](./data-archive-pipeline.md)