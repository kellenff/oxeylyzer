---
title: Desktop Startup Failure Reporting
type: component
sources: [S007]
updated: 2026-10-06
---

`initialize()` in `oxeylyzer-tauri/src-tauri/src/lib.rs` is the desktop startup
sequence, extracted out of the setup hook so it returns
`Result<Initialized, String>` instead of panicking (S007). Five steps, each
independently able to fail, in a deliberate order:

```text
resolve_dirs()            OXEYLYZER_DATA_DIR override, else platform dirs
  └─ needs_data()? → ensure_data   download + extract, blocking
       └─ ensure_config()          idempotent, recovers a missing config
            └─ Config::with_loaded_weights
                 └─ Data::load(corpus) → Oxeylyzer engine + layouts
```

The order is preserved because the steps depend on each other (S007). The
watcher setup and `app.manage(state)` run in the hook and only on success
(S007).

## Every message names a file and something the user can change

The mapping is uniform — the step, the concrete location, the remedy (S007):

| Step | Message names |
|---|---|
| directory resolution | the data directory, and that layouts/language data live there (S007) |
| download | the network, and that retrying may help (S007) |
| `ensure_config` | the config file path (S007) |
| config parse | the file, and that fixing or deleting it starts over (S007) |
| corpus load | the corpus file, the `corpus` setting that chose it, and the Language view (S007) |

**Rejected: returning `Err` to the framework.** That tears down the window —
see [The Setup Hook Runs Before The Webview](./tauri-setup-hook-runs-before-webview.md) (S007).

The corpus message points at the `corpus` option rather than a language field
because `Config` has no `language` field; the language lives on the `Oxeylyzer`
engine, which does not exist yet when the corpus is loaded (S007).

## The frontend reused an error field that already existed

`store.ts` carries `error: string | null` and `App.tsx` renders it while
suppressing the rest of the UI (S007). The listener routes into that existing
field rather than adding a second one — adding markup would duplicate
conditional rendering that already works (S007).

## `OXEYLYZER_DATA_DIR` is read in the Tauri app

`resolve_dirs()` checks `OXEYLYZER_DATA_DIR` first and calls `with_override`
when set, bypassing platform resolution entirely (S007). This confirms the
variable documented on `OxeylyzerDirs` in `dirs.rs` is a caller-side convention
— see [Config Root And Data Root](./config-root-and-data-root.md) (S007).

## Verification is manual by necessity

The desktop crate has no tests and startup runs a full framework builder, so
there is no headless harness (S007). The five failure paths were **reasoned from
the code, not observed** — the environment has no display, so forcing each
failure at runtime was not possible (S007). Reporting at runtime is unverified.

That gap is why the success path mattered more: 205 existing workspace tests
still pass and an unmodified install still starts normally, so a regression
would have shown up there even though the new reporting could not be exercised
(S007).

Related: [The Setup Hook Runs Before The Webview](./tauri-setup-hook-runs-before-webview.md) ·
[Why Corpus Load Stays Blocking](./why-corpus-load-stays-blocking.md) ·
[Data Archive Pipeline](./data-archive-pipeline.md)