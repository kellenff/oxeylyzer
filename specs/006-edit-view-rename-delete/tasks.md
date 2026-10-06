# Tasks: Layout Edit Consistency

**Branch**: `feature/time-machine-edit-view-rename-delete` | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## Phase 1: Row editing

- [x] **T001** Extract row key replacement into a pure helper taking the row, its
      index, the position within it, and the new character (FR-001)
- [x] **T002** Reconstruct the row using its own existing separator rather than a
      fixed five-key split, so no gap is introduced (FR-002)
- [x] **T003** Drop the 5+5 split assumption entirely — rows may differ in width and
      in count (FR-003)
- [x] **T004** Route `updateKeyAt` through the helper (FR-001)
- [x] **T005** Route `handleSwap` through the helper so only the two keys change
      (FR-005)

## Phase 2: Rename on save

- [x] **T006** Compute both the original path and the new-name path in
      `save_layout_edit` (FR-007)
- [x] **T007** Refuse a rename whose target already exists within the same language,
      naming the conflicting layout (FR-010)
- [x] **T008** Write to the new path, then remove the old one (FR-007, FR-008)
- [x] **T009** Order the operations write-then-delete so a failed write leaves the
      original intact (FR-011)
- [x] **T010** Leave the filename untouched when the name has not changed (FR-009,
      FR-014)
- [x] **T011** Report a missing original file rather than writing a duplicate
      (FR-013)
- [x] **T012** Update the in-memory map by removing the original key and inserting
      the new one so the layout appears once (FR-012)

## Phase 3: Verification

- [x] **T013** Verify the row helper across 10/10/10, 10/11/10, and 4×10 shapes at
      every position, asserting byte-identical output apart from the replaced key
      (SC-001, SC-002)
- [x] **T014** Verify a hand-alternated row using double spaces keeps them (FR-004)
- [x] **T015** Verify rename against a temp layouts directory: old path absent, new
      path present with contents (SC-003)
- [x] **T016** Verify collision refusal leaves both layouts intact (SC-005)
- [x] **T017** Verify a failed write leaves the original layout present (SC-006)
- [x] **T018** `cargo test --workspace` — 205 existing tests still pass
- [x] **T019** `cargo clippy -p oxeylyzer-tauri`, `tsc --noEmit`, `eslint src`,
      `vite build` all clean
- [x] **T020** Format via the JetBrains reformat action; revert unrelated churn

## Dependencies

T001 blocks T002–T005. T006 blocks T007–T012. Phase 3 depends on both phases.

## Notes

- No new dependencies.
- The row shapes were measured from the shipped data files, not assumed: 809 layouts
  are 3×10, five are 10/11/10, one is 4×10. T013 tests those three shapes because
  they are the ones that exist.
- T013's assertion is the point of the fix: byte-identical apart from the replaced
  key. A test that only checks the target key changed would have passed against the
  current code, which is why it is worded that way.
- The file watcher needs no change. A rename is a create plus a delete, and the
  watcher re-reads the directory, so it converges on the same result the save path
  produces.
- FR-014 is protected by T010: same-name saves must behave exactly as before.

## Completion notes

All 20 tasks done. 205 workspace tests pass; clippy, `tsc`, `eslint`, and `vite
build` all clean.

**The row test found a second bug in my own first fix.** The initial approach
captured a single separator — the gap between the first two keys — and rejoined with
it. That preserves a uniform row but silently normalises a hand-alternated row
(`a b c d e  f g h i j`) to single spaces, violating FR-004. Hand alternation is a
real ergonomic convention and double spaces are how it is expressed, so the helper
now reconstructs each gap individually by walking the original row. T014 is what
caught it.

**Row logic mutation-checked** across 126 positions — every key position in 3x10,
10/11/10, 4x10, and a hand-alternated shape. Against the fix: 0 failures. Against
the original 5+5-split implementation: 116 of 126 fail. The assertions are
"non-edited rows byte-identical" and "edited row equals the original with exactly one
token swapped", so a whitespace change cannot slip through.

**Rename verified against a real temp layouts directory**, mirroring the Rust
logic:

| Case | Result |
|---|---|
| Rename `QMK` to `My Layout` | File moves, normalising to `my_layout.dof` |
| Case-only change (`Ergo` → `ergo`) | Not a rename — same path, no move |
| Rename onto an existing name | Refused; both layouts and contents intact |
| Original missing from disk | Reported; no stray file created |
| Write fails | Original survives with contents intact |

Case-only renames are worth noting: they are not treated as renames because the
normalised path is identical, which is the correct behaviour and is covered by the
verification rather than assumed.

**Formatter churn reverted again** — third feature running, and the JetBrains action
mangled `generate_batch`'s final match arm both times. That function was restored
from HEAD by region extraction; `load_corpus` was corrected line-wise. Final diff
touches `save_layout_edit` and `layout_path` only. This is now a known annoyance:
run the formatter, then check the hunk list before committing.

**T018 passes.** No new Rust tests were needed — the existing 205 cover the rest of
the workspace, and both changed paths (a pure TS function and a filesystem command)
were verified directly rather than through the framework.