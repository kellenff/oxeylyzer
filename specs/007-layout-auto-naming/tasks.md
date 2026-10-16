# Tasks: Layout Auto-Naming Accuracy

**Branch**: `feature/time-machine-layout-auto-naming` | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## Phase 1: Shared rule

- [x] **T001** Add `auto_name(keys, shape, mapping)` to `fast_layout.rs`, taking the
      first four keys of the home row (FR-001, FR-002)
- [x] **T002** Fall back to the first row when the board has fewer than two rows
      (FR-008)
- [x] **T003** Bound the slice so a shape wider than the keys cannot panic (FR-009,
      FR-010)

## Phase 2: Call sites

- [x] **T004** Replace the offset in `layout.rs` `From<FastLayout> for Layout`, and
      remove the TODO (FR-004)
- [x] **T005** Replace the offset in `repl.rs` `placeholder_name`, keeping the
      counter and `*` replacement (FR-003)
- [x] **T006** Replace the offset in `lib.rs` `save_generated` (FR-003)
- [x] **T007** Confirm an existing name still short-circuits the rule (FR-006)

## Phase 3: Tests

- [x] **T008** Test 3×10 names the second row's first four keys (SC-001)
- [x] **T009** Test a four-row board names the second row, not the number row (SC-001)
- [x] **T010** Test ragged row widths (SC-001)
- [x] **T011** Test a short home row falls back to available keys (SC-003)
- [x] **T012** Test a single-row board (SC-003)
- [x] **T013** Test an empty board yields an empty name without panicking (SC-003)
- [x] **T014** Test a shape wider than the keys present (SC-003)
- [x] **T015** Verify all 815 shipped layouts name identically before and after
      (SC-002)

## Phase 4: Verification

- [x] **T016** `cargo test --workspace` — 205 pre-existing tests still pass
- [x] **T017** `cargo clippy --workspace` — no new warnings
- [x] **T018** Confirm no fixed offset remains in any crate (SC-005)
- [x] **T019** Format via the JetBrains reformat action; revert unrelated churn

## Notes

- No new dependencies. `Shape` is already on both layout types.
- The pre-existing `redundant reference in println!` warning at `repl.rs:759` is not
  from this change — confirmed present on a clean tree.
- T015 is the guard that matters: current behaviour is already correct for all 815
  shipped layouts, so the risk is changing names that were right.
