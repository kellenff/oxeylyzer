---
title: Why One Shared Naming Rule
type: decision
sources: [S009]
updated: 2026-10-06
---

The layout naming rule existed as three copies of the same `skip(10).take(4)` in
three crates — core, REPL, and Tauri — with the reasoning behind it recorded as a
`TODO` in **only one** (S009). That is how it drifted: a fix applied at one site
leaves the other two wrong (S009).

A generated layout can be named from the command line, the desktop app's save flow,
or during internal conversion, so divergent copies mean *the same layout saved two
places gets two names* — subtle, and user-visible (S009).

**Decision:** one public function in core, all three call sites use it (S009).

**Rejected alternatives (all from S009):**

| Alternative | Why rejected |
|---|---|
| Fix all three copies in place | the copies are why it drifted; nothing keeps them agreeing (S009) |
| Guard the offset at each site | same duplication, three places to re-check (S009) |
| Derive the row by summing all rows after the first | only correct for two-row boards (S009) |

Two intermediate rules were tried and the tests caught both: taking the first row's
width unconditionally returned an empty name for a single-row board, and summing
later rows only holds for two rows. The rule that survived is first row's width,
guarded on at least two rows (S009).

Naming is presentation only — it cannot alter a score, so it was safe to change
without touching analysis (S009). Existing names are never regenerated:
`From<FastLayout>` keeps a `Some(name)` (S009).

What the rule computes: [Auto-Naming Generated Layouts](./auto-layout-naming.md) ·
The save path it feeds: [Layout File Lifecycle](./layout-file-lifecycle.md)