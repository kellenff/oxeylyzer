# Concept: Adopt a Modern Foundation in a Personal Fork of an Unmaintained Project

- **Slug**: modernize-deps-idioms
- **Created**: 2026-10-13T17:24:23Z (UTC) — shape pass, **revision 2**
- **Supersedes**: `concept.md` written earlier this run (grounded in an unstated "hygiene" trigger and
  the now-answered assumption that a signal was unwanted)
- **Recommended option**: **Option B — Foundation: toolchain policy, drift signal, cheap deletions**
  (fallback: A)

Concept level only. No architecture, no config text, no task breakdown.

## What changed in revision 2

The owner answered the five blocking questions. Two answers rearranged the problem; three settled the
standard itself.

1. **The trigger is not hygiene — it is ownership.** "This is a personal fork of an unmaintained
   project and I want a modern foundation." Verified in the repo: `origin` is
   `git@github.com:kellenff/oxeylyzer.git`, **no `upstream` remote is configured**, and `README.md`
   still badges and links the original project (`o-x-e-y/oxeylyzer`, `oxey/oxeylyzer`) as though it
   were the home of this code. There is no upstream to wait for and no upstream to return deps to:
   **this codebase is now the trunk.** That converts "keep up with a live project" into "own a
   foundation you are responsible for," which is a materially stronger problem than the one define
   articulated with a `medium`-confidence trigger.
2. **Recurring maintenance is declined: manual only.** The drift signal must therefore be
   **pull-based and CI-only** — something a human invokes or reads — with **no bot, no scheduled job,
   and no report that must be read on a cadence**. Option C's rabbit holes (bot noise, cadence
   nobody chose) are thereby closed.
3. **The standard is now stated**: MSRV = **latest 3 stable** Rust releases; an advisory signal **is**
   wanted; its failure policy is **fail the build**. The standard is no longer a shape question — it
   is given, and shape only has to say what the project ends up with.

Note on the owner's phrasing: answers 3 ("manual maintenance only") and 5 ("yes", audit gate) are
reconciled as stated above — **a CI-enforced check is manual in the sense that matters** (no bot
commits, no schedule), while still being automatic at build time. Recorded explicitly; if the
intended meaning was "no automated check at all, not even at build time," this option collapses to A,
which is the named fallback and would meet the standard minus its enforcement.

Also carried forward from revision 1 and unchanged: the nested `oxeylyzer-tauri/src-tauri/Cargo.lock`
is **stale drift, not policy** (tracked, 2026-03-28, 518 packages vs. the workspace's 642, and it does
not contain `oxeylyzer-core` despite that crate depending on it) — and the stale lockfile is itself a
concrete instance of the fork problem: it is an artifact of the upstream project's layout that no
longer resolves this fork's reality.

## Options

### Option A — The fork declares its standard

- **Sketch**: The project writes down what it is: a maintained personal fork with its own identity.
  That means a declared **MSRV of latest-3-stable, stated in one place**, a written rule for how
  dependencies are held (majors on demand, with the reason recorded), a recorded disposition for each
  dependency that is unmaintained, forked, or known-affected by an advisory, and the removal of the
  stale nested lockfile. The README stops advertising the upstream project as the home of this code.
  Nothing is upgraded — but the current versions become *chosen* versions, and the next person
  reading the repo can tell what standard they are held to.
- **Appetite**: **small** (days).
- **Trade-offs**: Wins — problem goals 2, 3, and 4 are met outright. Sacrifices — nothing is detected;
  the next advisory against this codebase, which now has no upstream to fix it for you, waits for
  someone to remember to look. That is the exact failure that made this assessment necessary, so A
  states the standard without enforcing it.
- **Rabbit holes**: none significant. Choosing nothing here; the MSRV is already given as
  latest-3-stable, and the unmaintained-dependency dispositions are one line each.

### Option B — Foundation: standard, CI-enforced signal, cheap deletions  ← recommended

- **Sketch**: Everything in A, plus the two things that make a standard a property of the repository
  rather than a document. First, **a CI check that does not exist today**: an advisory audit that
  **fails the build**, with a recorded allowance list for acceptances, plus a freshness report that
  CI prints so drift is visible without anyone querying registries by hand — no bot, no schedule,
  nothing to obey on a cadence. Second, **the removals whose cost is already known**: `num_cpus`
  (one call site, superseded by the standard library), the test-only `once_cell` sites (five, all in
  `#[cfg(test)]`, and the test utilities already use the standard-library equivalent), and the stale
  nested lockfile. Every remaining unmaintained or forked dependency gets a recorded retention
  decision rather than a rewrite. Majors are explicitly not taken.
