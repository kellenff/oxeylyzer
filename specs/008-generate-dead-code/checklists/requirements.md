# Specification Quality Checklist: Scoring Cache Consistency

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-06
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

**Both halves of the queued entry were wrong, and the real finding is better.**

- *`obsolete.rs` is not dead code.* It holds `best_swap`, the uncached scoring path,
  called from a test that asserts the cached path agrees with it and from a benchmark
  that measures against it. It is the reference implementation. Removing it would
  delete the only independent check on the optimisation — FR-005 and FR-006 keep it.

- *The TODO marks the wrong line.* It sits on one match arm of the per-finger usage
  calculation, which does the same work as its five siblings. The cost is the
  summation above it.

**The actual finding:** the cached scoring path reads the cached per-finger usage in
order to subtract the pre-swap contribution, then recomputes the post-swap
contribution from scratch — consulting and discarding the cached value in one
expression. A cache for exactly this value already exists, is populated once when the
cache is built, and is used by the uncached-score path everywhere else. FR-002 is
therefore a consistency fix rather than a micro-optimisation, which is why the title
changed from the queued "Generation Dead Code and Hot Path".

**Why it matters for users:** `score_swap_cached` is what search calls per candidate,
millions of times per run, so more of the budget goes into exploring.

**The correctness risk is the real risk.** A score change would silently alter which
layouts users are shown and saved. FR-001 and SC-001 therefore require exact
equality with the recomputing path, and SC-005 requires unchanged generated output.
The existing cached-versus-uncached test is the instrument for both.

**On SC-002 wording:** "does not take longer" rather than a percentage, because the
gain depends on machine, corpus, and layout size. A specific speedup figure asserted
now would be a guess that ages badly; the requirement is directional.
