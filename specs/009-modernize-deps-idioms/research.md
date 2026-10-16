# Phase 0 Research: Modernize Dependency Foundation (009)

**Spec**: [spec.md](spec.md) · **Assessment provenance**: `.specify/assessments/modernize-deps-idioms/` (rev 3, verdict `go`)
All NEEDS CLARIFICATION from the spec were resolved in the 2026-10-13 clarification session; the open
questions listed in the assessment handoff are answered below as decisions, not re-opened.

## R1. Where the MSRV source of truth lives (FR-001, FR-002)

**Decision**: Declare `rust-version` in `[workspace.package]` in the root `Cargo.toml` (one authoritative
manifest location) and inherit it in all five member manifests via `rust-version.workspace = true`. CI
computes the "three most recent stable releases" window at build time from
`https://static.rust-lang.org/dist/channel-rust-stable.toml` (the official dist feed, no registry query),
derives the floor (oldest of the latest three), and performs two checks:
1. The declared `rust-version` string equals the computed floor (drift between stated floor and policy).
2. `cargo check --workspace` runs on a toolchain pinned to that floor (proof the floor actually builds).

The declared literal is a *mirror*, never the truth: when CI's computed floor moves, CI fails until the
mirror is updated, at which point the change is a one-line commit.

**Rationale**: The feed is the only stable machine-readable enumeration of stable releases; crates.io API
per-crate `rust_version` fields are sparse and unreliable for transitive deps. Two-sided checking (string
equality + floor-toolchain build) satisfies "machine-checkable" literally.
**Alternatives considered**:
- `rust-toolchain.toml` channel pin — rejected: it constrains *every* build to one toolchain and is not an
  MSRV declaration; cargo ignores it when checking `rust-version`.
- Dependabot/rust-toolchain automation — rejected by spec: "no pinned version literals serve source truth"
  and the toolchain channel is already `stable`; the window must be dynamic.
- Parsing `RUST_RELEASE.md` from the rust-lang/rust README — rejected: heuristic scraping vs. a structured TOML feed.

**Window at planning time**: latest stable = 1.99 → window {1.99, 1.98, 1.97} → floor **1.97**.
`ratatui 0.30` (requires 1.88) and `ureq 3` (1.71.1) sit comfortably inside; no FR-002a hold is currently
needed, but the mechanism must exist (see R2).

## R2. MSRV-window holds for dependencies (FR-002a, edge case "inherited MSRV exceeds floor")