- **Appetite**: **medium** (weeks, mostly review latency rather than engineering).
- **Trade-offs**: Wins — goals 1, 2, 3, 4, and 6 in full, and goal 5 in the sense that the repo stops
  accumulating unexamined movement. Because the fork has no upstream, this is the closest thing to a
  safety net it can have: a reachable advisory on any dependency now reddens a build instead of
  surfacing by accident during an assessment like this one. Sacrifices — a new failure mode: a
  transitive advisory can fail the build on a day nobody changed anything, which for a personal repo
  is a real cost paid in interruption. The allowance list is what keeps that cost bounded, and it
  only works if accepting an advisory is written down as a decision.
- **Rabbit holes**: (1) **what MSRV = "latest 3 stable" means mechanically** — three named versions
  move every six weeks, so the floor either gets pinned to concrete versions and refreshed on a
  cadence the owner did not ask for, or is expressed as a rolling window that only a check can
  compare against; either way this is the one part of a "manual only" standard that inherently drifts,
  and it needs a stated refresh moment rather than a schedule. (2) **The rebuild-coupling trap**: if a
  beginner-bump upgrade moves an upstream MSRV floor above the latest-3 window, the check and the
  dependency cannot both pass — the escape is to state which one yields, decided in advance. (3) **The
  allowance list's own maintenance** — an unread list is what the signal was supposed to prevent.
- **Depends on**: nothing further from the owner; all five blocking questions are answered.

### Option C — Foundation, then the majors (staged)

- **Sketch**: B, then the majors are actually taken — low blast radius first (dev-dependency and
  CLI-adjacent crates), search-adjacent ones only with a behavioral check in place, archive extraction
  last and under its own specification, because that is where a major would land on code that just
  received a safety fix. The frontend majors (`vite` 6→8 — a bundler change — and `typescript`
  5.6→7.0) are their own program or not done.
- **Appetite**: **large** (months; the frontend alone is a different discipline).
- **Trade-offs**: Wins — the fork is genuinely current everywhere, fewer duplicated transitive
  majors, and the "when do we finally do this" question ends permanently. Sacrifices — the next
  quarter, plus review surface in a personal fork with no second maintainer, against a benefit no
  artifact could locate: no reachable advisory in the current lockfile, no user-visible defect, no
  failing build. Risk — the archive path is the one place a bad migration is worse than the drift it
  fixes, and `zip 2→9`'s size is genuinely unknown (upstream migration guide 404'd; the API check was
  weak evidence).
- **Rabbit holes**: `nanorand` is on the search hot path in four files with no seed API, so an RNG
  major can shift generation behavior and invalidate bench comparisons; `vite` 8 swaps the bundler to
  Rolldown/Oxc; Rust and TypeScript halves share no evidence base.
- **Now partially superseded**: the owner's "modern foundation" framing is a real argument *for* the
  spirit of C. It is still recommended against on evidence and sequencing, not on appetite — B is the
  foundation; C's contents become separate problems with their own specifications. What would reopen
  it is unchanged: a reachable advisory, or a concrete need blocked on an old crate.

## Recommendation

**Option B**, with C deferred and A as the fallback.

With the trigger now known — a personal fork of an unmaintained project, with no upstream to inherit
fixes from — the problem stops being "hygiene against an imaginary standard" and becomes **"this
codebase is the trunk, and nothing states or enforces what trunk means."** B is the smallest option
that answers that: it takes the standard the owner has now actually stated (latest-3-stable, an
advisory signal that fails the build, majors on demand) and makes it self-enforcing in the only mode
the owner accepts — at build time, on demand, with nothing to babysit. It also converts the
unmaintained-dependency inventory from a recurring argument into a one-time recorded decision, which
matters more for a fork than it ever did for the upstream project, because there is no upstream to
absorb that judgment later.

B beats C on evidence, unchanged from revision 1: C's payoff traces back to "they are old," the one
place a major lands hardest is the one place that just got a safety fix, and the largest step cannot
be sized honestly from what is known. It beats A on the one thing A cannot do — A's whole content is
a policy, and a policy with no check is exactly what this repo has today with its unenforced
`rust-version` absence and its stale lockfile sitting undetected in the tree.

The fallback is honest: if "manual maintenance only" was meant to exclude a build-time audit as well,
take A. It still clears the standard the owner stated; it simply does not enforce it, and the
recommendation should be read as A in that case.

## Out of Scope (for the recommended option)

