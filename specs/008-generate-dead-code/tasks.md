# Tasks: Scoring Cache Consistency

**Branch**: `feature/time-machine-generate-dead-code` | **Spec**: [spec.md](./spec.md) | **Plan**: [plan.md](./plan.md)

## Phase 1: Split the penalty

- [x] **T001** Extract `max_finger_use_threshold(finger)` returning the per-finger
      threshold as an integer (FR-008)
- [x] **T002** Extract `finger_usage_score(finger, usage)` applying the penalty to a
      given usage, preserving exact integer arithmetic (FR-001, FR-008)
- [x] **T003** Reduce `finger_usage` to the summation followed by the extracted
      penalty (FR-001)

## Phase 2: Remove the misleading marker

- [x] **T004** Remove the `TODO: optimize` from the single match arm it sits on,
      which does the same work as its five siblings (FR-010)
- [x] **T005** Document where the scoring cost actually sits (FR-010)

## Phase 3: Tests

- [x] **T006** Assert the extracted penalty equals the original inline expression for
      all ten fingers (SC-001)
- [x] **T007** Assert that after a swap only the two swapped fingers change usage, and
      the cache matched a full recompute beforehand (SC-001)

## Phase 4: Retain the reference implementation

- [x] **T008** Leave `obsolete.rs` in place — it is the uncached baseline the
      comparison test and benchmark depend on (FR-005, FR-006)
- [x] **T009** Confirm the comparison test still passes (SC-003)
- [x] **T010** Confirm the benchmark still compiles and registers both paths (SC-004)

## Phase 5: Verification

- [x] **T011** `cargo test --workspace` — 212 pre-existing tests still pass
- [x] **T012** `cargo clippy -p oxeylyzer-core` — clean
- [x] **T013** `cargo bench --no-run` — benchmarks build
- [x] **T014** Format via the JetBrains reformat action; revert unrelated churn

## Notes

- No new dependencies.
- T006 and T007 are the point of this change: the refactor is only safe because the
  arithmetic is provably unchanged, and the cached path's planned optimisation depends
  on T007's property.
- The cached-path rewrite itself is deliberately deferred. See the plan's scope
  section: mutating the cache during scoring risks silent corruption, and the exact
  delta alternative needs the purity contract to be settled first.
