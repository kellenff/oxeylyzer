# Generated Layout Save Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans
> for native execution, or superpowers:subagent-driven-development if selected.

**Goal:** Refuse generated-layout name conflicts without replacing existing files.

**Architecture:** Serialize in `save_generated`, then call a private filesystem
helper that creates the destination exclusively and writes all bytes. Keep the
existing command and frontend error handling.

**Tech Stack:** Rust standard library, existing serde/Tauri integration, rustfmt.

**Spec:** `docs/superpowers/specs/2026-11-06-generated-layout-save-design.md`

## Global Constraints

- Preserve the existing name-to-filename normalization and command signature.
- Reuse the Generate screen's existing error display and retry controls.
- Do not add dependencies.
- Serialization must finish before opening the destination.
- Atomic creation does not promise crash-safe publication of a new file.

## Review Focus

- Existing destinations: return a conflict and preserve all original bytes.
- New destinations: create missing parents and write every serialized byte.
- Repeat saves: refuse the second write and preserve the first result.
- Invalid parent paths: return an I/O error without altering the parent file.
- Competing writers: exactly one exclusive creation succeeds.

## Task 1: Protect the generated-layout write boundary

**Files:**
- Create/test: `oxeylyzer-tauri/src-tauri/src/generated_layout.rs`.
- Modify: `oxeylyzer-tauri/src-tauri/src/lib.rs`, module declaration and
  `save_generated` write call only.

**Interfaces:**
- Consumes: destination `&Path` and already-serialized DOF JSON `&str`.
- Produces: private `write_new(path: &Path, json: &str) -> Result<(), String>`.
- `AlreadyExists` message includes the destination and asks for another name.
- Other creation/write failures propagate as errors; layout loading and insertion
  remain after the successful write.

- [ ] Extract the current parent-directory creation and overwriting write into
  `write_new`, preserving existing behavior initially. Wire the command to it.
- [ ] Add isolated standard-library filesystem tests: `existing_file_is_preserved`
  compares arbitrary original bytes after a refused save; `new_file_is_written`
  compares the complete JSON bytes and checks a second save fails;
  `invalid_parent_is_reported` preserves a parent that is a regular file;
  `concurrent_saves_have_one_winner` checks one success and one intact payload.
- [ ] Run the tests against the old write behavior and confirm preservation,
  repeat-save, and competing-writer assertions fail for the expected reason.
- [ ] Replace overwriting writes with `OpenOptions::create_new(true)` and
  `Write::write_all`, mapping `AlreadyExists` to the actionable conflict message.
- [ ] Run `cargo test -p oxeylyzer-tauri --lib` and
  `cargo fmt --all -- --check`. If desktop dependencies prevent crate tests,
  run the exact production helper's tests directly with
  `rustc --edition=2024 --test oxeylyzer-tauri/src-tauri/src/generated_layout.rs`;
  document the crate-build limitation separately.
- [ ] Review the command wiring, unchanged serialization order, and error path
  through `saveGenerated` and `GenerateView.handleSave`.
- [ ] Commit the coherent fix and design/plan artifacts; do not push or update
  the GitHub issue.
