# Feature Specification: Scoring Cache Consistency

**Feature Branch**: `feature/time-machine-generate-dead-code`
**Created**: 2026-10-06
**Status**: Draft
**Input**: User description: "Generation Dead Code and Hot Path. Remove obsolete generation code that is compiled but unreachable, and address the scoring hot path that recomputes per-finger usage on every candidate."

## Correction to the original request

Both halves of the queued entry turned out to be wrong on inspection. This spec is
written to what the code actually does.

**`obsolete.rs` is not dead code.** It contains `best_swap`, the uncached scoring
path. It is called from a test that asserts the cached implementation agrees with it,
and from a benchmark that measures the cached path against it. It is a reference
implementation, kept on purpose. Removing it would delete the thing that verifies the
optimised path is correct.

**The scoring hot path is real, but not where the TODO says.** The TODO sits on one
match arm of the per-finger usage calculation, and that arm does the same work as its
five siblings. The cost is the summation above it, which runs on every scored
candidate.

The actual finding is better than either: **a cache for this value already exists and
is being ignored at the one place it matters most.**

## What the code does

Scoring a candidate swap happens millions of times per search. Each call recomputes
the character usage of the two fingers involved, walking every key on those fingers
and looking up each key's frequency.

A per-layout cache already stores exactly that value. It is populated once when the
cache is built — the calculation runs for all ten fingers at that point — and read
whenever the score is computed without a cache.

The cached scoring path reads the cached value in order to subtract the pre-swap
contribution, and then recomputes the post-swap contribution from scratch. So the
cached value is consulted and discarded in the same expression.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Generation Searches Faster (Priority: P1)

A user runs a generation with a time budget or an iteration count. More of that
budget goes into exploring candidate layouts, so a search finishes sooner or reaches
a better layout within the same time.

Today a substantial part of each scored candidate is spent recomputing a value that
was already calculated when the layout was cached.

**Why priority**: This is the entire user-visible effect, and the whole reason the
feature exists. Everything else is implementation detail serving it.

**Independent Test**: Score a fixed set of candidate swaps and compare total
work — or simply time a fixed workload before and after.

**Acceptance Scenarios**:

1. **Given** a layout with a populated cache, **When** a candidate swap is scored,
   **Then** the per-finger usage contribution is derived from the cached value rather
   than recomputed.
2. **Given** a fixed workload of scored candidates, **When** the change is applied,
   **Then** it completes in no more time than before.
3. **Given** two fingers swapped where both are the same finger, **When** the score is
   computed, **Then** the result matches the value produced by recomputing.
4. **Given** two fingers on different fingers, **When** the score is computed, **Then**
   the result matches the value produced by recomputing.

---

### User Story 2 - Scores Stay Exactly The Same (Priority: P1)

A score is what every comparison in the app is built on: which layout is better,
whether a swap was an improvement, when generation has converged. A performance
change that alters a score by even one unit changes which layouts users are shown and
which ones get saved.

Optimising this must be provably value-preserving. Every optimisation here must
produce bit-identical scores, verified against the recomputing implementation across
real layouts.

**Why priority**: A faster search that returns different results is not a performance
improvement — it is a different product, delivered silently.

**Independent Test**: For a corpus of layouts and a wide set of swaps, assert the
optimised score equals the recomputed score exactly.

**Acceptance Scenarios**:

1. **Given** a layout and a swap, **When** the score is computed both ways, **Then**
   the two values are identical.
2. **Given** the standard test corpus, **When** generation runs, **Then** generated
   layouts are identical to those produced before the change.
3. **Given** a layout whose cache was built before a swap, **When** the swapped layout
   is accepted and the cache updated, **Then** the cache reflects the new layout.
4. **Given** repeated scoring of the same unmodified layout, **When** results are
   compared, **Then** they are identical, so nothing depends on iteration order.

---

### User Story 3 - The Reference Implementation Stays (Priority: P2)

