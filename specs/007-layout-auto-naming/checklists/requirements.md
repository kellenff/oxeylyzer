# Specification Quality Checklist: Layout Auto-Naming Accuracy

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

**The queued premise was half wrong, and measuring corrected it.** The entry said the
offset "misnames non-standard boards" as though it were misnaming real ones. It does
not: all 815 shipped layouts are three rows of ten keys, and for every one of them
`skip(10).take(4)` yields exactly the first four keys of the home row. Verified
directly against the archive.

The genuine defect is narrower and is now stated first in the spec: a **four-row**
board makes index 10 the start of row 0, so a generated layout gets named after its
number row — `1 2 3 4` rather than `k l m n`. Two-row boards happen to stay correct,
because index 10 still lands in row 1.

**What the code already provides.** `Shape` is an ordered list of row widths, and
both layout types carry one, so the row containing a given key is derivable without
adding any data. The rule therefore needs no new inputs — only a shared location.

**Decisions taken during clarify, both confirmed:**

- *One shared rule over three copies.* The offset appears in three crates with the
  reasoning recorded as a TODO in only one. FR-003 and FR-004 make agreement and
  single-sourcing requirements, because a name differing between the CLI and the
  desktop app is a real and confusing bug.
- *Degenerate boards produce a name from available keys.* For boards with fewer than
  four keys in the home row, or fewer than two rows, naming continues into whatever
  keys exist (FR-007, FR-008) rather than yielding an empty name. An empty board
  yields an empty name (FR-009), which is correct rather than synthesised.

**SC-002 is the important guard.** Since the current behaviour is correct for every
layout that exists, the risk in this feature is *changing* names that were already
right. SC-002 requires all 815 shipped layouts to name identically before and after,
which is checkable against the archive rather than assumed.
