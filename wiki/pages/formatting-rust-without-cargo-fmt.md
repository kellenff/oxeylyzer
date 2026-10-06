---
title: Formatting Rust Without cargo fmt
type: howto
sources: [S006, S007]
updated: 2026-10-06
---

`cargo fmt` fails on this machine: it cannot load `librustc_driver` (S006). The
toolchain itself is fine — `cargo test` and `cargo clippy` both run — so a
`cargo fmt` failure here is a broken formatter, not a broken toolchain, and
should not be "fixed" by switching channels or reinstalling Rust (S006).

Format with the JetBrains reformat action instead, and revert any unrelated
reformatting it introduces. The action is not scoped to the files you edited, so
the revert is part of the task rather than optional cleanup (S006).

**The churn is recurring, and the revert is not free.** On the next feature the
action reformatted unrelated regions again (`load_corpus`, `generate_batch`), and
a blanket string replace to undo it corrupted `generate_batch`'s final match arm
— the function had to be restored from HEAD by extracting and reinserting the
region (S007).

So: review the reformat diff by function name, not by line count, and never
undo churn with a broad string replace. Every removed line in the resulting diff
should be a line this change deliberately replaces (S007).

Related: [Why Coverage Is Scoped By Measurement](./why-coverage-is-scoped-by-measurement.md) ·
[Desktop Startup Failure Reporting](./desktop-startup-failure-reporting.md)
