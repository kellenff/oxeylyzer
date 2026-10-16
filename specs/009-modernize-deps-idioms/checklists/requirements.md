# Specification Quality Checklist: Modern Dependency Foundation for a Maintained Fork

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-10-13
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

**All three original `[NEEDS CLARIFICATION]` markers are resolved**, plus two further decisions taken
in the same session (see `spec.md` → Clarifications, Session 2026-10-13, 5 Q/A):

1. **FR-013 — zip containment: refuse.** Validation runs on the raw entry name, `is_contained` stays
   authoritative, and the six hostile-input tests keep their reject semantics.
2. **FR-015 — fork deps: replace both** with maintained equivalents. This adds a CLI-parser migration
   across the REPL and bench, so FR-015 and a new edge case pin the accepted/rejected argument surface.
3. **Frontend majors: included** (new FR-018). `vite 6→8` is a Rolldown/Oxc bundler migration and
   `typescript 5.6→7.0` was never probed — the one workstream in this spec with **no measured cost**.
4. **MSRV window wins** over an incompatible dependency (new FR-002a), so FR-001 is an enforceable
   floor rather than documentation.
5. **The window is computed dynamically in CI** (amended FR-001) — no pinned literals that can go stale.

**Scope grew as a result of these answers.** Question 3 pulled the Tauri frontend into what was a
Rust-workspace pass, and question 2 pulled in a CLI-parser migration. Every requirement remains
testable and the spec is still bounded, but SC-003 now spans two toolchains and one of them is
unmeasured — the plan should state that rather than let a uniform-looking criteria list imply otherwise.

Everything else was resolvable from the assessment and is stated rather than asked. Deliberately
**not** raised as clarifications: the MSRV value (answered: latest 3 stable), the enforcement mode
(answered: manual, no bot — a build-time check is consistent with that), whether the rescope is
accepted (answered: accepted), and whether to unbreak CI (not a question — CI is red and cannot gate
this work until fixed).

**Measured rather than asserted.** The cost estimates in the Assumptions section come from actually
building a full major bump on a throwaway copy (5 errors in 2 files, 6 test failures, 9 clippy
warnings), not from reading changelogs. The reverted implementation is preserved at
`.specify/assessments/modernize-deps-idioms/evidence/zip9-upgrade-reverted.patch` and the plan may use
it directly rather than rediscovering it.

**Two assessment corrections carried deliberately**, because the naive version of this spec would
repeat them: `oxeylyzer-core` is not published (so MSRV is preference, not contract — FR-001 says
"declare and check", not "honor downstream support"), and generation has no seed API (so the
RNG-on-hot-path risk is about *behavior*, not reproducibility — FR-017).

**One requirement is not modernization at all.** FR-009 exists because CI is currently red on `main`
(9 `fmt` blocks, one `clippy` error at `repl.rs:759`), verified identical on a pristine `HEAD` export.
It is included rather than split out because no other requirement here can be trusted until the gate
works — an upgrade merged into a red CI has demonstrated nothing.

**On SC-003's baseline of "≥ 12":** the count is drawn from manifest and registry comparison, not a
`cargo outdated` run, so it is a lower bound. The target of 0 is unaffected; if `cargo outdated`
finds additional majors, they fall inside the same requirement.