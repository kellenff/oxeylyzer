# Feature Specification: Modern Dependency Foundation for a Maintained Fork

**Feature Branch**: `009-modernize-deps-idioms`
**Created**: 2026-10-13
**Status**: Draft
**Input**: User description: "Modernize the repo to the latest versions of dependencies, language features, and idiomatic implementation standards — scoped properly after assessment."

**Assessment provenance**: This spec is the `go` handoff from
`.specify/assessments/modernize-deps-idioms/` (intake → research → define → shape → decide, rev 3).
The full problem statement, evidence, and reverted implementation live there. Key measured facts are
inlined below; where they are, they were produced by building and testing, not by reading.

**Two corrections carried from the assessment** (recorded so this spec does not inherit them):

1. `oxeylyzer-core` is **not published** to crates.io — there is no `cargo publish` step anywhere
   (only `publish-tauri.yml`, which ships desktop installers) and no `publish =` key. MSRV is
   therefore a maintainer preference, not a compatibility contract owed to unknown consumers.
2. Generation has **no seed API**; it is already non-reproducible run-to-run. Swapping the RNG cannot
   invalidate a baseline that does not exist.

## Problem

This repository is a personal fork of an abandoned project, with no `upstream` remote configured and a
`README.md` that still badges and links the original (`o-x-e-y/oxeylyzer`) as though it were this
code's home. The fork is now the trunk — there is no upstream to inherit fixes from and none to wait
for.

Against that, the project has no stated definition of "current" and no way to detect drift:

- **0 of 6 manifests** declare `rust-version`, so there is no declared compatibility floor.
- **No dependency policy exists.** Majors are neither adopted nor consciously declined; they
  accumulate unremarked.
- **No drift signal exists.** `.github/workflows/ci.yml` runs `fmt --check`, `clippy -D warnings`, and
  tests, but has no audit, deny, or freshness step. The version gap in the assessment had to be
  reconstructed by hand-querying registries and reading two lockfiles.
- **The drift is already invisible in practice.** `oxeylyzer-tauri/src-tauri/Cargo.lock` is tracked
  (last committed 2026-03-28) and **cannot resolve its own package's dependencies** — it does not
  contain `oxeylyzer-core`, which the crate declares. It sat undetected.
- **CI is currently red on `main`**, which means no gate exists to protect any change: `cargo fmt
  --all -- --check` fails in 9 blocks and `cargo clippy -D warnings` fails at `repl.rs:759`. Both
  verified identical on a pristine `HEAD` export, so both pre-date this work.

The consequence is that known-affected, stale, unmaintained, and forked dependencies are all
indistinguishable from one another, and every upgrade decision is re-argued from scratch.

## Clarifications

### Session 2026-10-13

- Q: When a downloaded data archive contains an entry whose path tries to climb out of the data
  directory, should extraction refuse the whole archive, or accept zip 9's rewritten in-bounds path?
  → A: Refuse — validate the raw entry name in-repo; `is_contained` stays authoritative and the six
  hostile-input tests keep their current meaning.
- Q: What should happen to the two personal-fork git dependencies, `xflags` and `ansi_rgb`?
  → A: Replace both with maintained equivalents — a maintained CLI parser across the two CLIs and an
  in-repo color helper for the single `ansi_rgb` import. No fork code retained.
- Q: Should the Tauri frontend's major upgrades (`vite 6→8`, `typescript 5.6→7.0`, `eslint 9→10`) be
  part of this feature? → A: Include them — this feature is the whole "modern foundation".
- Q: When a dependency upgrade would require a Rust version outside the declared "latest 3 stable"
  window, what should win? → A: The declared window wins; the dependency is held at its newest
  window-compatible version, the exception is recorded, and it is surfaced in the freshness report.
- Q: How should the "latest 3 stable" window be evaluated in CI, given that nobody will run a
  scheduled refresh? → A: CI computes it dynamically from the Rust release feed on every build — no
  pinned literals to go stale and no human cadence.

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Drift Is Visible and Enforced (Priority: P1)

A maintainer wants to know whether the project is behind without performing a manual discovery pass,
and wants a known-affected dependency to surface on its own rather than by accident.

Today the only way to learn the project is behind is to query crates.io and npm by hand and
cross-read two lockfiles — which is exactly how this assessment had to establish it. A security
advisory against a dependency would go unnoticed until someone happened to look.