The uncached scoring path is what the optimised path is verified against, both by a
test and by a benchmark. Removing it would remove the only way to check that the
optimisation is faithful, and the only way to measure what the optimisation bought.

It must remain callable and remain correct, so that a future change to the optimised
path can still be checked against it.

**Why priority**: It is what makes Story 2 checkable. Its value is entirely in being
an independent way to compute the same answer.

**Independent Test**: The test comparing cached against uncached still passes, and
the benchmark still runs.

**Acceptance Scenarios**:

1. **Given** the cached and uncached implementations, **When** a swap is scored both
   ways, **Then** the test asserting they agree still passes.
2. **Given** the benchmark comparing both paths, **When** it runs, **Then** it still
   executes and reports both.
3. **Given** a change to the optimised path, **When** the comparison test runs, **Then**
   a discrepancy in scores fails the test rather than passing silently.

---

### Edge Cases

- **What happens when a swap touches the same finger twice?** The cached value is
  subtracted once, matching what the recomputing path does.
- **What happens when a swap touches two different fingers?** Both cached values are
  subtracted, again matching the recomputing path.
- **What happens when a cached value is stale?** It must not be: the cache is updated
  whenever a swap is accepted, so a stale read would silently corrupt scores.
- **What happens when a finger has no keys?** Its cached usage is zero, and both
  paths must agree.
- **What happens when scoring without a cache?** That path is unchanged and continues
  to recompute, which is correct since nothing is cached.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST produce identical scores before and after the change for
  every layout and swap.
- **FR-002**: System MUST derive the per-finger usage contribution for a scored swap
  from the cached value when a cache is available.
- **FR-003**: System MUST continue to recompute when no cache is available.
- **FR-004**: System MUST update the cached per-finger usage whenever a swap is
  accepted, so a cached value is never stale.
- **FR-005**: System MUST keep the comparison between cached and uncached scoring
  passing.
- **FR-006**: System MUST keep the benchmark comparing both paths runnable.
- **FR-007**: System MUST NOT change generated output for a given seed and budget.
- **FR-008**: System MUST NOT change any score's sign, magnitude, or ordering.
- **FR-009**: System MUST NOT regress scoring throughput.
- **FR-010**: System MUST record where the real cost sits, replacing a marker that
  points at the wrong line.

### Key Entities

- **Layout Cache**: The per-layout precomputed values, including per-finger usage and
  its total, built once and updated as swaps are accepted.
- **Per-Finger Usage**: The summed character frequency of every key assigned to one
  finger.
- **Usage Contribution**: The penalty applied to one finger's usage, being its usage
  minus a fraction of the corpus total scaled by that finger's threshold.
- **Cached Scoring Path**: Scoring that consults the layout cache.
- **Uncached Scoring Path**: The reference implementation that recomputes from the
  layout.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: For every layout and swap exercised, the cached score equals the
  uncached score exactly.
- **SC-002**: Scoring a fixed workload does not take longer than before.
- **SC-003**: The existing cached-versus-uncached comparison test passes unchanged.
- **SC-004**: The existing benchmark still runs against both paths.
- **SC-005**: Generated layouts for a fixed seed and budget are unchanged.
- **SC-006**: The misleading marker is gone and the cost of the scoring path is
  documented where it actually sits.

## Assumptions

- The cached per-finger usage is exactly equal to what recomputation produces, since
  both derive from the same layout and corpus. This is the assumption the
  cached-versus-uncached test already checks.
- A layout's cache is kept consistent with the layout by updating it on every accepted
  swap, which the existing code already does for other cached values.
- Scoring performance is bounded by the frequency lookups being repeated rather than
  by the arithmetic, so removing the repetition is where any gain would come from.
- Scores are compared as exact integers throughout, so "identical" means bit-for-bit
  equal rather than approximately equal.
- Removing the reference implementation is explicitly out of scope; it is the only
  independent check on the optimised path.
- Measurement is expected to be directional rather than a precise speedup figure,
  since the gain depends on the machine and on corpus and layout size.