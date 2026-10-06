---
title: Generated Config Round-Trip
type: component
sources: [S006]
updated: 2026-10-06
---

`default_config_toml(dirs)` in `seed.rs` writes the default config on first run;
the app reads it back through `Config::with_loaded_weights`. Five tests cover that
trip end to end (S006).

Three path fields are joined onto the resolved data root — corpus, layouts, and
corpus configs — and every other field must come back as `Config::default()`, so
a generated config is equivalent to no config at all except for where data lives
(S006).

`layouts` accepts either form and both normalize to a one-entry vector:
`layouts = "x"` and `layouts = ["x"]` both load as `vec!["x"]` (S006).

## Escaping is the round trip's real edge

A data root like `oxeylyzer test é` must survive generation and load back
byte-identical. `toml::to_string_pretty` does the escaping and the loader undoes
it (S006). An ASCII-only root would not exercise the case the requirement exists
for, which is why the test root carries a space and a non-ASCII character.

## `Weights` has no `PartialEq`

Comparing a loaded `Config` against `Config::default()` could not work directly.
The tests compare `toml::to_string` output of both sides instead — a stronger
check, since it asserts the values serialize identically rather than that some
derived equality happens to agree (S006).

## Hand-written fixtures lie: build on a generated config

A minimal hand-written TOML failed with `missing field trigram_precision` — not
a `layouts` problem at all, just a missing required field (S006). Rebuilding it
on top of a real generated config fixed that, and immediately exposed a second
failure: `missing field layouts`, because the replacement `layouts` line had
been *appended* after `[weights]`, where TOML reads it as a weights key. The line
has to be substituted in place (S006).

That placement failure is exactly the class of bug the single-string/list
requirement is about, so the assertion now catches placement, not just presence
(S006).

Related: [The Config Loader Is The Contract](./config-loader-is-the-contract.md) ·
[Shipped Data Files Layout](./shipped-data-files-layout.md) ·
[Platform Root Resolution](./platform-root-resolution.md)
