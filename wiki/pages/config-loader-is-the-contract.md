---
title: The Config Loader Is The Contract
type: decision
sources: [S006]
updated: 2026-10-06
---

Generated default configuration is tested by loading it through
`Config::with_loaded_weights` — the same call `oxeylyzer-repl` and the Tauri app
make at startup (S006).

**Rejected: parsing the TOML directly.** A hand-rolled parse would pass even if
the app's own loader rejected the file, certifying a document the app cannot
read. `Config` and its loader come from `oxeylyzer-core`, already a dependency of
the resources crate, so testing through the app's entry point costs nothing
(S006).

## What the loader being the contract bought

The only existing coverage of the generated config asserted that a file appeared
and was not overwritten. A default config that no loader could parse would have
passed the entire suite, and nothing ever checked that what `default_config_toml`
writes is loadable at all (S006).

## Same mistake, different artifact

Both of this project's loader mistakes reduce to one error: asserting on an
artifact instead of on the code that consumes it. The version-gate tests hit the
network because they called `ensure_data` rather than the comparison underneath —
see [Testing Zip Extraction In-Crate](./testing-zip-extraction-in-crate.md)
(S004, S006).

Related: [Generated Config Round-Trip](./generated-config-round-trip.md) ·
[Why Coverage Is Scoped By Measurement](./why-coverage-is-scoped-by-measurement.md) ·
[Data Archive Pipeline](./data-archive-pipeline.md)
