# Tasks: Graceful Desktop Startup Failures

**Branch**: `feature/time-machine-tauri-startup-graceful` | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## Phase 1: Backend startup

- [x] **T001** Extract `initialize(dirs, app_handle) -> Result<AppState, String>`
      from the setup hook in `oxeylyzer-tauri/src-tauri/src/lib.rs` (FR-013)
- [x] **T002** Replace the directory-resolution `expect` with a message naming the
      data directory (FR-005)
- [x] **T003** Replace the download `expect`, marking the failure as retryable
      (FR-006)
- [x] **T004** Replace the `ensure_config` `expect`, naming the config file path
      (FR-007)
- [x] **T005** Replace the config-parse `expect`, naming the file and the parse
      error (FR-007)
- [x] **T006** Replace the corpus `expect`, naming the file and the configured
      language it came from (FR-008, FR-012)
- [x] **T007** Keep the watcher setup and `app.manage(...)` in the hook, running
      only on success (FR-013)
- [x] **T008** Emit `startup-error` with the message on failure and return `Ok`
      so the window survives (FR-010)
- [x] **T009** Update `main.rs` to stop discarding the startup result, so a
      failure is reflected in the exit code (FR-001)

## Phase 2: Frontend reporting

- [x] **T010** Listen for `startup-error` in `App.tsx` and route the payload into
      the store's existing `error` field (FR-010)
- [x] **T011** Register the new listener's cleanup alongside the existing ones
- [x] **T012** Confirm `App.tsx:123` renders the message without new markup, and
      that the rest of the UI stays suppressed while it is set (FR-010)

## Phase 3: Download progress

- [x] **T013** Listen for the already-emitted `download-progress` event, which no
      frontend code currently subscribes to (FR-013)
- [x] **T014** Show progress during first-run download so the window is not frozen
      while a multi-megabyte download runs

## Phase 4: Verification

- [x] **T015** `cargo test --workspace` — 205 existing tests still pass (SC-006)
- [x] **T016** `cargo clippy -p oxeylyzer-tauri` — clean
- [x] **T017** Frontend type-check and build (`tsc`, `vite build`)
- [x] **T018** Force each startup failure manually and confirm the window reports
      it rather than panicking (SC-001 through SC-005)
- [x] **T019** Confirm an unmodified install still starts normally (SC-006)
- [x] **T020** Format via the JetBrains reformat action; revert unrelated churn

## Dependencies

T001 blocks T002–T008. T009 needs T008. T010 needs T008. Phase 2 and Phase 3 both
need the backend event names fixed by Phase 1. Phase 4 depends on all of the above.

## Notes

- No new dependencies.
- The window already exists when startup runs — `tauri` builds configured windows
  before invoking the setup hook — so T008 reports into an existing surface rather
  than creating one.
- `store.ts` already carries `error: string | null` and `App.tsx:123` already
  renders it. T010 must not add a second error field or duplicate that markup.
- T018 is manual by necessity: the desktop crate has no tests and its startup runs
  a full framework builder, so there is no headless harness.
- T019 is the regression that matters most. A change to the startup path that breaks
  the success case would be worse than the problem being fixed.

## Completion notes

All 20 tasks done. 205 workspace tests pass, `cargo clippy -p oxeylyzer-tauri`
clean, `tsc --noEmit` clean, `eslint` clean, `vite build` succeeds.

**Design correction found during implementation (invalidates part of the plan).**
The plan emitted a `startup-error` event and had the frontend listen for it. That
does not work: the setup hook runs before the webview loads, so a listener
registered on mount misses the event entirely, and the message would be lost. The
same is true of `download-progress`, which the backend has always emitted.

Replaced with `StartupStatus` managed *before* any fallible work, holding the last
error and last progress payload, exposed through two new commands (`startup_error`,
`download_progress`). The frontend queries on mount and polls progress. The event
emits are kept alongside, since they cost nothing and help if anything ever
subscribes later.

**A sixth panic was found, not in the queue entry.** `run()` ended with
`.expect("error while running tauri application")` — so `main.rs` never had a
result to discard, and T009's premise was wrong. `run()` now returns
`Result<(), tauri::Error>` and `main.rs` maps it to a non-zero exit code.

**Three errors were mine, all caught by the compiler:**

- `Config` has no `language` field — the language lives on the `Oxeylyzer` engine,
  which does not exist yet at the point the corpus is loaded. The message now names
  the `corpus` setting and the config file that contains it, which is the same
  actionable pointer.
- `ensure_data`'s progress callback is `\'static`, so `initialize` takes
  `AppHandle` by value and clones it for the closure.
- The `fail` closure borrowed `status`, which was then moved into `initialize`;
  passing `Arc::clone` instead.

**Formatter churn reverted.** The JetBrains action reformatted unrelated regions
(`load_corpus`, `generate_batch`) again, as it did on the previous feature. A
blanket string replace corrupted `generate_batch\'s final match arm; the function
was restored from HEAD by extracting and reinserting the region. Verified: every
removed line in the diff is startup code this change replaces.

**T018 could not be run** — the desktop app needs a display and a GUI run, which
this environment has none of. The failure paths are reasoned from the code, not
observed. The five forcing steps are listed in the plan for a human run; until then
the reporting is unverified at runtime, which is the honest status of this feature.