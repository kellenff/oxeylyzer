---
title: Data Version Currency
type: concept
sources: [S004, S005]
updated: 2026-10-06
---

Whether an installation's built-in data files match what the running build
expects. Three states, not a boolean (S004):

```rust
pub enum DataCurrency {
    Absent,                          // nothing recorded, or the sentinel is empty
    Stale   { installed: String },   // recorded, and behind
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

> ⚠ conflict: `specs/002-data-version-upgrade/tasks.md` T010 says unparseable
> versions "map to `Absent`" (S004). The shipped code maps a sentinel that has
> content but will not parse to `Stale { installed: "not-a-version" }`, and
> `Absent` only when the file is missing or empty after trimming
> (`dirs.rs:98-103, 120-128`) (S005). The distinction does not change behavior
> — both need a download — but it changes which variant a caller pattern-matches
> on, and `Stale` carries the garbage string through. T010's wording is
> inaccurate; the test named
> `unreadable_sentinel_version_is_treated_as_stale` matches the code.

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

Related: [Data Archive Pipeline](./data-archive-pipeline.md) ·
[Why the Data Version Is Hand-Bumped](./why-the-data-version-is-hand-bumped.md) ·
[Refresh Replaces Shipped Paths](./refresh-replaces-shipped-paths.md)