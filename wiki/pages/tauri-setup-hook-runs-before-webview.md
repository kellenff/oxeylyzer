---
title: The Setup Hook Runs Before The Webview
type: component
sources: [S007]
updated: 2026-10-06
---

Tauri's `.setup()` hook runs **before the webview loads**, so anything the
frontend is meant to learn by *listening* for during startup is emitted into a
webview that does not exist yet and is missed entirely (S007).

This invalidated the plan for this feature, which emitted a `startup-error`
event and had `App.tsx` subscribe on mount. The message would have been lost
(S007). `download-progress` has the same defect and had been emitted by the
backend all along with no subscriber, so the window simply froze during a
first-run download (S007).

## Poll a managed value instead of racing an event

The fix manages a `StartupStatus` (an `Arc<Mutex<…>>`) *before any fallible
work*, so the frontend can always query it even when startup failed (S007). Two
commands expose it — `startup_error` and `download_progress` — and the frontend
queries on mount and polls progress (S007).

The event emits were **kept alongside** the state, since they cost nothing and
help if anything ever subscribes later (S007). So `startup-error` is emitted and
also stored; only the stored copy is actually read today (S007).

## Returning `Err` from setup would destroy the reporting surface

Returning an error makes the framework tear the window down, which destroys the
surface just populated with the message (S007). The hook therefore records the
failure, emits, and returns `Ok(())` — deliberately swallowing the error so the
window survives (S007).

Corollary for anything reading this code later: a startup failure is visible in
the UI *and* in the exit code, and these are different channels. `main.rs`
maps `run()`'s `Err` to exit code 1, but startup failures reported in-window do
not come through `run()` at all (S007).

Related: [Desktop Startup Failure Reporting](./desktop-startup-failure-reporting.md) ·
[Why Corpus Load Stays Blocking](./why-corpus-load-stays-blocking.md) ·
[Config Root And Data Root](./config-root-and-data-root.md)