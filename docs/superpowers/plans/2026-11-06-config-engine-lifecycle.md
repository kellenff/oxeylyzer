# Configuration and Engine Lifecycle Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox syntax for tracking.

**Goal:** Resolve issues #4, #11, #12, and #18 without losing user settings or publishing stale layouts.

**Architecture:** Serialize configuration transitions and generation publication; retain the persisted corpus independently from the active session corpus. Debounce file events and monitor the parent directory so atomic replacements remain observable.

**Tech Stack:** Rust, Tauri, notify, SolidJS.

**Spec:** `docs/superpowers/specs/2026-11-06-config-engine-lifecycle-design.md`

## Global Constraints

- Session language is not persisted; external corpus-path changes select the new corpus.
- Failed saves preserve the prior file and active state; ordinary cancellation retains valid partial results.
- No stale generated result may be published or saved after an engine change.

## Review Focus

- Atomic replacement preserves later watcher notifications.
- A settings save racing an external edit must not silently overwrite that edit.
- Engine replacement racing completion cannot emit stale results after invalidation.
- Failed transient reloads do not suppress later valid events.
- A burst containing both layout and config changes refreshes both.

### Task 1: Coordinate engine and configuration transitions

**Files:** Modify `oxeylyzer-tauri/src-tauri/src/lib.rs`; tests in its existing test module.

- [ ] Add focused failing tests for switch-then-save, unchanged versus changed external corpus, and failure before persistence preserving both disk and active engine.
- [ ] Run the focused tests and verify failures correspond to the reported defects.
- [ ] Serialize transition preparation, persistence, and publication; track the active corpus independently from the configured corpus. Use same-directory atomic replacement, with failure cleanup and protection against concurrent external edits.
- [ ] Run focused tests and verify they pass.

### Task 2: Invalidate stale generation

**Files:** Modify `oxeylyzer-tauri/src-tauri/src/lib.rs` and `oxeylyzer-tauri/src/views/GenerateView.tsx`; add focused tests alongside existing tests.

- [ ] Add a deterministic failing test for engine replacement racing generation completion and an assertion that ordinary cancellation still retains partial results.
- [ ] Run the test and verify the stale-publication failure.
- [ ] Coordinate generation startup, invalidation, publication, and completion with the transition lock. Cancel stale work at a batch boundary and emit a completion with a reason and no results; show the reason in the view.
- [ ] Run focused tests and frontend checks.

### Task 3: Debounce filesystem changes

**Files:** Modify `oxeylyzer-tauri/src-tauri/src/lib.rs`; add tests alongside existing tests.

- [ ] Add failing tests for rapid edits, a transient invalid config followed by a valid one, mixed config/layout events, and continued monitoring after atomic replacement.
- [ ] Run tests and verify the relevant failures.
- [ ] Watch the config parent directory and debounce relevant events with a trailing reload, preserving all event categories. Ignore unrelated parent directory events.
- [ ] Run focused tests and confirm all pass.

### Task 4: Verify the batch

- [ ] Run Rust formatting checks and relevant Rust and frontend test suites; investigate failures.
- [ ] Review locking, filesystem errors, event ordering, and generated-result saving against the spec.
- [ ] Record any native desktop or platform-specific checks that could not be run.
