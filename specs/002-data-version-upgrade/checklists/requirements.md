# Specification Quality Checklist: Data File Version Upgrades

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

**Validation pass, 2026-10-06**

The specify draft promised that user-created layouts always survive a refresh.
That was wrong, and the code shows why: `save_generated` writes user layouts to
`static/layouts/<lang>/` inside the data directory (`oxeylyzer-tauri/src-tauri/src/lib.rs:1089`),
and that directory is gitignored — it is user-owned data sharing a tree with the
released files. There is no marker distinguishing the two, so "never clobber a
user layout" is not implementable by re-extracting in place.

Direction taken: refresh re-extracts the release over the data directory. Files
the release ships are replaced; files it does not ship are left alone, so a
user's layout survives unless the release happens to ship the same path. This is
User Story 2 as rewritten, and it is now the stated accepted tradeoff rather than
a promise the implementation cannot keep.

Consequence worth remembering: a saved layout is lost if the release ships the
same path. Nothing mitigates that short of saving layouts outside the data
directory, which would be a larger change to `save_generated` and the layout
glob, and is out of scope here.

Also settled during clarify, from the code:

- `is_first_run()` (`dirs.rs:43`) currently means "the sentinel is absent" and
  the Tauri app gates its download on it (`lib.rs:1608`). Once versions exist,
  absence no longer decides whether a download is needed, so the concept is
  renamed to describe data currency rather than install history.
- The current data version belongs to the build, not the archive, or a user on an
  older app could not detect they were behind.
- A newer recorded version than current means the user downgraded; the spec
  treats that as usable rather than rolling their data back.