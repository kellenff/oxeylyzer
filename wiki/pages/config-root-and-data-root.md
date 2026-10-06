---
title: Config Root And Data Root
type: reference
sources: [S005, S006, S007, S008]
updated: 2026-10-06
---

`OxeylyzerDirs` holds exactly two roots, and every path the crate hands out is a
`join` onto one of them (S005). The struct's doc table documents the mapping:

| | Linux | Windows / macOS |
|---|---|---|
| `config_root` | `$XDG_CONFIG_HOME/oxeylyzer` | `%APPDATA%\oxeylyzer` |
| `data_root` | `$XDG_DATA_HOME/oxeylyzer` | same as `config_root` |

**On Windows and macOS the two roots are the same directory** — only Linux keeps
them apart (S005). Corroborated from the other direction: T016 found
`~/Library/Application Support/oxeylyzer` present and
`~/.config/oxeylyzer` absent, and the macOS platform directory for both roles is
that one path (S006).

Config root holds `config.toml`, `session.json`, `history.txt`, and
`weight-presets/`. Data root holds `static/layouts/`, `static/language_data/`,
`static/corpus_configs/`, and `static/text/` (S005).

Nine accessors exist — `data_dir`, `layouts_dir`, `language_data_dir`,
`corpus_configs_dir`, `text_dir`, `config_file`, `session_file`, `history_file`,
`weight_presets_dir` — and the containment test walks exactly those nine (S005).

**`static/layouts/` is not flat.** Layouts live in one directory per language,
and names are unique within a language rather than globally — the same layout
name may legitimately exist under two languages (S008). A collision check that
searches the whole layouts tree would refuse renames that are fine; see
[Layout File Lifecycle](./layout-file-lifecycle.md) (S008).

## `OXEYLYZER_DATA_DIR` is documented here but read elsewhere

The struct's doc comment tells a developer to set `OXEYLYZER_DATA_DIR=.` to use
the workspace's existing `./static/` tree (S005). **`dirs.rs` never reads that
variable** — nothing in the file consults the environment (S005). The in-crate
mechanism is `with_override(root)`, which points both roots at one directory for
development and tests (S005).

The variable is a caller-side convention, and the desktop app honors it first
thing in `resolve_dirs()`: set, it calls `with_override` and skips platform
resolution entirely (S007). So the two mechanisms are documented as one, and the
env var wins at the app boundary while being invisible from the crate that
documents it (S005, S007).

## `with_override` collapses both roots, same as macOS and Windows

`with_override` sets `config_root` and `data_root` to the same path (S005). That
is not a test-only shape — it is the normal shape on two of three target
platforms (S005). Tests that assume config and data are separate directories are
testing a Linux-only arrangement.

Related: [Platform Root Resolution](./platform-root-resolution.md) ·
[Data Archive Pipeline](./data-archive-pipeline.md) ·
[Data Version Currency](./data-version-currency.md) ·
[Shipped Data Files Layout](./shipped-data-files-layout.md) ·
[Desktop Startup Failure Reporting](./desktop-startup-failure-reporting.md)