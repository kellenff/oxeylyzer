---
title: Formatting Rust Without cargo fmt
type: howto
sources: [S006]
updated: 2026-10-06
---

`cargo fmt` fails on this machine: it cannot load `librustc_driver` (S006). The
toolchain itself is fine — `cargo test` and `cargo clippy` both run — so a
`cargo fmt` failure here is a broken formatter, not a broken toolchain, and
should not be "fixed" by switching channels or reinstalling Rust (S006).

Format with the JetBrains reformat action instead, and revert any unrelated
reformatting it introduces. On the last run it produced no churn, but the
reformat action is not scoped to the files you edited, so the check is part of
the task rather than an optional cleanup (S006).

Related: [Why Coverage Is Scoped By Measurement](./why-coverage-is-scoped-by-measurement.md)
