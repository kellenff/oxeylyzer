# Implementation Plan: Resource Crate Test Coverage

**Branch**: `feature/time-machine-resources-test-coverage` | **Date**: 2026-10-06 | **Spec**: [spec.md](./spec.md)

**Input**: Feature specification `specs/003-resources-test-coverage/spec.md`

## Summary

Add tests for the two behaviors `oxeylyzer-resources` still does not cover: the
generated default config round-tripping through the app's real config loader, and
path containment against platform-reported directories.

One small production change is required: a way to ask what the platform would
report without creating anything, so tests can verify containment without writing
to the user's real configuration or data locations.

**Technical approach**: split the pure path derivation from the filesystem side
effects in `resolve()`, then test through `Config::with_loaded_weights` — the
loader the app itself uses — rather than a hand-rolled TOML parse.

## Technical Context

**Language/Version**: Rust, edition 2024, `stable` per `rust-toolchain.toml`
**Primary Dependencies**: none added — `toml` and `directories` already present
**Storage**: temp directories only; nothing touches the real user config or data paths
**Testing**: `cargo test` — in-crate unit tests
**Target Platform**: Linux, macOS, Windows
**Project Type**: library crate (`oxeylyzer-resources`)
**Performance Goals**: not applicable — no production hot path changes
**Constraints**: no new dependencies; tests must not create or modify anything in the user's real `~/.config` or `~/.local/share`; `cargo fmt` is broken on this machine, so format via the IDE reformat action
**Scale/Scope**: one new test module in `seed.rs`, tests added to `dirs.rs`, one small refactor in `dirs.rs`

## Production change

`OxeylyzerDirs::resolve()` currently does three things at once: ask the platform
where directories are, create them, and record whether the sentinel exists (the
last is now `data_currency()`'s job). The creation is the only part tests must not
trigger.

Split it:

```rust
/// Where this platform says the application's files live, without creating anything.
fn platform_roots() -> Option<(PathBuf, PathBuf)> {
    let proj = ProjectDirs::from("", "", "oxeylyzer")?;
    Some((proj.config_dir().to_path_buf(), proj.data_dir().to_path_buf()))
}
```

`resolve()` becomes `platform_roots()` plus `create_dir_all` on each. Private,
crate-internal, no public API change — `resolve()` keeps its signature and
behavior exactly as callers see it.

## Tests

### Config round-trip (in `seed.rs`)

Covered by loading through `Config::with_loaded_weights`, the same call the REPL
and Tauri app make at startup:

| Test | Asserts |
|---|---|
| `generated_config_loads_through_the_app_loader` | FR-001 — no error |
| `generated_config_paths_point_inside_the_data_root` | FR-002 — all three paths under the root |
| `generated_config_preserves_default_weights` | FR-003 — loaded `Config` equals `Config::default()` except the three paths |
| `layouts_accept_both_single_string_and_list` | FR-004 — `layouts = "x"` and `layouts = ["x"]` both load to `vec!["x"]` |
| `paths_needing_escaping_survive_the_round_trip` | FR-005 — root containing a space and non-ASCII |

The escaping test is the one that earns its place: `OxeylyzerDirs::with_override`
lets a test use a root like `oxeylyzer test é`, and `toml::to_string_pretty` must
escape it while `with_loaded_weights` reads it back byte-identical.

### Platform containment (in `dirs.rs`)

| Test | Asserts |
|---|---|
| `platform_derived_paths_all_sit_under_platform_roots` | FR-006 — every derived path under the reported roots |
| `platform_paths_are_stable_across_calls` | FR-008 — repeated calls identical |
| `absent_sentinel_on_a_fresh_root_needs_data` | FR-006, currency on a real-shaped root |

These use a temp root shaped like a platform root rather than calling
`platform_roots()` directly, because the assertion that matters is the containment
property, and deriving from the *actual* platform paths would make the test
machine-dependent while still writing nothing. One test does call
`platform_roots()` to assert it returns a pair without creating either directory.

## Project Structure

```text
oxeylyzer-resources/
└── src/
    ├── dirs.rs     # platform_roots() extraction + 3 new tests
    ├── download.rs # unchanged
    ├── lib.rs      # unchanged
    └── seed.rs     # 5 new tests
```

## Verification

```bash
cargo test -p oxeylyzer-resources
cargo clippy -p oxeylyzer-resources
cargo test --workspace          # 196 pre-existing must still pass
```

Confirm no filesystem writes outside temp dirs: run the suite and check that
`~/.config/oxeylyzer` and the platform data dir are untouched afterward.

Format with the JetBrains reformat action rather than `cargo fmt`, which fails to
load `librustc_driver` on this machine. Revert any unrelated reformatting it
introduces.

## Complexity Tracking

No constitution violations.

| Choice | Simpler alternative rejected because |
|---|---|
| `platform_roots()` returning `Option` | Returning `Result` duplicates `ResourceError::NoDirFound` mapping already in `resolve()` |
| Test config via `with_loaded_weights` | Parsing the TOML directly would pass even if the app's loader rejected it — the loader is the contract |
| Escaping test with a non-ASCII root | ASCII-only roots would not exercise the escaping the requirement exists for |