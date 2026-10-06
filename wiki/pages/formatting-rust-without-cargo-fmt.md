---
title: Formatting Rust Without cargo fmt
type: howto
sources: [S006, S007, S008]
updated: 2026-10-06
---

`cargo fmt` fails on this machine: it cannot load `librustc_driver` (S006). The
toolchain itself is fine — `cargo test` and `cargo clippy` both run — so a
`cargo fmt` failure here is a broken formatter, not a broken toolchain, and
should not be "fixed" by switching channels or reinstalling Rust (S006).

Format with the JetBrains reformat action instead, and revert any unrelated
reformatting it introduces. The action is not scoped to the files you edited, so
the revert is part of the task rather than optional cleanup (S006).

**The churn is recurring, and the revert is not free.** Across three consecutive
features the action reformatted unrelated regions every time — `load_corpus`
and `generate_batch` among them (S007, S008). `generate_batch`'s final match
arm was damaged twice and had to be restored from HEAD, once by region
extraction and reinsertion and once line-wise (S007, S008). The churn itself is
not the expensive part; recovering the mangled function is.

> ⚠ conflict: the two accounts disagree about what damaged the match arm.
> `specs/005-tauri-startup-graceful/tasks.md` says the **blanket string replace**
> used to undo the churn corrupted it (S007). `specs/006-edit-view-rename-delete/tasks.md`
> says the **JetBrains action itself** mangled it, both times (S008).

The procedure covers either cause, which is why it does not have to pick: run
the formatter, then read the diff's hunk list before committing, and never undo
churn with a broad string replace (S007, S008). Review by function name, not
line count — every removed line should be one the change deliberately replaces,
and if a function you did not touch appears in the diff, treat its contents as
suspect rather than assuming the diff is honest (S007, S008).

Three features running makes this expected work rather than an anomaly (S008).

Related: [Why Coverage Is Scoped By Measurement](./why-coverage-is-scoped-by-measurement.md) ·
[Desktop Startup Failure Reporting](./desktop-startup-failure-reporting.md) ·
[Layout File Lifecycle](./layout-file-lifecycle.md)
