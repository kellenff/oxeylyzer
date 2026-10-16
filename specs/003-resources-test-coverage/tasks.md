# Tasks: Resource Crate Test Coverage

**Branch**: `feature/time-machine-resources-test-coverage` | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## Phase 1: Production change

- [x] **T001** Extract `platform_roots() -> Option<(PathBuf, PathBuf)>` in
      `dirs.rs`, asking the platform for its directories without creating them
      (FR-007)
- [x] **T002** Rewrite `resolve()` to call `platform_roots()` and keep the
      `create_dir_all` calls, so its public signature and behavior are unchanged
- [x] **T003** Keep `NoDirFound` mapping intact — resolution failure must still
      produce the same error

## Phase 2: Config round-trip tests

- [x] **T004** Add `generated_config_loads_through_the_app_loader`, loading via
      `Config::with_loaded_weights` (FR-001)
- [x] **T005** Add `generated_config_paths_point_inside_the_data_root`, asserting
      corpus, layouts, and corpus configs all lie under the root (FR-002)
- [x] **T006** Add `generated_config_preserves_default_weights`, comparing the
      loaded config to `Config::default()` apart from the three paths (FR-003)
- [x] **T007** Add `layouts_accept_both_single_string_and_list`, parsing both
      forms through the loader (FR-004)
- [x] **T008** Add `paths_needing_escaping_survive_the_round_trip`, using a root
      with a space and a non-ASCII character (FR-005)

## Phase 3: Platform containment tests

- [x] **T009** Add `platform_derived_paths_all_sit_under_platform_roots` (FR-006)
- [x] **T010** Add `platform_paths_are_stable_across_calls` (FR-008)
- [x] **T011** Add `absent_sentinel_on_a_fresh_root_needs_data`, confirming
      currency reports a download is needed (FR-006)
- [x] **T012** Add a test that `platform_roots()` returns a pair without
      creating either directory (FR-007)

## Phase 4: Verification

- [x] **T013** `cargo test -p oxeylyzer-resources` — all pass, no network access
- [x] **T014** `cargo clippy -p oxeylyzer-resources` — clean
- [x] **T015** `cargo test --workspace` — 196 pre-existing tests still pass
- [x] **T016** Confirm the suite created nothing under the platform config or data
      directories
- [x] **T017** Format via the JetBrains reformat action; revert unrelated churn

## Dependencies

T001–T003 precede all tests. T004 blocks T005–T008. Phase 4 depends on Phases 2
and 3.

## Notes

- No new dependencies. `Config` and `Config::with_loaded_weights` come from
  `oxeylyzer-core`, already a dependency.
- Tests must not touch the real `~/.config/oxeylyzer` or platform data directory.
  That constraint is why T001 exists — it exists to make the containment check
  testable without side effects.
- `cargo fmt` is broken on this machine (`librustc_driver` will not load). Used the
  JetBrains reformat action per T017; it produced no unrelated churn this time, so
  nothing needed reverting.

## Completion notes

All 17 tasks done. Workspace: 205 tests pass (196 pre-existing, 9 new); the
resources crate went 25 to 34.

**One production change, as planned.** `platform_roots()` extracted from
`resolve()`. Private, no signature change, `NoDirFound` mapping preserved. It
exists solely so containment can be checked against the real platform directories
without creating anything (T012).

**Three test bugs found and fixed while writing them:**

- `generated_config_preserves_default_weights` could not compare `Weights`
  directly — it derives no `PartialEq`. Now compares `toml::to_string` output of
  both, which is a stronger check anyway: it verifies the values serialize
  identically, not just that some derived equality agrees.
- `layouts_accept_both_single_string_and_list` first hand-wrote a minimal TOML and
  failed with `missing field trigram_precision` — not a `layouts` problem at all.
  Rebuilt on top of a real generated config so every required field is present.
- Same test then failed with `missing field layouts`, because the replacement
  appended the new `layouts` line after `[weights]`, where TOML reads it as a
  weights key. Changed to substitute the line in place. This is exactly the class
  of bug FR-004 is about, so it is worth recording that the assertion now catches
  placement, not just presence.

**T016 verified against the real filesystem.** `~/Library/Application Support/oxeylyzer`
exists but is empty and dated Oct 4, before this session — pre-existing and
untouched. `~/.config/oxeylyzer` does not exist. No test wrote to the user's real
config or data directories.

**Phantom bug avoided.** `default_config_toml` generates a layout path of
`static/layouts/english/*.dof`, and `english` looked absent from a partial listing
of shipped layout directories. Downloading the real `data-files.zip` showed it
ships `english` with 146 `.dof` files, plus `static/language_data/english.json`
and 58 corpus config `.toml` files. All three generated paths resolve. No defect;
recorded in the checklist so it is not re-investigated.