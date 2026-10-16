# Implementation Plan: Safe Data Archive Extraction

**Branch**: `feature/time-machine-archive-safety` | **Date**: 2026-10-05 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification `specs/001-archive-safety/spec.md`

## Summary

Close the archive-extraction path-traversal hole in `oxeylyzer-resources`. Entry
paths are currently taken from the zip and joined onto the destination directory
verbatim, so a crafted archive can write outside the data root. Replace the raw
`file.name()` with the extraction library's built-in containment check, which
returns `None` for any entry that would escape.

**Technical approach**: use `ZipFile::enclosed_name()` from `zip` 2.4.2 (already a
dependency) rather than hand-rolling path normalization. It rejects absolute
paths, drive prefixes, NUL bytes, and any `..` sequence that would climb above
the entry root.

**Ordering constraint**: the top-level-folder prefix must be stripped *before*
the containment check, not after. `enclosed_name()` tracks a depth counter that
the prefix itself increments, so
`oxeylyzer-data-files/../../../etc/passwd` validates as acceptable when checked
whole — the prefix supplies the depth that the three `..` segments then consume.
Stripping first and validating the remainder rejects it correctly.

## Technical Context

**Language/Version**: Rust, edition 2024, `stable` per `rust-toolchain.toml`
**Primary Dependencies**: `zip` 2.4.2 (`default-features = false`, `deflate`), `ureq` 2 — both already in `oxeylyzer-resources/Cargo.toml`
**Storage**: filesystem — a zip archive on disk, extracted into the resolved data root
**Testing**: `cargo test` — in-crate unit tests, no dev-dependency needed
**Target Platform**: Linux, macOS, Windows
**Project Type**: library crate (workspace member `oxeylyzer-resources`)
**Performance Goals**: unchanged — no measurable cost; the check is per-entry string scanning during a network-bound operation
**Constraints**: must not change extraction output for well-formed archives; must not add dependencies; must not alter the progress-callback contract
**Scale/Scope**: one function body in one file, plus one test module

## Changes

### Only file modified

`oxeylyzer-resources/src/download.rs`

The extraction loop currently reads:

```rust
let raw_name = file.name().to_string();
let stripped = match raw_name.strip_prefix(ZIP_INNER_PREFIX) {
    Some(s) => s,
    None => &raw_name,
};
if stripped.is_empty() {
    continue;
}

let out_path = dest.join(stripped);
```

Three changes to that block:

1. **Derive the relative path via `file.enclosed_name()`** instead of
   `file.name()`. Returns `None` when the entry cannot be contained.
2. **Strip `ZIP_INNER_PREFIX` from the enclosed path**, not from the raw name.
   The library returns the entry name as a `PathBuf`; strip the prefix from
   its string form and rebuild.
3. **On `None`, return an error naming the offending entry** rather than
   skipping it (FR-003). The error uses the existing `ResourceError::Extraction`
   variant, which is already declared and already carries a `String`.

The `stripped.is_empty()` early-continue is preserved — it handles the archive's
own root directory entry, which is not an error.

### Why abort rather than skip

`download.rs` propagates every failure with `?`; `seed.rs` treats a failed
download as fatal; and the `.data-version` sentinel at line 86 is written only
after the loop completes. Skipping a hostile entry and continuing would write
that sentinel, marking a partially applied archive as a complete install. Aborting
is both the existing style and the only behavior that keeps SC-003 true.

### No symlink handling

The loop never calls `is_symlink()` and nothing in the workspace creates
symlinks. A symlink entry's content is the target path, written as ordinary file
bytes — no link is created, so there is nothing to follow. Out of scope.

### No download streaming

The archive is ~4 MB (`static/text` 2.2M, `static/language_data` 2.0M).
Buffering it is not a practical failure, and streaming would add temp-file
staging, its cleanup path, and its own failure mode. Deferred.

## Project Structure

```text
oxeylyzer-resources/
├── src/
│   ├── lib.rs      # ResourceError enum — unchanged, Extraction(String) already exists
│   ├── dirs.rs     # unchanged
│   ├── download.rs # only modified file
│   └── seed.rs     # unchanged
```

## Verification

**GATE 0 — spec checklist** — [requirements.md](./checklists/requirements.md) complete, no open items.

**Existing behavior preserved**

```bash
cargo test -p oxeylyzer-resources     # no tests yet; must compile clean
cargo clippy -p oxeylyzer-resources
cargo test --workspace               # 171 existing tests must still pass
```

**New tests** — hostile archives, built in-crate with `zip::ZipWriter` writing
into an in-memory `Cursor`. No fixtures, no dev-dependencies. Each test extracts
into a temp directory and asserts on what landed.

Per SC-001, cover each hostile shape:

| Test | Archive entry | Expect |
|---|---|---|
| traversal escapes | `data-files/../../../etc/passwd` | `Err`, nothing written outside dest |
| absolute path | `/etc/passwd` | `Err` |
| parent within bounds | `data-files/a/../b.json` | `Ok`, lands at `dest/b.json` |
| prefix stripped | `data-files/static/text/en.txt` | `Ok`, at `dest/static/text/en.txt` |
| no prefix | `static/text/en.txt` | `Ok`, at `dest/static/text/en.txt` |
| root entry | `data-files/` | skipped, no error |

The first test asserts on the filesystem, not just the return value — the
security property is that no file exists outside `dest`, so the test must walk
the parent directory and confirm nothing new appeared.

Test-only helpers for creating a temp directory: use `std::env::temp_dir()`
joined with a unique name. Adding `tempfile` as a dev-dependency is deliberately
avoided — `tempfile` appears in `Cargo.lock` only as a transitive dependency of
`uds_windows`, not as anything this crate can rely on, and the tests need one
directory, not a general facility.

## Complexity Tracking

No constitution violations. The project constitution at
`.specify/memory/constitution.md` is still the unmodified Spec Kit template with
placeholders, so there are no project principles to conflict with.

| Choice | Simpler alternative rejected because |
|---|---|
| Abort on hostile entry | Skipping writes the success sentinel and records a partial install as complete |
| `enclosed_name()` | Hand-rolled normalization must reimplement `..` depth tracking, absolute-path detection, and NUL handling — all cases the library already covers |
| Strip prefix before validating | Validating first lets the prefix's own depth absorb the `..` segments |