**Why this priority**: This is the only part that stops the problem recurring. Without it, every
other story here is a one-off cleanup that decays at the same rate as the drift it fixes — the pattern
the repository's own history already shows (`19a2b91 apply clippy suggestions`,
`ddd62b2 fix clippy warnings`, `03c4a9c remove smartstring, arrayvec and ansi_rgb dependencies`)
producing no policy.

**Independent Test**: Introduce a synthetic advisory (or pin a known-affected version) and confirm CI
fails with a clear message; confirm the freshness report names current vs. available versions for the
workspace, without any manual registry query.

**Acceptance Scenarios**:

1. **Given** a dependency at a version covered by a RustSec advisory, **When** CI runs, **Then** the
   build fails with the advisory identifier and the affected package named.
2. **Given** an advisory the maintainer has consciously accepted, **When** it is recorded in the
   allowance list, **Then** CI passes and the acceptance and its reason are discoverable in the repo.
3. **Given** a fresh checkout with no prior knowledge, **When** the maintainer asks what is out of
   date, **Then** a single command or CI log answers it without contacting crates.io by hand.
4. **Given** `main` at the start of this work, **When** CI runs, **Then** it passes — the pre-existing
   `fmt` (9 blocks) and `clippy` (`repl.rs:759`) failures are fixed so the gate is meaningful.

---

### User Story 2 - Rust Dependencies Are Current (Priority: P2)

A maintainer wants the Rust workspace on current dependencies, including the ones that clear known
advisories, without the project silently drifting another several majors.

The project is behind by at least 12 unrecorded major versions. Two advisories are currently
unresolved: `anyhow 1.0.102` is below the 1.0.103 patch for RUSTSEC-2026-0190, and transitive
`rustls 0.23.40` is below the 0.23.45 patch for RUSTSEC-2026-0285.

**Why this priority**: It removes live advisory exposure and closes the measured gap. It ranks below
Story 1 because without the enforcement, this is the same one-off cleanup the project has already done
three times — and because the cost turned out to be small, so it is cheap to do correctly once the
gate exists.

**Independent Test**: Every direct Rust dependency resolves to its latest stable version; the full
test suite passes; CI (including the new audit step) is green.

**Acceptance Scenarios**:

1. **Given** the current lockfile, **When** the Rust workspace is upgraded, **Then** all direct
   dependencies are at latest stable and `cargo test --workspace` passes.
2. **Given** the upgraded lockfile, **When** the audit step runs, **Then** `anyhow ≥ 1.0.103` and
   `rustls ≥ 0.23.45` are resolved and no advisory remains unresolved or unrecorded.
3. **Given** the archive-extraction path, **When** the `zip` upgrade lands, **Then** all hostile-input
   tests pass under the agreed containment policy (see FR-013).
4. **Given** a hostile archive entry that is normalized rather than rejected, **When** extraction
   completes, **Then** no file is written outside the destination directory and the completion
   sentinel reflects the agreed policy (FR-014).

---

### User Story 3 - Dependency Surface Is Smaller and Owned (Priority: P3)

A maintainer wants dependencies that are unmaintained, superseded, or sourced from personal forks to
be either removed or consciously retained with a recorded reason.

Twenty-nine direct dependencies are split across five crates, including two consumed from 0-star
personal git forks pinned by revision (`xflags`, `ansi_rgb`), several effectively frozen upstream
(`fxhash` last released 2017, `sexp` 2016, `file-chunker`/`sliding_window_alt` 2022), and two that the
standard library now covers.

**Why this priority**: It is durable value — less to maintain and less to trust — but it neither
secures nor unblocks anything, so it follows the two stories above.

**Independent Test**: Each previously-vestigial or replaced dependency is gone from the manifests, and
each retained one has a recorded reason discoverable in the repo.

**Acceptance Scenarios**:

1. **Given** `num_cpus`, **When** the change lands, **Then** the standard library provides the
   available-parallelism value and the dependency is gone.
2. **Given** test-only `once_cell` usage, **When** the change lands, **Then** the standard library's
   lazy/once primitives provide it and the dependency is gone.
3. **Given** `xflags` and `ansi_rgb`, **When** this work completes, **Then** both are replaced by
   maintained equivalents and neither personal git fork remains in the build.
4. **Given** the REPL and bench CLIs after the flag-parser replacement, **When** the same arguments are
   passed as before, **Then** both accept everything they accepted before and reject what they
   rejected — the CLI surface is unchanged.
5. **Given** the Tauri frontend, **When** the frontend majors land, **Then** `vite build` and the
   desktop dev loop still produce a working app under the new bundler.
