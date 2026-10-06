# Specification Quality Checklist: Graceful Desktop Startup Failures

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

The queue entry said "report ... failures as actionable startup errors instead of
panicking." The code confirms the panics and also shows the reporting channel
already exists, so this is narrower than it first appears:

- The desktop app's startup hook returns a failure-capable result, and the desktop
  framework converts a returned error into a named setup failure. The five
  `.expect()` calls in startup bypass that channel entirely. The fix is returning
  errors, not inventing a mechanism.
- `main.rs` calls the startup function and discards its result, so a returned
  error currently still reaches no one. That discard is part of the defect.
- Every error type involved already exists and already carries a specific message —
  the directory, download, configuration, and corpus errors are all distinct
  variants. Nothing needs to be invented to report them.

**Scope boundaries set deliberately:**

- *Corpus load stays blocking.* Story 2 was originally written to let the app open
  without a corpus so the user could switch language in-app. Investigation showed
  that the analysis engine is built from the corpus and roughly twenty commands
  hold a lock on it, so opening without one would require threading an absent
  engine through most of the command surface — a much larger change than the rest
  of this feature, and arguably its own piece of work. Direction taken instead:
  keep it blocking, and make the message name the missing file and the language
  setting that chose it. Actionable without restructuring.
- *Directory and download failures also stay blocking.* Neither leaves a usable
  app; the requirement is to explain rather than to continue.
- *Non-startup failures are out of scope.* Watcher-triggered reload already logs and
  continues; FR-014 keeps that unchanged rather than folding it in.
- *The command-line interface is out of scope.* It reports errors on its own terms
  and this feature concerns the desktop app.

**Ordering clarified during clarify.** The window is opened before startup work
begins, so every failure has somewhere to be reported. This does not disturb the
order of the data-dependent steps: the window does not depend on directories, data,
configuration, or corpus, so it can precede all of them. The framework already
creates the configured window ahead of the startup hook, so this confirms the
existing shape rather than reordering anything.

**On testability:** the desktop crate has no tests and its startup runs a full
framework builder, so end-to-end verification is a manual run. FR-013 and SC-006
therefore lean on preserving the success path exactly rather than on new automated
coverage — a change here that breaks normal startup is the more likely regression,
so the tests that exist elsewhere must keep passing unchanged.
