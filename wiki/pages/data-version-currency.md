---
title: Data Version Currency
type: concept
sources: [S004, S005, S006]
updated: 2026-10-06
---

Whether an installation's built-in data files match what the running build
expects. Three states, not a boolean (S004):

```rust
pub enum DataCurrency {
    Absent,                          // nothing recorded: unreadable or empty sentinel
    Stale   { installed: String },   // recorded, and behind — or not parseable
    Current { installed: String },   // recorded, and at-or-ahead of current
}
// needs_download() == !matches!(self, Current { .. })
```

A boolean loses the distinction between "never downloaded" and "downloaded but
behind", which the Tauri startup screen needs for messaging (S004).

## The version comparison

`compare_versions(a, b) -> Option<Ordering>` in `dirs.rs` splits on `.` and
compares component-wise as `u64`; when all shared components match, the longer
version is newer, so `1.0` is older than `1.0.1` (S004, S005). `None` means
either side is not a dot-separated list of numbers — empty and `1..0` both
count (S005).

No `semver` dependency: component-wise `std::cmp::Ordering` covers two
dot-separated triples (S004).

## Downgrades deliberately land in `Current`

A recorded version *newer* than current means the user downgraded the app. That
is `Current`, not `Stale`, so no download occurs and data is not forcibly rolled
back — FR-005 and SC-004, satisfied without a fourth variant (S004, S005).



The current version is a build constant under project control, so if it ever
fails to parse the safest reading is to download rather than assume currency
(S004).

## What replaced `is_first_run()`

`OxeylyzerDirs::is_first_run()` became `needs_data()`, which just delegates to
`data_currency().needs_download()` (S005). Under version semantics "first run"
asserts install history, while the actual question is whether data must be
fetched (S004). The `first_run` field and its hardcoded `false` in
`with_override` were deleted (S004).

One external call site, in the Tauri app's startup path; `oxeylyzer-repl` calls
`ensure_data` unconditionally and relies on the internal gate, so it needed no
change (S004, S005).

## "Unreadable" meant two things, twenty lines apart

`installed_version` returns `None` when `read_to_string` fails — which covers a
missing file *and* a permission-denied one — or when the trimmed content is
empty. That is the only path to `Absent` (S005).

A sentinel that reads back but does not parse returns its string and lands in
`Stale`: a pre-release suffix, a `v` prefix, or a negative component fails
`compare_versions`, and the caller maps `None` to `Stale` (S005).

> ⚠ conflict: `specs/002-data-version-upgrade/tasks.md` T010 says unparseable
> versions "map to `Absent`" (S004). Shipped code maps them to
> `Stale { installed: "not-a-version" }` (S005). Resolved in the code, not in the
> artifact: T010 is not editable, so its wording stays wrong in history.

The variant rustdocs had the same collision. `Absent` read "an unreadable
sentinel", which is true in the filesystem sense but reads as *unparseable* —
and the test `unreadable_sentinel_version_is_treated_as_stale`, twenty lines
below it, uses the word to mean the opposite (S005). `Stale` was the one that
was actually false: it claimed a recorded version "is older than the current
one", while unparseable versions land there too and are not older (S005).

Both comments now state the split explicitly — `Absent` for what cannot be read
or is empty, `Stale` for what is older *or* unparseable (S005).

Currency is now also checked against a real-shaped root: a fresh root with no
sentinel reports data needing download, and repeated platform-root lookups return
identical roots (S005, S006). See [Platform Root Resolution](./platform-root-resolution.md).

Related: [Data Archive Pipeline](./data-archive-pipeline.md) ·
[Why the Data Version Is Hand-Bumped](./why-the-data-version-is-hand-bumped.md) ·
[Refresh Replaces Shipped Paths](./refresh-replaces-shipped-paths.md) ·
[Platform Root Resolution](./platform-root-resolution.md) ·
[Config Root And Data Root](./config-root-and-data-root.md)