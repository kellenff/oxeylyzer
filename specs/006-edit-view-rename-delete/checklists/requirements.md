# Specification Quality Checklist: Layout Edit Consistency

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

**B6 is materially worse than the original note recorded.** The earlier pass
described it as "applyCharAt assumes 5+5 row split". Measured against the data
files actually shipped: 809 of 815 layouts are three rows of ten keys, five are
10/11/10, and one has four rows. The assumed split therefore falls in the middle of
essentially every row the user can open, not only hypothetical non-standard boards.

Editing one character of `a b c d e f g h i j` produces
`a b c X e  f g h i j` — a double space injected mid-row. The row is then
re-serialized, so the damage persists and compounds on the next edit. This is
FR-002, and it is why the spec is scoped as a correctness fix rather than a
formatting preference.

**B5 confirmed as recorded.** The save derives its write path from the original
name while updating the in-memory map under the new name, so disk and memory
disagree after every rename. FR-007 and FR-008 close that.

**Board coverage was measured rather than assumed.** Rather than reasoning about
which shapes are valid, the shipped archive was enumerated: `ortho` (811) and `ansi`
(4), with the row patterns above. FR-003 is therefore written against what exists
plus arbitrary shapes, not against a guess about non-standard boards.

**Rename collision handling (FR-010) is an addition.** The original note said only
that a rename "leaves the old file". A rename that silently overwrites another
layout would be a worse outcome than the bug being fixed, so refusing the
collision is required rather than optional.

**Scope note:** SC-002 is worded to cover every shipped layout, because the row
handling is the same code path for all of them and the shapes are already known —
so this is checkable rather than aspirational.