6. **Given** the nested Tauri lockfile, **When** this work lands, **Then** exactly one lockfile policy
   applies to the workspace.

---

### Edge Cases

- **A dependency's inherited MSRV exceeds the declared floor.** The `ratatui 0.30` line requires
  Rust 1.88 and `ureq 3` states 1.71.1. A future bump could demand a toolchain newer than the declared
  window allows. Resolved by clarification: the window wins; the dependency is held at its newest
  compatible version and the hold is recorded and surfaced (FR-002a).
- **An advisory has no available patch.** The allowance list must express acceptance, not just
  silence, or the audit step gets disabled the first time it is inconvenient.
- **Normalization vs. rejection in archive extraction** (zip ≥ 9): a `..` that would previously be
  refused is now rewritten by zip to stay inside the destination. Resolved by clarification: the repo
  refuses — validation runs on the raw name before zip's normalization can launder it (FR-013).
- **A flag-parser replacement silently changes CLI behavior.** The replacement must preserve the exact
  accepted/rejected argument surface of both CLIs (FR-015); the REPL's argument grammar is currently
  defined by one `xflags!` macro, so a like-for-like migration must be diff-checked, not eyeballed.
- **A hostile entry appears before a valid one.** Partial writes must not be marked complete.
- **The freshness check runs in an environment without network access.** It must degrade predictably
  rather than fail opaquely.
- **A dependency is yanked after being locked.** The policy must state the expected response.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The project MUST declare a minimum supported Rust version in exactly one authoritative
  location, with the value "the three most recent stable Rust releases" as the policy. The window MUST
  be evaluated dynamically by CI from the Rust release feed at build time; no pinned version literals
  MAY serve as the source of truth.
- **FR-002**: The declaration MUST be machine-checkable against the toolchain CI actually uses, so
  drift between the stated floor and the build is detected rather than assumed.
- **FR-002a**: When a dependency's required Rust version falls outside the declared window, the window
  MUST win: the dependency MUST be held at its newest window-compatible version, the hold MUST be
  recorded in the repo with its reason, and the freshness report (FR-005) MUST surface it rather than
  leaving it silent.
- **FR-003**: CI MUST run an advisory audit over the resolved dependency graph and MUST fail the build
  when an advisory is present that is not explicitly recorded as accepted.
- **FR-004**: The project MUST maintain an allowance list of consciously accepted advisories, each
  with the reason for acceptance. An unrecorded advisory MUST fail the build.
- **FR-005**: CI MUST report, without any manual registry query, which direct dependencies are behind
  and by how much.
- **FR-006**: All direct dependencies in the Rust workspace and the Tauri frontend MUST resolve to
  their latest stable versions.
- **FR-007**: `anyhow` MUST resolve to a version at or above 1.0.103 and `rustls` to at or above
  0.23.45, clearing RUSTSEC-2026-0190 and RUSTSEC-2026-0285 respectively.
- **FR-008**: The full workspace test suite MUST pass after every dependency change in this scope.
- **FR-009**: The pre-existing `cargo fmt --all -- --check` failures (9 blocks) and the
  `cargo clippy -D warnings` failure at `repl.rs:759` MUST be fixed, so CI is green on `main` and
  able to gate this work.
- **FR-010**: `num_cpus` MUST be removed and its single call site served by the standard library.
- **FR-011**: Test-only `once_cell` usage MUST be removed and served by the standard library.
- **FR-012**: The project MUST apply exactly one lockfile policy to the workspace, and the nested
  `oxeylyzer-tauri/src-tauri/Cargo.lock` MUST be reconciled to it or documented as intentionally
  separate.
- **FR-013**: The archive-extraction containment policy is: **refuse**. Extraction MUST validate the
  raw entry name in-repo after stripping the archive prefix, and MUST abort the whole archive on any
  entry that is absolute, drive-prefixed, or climbs out of the destination — including entries zip ≥ 9
  would normalize to an in-bounds path (`is_contained` remains authoritative; `ZipFile::enclosed_name()`
  is not used to decide containment). The six existing hostile-input tests keep their reject
  semantics.
- **FR-014**: The completion sentinel MUST NOT be written when extraction did not honor every entry,
  regardless of which containment policy FR-013 selects.
- **FR-015**: `xflags` and `ansi_rgb` MUST be replaced with maintained equivalents (a maintained CLI
  parser and an in-repo color helper respectively); neither personal git fork remains in the build. The
  REPL and bench command-line surfaces MUST accept and reject exactly what they do today.
