# Path Boundaries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement task-by-task. Steps use checkbox syntax.

**Goal:** Prevent path traversal through managed layout and preset names and downloaded ZIP entries.

**Architecture:** Validate path inputs where they enter filesystem operations. Keep layout normalization and archive prefix handling, and test the actual file operations using temporary directories.

**Tech Stack:** Rust, existing zip crate, Cargo tests.

**Spec:** `docs/superpowers/specs/2026-11-06-path-boundaries-design.md`

## Global Constraints

- Preserve valid names and legitimate nested archive entries.
- Reject unsafe paths with an error; do not add dependencies or options.
- Existing malicious symlinks and transactional archive rollback are outside scope.

## Review Focus

- Empty, dot, and parent names must not designate a directory.
- Windows separators and drive-like names must be rejected on every platform.
- Valid names containing spaces must retain existing layout normalization.
- Traversal following the archive prefix must be rejected before writing that entry.
- Nested valid archive entries and the root prefix directory must continue to extract.

### Task 1: Managed names (issue #3)

**Files:** Modify and test `oxeylyzer-tauri/src-tauri/src/lib.rs`, using existing test conventions.

**Interface:** Add a private `validate_filename(name: &str) -> Result<(), String>`; call it before path formation in generated, custom, edited, forked and deleted layouts and preset save/load. Validate the actual normalized layout filename as well as any input needed to prevent transformations hiding invalid names.

- [ ] Add failing tests for valid names, spaces, empty names, `.`, `..`, `/`, `\\`, absolute and drive-like names. Exercise real preset and layout path operations with temporary directories and assert an attempted `../config` leaves a neighboring config unchanged.
- [ ] Run the focused tests and confirm the unsafe inputs currently fail the assertions.
- [ ] Implement boundary validation and wire all path-forming callers; preserve their existing naming conventions and error handling.
- [ ] Run `cargo test -p oxeylyzer-tauri` and `cargo fmt --all -- --check`; confirm success.

### Task 2: Archive extraction (issue #14)

**Files:** Modify and test `oxeylyzer-resources/src/download.rs`.

**Interface:** Extract the existing archive-processing block into a private function accepting archive bytes or a reader and a destination, returning `Result<(), ResourceError>`; the download path calls this function before writing the version marker.

- [ ] Add failing tests using generated in-memory ZIPs: prefixed and unprefixed traversal, absolute paths, platform-specific unsafe paths, and a legitimate nested file and prefix root. Verify rejected entries do not change a neighboring sentinel and do not produce a success marker.
- [ ] Run the focused resource tests and confirm traversal tests fail against the current behavior.
- [ ] Reject unsafe paths before joining or writing, checking the entry and stripped path; preserve prefix handling and valid nested extraction.
- [ ] Run `cargo test -p oxeylyzer-resources` and `cargo fmt --all -- --check`; confirm success.

### Task 3: Integrated verification

- [ ] Review all path-forming callers and the final diff against the spec; verify no unsafe name route was missed.
- [ ] Run the relevant package tests and formatter after all changes; report any environment-limited checks and the symlink and rollback limits.
