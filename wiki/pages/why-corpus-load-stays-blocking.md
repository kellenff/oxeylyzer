---
title: Why Corpus Load Stays Blocking
type: decision
sources: [S007]
updated: 2026-10-06
---

A missing corpus stops the desktop app at startup; the app does not open without
one (S007).

**Rejected: opening the app so the user can switch language in-app.** Story 2 of
this feature was originally written that way. Investigation showed the analysis
engine is built *from* the corpus and roughly twenty commands hold a lock on it,
so launching without one means threading an absent engine through most of the
command surface — a change larger than the rest of the feature combined, and
arguably its own piece of work (S007).

Direction taken instead: keep the load blocking and make the message do the work
— name the missing file, the `corpus` setting that chose it, the config file
holding that setting, and the Language view where it can be changed (S007). A
user who cannot open the app can still act on a message that tells them exactly
which setting to fix (S007).

## The same reasoning covers directory and download failures

Neither leaves a usable app, so the requirement became *explain* rather than
*continue* (S007). "Degraded startup" is a named concept in the spec and ships
nowhere: every failure is blocking (S007).

## What stays out of the boundary

- **Non-startup failures.** Watcher-triggered reload already logs and continues;
  leaving it unchanged was written into the requirements rather than folded in
  (S007).
- **The command-line interface.** It reports errors on its own terms; the
  feature concerns the desktop app (S007).

Related: [Desktop Startup Failure Reporting](./desktop-startup-failure-reporting.md) ·
[Data Version Currency](./data-version-currency.md) ·
[Config Root And Data Root](./config-root-and-data-root.md)