---
title: Why the Data Version Is Hand-Bumped
type: decision
sources: [S004]
updated: 2026-10-06
---

`DATA_VERSION` in `oxeylyzer-resources/src/download.rs` is a string constant
bumped by hand when the published data release changes (S004).

**Rejected alternative:** derive it from the crate version (S004).

**Why rejected:** a bugfix release of the app would change the derived version,
which re-downloads every user's data files for no reason (S004). The data
archive and the app code release on different schedules, so the constant that
gates one cannot come from the other.

## Consequences of hand-bumping

The gate only works if someone remembers to bump it. Forgetting is silent: users
keep stale data and the app keeps reporting scores derived from it, with no way
for the user to see or correct it — which is exactly the failure the feature
exists to remove (S004).

The version bumped at build time is compared against a version recorded at
download time. Verification of the real transition happened by accident: a test
that was supposed to use a fake download hit the network, fetched the actual
archive, and advanced a `0.0.1` sentinel to `1.0.0` (S004). See
[Testing Zip Extraction In-Crate](./testing-zip-extraction-in-crate.md) for why
that run is not evidence about the extraction guard.

## Refresh is wholesale, not merged

Data files are refreshed by re-extracting the release over the data directory —
all-or-nothing. There is no per-file merge, so there is no partial-upgrade state
to reason about (S004). That is what makes "never prune" and "sentinel written
only after success" sufficient rather than needing a rollback mechanism.

Related: [Data Version Currency](./data-version-currency.md) ·
[Refresh Replaces Shipped Paths](./refresh-replaces-shipped-paths.md) ·
[Data Archive Pipeline](./data-archive-pipeline.md)