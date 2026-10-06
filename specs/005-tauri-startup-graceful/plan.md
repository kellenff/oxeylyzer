# Implementation Plan: Graceful Desktop Startup Failures

**Branch**: `feature/time-machine-tauri-startup-graceful` | **Date**: 2026-10-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification `specs/005-tauri-startup-graceful/spec.md`

## Summary

Replace five `.expect()` calls in the desktop app's startup hook with errors that
reach the user. The framework, the frontend, and the surrounding Rust code already
provide every piece this needs — the work is removing the shortcuts, not building
machinery.

**Technical approach**: extract startup into a function returning
`Result<AppState, String>` using the same error-mapping style as the existing
`reload_state`, then have the hook report a failure into the already-open window
instead of panicking.

## Technical Context

**Language/Version**: Rust edition 2024 (`src-tauri`), TypeScript/SolidJS (frontend)
**Primary Dependencies**: none added — `tauri` 2.10.3 and `@tauri-apps/api` already present
**Storage**: filesystem only
**Testing**: `cargo test --workspace` for regressions; startup failure paths verified by forcing them manually
**Target Platform**: desktop — macOS, Linux, Windows
**Project Type**: desktop app
**Performance Goals**: no measurable change to startup
**Constraints**: no new dependencies; startup step order preserved; `cargo fmt` broken on this machine, so format via the IDE reformat action
**Scale/Scope**: one Rust file, one frontend component

## What the code already provides

Established before planning — none of this is new work:

- **The window exists before startup runs.** `tauri` builds every configured window
  before invoking the setup hook (`app.rs`: the window loop runs at line 2372, setup
  is called at line 2380). The configured `windows` entry has no `visible: false`,
  so the window is already showing when startup begins. FR-009 is satisfied by the
  existing shape.
- **The frontend already renders errors.** `store.ts` carries `error: string | null`;
  `App.tsx:123` renders it and suppresses the rest of the UI while it is set. Startup
  failures reuse this rather than needing new UI.
- **The error-mapping style already exists.** `reload_state` (`lib.rs:1578`) returns
  `Result<(), String>` with messages naming the file: `"Failed to load config: {e}"`,
  `"Failed to load corpus: {e}"`. Startup adopts the same wording.
- **Every error type is already specific.** `ResourceError` distinguishes
  `NoDirFound`, `Download`, `Config`, and `Io`; `OxeylyzerError` covers corpus and
  config parse failures. Nothing needs inventing to report the cause.

## Backend changes

### Extract a fallible startup

```rust
/// Performs startup work, returning a human-readable message per failure.
fn initialize(dirs: OxeylyzerDirs, app_handle: &AppHandle) -> Result<AppState, String>
```

Each of the five sites becomes a `map_err` that names both the step and the
location:

| Current | Becomes |
|---|---|
| `OxeylyzerDirs::resolve().expect("failed to resolve data directory")` | `Could not set up the oxeylyzer data directory: {e}` |
| `dirs.ensure_data(..).expect("failed to download resources")` | `Could not download the built-in data files: {e}` — retryable, message says so |
| `dirs.ensure_config().expect("failed to write default config")` | `Could not write {path}: {e}` |
| `Config::with_loaded_weights(..).expect("failed to load config.toml")` | `Could not parse {path}: {e}` |
| `Data::load(..).expect("failed to load corpus")` | `The corpus file {path} configured for language {lang} could not be read: {e}` |

The last one satisfies FR-012: naming the configured language is what makes it
actionable, since the user fixes it by changing that setting.

The config reload watcher setup stays where it is — it closes over `state`, so it
remains in the hook after `initialize` succeeds.

### Report into the window

The hook returns `Ok(())` on success as before. On failure it emits a
`startup-error` event carrying the message, then returns the error so the framework
still records the failure:

```rust
.setup(|app| {
    match initialize(dirs, app.handle()) {
        Ok(state) => { /* existing watcher setup, manage(state), Ok(()) */ }
        Err(message) => {
            let _ = app.handle().emit("startup-error", message);
            Ok(())
        }
    }
})
```

Returning `Err` as well would make the framework tear the window down, which is the
opposite of showing the user a message. `main.rs` additionally stops discarding the
result so the exit code reflects the failure.

## Frontend changes

One listener in `App.tsx`, wired into the store's existing error field:

```ts
const unlisten = await listen<string>("startup-error", (e) => {
  setAppStore("error", e.payload);
});
```

`App.tsx:123` then renders it with no new markup. One cleanup: `onCleanup` already
unregisters the two existing listeners, so add this one there.

### Download progress, also missing

The backend emits `download-progress` (`lib.rs:1628`) but **nothing listens for it**
— grep across `src/` finds no subscriber, so the loading screen its comment
describes does not exist. Since startup failure reporting and startup progress
share the same channel and the same phase, this is included: a first-run user
currently sees a frozen window during a multi-megabyte download, then either a
working app or an unexplained failure. A small status line driven by the existing
event is proportionate.

## Not in scope

- **Corpus load stays blocking.** `AppState.engine` is `Arc<Oxeylyzer>` built from
  the corpus; ~20 commands lock it. Making it optional is a separate piece of work.
  FR-011 makes the message actionable instead.
- **Retry without restart.** The window shows what to do; adding a retry button means
  a new command that re-runs initialization against live state.
- **Non-startup failures.** `reload_state` already returns errors that the watcher
  logs; FR-014 leaves it alone.
- **The REPL.** Already reports errors on its own terms.

## Verification

```bash
cargo test --workspace          # 205 tests must still pass
cargo clippy -p oxeylyzer-tauri
bun run tsc --noEmit            # if configured
bun run build
```

Startup failure paths are forced manually, one per site — there is no harness for
running the desktop app headless, and the crate has no tests:

| Force | Expect |
|---|---|
| `OXEYLYZER_DATA_DIR` pointed at an unwritable path | Window opens, names the directory |
| Airplane mode on a fresh install | Window opens, download failure named, no panic |
| Malformed `config.toml` | Window opens, names the file and the parse error |
| `corpus` in config pointing at a deleted file | Window opens, names the file and the language |
| Unmodified install | Startup unchanged, progress line shows during first-run download |

That last row is the regression risk that matters most: a change to the startup path
that breaks the success case would be worse than the problem being fixed.

## Complexity Tracking

No constitution violations.

| Choice | Simpler alternative rejected because |
|---|---|
| `initialize` returning `Result<AppState, String>` | Passing closures through `expect`-free code is harder to read than one function with early returns |
| Emit event, return `Ok` | Returning `Err` makes the framework tear down the window, destroying the surface we just populated |
| Reuse `appStore.error` | New error UI duplicates an existing field and its conditional rendering |
| Include download progress | The same channel and phase; leaving it frozen while reporting failures would be a half-fix |