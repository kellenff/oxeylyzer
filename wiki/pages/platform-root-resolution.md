---
title: Platform Root Resolution
type: component
sources: [S005, S006]
updated: 2026-10-06
---

`OxeylyzerDirs::resolve()` in `dirs.rs` used to ask the platform where
directories are, create them, and record the sentinel. The asking half is now a
private `platform_roots() -> Option<(PathBuf, PathBuf)>` that returns
`(ProjectDirs::config_dir, ProjectDirs::data_dir)` for `oxeylyzer` without
creating anything; `resolve()` is that call plus `create_dir_all` on each root
(S006).

The public signature and behavior are unchanged. The `NoDirFound` mapping stayed
in `resolve()`, which is why the new function returns `Option` and not `Result` —
returning `Result` would duplicate an error mapping that already exists one level
up (S006).

## The split exists only so containment is testable without side effects

The constraint driving it: tests must not create or modify anything under the
user's real `~/.config` or platform data directory. Calling `resolve()` in a test
does exactly that (S006). So the property worth asserting — every derived path
sits under the platform-reported root — could not be checked against the real
platform roots until creation was separated from asking.

Every fixed location is derived by joining onto the config root or data root, so
containment holds by construction; the tests verify it rather than trusting the
construction (S006).

## The plan's test strategy contradicted FR-006

> ✔ resolved: `plan.md` said the containment tests use "a temp root shaped like a
> platform root", because deriving from the actual platform paths "would make the
> test machine-dependent"; `spec.md` FR-006 requires exactly that — paths
> derived from the directories the platform *reports*, verified without creating
> anything on disk (S006).

**Settled by the code: the spec's approach shipped.** `platform_derived_paths_all_sit_under_platform_roots`
calls `platform_roots()` and asserts each of the nine derived paths starts with
either root; `platform_roots_are_reported_without_being_created` and
`platform_paths_are_stable_across_calls` do the same (S005). The plan's
temp-root approach was not used, so `plan.md`'s rationale is stale prose in an
immutable artifact and corrects nothing here (S005, S006).

The marker is closed rather than deleted because `plan.md` cannot be edited from
here and its machine-dependence argument still describes the shipped code: the
platform tests *are* machine-dependent, skipping silently where a platform
reports no conventional location rather than failing (S005).

Also verified alongside the split: repeated calls report identical roots, and a
fresh root with no `.data-version` reports data needing download — see
[Data Version Currency](./data-version-currency.md) (S005, S006).

## The tests skip rather than fail where there is nothing to assert

All three platform tests early-return when `platform_roots()` yields `None`,
with the comment "No conventional location on this platform; nothing to assert"
(S005). On such a machine the containment property is unverified, not verified.
That is the honest failure mode — the alternative, asserting against an invented
root, would prove nothing — but it means a green suite does not imply the
property held everywhere it ran (S005).

Related: [Data Archive Pipeline](./data-archive-pipeline.md) ·
[Why Coverage Is Scoped By Measurement](./why-coverage-is-scoped-by-measurement.md) ·
[The Config Loader Is The Contract](./config-loader-is-the-contract.md) ·
[Config Root And Data Root](./config-root-and-data-root.md)
