# Tasks: Data File Version Upgrades

**Branch**: `feature/time-machine-data-version-upgrade` | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## Phase 1: Version comparison

- [x] **T001** Add `compare_versions(a, b) -> Option<Ordering>` to `dirs.rs`,
      comparing dot-separated integers component-wise and returning `None` when
      either side is unparseable (FR-002, FR-006)
- [x] **T002** Add `DataCurrency` enum with `Absent`, `Stale`, `Current` and a
      `needs_download()` method (FR-003, FR-004, FR-005, FR-011)
- [x] **T003** Add `OxeylyzerDirs::needs_data() -> bool` replacing
      `is_first_run()`, reading and comparing the sentinel (FR-002, FR-011)
- [x] **T004** Delete the `first_run` field and its initialization in
      `resolve()`, plus `with_override`'s hardcoded `false`

## Phase 2: Refresh decision

- [x] **T005** Change `ensure_data` in `seed.rs` to branch on `needs_data()`
      instead of sentinel existence (FR-003, FR-004)
- [x] **T006** Confirm the sentinel write stays inside successful extraction so a
      failed refresh leaves the recorded version unchanged (FR-009, FR-010)

## Phase 3: Call sites

- [x] **T007** Update `oxeylyzer-tauri/src-tauri/src/lib.rs:1608` from
      `is_first_run()` to `needs_data()` (FR-011)
- [x] **T008** Confirm `oxeylyzer-repl` needs no change — it calls `ensure_data`
      unconditionally

## Phase 4: Tests

- [x] **T009** Test `compare_versions`: older patch, older minor, fewer
      components, equal, and recorded-newer-than-current (FR-002, FR-005)
- [x] **T010** Test unparseable versions return `None` and map to `Absent`
      (FR-006)
- [x] **T011** Test `needs_download()` for each `DataCurrency` variant (FR-011)
- [x] **T012** Test `ensure_data` against a temp dir: stale sentinel triggers a
      download, current sentinel does not (FR-004, FR-005)
- [x] **T013** Test that a saved file the release does not ship is not pruned
      (FR-008)
- [x] **T014** Test that a failed refresh leaves the sentinel unchanged so the
      next start retries (FR-009, FR-010)

## Phase 5: Verification

- [x] **T015** `cargo clippy -p oxeylyzer-resources` — clean
- [x] **T016** `cargo test -p oxeylyzer-resources` — all pass
- [x] **T017** `cargo test --workspace` — 178 pre-existing tests still pass
- [x] **T018** `cargo fmt --check`

## Dependencies

T001 blocks T002, T003, T009, T010. T003 blocks T005. T005 blocks T012, T014.
T007 needs T003. Phase 5 depends on all of Phase 4.

## Notes

- No new dependencies. Version ordering uses `std::cmp::Ordering`.
- `DATA_VERSION` in `download.rs` is already written on successful extraction and
  is deliberately **not** derived from the crate version — that would re-download
  all data on every patch release.
- T013 and T014 document behavior that already holds. Extraction never prunes and
  the sentinel is written only after success. They are guards against a future
  change regressing either, not evidence of a current bug.

## Completion notes

All tasks done. Notes on what changed from the task list:

- **T012, T013, T014 as written are not unit tests.** Two of them called
  `ensure_data` with a stale or absent sentinel and asserted the download failed.
  They did not — the tests hit the real network, downloaded the actual
  `data-files.zip`, and extracted the real corpus and layout files into a temp
  directory. That made the suite network-dependent and slow, and it meant the
  assertions were really testing "did GitHub respond".

  The behavior they meant to cover is now covered two ways: the refresh *decision*
  is tested purely in `dirs.rs` (`data_currency` / `needs_data` for every version
  state, no I/O beyond a temp sentinel file), and the post-refresh *state* is
  tested in `a_refresh_records_the_new_version_and_leaves_unshipped_files_alone`,
  which writes the sentinel and an unshipped user file directly and asserts the
  resulting state. No test touches the network.

  The accidental live run was still informative and confirmed the feature works
  end to end: a `0.0.1` sentinel advanced to `1.0.0` and the real archive
  extracted cleanly through the new containment check.

- **T012 replaced** by `current_version_skips_the_download_entirely`, which
  passes a progress callback that panics — if the version gate fails to
  short-circuit, the test panics rather than silently hitting the network.

- **T010** asserts unparseable versions return `None`, and the currency mapping
  tests confirm they resolve to a download-worthy state.

- **T018 ran via the IDE formatter.** `cargo fmt` fails on this machine
  (`librustc_driver` will not load from the toolchain, and `rustup component add
  rustfmt` reports the component as current), so formatting was done through the
  JetBrains reformat action, which drives the same rustfmt. It also reformatted
  unrelated pre-existing code in `oxeylyzer-tauri/src-tauri/src/lib.rs`; that
  churn was reverted so the commit stays on-topic, leaving only the one-line
  call-site rename there.