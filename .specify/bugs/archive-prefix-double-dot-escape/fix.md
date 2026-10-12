# Bug Fix: validate the stripped remainder against the archive depth rule

- **Slug**: archive-prefix-double-dot-escape
- **Fixed**: 2026-10-10
- **Assessment**: ./assessment.md
- **Status**: applied

## Summary

Closed the path-traversal hole in `extract_bytes` by validating the entry path
*after* `ZIP_INNER_PREFIX` is stripped, using the same component/depth rule
`zip`'s `enclosed_name()` applies. The existing library check is kept for the
unroutable forms (NUL, absolute, drive prefix); the new `is_contained` handles
the remainder, which is the string actually joined onto `dest`.

## Changes

| File | Change | Notes |
|------|--------|-------|
| `oxeylyzer-resources/src/download.rs` | added `is_contained` | Depth-counter check over the stripped remainder, same rule as `zip::read::ZipFile::enclosed_name` |
| `oxeylyzer-resources/src/download.rs` | modified `extract_bytes` | Rejects when the stripped remainder is not contained; abort-on-refusal preserved, so `.data-version` is still not written on a hostile archive |
| `oxeylyzer-resources/src/download.rs` | modified comment | The old comment described the correct rule while the code below it did the opposite; it now matches the code and its example uses a single `..` |
| `oxeylyzer-resources/src/download.rs` | test helpers | New `files_outside` wires the previously-dead `files_under`/`walk` helpers into an actual outside-the-destination assertion |
| `oxeylyzer-resources/src/download.rs` | tests added/updated | Single-`..`, padded single-`..`, sideways, and doubled-separator escapes; walk-based assertion in the existing traversal test; a test pinning the `files_outside` helper itself |

Scope expansion (logged per the guardrails): one extra test,
`doubled_separator_after_prefix_is_rejected`, covers a finding not in the
assessment — see **Deviations from Assessment**.

## Diff Highlights

```rust
fn is_contained(rel: &str) -> bool {
    let mut depth = 0usize;
    for component in Path::new(rel).components() {
        match component {
            Component::Prefix(_) | Component::RootDir => return false,
            Component::ParentDir => match depth.checked_sub(1) {
                Some(next) => depth = next,
                None => return false,
            },
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
        }
    }
    true
}
```

```rust
        if stripped.is_empty() {
            continue;
        }
        if !is_contained(stripped) {
            return Err(escapes(file.name()));
        }
```

## Tests Added or Updated

- `download::tests::single_parent_after_prefix_is_rejected` — the exact shape
  the ordering bug let through; asserts nothing outside `dest`.
- `download::tests::padded_single_parent_after_prefix_is_rejected` —
  `a/b/../../../escaped.txt`; padding with in-bounds segments must not launder
  the `..`.
- `download::tests::sideways_write_above_the_destination_is_rejected` —
  `../someotherapp/evil/payload.txt`; sibling of the data root is not writable.
- `download::tests::doubled_separator_after_prefix_is_rejected` — `//` after the
  prefix collapses to a rooted remainder under `enclosed_name()`, which the old
  ordering wrote to its literal path.
- `download::tests::traversal_escaping_root_is_rejected` — updated to assert via
  the walk (`files_outside`) instead of one hard-coded path, satisfying T007.
- `download::tests::files_outside_reports_everything_outside_the_destination` —
  pins the helper so a green walk assertion cannot be vacuous.
- Unchanged positive controls: `parent_segment_within_bounds_is_allowed`,
  `prefixed_entry_has_prefix_stripped`, `unprefixed_entry_extracts_at_root`,
  `archive_root_entry_is_skipped`, `mixed_valid_and_hostile_fails_without_partial_files`.

## Local Verification

- `rustfmt --edition 2024 --check oxeylyzer-resources/src/download.rs` → clean.
- `cargo clippy -p oxeylyzer-resources` → 0 warnings.
- `cargo test -p oxeylyzer-resources` → 39 passed, 0 failed (was 34 before this
  fix; 4 new escape tests + 1 helper-pinning test).
- `cargo test --workspace` → 219 passed, 0 failed across all targets.
- `cargo clippy --workspace` → one pre-existing warning in
  `oxeylyzer-repl/src/repl.rs:759` (`redundant reference in println!`), not
  touched by this change.
- `cargo fmt --check` (workspace) → pre-existing diffs in
  `oxeylyzer-core/src/generate.rs` only; the changed file is format-clean.
- Mutation check: temporarily replacing the new guard with `if false && ...`
  turned all four new escape tests red (`35 passed; 4 failed`), confirming the
  guard is load-bearing; the guard was then restored and the suite re-run green.
- Pre-fix baseline: the three single-`..`-family tests failed against the
  shipped ordering exactly as the assessment predicted.
- Manual check: replicated the old ordering in an out-of-repo scratch crate and
  confirmed the doubled-separator entry returns `Ok(())` and creates the file at
  its literal absolute path. Also re-confirmed, against the real crate version,
  `oxeylyzer-data-files/../escaped.txt` → `Ok` + file one level above `dest`
  before the fix, and `Err` + nothing written after it.

## Deviations from Assessment

- **Blast radius was understated by the assessment, and by the original report.**
  Both bounded the escape to one level above `dest` via a single `..`. A second
  separator after the prefix (`oxeylyzer-data-files//<absolute>`) leaves a rooted
  remainder once stripped, and `enclosed_name()` does not reject it because
  `Path::components()` collapses repeated separators. The old ordering therefore
  also permitted absolute-path writes, not just parent-directory writes. This
  raises the pre-fix impact above the assessment's "high" framing toward the
  report's original "arbitrary file write" claim, though reachability limits
  (fixed HTTPS GitHub URL; exploitation needs repo write access or a CA-level
  MITM) still keep it below critical. Covered by the new
  `doubled_separator_after_prefix_is_rejected` test; the fix already handled it
  via `Component::RootDir => false`, so no code change beyond the planned one was
  needed.
- The assessment suggested a separate follow-up change for
  `specs/001-archive-safety/tasks.md` T002; not done here (spec artifacts are
  not code), listed under Follow-ups.

## Follow-ups

- Correct `specs/001-archive-safety/tasks.md` T002, which still prescribes
  check-then-strip and will keep re-seeding this bug on wiki ingest. The
  assessment noted this should be a deliberate, separate change.
- Update `TODO.md` (the report's fix plan, around lines 81-145) to record the fix
  as landed.
- Re-ingest the wiki page `wiki/pages/archive-path-containment.md` now that the
  breach is fixed; until then it documents a hole that no longer exists.
- Consider a NUL-byte entry test to pin the one library behavior (NUL rejection)
  that a future hand-rolled rewrite of `enclosed_name` would silently drop — the
  assessment left this as a `[NEEDS CLARIFICATION]`.
- Optional: collapse the two containment checks in `extract_bytes` behind a
  single helper, now that `is_contained` exists, if a future reader finds the
  two-step (library check, then remainder check) surprising.