**Decision**: A `msrv-holds.toml` file at repo root lists any dependency deliberately held below its latest
stable *because* the latest version's declared `rust-version` is newer than the CI window floor. Each entry:
crate, held version, latest version, required Rust of latest, reason. The freshness step (R3) reads this file
and downgrades "behind" to "held (recorded)" for listed crates; any *other* crate behind is a reportable gap.
CI additionally fails if a hold becomes stale (crate's latest now fits the window → hold must be removed).

**Rationale**: The clarification says window wins and holds must be recorded and *surfaced*, not silent. A
data file, not a comment, makes both the freshness report and the stale-hold check mechanical.
**Alternatives considered**: comments in `Cargo.toml` (unparseable by CI steps without fragile regexes);
`cargo deny` `sources` skips (wrong semantic — deny governs licenses/advisories, not version holds).

## R3. Advisory audit and allowance list (FR-003, FR-004, FR-007)

**Decision**: CI step `cargo install cargo-audit --locked && cargo audit`, with a repo-root `audit.toml`
(`[advisories] ignore = [...]`) as the allowance list. Each ignore entry MUST carry a one-line `# reason:`
comment directly above it; a CI check enforces that every `ignore` entry has a reason comment (the comment
is what makes the acceptance "discoverable in repo" per FR-004 — cargo-audit itself only stores the id).
Network fetch of advisory-db is required; on network failure the step fails loudly (see "no network" edge
case below).

**Rationale**: cargo-audit is the standard RustSec tool, reads the *resolved* lockfiles, exit code gates the
build. Both named advisories clear by version bumps alone: `anyhow ≥ 1.0.103` (currently 1.0.102, minor
`cargo update`), `rustls ≥ 0.23.45` (transitive under tauri; resolved by the tauri-side majors/lockfile
refresh per assessment evidence).

**Alternatives considered**:
- `cargo deny` with `[[policy.skip]] notes` — stronger structured justifications, but it also wants to own
  license bans and source allow-lists, which is new policy nobody asked for (YAGNI). cargo-audit + commented
  ignore list meets FR-004 minimally.
- `cargo audit --fix` — nonexistent; never silently patch policy.
**Edge cases**:
- *Advisory with no available patch*: keep the id in `ignore` with reason `no upstream fix; <plan>` — the
  list expresses acceptance, per spec; the audit step itself is never disabled.
- *No network*: the audit and freshness steps must `exit 1` with an explicit "audit skipped: network
  unavailable → build is not advisory-verified" message rather than a opaque YAML/git error. CI treats that
  as failure on `main` (fail-closed), since silent-pass would recreate the "undetected stale lockfile" problem.
- *Yanked-but-locked*: freshness step (`cargo outdated` reports yanked with a flag) surfaces it; policy is
  "report, then `cargo update -p <crate>` in the next maintenance pass" recorded in the policy doc.

## R4. Freshness report without manual registry queries (FR-005, FR-006, SC-003)

**Decision**: CI step `cargo outdated --workspace --exit-code portable` (non-gating in the report step; the
*gating* of majors is FR-006, enforced by the lockfile/manifest work in this feature and by the
behind-window check below), plus a small check that no direct dependency is behind the latest *stable* —
except crates listed in `msrv-holds.toml`. Output is the cargo-outdated table (name, project, compat, latest)
in the CI log; that table is the "single command or CI log answers it" of acceptance scenario US1-3.

**Rationale**: cargo-outdated resolves against the registry index already fetched by the build — no
hand-querying crates.io. FR-002a surface = the hold file is diffed against the outdated table.
**Alternatives considered**: `cargo upgrade --dry-run` (cargo-edit) — shows semver-compatible drift only, not
how far behind majors are; `cargo releases` — unmaintained. `versions-cli` — single-purpose, less common.

## R5. zip 2 → 9 and the containment policy (FR-013, FR-014, US2-3/4)

**Decision**: Upgrade `zip` to latest 9.x. In `oxeylyzer-resources/src/download.rs::extract_bytes`:
stop calling `ZipFile::enclosed_name()` for the containment decision; validate the **raw** entry name
(`file.name()`) with the in-repo `is_contained()` *after* stripping `ZIP_INNER_PREFIX`, before joining. The
raw-name validator must reject: absolute paths (`/x`, `C:\`, drive prefixes), NUL bytes, any path whose
component walk climbs above the destination after prefix stripping (existing six-test semantics), and —
new for zip 9 — names zip 9 would silently normalize (e.g. `oxeylyzer-data-files/../escaped.txt`
rewritten to `escaped.txt`): because validation runs on the raw string *before* any normalization, such
entries are refused, per the clarification "validate raw entry name in-repo; `is_contained` stays
authoritative". Loop aborts the whole archive before writing anything on the first hostile entry (current
behavior preserves this: writes are per-entry but `.data-version` is written only after full success —
FR-014 is already the shape; the test `mixed_valid_and_hostile_fails_without_partial_files` is the
guard and is kept).

**Rationale**: Measured in the assessment (rev-3 evidence): under zip 9, `enclosed_name()` *normalizes*
hostile names instead of rejecting them; nothing escaped, but normalization lets the loop write paths the
archive never claimed. Validating the raw name pre-empts normalization laundering entirely and keeps all
six hostile-input tests reject-semantic as the spec chose.
**Alternatives considered**: accept zip 9 normalization and rewrite the six tests (rejected by clarification
— refuse); drop `is_contained` and trust `enclosed_name()` (rejected — FR-013 names it explicitly).
Verified patch for the non-containment parts of the bump lives at
`.specify/assessments/modernize-deps-idioms/evidence/zip9-upgrade-reverted.patch` (5 compile errors / 6 test
failures / 9 clippy warnings measured, all resolved there).
**Residual known limit** (comment in code as `shortcut:`): partial per-entry writes before abort leave files
from already-written entries on disk; the *sentinel* is what marks completeness (FR-014), and the test asserts
no partial files for the mixed case. Full staging-to-temp-dir atomicity was NOT requested by the spec and is
out of scope.

## R6. ureq 2 → 3, ratatui 0.29 → 0.30 (+crossterm), rustyline 17 → 18, itertools 0.14 → 0.15, toml 0.9 → 1, directories 5 → 6, shlex 1 → 3, nanorand 0.7 → 0.8, fancy-regex 0.17 → 0.19 (FR-006, FR-008)

**Decision**: apply all measured-green upgrades. Assessment rev-3 built and tested every one of these on a
throwaway copy: each needed zero or one-line changes. The tauri `src-tauri` member additionally needs
`toml 0.8` bumped to workspace `toml` and `notify 6` checked for a maintained successor — **held as
as-of-yet-unmeasured items**: the probe dropped the tauri member to stay fast, so tauri-side compile fixes
are new work; task order puts them after the probe-verified crates.

**Rationale**: FR-006 demands latest-stable everywhere; the evidence says Rust-side cost ≈ three one-line
fixes plus the R5 containment decision. **Alternatives**: selective majors-only upgrades — rejected by
FR-006 and the "no half-policies" premise of the spec.

## R7. Personal-fork replacements: xflags → clap; ansi_rgb → in-repo helper (FR-015, US3-4)

**Decision**:
- `xflags` (git fork `O-X-E-Y/xflags`) replaced with `clap` (v4, derive or builder — builder chosen for the
  REPL: the xflags `cmd`/sub-command tree with aliases, `required`/`optional` positionals, `repeated`, and
  typed flag args maps directly onto clap subcommands; the exact accepted/rejected surface must be preserved
  and diff-checked). Both usage sites migrate: `oxeylyzer-repl/src/flags.rs` (125-line `xflags!` macro, ~20
  subcommands with aliases) and `oxeylyzer-bench/src/flags.rs` (19 lines, flat option set).
- `ansi_rgb` (git fork) replaced with a ~10-line in-repo color helper in `oxeylyzer-repl` (single import site
  in `display.rs`).

**Rationale**: Clarification 2026-10-13: replace both with maintained equivalents, no fork code retained.
clap is the maintained standard; the CLI surface contract is enforced by tests (see
`contracts/cli-surface.md`), because the edge case calls out that the xflags grammar "must be diff-checked,
not eyeballed".
**Alternatives considered**: `argh` (too weak for subcommand aliases + repeated args), `bpaf` (excellent but
less ubiquitous; team-recognition beats elegance here), hand-rolled parser (144 lines was the fork's size —
re-writing a *maintained* parser is the exact scope creep FR-015 avoids by saying "maintained equivalents").

## R8. Standard-library deletions: num_cpus, once_cell (FR-010, FR-011)

**Decision**:
- `num_cpus::get()` (single call site, `oxeylyzer-core/src/data.rs:431`) →
  `std::thread::available_parallelism()` — returns `NonZeroUsize`, one `.unwrap()` or `get()` at the boundary;
  drop the dependency.
- Test-only `once_cell::sync::Lazy` (4 `#[cfg(test)]` use sites in `trigram_patterns.rs`, `fast_layout.rs`,
  `generate.rs` (core) and `repl.rs`) → `std::sync::LazyLock` (stable since 1.80, well inside the 1.97 floor).
  Drop `once_cell` from `[workspace.dependencies]`, `oxeylyzer-core`, and `oxeylyzer-repl` manifests.

**Rationale**: Measured sites above; both are exact stdlib overlaps. **Alternatives**: none meaningful.

## R9. One lockfile policy (FR-012, US3-6)

**Decision**: The workspace (root `Cargo.toml`) includes `oxeylyzer-tauri/src-tauri` as a member, so cargo's
single root `Cargo.lock` covers it. Delete the nested `oxeylyzer-tauri/src-tauri/Cargo.lock` (last committed
2026-04-08, cannot resolve its own workspace's `oxeylyzer-core` — the undetected-drift exhibit), and
`.gitignore` the nested path so it never returns. Policy stated in `docs/dependency-policy.md` (R10):
"exactly one Cargo.lock at the workspace root; nested lockfiles are build artifacts of non-workspace
invocations and must not be committed". `bun.lock` in `oxeylyzer-tauri/` is the frontend's own single lockfile
— separate ecosystem, explicitly covered by the policy doc as the second (and only second) lockfile.

**Rationale**: Reconcile (delete) rather than "document intentionally separate": tauri member compiles from
workspace today (CI runs `cargo test --all` across it), so separateness is fiction already falsified by the
broken nested lock. **Alternatives**: keep nested lock + document — rejected: it is unresolvable garbage, not
a reproducibility asset.

## R10. The one authoritative policy document (FR-001..FR-006, FR-016)

**Decision**: `docs/dependency-policy.md` at repo root states: the MSRV window rule (and that the feed, not
the manifest literal, is truth), the audit gate + allowance-list procedure, the freshness/hold procedure, the
lockfile rule, the containment policy (FR-013), and the retained-with-rationale list for unmaintained deps
(`fxhash` — deliberate perf choice over ahash at its one call site, swap requires benchmark, out of scope per
spec Assumptions). FR-016 identity: `Cargo.toml` description/repository fields and `README.md` first
paragraphs retargeted to the maintained fork (`o-x-e-y/oxeylyzer`), removing abandoned-upstream framing.

**Rationale**: FR-004 "reason discoverable in repo", FR-016 "reader can tell this is a maintained fork", and
SC-005 "each retained dependency has one recorded reason" all point at one small file, not scattered
comments. **Alternatives**: wiki (external, drifts), constitution (spec-kit's template is unfilled and is
governance-of-process, not dependency policy — noted in the Constitution Check below).

## R11. Frontend majors: vite 6 → 8 (Rolldown), typescript 5.6 → 7.0, eslint 9 → 10 (FR-018)

**Decision**: In scope (clarification: "window-compatible frontend majors ARE in freshness report"; FR-018
states it MUST complete). Treat as the plan's principal estimating risk — the assessment never probed it
(evidence-strength caveat, rev 3): the tauri member was dropped from the throwaway build, and no frontend
compile/test was ever run at these versions. Sequence frontend work *after* all Rust-side work and the
containment decision, with an explicit checkpoint task: `bun run build` (vite 8 / Rolldown) + `tsc --noEmit`
(typescript 7) + `bun run tauri dev` manual smoke. If Rolldown breaks `vite-plugin-solid`/`vite-plugin-wasm`
or the Tailwind v4 plugin, the task's exit is a recorded, *surfaceable* hold (package.json override pins with
reasons in the policy doc), not a silent pin — the same holds-recorded-and-surfaced pattern as FR-002a.

**Rationale**: FR-018 is mandatory; the only honest de-risking available without measuring is to place it
last, bound its blast radius to the frontend directory, and define the fallback (recorded hold) up front so
"cannot land vite 8" is a documented outcome rather than a stalled feature.
**Alternatives considered**: deferring frontend to a second pass (rejected — spec's clarification session
explicitly kept `vite 6→8`, `typescript 5.6→7.0`, `eslint 9→10` in the window and FR-018 mandates them);
probing first with a throwaway copy now (deferred to task phase; the plan cannot spend a build cycle the
assessment already declined for cost reasons).

## R12. Pre-existing CI redness (FR-009)

**Decision**: Fix `cargo fmt --all -- --check` (9 blocks) and the `clippy -D warnings` failure at
`oxeylyzer-repl/src/repl.rs:759` as the *first* commit(s) of the work, before any dependency change, and
verify CI red/green state between them. Both verified pre-existing on pristine HEAD by the assessment.

**Rationale**: Spec: "no gate exists to protect any change" until CI is green; doing these first means every
subsequent commit's CI signal is attributable. **Alternatives**: bundle with upgrades — rejected: mixes
cause-of-failure during a red→green transition.

## R13. Scope fences (FR-017, Assumptions)

Not doing, explicitly: scoring/generation-algorithm changes; corpus data, `language_data/*.json`,
`.data-version` value changes; data-version migration; `fxhash` swap; seed/reproducibility APIs; upstream
reconciliation; general "more idiomatic" sweeps beyond the items named in the FRs.
