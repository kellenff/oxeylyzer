# Decision: Upgrade to zip 9 — Attempted, Verified, Reverted to Batch Into a Specified Pass

- **Slug**: modernize-deps-idioms
- **Decided**: 2026-10-13T17:24:23Z (UTC) — decide pass, **revision 3**
- **Verdict**: **go** (unchanged) — scope widened by hands-on evidence
- **Status of the attempted upgrade**: **implementation reverted**; patch preserved at
  `evidence/zip9-upgrade-reverted.patch`; work deferred into `/speckit.specify`
- **Artifacts reviewed**: intake.md ✓ | research.md ✓ | problem.md ✓ | concept.md ✓ (rev 2) |
  decision.md rev 2 ✓ | `evidence/zip9-upgrade-reverted.patch` ✓ (new, revision 3)
- **Supersedes**: decision.md rev 2. Its scorecard and handoff are preserved below under
  "Prior decision (superseded)". Revision 3 changes the **scope** and the **evidence base**, not the
  verdict.

## What prompted revision 3

Between revision 2 and now, the following happened — this is the first evidence in this assessment
that comes from running the code rather than reading it:

1. **A full major bump was built and tested on a throwaway copy** (`/tmp/oxcopy`, since deleted). Every
   direct Rust dependency was moved to latest: `zip 2→9`, `ureq 2→3`, `ratatui 0.29→0.30`,
   `crossterm 0.28→0.29`, `rustyline 17→18`, `itertools 0.14→0.15`, `directories 5→6`, `toml 0.9→1`,
   `shlex 1→2`, `nanorand 0.7→0.8`, `fancy-regex 0.17→0.19`.
2. **Result: 5 compile errors in 2 files, 6 test failures in 1 module, 9 new clippy warnings.** The
   whole workspace reached "compiles + tests green" with three one-line fixes plus a containment
   decision. Details in `evidence/zip9-upgrade-reverted.patch` and "Measured cost" below.
3. **The owner directed the zip-9 upgrade be implemented** ("it counts as an outdated dependency").
   It was implemented in the working tree, verified green (39/39 resources tests, 220 workspace-wide,
   fmt- and clippy-clean for the touched crate), and then **reverted at the owner's direction** so that
   all the modernization work can be scoped in one `/speckit.specify` pass instead of landing as
   ad-hoc edits.

This revision records the reversal and folds the measured evidence into the handoff so the
specification starts from facts rather than estimates.

## Scorecard