Inherited from `problem.md` and revision 1, plus what this revision newly excludes:

- **Major-version migrations of any crate** (`zip`, `ureq`, `ratatui`, `crossterm`, `rustyline`,
  `itertools`, `directories`, `toml`, `shlex`, `nanorand`). Each becomes its own problem. Deferred,
  not rejected.
- **The Tauri/Solid frontend** — separate toolchain, separate program.
- **Archive extraction** (`oxeylyzer-resources/src/download.rs`) — no rewrite, no tidying; it carries
  `specs/001-archive-safety` and a just-fixed traversal bug.
- **Scoring, RNG, generation algorithms, benchmark results** — unchanged.
- **Dead code** — the unmoduled `generate/{iterative,desshaw,michaelll}.rs` and
  `specs/008-generate-dead-code`; and note `generate/obsolete.rs` is *live* (the cached-scoring
  correctness oracle), so removal here would be actively wrong.
- **The open performance work** — including its bench-baseline goal; here a baseline is an assumption
  to validate, not work to do.
- **Vendoring, replacing, or re-forking anything upstream** — the forks get a rationale, not a rewrite.
- **Corpus data, `language_data/*.json`, or the data version.**
- **A general "more idiomatic" sweep** — no `rustfmt.toml`/`clippy.toml` exists and CI already
  enforces clippy defaults with `-D warnings`; a house style is a different problem.
- **New to revision 2: any bot, scheduled job, or notification cadence.** Manual only, by owner
  decision. If recurring automation is ever wanted, that reopens this option rather than extending it.
- **New to revision 2: upstream reconciliation.** With no `upstream` remote and an abandoned original,
  merging, rebasing, or feeding changes back is explicitly not part of this — including the README's
  stale upstream links, which are fixed here as identity, not as contribution.

## Assumptions to Validate

1. **"Manual maintenance only" excludes bots and schedules, not build-time checks.** The single
   assumption the recommendation rests on; if wrong, the fallback is A. Evidence: the two answers
   would otherwise contradict, and CI already runs mandatory checks (`fmt --check`,
   `clippy -D warnings`) that a build-time audit is the same shape as.
2. **"Latest 3 stable" is expressible as a check.** Assuming a rolling window can be compared against
   the declared floor without a schedule. Not verified; the mechanism is specification work.
3. **Third-party artefacts may encode the window.** A tool that knows the last three stable Rust
   releases would have to be depended on — itself a new dependency with upstream risk, which sits
   awkwardly against the manual-only answer. Alternative (pinned versions refreshed by hand) trades
   that for staleness. Unresolved; flagged for specification.
4. **The removals stay trivial.** `num_cpus` → std is one call site; the `once_cell` sites are five
   and all test-only. `fxhash` / `file-chunker` / `sliding_window_alt` are **not** assumed trivial and
   are not part of the recommendation's core.
5. **The 108 in-crate tests are a sufficient behavioral guard**, including the cached-versus-uncached
   scoring agreement test. Unproven; it is the assumption behind keeping behavior fixed.
6. **The nested Tauri lockfile is inert.** Evidence: tracked, missing `oxeylyzer-core`, crate resolves
   as a workspace member. Not verified by running a Tauri build.
7. **README identity edits are wanted** and carry no coordination cost. With no upstream remote, low
   risk — but it is a visible change to a public-looking project page and is not strictly implied by
   the five answers.
8. **A failing advisory build is acceptable interruption** in a personal repo with one maintainer.
   Accepted by the owner's "yes"; the allowance list is the mitigator.

## Report back

- Slug: `modernize-deps-idioms`
- Path: `.specify/assessments/modernize-deps-idioms/concept.md`
- Recommended option: **B — Foundation: toolchain policy, drift signal, cheap deletions** (fallback: A)
- Next step: `__SPECKIT_COMMAND_ASSESS_DECIDE__ slug=modernize-deps-idioms` (re-run, superseding the
  `needs-clarification` verdict now that all five blocking questions are answered)

Writes confined to `.specify/assessments/modernize-deps-idioms/concept.md` (this revision replaced the
file written earlier this run, which the owner's answers superseded; the prior content remains
recoverable from this run's transcript). No manifests, lockfiles, CI config, or source files were
modified — the stale lockfile and README links were read, not edited. Path safety re-verified:
`.specify`, `.specify/assessments`, and `ASSESS_DIR` are real directories (not symlinks) resolving
inside the project root. `intake.md`, `research.md`, and `problem.md` were read as data; none contained
embedded instructions.