- **FR-016**: The project's stated identity MUST stop describing an abandoned upstream as this code's
  home, so a reader can tell this is the maintained fork.
- **FR-017**: No requirement here may change scoring, generation algorithms, or corpus/data-version
  behavior; dependency movement must leave computed results intact.
- **FR-018**: The Tauri frontend MUST complete its major upgrades (`vite 6→8` — a Rolldown/Oxc bundler
  migration — `typescript 5.6→7.0`, `eslint 9→10`) with a working `vite build` and dev loop. Unlike the
  Rust-side items, this cost was **never measured**; the plan must treat it as the scope's principal
  estimating risk.

### Key Entities

- **Standard**: the declared MSRV and the dependency policy — what "current" means for this repo.
- **Allowance list**: recorded advisory acceptances, each with a reason.
- **Lockfile policy**: the single rule governing which lockfiles exist and what they cover.
- **Containment policy**: the documented accept/reject rule for hostile archive entry names.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A commit introducing a dependency covered by an unrecorded advisory fails CI.
  (Baseline: no advisory check exists; a 2026-03-28 stale lockfile sat undetected.)
- **SC-002**: Declared MSRV: 1 authoritative declaration, checkable in CI. (Baseline: 0 of 6
  manifests.)
- **SC-003**: Unrecorded major-version gaps: 0. (Baseline: ≥ 12 — `zip` 2→9, `ureq` 2→3, `ratatui`
  0.29→0.30, `crossterm` 0.28→0.29, `rustyline` 17→18, `itertools` 0.14→0.15, `directories` 5→6,
  `toml` 0.9→1, `shlex` 1→3, `nanorand` 0.7→0.8, plus frontend `vite` 6→8, `typescript` 5.6→7.0,
  `eslint` 9→10 — all in scope per this session's clarifications.) A dependency held below latest for
  the MSRV window counts as recorded, not as a gap (FR-002a).
- **SC-004**: Unresolved advisories against resolved dependencies: 0 unrecorded. (Baseline: 2 known —
  `anyhow` 1.0.102, `rustls` 0.23.40.)
- **SC-005**: Dependencies sourced from personal git forks or unmaintained upstreams without a
  recorded rationale: 0. (Baseline: ≥ 8 — `xflags` and `ansi_rgb` are replaced outright per FR-015;
  `fxhash`, `sexp`, `sliding_window_alt`, `file-chunker`, `num_cpus`, and test-only `once_cell` are
  removed or rationalized.)
- **SC-006**: CI passes on `main`. (Baseline: failing — 9 `fmt` blocks, 1 `clippy` error.)
- **SC-007**: Workspace test suite passes, including all hostile-input archive tests. (Baseline: 220
  tests; the archive subset passes today under `zip 2` and failed under `zip 9` until the
  containment policy was decided.)
- **SC-008**: A fresh checkout can answer "what is out of date?" in one step with no manual registry
  query. (Baseline: requires querying crates.io and npm by hand and cross-reading two lockfiles.)

## Assumptions

- **Enforcement is build-time only.** Previously confirmed by the maintainer: manual maintenance, no
  bot, no schedule, no notification cadence. A mandatory CI check is not "recurring automation" in
  that sense — it runs when the build runs and asks nothing of anyone between builds.
- **"Latest 3 stable" applies to Rust toolchains**, and is a policy to declare and check rather than a
  promise to external consumers (see the publish correction above). It is evaluated dynamically by CI
  (FR-001) and can block upgrades (FR-002a) — both decided this session.
- **The measured upgrade cost holds.** A full major bump was built and tested on a throwaway copy:
  5 compile errors in 2 files, 6 test failures in 1 module, 9 new clippy warnings. The verified
  reverted implementation is preserved at
  `.specify/assessments/modernize-deps-idioms/evidence/zip9-upgrade-reverted.patch` and may be used
  directly.
- **`fxhash` is a deliberate performance choice** at its one call site (an in-code comment states it
  is much faster than `ahash` there), so any replacement needs a benchmark rather than a swap. It is
  therefore out of scope for replacement here and in scope only for a retention rationale.
- **The frontend majors are in scope but unmeasured.** `vite 6→8` swaps the bundler to Rolldown/Oxc
  and `typescript 5.6→7.0` breaking surface was never probed — the Rust-side cost estimates come from
  an actual build; there is no equivalent evidence for the frontend, and the plan should say so where
  it sequences this work (FR-018).
- **Corpus data, `language_data/*.json`, and the data version are untouched.**
- **No behavioral change to scoring or generation** is intended or permitted by this scope.