| Criterion | Rating | Justification |
|-----------|--------|---------------|
| Problem validity | **strong** | Unchanged in substance from rev 2, but **strengthened** by measurement: the repo *is* materially behind (≥12 unrecorded major gaps), the staleness *is* invisible (a 2026-03-28 nested lockfile that can't resolve its own package sat undetected), and the cost of catching up turns out to be small — which removes "too expensive to matter" as an excuse for the drift. Note a **correction to rev 2**: the "downstream consumers get no MSRV promise" argument is weakened — there is **no `cargo publish` anywhere** (only `publish-tauri.yml`, which ships desktop builds) and no `publish =` key, so `oxeylyzer-core` is not a published contract. MSRV is the owner's preference, not an obligation. The problem statement should carry that correction. |
| Evidence strength | **strong** (raised from adequate) | The bar that held rev 2 at `adequate` — "no `cargo outdated`/`cargo audit`, migration guides 404'd, `zip` 9 size unknown" — is now partly cleared by **direct execution**: a real full-major bump was compiled and tested, the `zip 9.0.2` API surface was probed, containment behavior was measured case-by-case, and `ureq 3`/`ratatui 0.30`/`rustyline 18`/`itertools 0.15`/`toml 1`/`directories 6`/`shlex 2` were each proven to need zero or one-line changes. Not fully `strong` across the board: the **frontend (`vite 6→8`, `typescript 5.6→7.0`) was excluded from the probe** (the tauri workspace member was dropped to keep it fast) and remains unmeasured. Also no full `cargo audit` over the transitive graph. |
| Value vs. inaction | **strong** | Unchanged from rev 2, and clarified: measurement showed the cost side is *small*, so the value/cost ratio improved. The decisive driver remains ownership — a personal fork of an abandoned project with no upstream, so drift is unowned risk. |
| Feasibility / appetite | **strong** (raised from adequate) | Rev 2 flagged one unresolved mechanism (adapting "latest 3 stable" to a check without a schedule). That is still open, but the *implementation* feasibility question is settled: the majors are days, not months. The scope is now known well enough to specify. |
| Strategic fit | **unknown** | Unchanged and still acknowledged: `.specify/memory/constitution.md` is an unpopulated template, so there is no ratified principle to judge against. |
| Risk posture | **adequate** (raised in specificity, still not strong) | Risks are now *measured* rather than hypothesized, and one was resolved: the feared `zip 2→9` archive-safety regression is **not** a security regression — under zip 9, hostile entries stay inside `dest`; the change is `enclosed_name()` switching from **reject** to **normalize**, which breaks the repo's tests on *policy* rather than safety. Remaining unmitigated risks: (a) the frontend majors are unmeasured; (b) the `.data-version` sentinel is written when hostile entries are *normalized-and-accepted* rather than rejected, so a partially-written install could be marked complete — a behavioral regression worth a specification decision; (c) `fxhash` is *deliberately* faster than `ahash` at its one call site and any swap needs a benchmark. |

## Verdict & Rationale

**go**, unchanged — but the handoff is materially different, and the reason is that the assessment's
central uncertainty collapsed under measurement.

Rev 2's `go` rested on an argument: *the standard is missing, and that is worth fixing*. That argument
survives. What does not survive is rev 2's framing of the **majors** as an unscopable large risk. A
real full bump was performed and the true cost was 5 errors in 2 files and one policy decision. The
`zip 9` API surface was probed rather than inferred, and the feared security consequence was
disproved: containment holds, and what actually changed is that upstream now *normalizes* hostile
paths instead of *rejecting* them.

That distinction is the most important thing revision 3 adds, so it is stated plainly: **the six
previously-failing archive tests failed on policy, not on safety.** `enclosed_name()` in zip 9
rewrites `oxeylyzer-data-files/../escaped.txt` to `escaped.txt` (inside `dest`) where zip 2 refused it
outright; a leading `/` or drive prefix likewise loses its root rather than being rejected. Nothing
escaped. But normalization lets the loop write to paths the archive never claimed, and the
`.data-version` sentinel is still written in those cases — which is a real behavior change the
specification must decide, either by validating raw names (what the reverted implementation did) or
by accepting normalization and rewriting the tests.

**Why the upgrade was reverted despite being green.** The implementation was correct and verified.
It was reverted because shipping `zip 9` alone would have left the same drift in eleven other
dependencies, the frontend, the stale lockfile, the unenforced MSRV, and the fork dependencies —
precisely the recurring ad-hoc pattern this assessment exists to stop. Deferring it into one
specified pass costs nothing (the patch is preserved and verified) and buys a coherent scope.

**Two corrections to keep honest**, carried forward so the specification does not inherit them:

1. `oxeylyzer-core` is **not published**; the MSRV case is preference, not contract.
2. `nanorand` on the search hot path was called a reproducibility risk in rev 1/2. There is **no seed
   API anywhere** — generation is already non-reproducible run-to-run, so there is no baseline to
   invalidate. The risk was overstated.

## If needs-clarification

Not applicable; all owner questions are answered. Two scope questions remain genuinely open and are
handed to `specify` rather than blocking: the `zip` containment policy, and whether the frontend is in
scope.

## If go — Handoff to `__SPECKIT_COMMAND_SPECIFY__`

- **Problem**: The fork is now the trunk of an abandoned project, yet nothing states what "current"
  means (no `rust-version` in 6 manifests, no dependency policy, no lint config) and nothing detects
  drift (no audit or freshness step in CI), so known-affected and stale dependencies are
  indistinguishable from merely old ones — and a 2026-03-28 nested lockfile that cannot resolve its
  own package sat undetected. — [`problem.md`; `decision.md` rev 2]
- **Chosen approach**: **Option B** (rev 2) — declare the standard, add a CI advisory audit that fails
  the build with a recorded allowance list (manual only: no bot, no schedule), take the cheap
  deletions and the update set, and record retention decisions for forked/unmaintained dependencies —
  **expanded by measured evidence** into a batched modernization pass rather than the majors-only
  deferral rev 2 assumed.
- **In scope (revised by evidence)** — *all measured green on a throwaway copy unless noted*:
  - **The majors**: `zip 2→9`, `ureq 2→3`, `ratatui 0.29→0.30`, `crossterm 0.28→0.29`,
    `rustyline 17→18`, `itertools 0.14→0.15`, `directories 5→6`, `toml 0.9→1`, `shlex 1→2`,
    `nanorand 0.7→0.8`, `fancy-regex 0.17→0.19`. (≈5 error fixes total; patch available.)
  - **The remaining transitive advisory**: `ureq 3` pulls `rustls ≥0.23.45`, clearing
    RUSTSEC-2026-0285 (lockfile currently on 0.23.40). `anyhow` needs ≥1.0.103 for RUSTSEC-2026-0190
    (currently 1.0.102) — a within-range `cargo update`.
  - **The standard**: declare MSRV (latest 3 stable) in one place; documented dependency policy.
  - **The signal**: CI advisory audit that fails the build + allowance list; freshness report. No bot,
    no schedule.
  - **The deletions**: `num_cpus` → `std::thread::available_parallelism` (1 site); test-only
    `once_cell` → `std::sync::LazyLock` (5 sites, all `#[cfg(test)]`); the stale nested
    `oxeylyzer-tauri/src-tauri/Cargo.lock`.
  - **The forks**: decide/replace `xflags` (one macro + one error variant, 144 lines across 2 crates)
    and `ansi_rgb` (one import).
  - **The pre-existing CI breakage** (verified identical on pristine HEAD, unrelated to any upgrade):
    9 `cargo fmt` blocks and 1 clippy error at `repl.rs:759`. Must be fixed or CI cannot gate the
    above.
- **Out of scope**: frontend majors (`vite 6→8`, `typescript 5.6→7.0`) — **unmeasured**, recommend a
  separate pass; `fxhash` swap (deliberate perf choice at its 1 call site, needs a benchmark);
  corpus data / `language_data` / data version; a general "more idiomatic" sweep; upstream
  reconciliation.
- **Success metrics**: as rev 2, plus: 0 unrecorded major gaps (baseline ≥12); the 12 hostile-input
  tests pass under `zip 9` with the containment policy explicitly chosen; CI green on `main`.
- **Carried-forward open questions** (for specification):
  - [NEEDS CLARIFICATION: `zip 9` containment policy — validate raw entry names and refuse
    normalization (the reverted implementation), or accept normalization and rewrite the 6 tests?]
  - [NEEDS CLARIFICATION: should `.data-version` be withheld when hostile entries are normalized-and-
    accepted rather than rejected?]
  - [NEEDS CLARIFICATION: how "latest 3 stable" is expressed and refreshed without a schedule.]
  - [NEEDS CLARIFICATION: which yields when MSRV and a dependency's inherited floor collide.]
  - [NEEDS CLARIFICATION: allowance-list mechanics; and confirm audit policy is
    "any advisory fails, acceptances recorded".]
  - [NEEDS CLARIFICATION: `xflags`/`ansi_rgb` — replace, vendor, or retain with rationale?]
  - [NEEDS CLARIFICATION: is the frontend in scope for this specify, or a separate one?]
- **Evidence available to the specification**: `evidence/zip9-upgrade-reverted.patch` — the complete,
  verified `zip 9` implementation (manifest, lock, and the raw-name-containment `download.rs`),
  8,265 bytes. This is the strongest artifact in the assessment: it is the change, tested, not a
  description of it.

## Prior decision (superseded)

Preserved for audit. Revision 2 verdict was **go** on the rev-2 concept (Option B), scorecard then:
problem validity `strong`, evidence `adequate`, value vs. inaction `strong`, feasibility/appetite
`adequate`, strategic fit `unknown`, risk posture `adequate`. Revision 1's `needs-clarification`
decision and its five blocking questions are preserved in rev 2's "Prior decision" section; the owner
answered all five (personal fork of an unmaintained project; rescope accepted; manual maintenance
only; MSRV latest 3 stable; audit gate yes).

Writes confined to `.specify/assessments/modernize-deps-idioms/` (`decision.md`, `evidence/`). The
reverted upgrade's source-file changes were discarded with `git checkout`; the working tree is clean
apart from `.specify/`. No source file remains modified.