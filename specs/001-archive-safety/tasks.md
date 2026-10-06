# Tasks: Safe Data Archive Extraction

**Branch**: `feature/time-machine-archive-safety` | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## Phase 1: Containment

- [x] **T001** Replace `file.name()` with `file.enclosed_name()` in the extraction
      loop of `oxeylyzer-resources/src/download.rs`, mapping `None` to a
      `ResourceError::Extraction` error that names the offending entry (FR-001,
      FR-002, FR-003)
- [x] **T002** Strip `ZIP_INNER_PREFIX` from the *enclosed* path, after the
      containment check — not from the raw name before it (FR-004)
- [x] **T003** Preserve the `stripped.is_empty()` early-continue for the
      archive's own root entry (FR-004)
- [x] **T004** Confirm no new `ResourceError` variant is needed —
      `Extraction(String)` already exists in `lib.rs` (FR-008)

## Phase 2: Tests

- [x] **T005** Add a `#[cfg(test)]` module to `download.rs` with a helper that
      builds an in-memory archive from `(name, contents)` pairs via
      `zip::ZipWriter` over a `Cursor` (SC-001)
- [x] **T006** Add a helper that creates a unique temp directory under
      `std::env::temp_dir()` without adding a `tempfile` dev-dependency
- [x] **T007** Test: `..` traversal out of the root returns `Err`, and asserts
      no file exists outside the destination — walk the parent dir, do not just
      check the return value (SC-001)
- [x] **T008** Test: absolute-path entry returns `Err` (SC-001)
- [x] **T009** Test: `a/../b.json` resolves to `dest/b.json` and succeeds (FR-001)
- [x] **T010** Test: prefixed entry `data-files/static/text/en.txt` lands at
      `dest/static/text/en.txt` (FR-004)
- [x] **T011** Test: unprefixed entry `static/text/en.txt` lands at
      `dest/static/text/en.txt` (FR-004)
- [x] **T012** Test: bare root entry `data-files/` is skipped without error (FR-004)

## Phase 3: Verification

- [x] **T013** `cargo clippy -p oxeylyzer-resources` — clean
- [x] **T014** `cargo test -p oxeylyzer-resources` — all new tests pass
- [x] **T015** `cargo test --workspace` — the 171 pre-existing tests still pass,
      proving extraction output for well-formed archives is unchanged

## Dependencies

T001 blocks T002, T003, and all of Phase 2. T005 and T006 block T007–T012.
T013–T015 depend on Phase 2 being complete.

## Notes

- No new dependencies. `zip`, `ureq`, and `thiserror` are already declared.
- T007 is the test that matters most: the security property is about the
  filesystem, not the return value, so it must inspect the parent directory.
- The `zip` version in the lockfile is 2.4.2 and `enclosed_name()` is public API
  in that version.

## Completion notes

All tasks done. Two deviations from the task list, both forced by the code:

- **T006 helper changed.** The `files_under` / `walk` directory-tree helper planned
  in T006 was written but then dropped as unused once the assertions were rewritten
  to check specific paths. YAGNI.
- **T005 required a seam.** Tests could not call `download_and_extract` without
  hitting the network, so extraction was split into a private `extract_bytes(&[u8],
  &Path)` that `download_and_extract` delegates to. This is a small structural
  change beyond the planned "loop body only" edit, and it is what makes the
  containment rules testable at all.

**Mutation-checked**: reverting the guard to the old raw `file.name()` makes
`traversal_escaping_root_is_rejected`, `absolute_path_entry_is_rejected`, and
`mixed_valid_and_hostile_fails_without_partial_files` fail, while the four
positive-path tests keep passing. The security tests are proven to actually
detect the vulnerability, not merely pass.

The first version of `absolute_path_entry_is_rejected` used `/etc/passwd` as the
target and passed even against vulnerable code — the write failed on
permissions, not containment, so it proved nothing. It now targets a writable
absolute path under the temp dir and asserts the file is absent.

Workspace: 178 tests pass (171 pre-existing, 7 new). `cargo clippy` and
`cargo fmt` clean.