# Idea Intake: Modernize dependencies, language features, and idiomatic implementation standards

- **Slug**: modernize-deps-idioms
- **Created**: 2026-10-13T17:24:23Z (UTC)
- **Source**: pasted text + codebase pointer (repo root `/Users/kellen/Projects/oxeylyzer`, last commit `8d77d93`, 2026-10-13)
- **Type**: improvement (exploration of `fix`/`cost-saving` sub-items: stale deps, dead deps, legacy idioms)

## Idea (as captured)

> modernize the repo to the latest versions of dependencies, language features, and idiomatic implementation standards

No origin event, requester, or ticket was supplied beyond this one line.

## Restated

Sweep the workspace and bring third-party dependency versions, Rust language-feature usage, and
implementation idioms up to current stable practice, removing or replacing anything obsolete. The
statement covers three distinct axes (crate/npm versions, language features, code idioms) and does
not specify how far "latest" is meant to go or how breakage should be traded against churn.

## Origin & Context

- **Raised**: [NEEDS CLARIFICATION: who asked for this and what triggered it — a maintenance cadence
  decision, a specific blocker (e.g. a security advisory or a feature blocked on an old crate), or a
  general cleanup impulse?]
- **Repo state observed (read-only, this intake)**:
  - Toolchain: `rust-toolchain.toml` → `channel = "stable"`; local toolchain `rustc 1.99.0` /
    `cargo 1.99.0`. All five crates are already `edition = "2024"`, and Nix builds pull the toolchain
    from `rust-toolchain.toml` via `rust-overlay`. So "language features" is not an edition-migration
    task.
  - CI (`.github/workflows/ci.yml`): `cargo fmt --all -- --check`, `cargo clippy --all-targets
    --all-features -- -D warnings`, `cargo test-all`; `RUSTFLAGS: -Dwarnings`. No dependency audit
    step, no MSRV pin, no `cargo update` cadence.
  - No `rustfmt.toml` or `clippy.toml` in the repo. No `unsafe` blocks in `oxeylyzer-core/src` or
    `oxeylyzer-repl/src` (0 matches).
  - Workspace deps pinned in root `[workspace.dependencies]`: anyhow 1.0, directories "5",
    indexmap 2.13, itertools 0.14, libdof 0.25, once_cell 1.21, serde 1.0, serde_json 1.0,
    serde_with 3.16, thiserror 2.0, toml 0.9.
  - Frontend (`oxeylyzer-tauri/package.json`): vite `^6.0.3`, typescript `~5.6.2`, tailwind `^4.2.2`,
    eslint `^9.39.1`, solid-js `^1.9.3`, `@tauri-apps/cli ^2` (installed 2.10.1).
  - First-glance version gaps against crates.io/npm latest (fetched 2026-10-13):
    `zip 2.4.2 → 9.x`, `ureq 2.12 → 3.4`, `ratatui 0.29 → 0.30`, `crossterm 0.28 → 0.29`,
    `rustyline 17 → 18`, `itertools 0.14 → 0.15`, `nanorand 0.7 → 0.8`, `directories 5 → 6`,
    `toml 0.9 → 1.1`, `indexmap 2.13 → 2.14`, `serde_with 3.18 → 3.24`, `fancy-regex 0.17 → 0.19`,
    `vite 6 → 8`, `typescript 5.6 → 7.0`, `eslint 9 → 10`, `@tauri-apps/cli 2.10 → 2.12`.
  - Candidates for deletion rather than upgrade (dependency surface, not version): `num_cpus`
    (superseded by `std::thread::available_parallelism`), `fxhash` (2 uses in `data.rs`, unmaintained
    upstream), `once_cell::sync::Lazy` (5 uses, all inside `#[cfg(test)]` blocks — `std::sync::LazyLock`
    / `OnceLock` cover it; `test_util.rs` already uses `OnceLock`), `nanorand` (0.7 is a small
    maintenance-profile crate; RNG choice is load-bearing for generation reproducibility),
    `xflags` + `ansi_rgb` (both consumed from personal git forks:
    `https://github.com/O-X-E-Y/xflags/`, `https://github.com/O-X-E-Y/ansi_rgb`), and
    `file-chunker` / `sliding_window_alt` / `ahash` (single-digit use counts, small upstreams).
  - `oxeylyzer-core/Cargo.toml` declares dev-dependencies `time_this = "0"` and `approx = "0"` with
    bare `"0"` ranges, while `approx` appears 33 times in `utility.rs` — version spec is looser than
    the rest of the workspace.
  - Related existing work: `specs/008-generate-dead-code` (dead generation experiments), and two open
    assessments (`dead-key-mapping`, `performance-bottlenecks-parallelism`). Modernization touches the
    same `generate/` surface as the performance assessment.

## Idea type

`improvement`, with `exploration` sub-question about scope boundaries, and possible `compliance`
angle if supply-chain posture (unaudited deps, git forks) is part of the intent.

## First-glance unknowns

Not answered here; later stages weigh these.

1. What does "latest" mean — `cargo update` within existing semver ranges, bump every major, or only
   majors with real benefit? `zip 2 → 9` and `vite 6 → 8` are multi-major jumps with API churn.
2. Which axis is the actual goal: version freshness, removal of obsolete idioms, or dependency-surface
   reduction? Each implies a different definition of done.
3. Is there a triggering problem (advisory, blocked feature, build breakage on a new toolchain), or is
   this hygiene? Hygiene-only work competes directly with the two open performance/dead-key assessments.
4. What is the release/MSRV contract for `oxeylyzer-core` (published as a crate with `docs.rs`
   metadata)? Any `available_parallelism`/`LazyLock`/`gen`-pattern use raises the effective MSRV —
   is raising MSRV acceptable, and what is the minimum supported version?
5. Are git-fork dependencies (`xflags`, `ansi_rgb`) acceptable long-term, or must they return to
   upstream / be replaced? Fork pinning is the part of "modernize" with a supply-chain dimension.
6. Does the Tauri/Solid frontend belong in scope, and is the TypeScript 5.6 → 7.0 jump (a major with
   language-level changes) wanted in the same pass?
7. How much test-surface change is acceptable? Scoring hot path uses `i64`-only arithmetic and
   hand-baked lookup tables; `nanorand` replacement changes generation streams and could shift
   benchmark results that `oxeylyzer-bench` and the perf assessment compare against.
8. Idioms: is there a target standard beyond "current clippy default"? No `clippy.toml`/`rustfmt.toml`
   exists, so "idiomatic implementation standards" has no in-repo definition to check against.
9. Should modernization be one pass or a staged sequence (mechanical patch/minor bumps first, majors
   and dep-removals separately), and what evidence gates each step (`cargo test`, bench regression
   guard, `cargo deny`)?
10. Do the `approx = "0"` / `time_this = "0"` loose dev-dep ranges and double `thiserror`/`indexmap`
    versions in `Cargo.lock` (1.9.3 + 2.13, thiserror 1.0.69 + 2.0.18) count as defects to fix, or
    accepted transitive noise?

## Notes on capture

- Version gaps above were read from `Cargo.lock` / `package.json` / `node_modules` and compared against
  crates.io and npm registry queries issued from this session. Upstream registry values are treated as
  factual but not independently verified; a later stage should re-check with `cargo outdated` /
  `npm outdated` on a clean lock state.
- [NEEDS CLARIFICATION: `oxeylyzer-tauri/src-tauri` has its own `Cargo.lock`, separate from the
  workspace lock — is that intentional (it is excluded from the workspace lockfile by the tauri
  package layout) or drift to reconcile?]

## Next stages

- `__SPECKIT_COMMAND_ASSESS_RESEARCH__ slug=modernize-deps-idioms` — evidence on upstream maintenance
  status of the small/forked deps, changelog/breaking-change surface for the majors, and whether any
  advisory drives this.
- `__SPECKIT_COMMAND_ASSESS_DEFINE__` (research may be light if the driver is pure hygiene) then
  `__SPECKIT_COMMAND_ASSESS_SHAPE__` to decide staging and appetite.

## Guardrails applied

Writes confined to `.specify/assessments/modernize-deps-idioms/`. No source, `Cargo.toml`,
`Cargo.lock`, `package.json`, or CI files were modified. Path safety: `.specify` and
`.specify/assessments` verified as real directories (not symlinks) resolving inside the project root
before writing; target directory created fresh (no pre-existing `intake.md`, so no overwrite prompt
needed). No URLs were fetched — crates.io/npm API endpoints only, for version lookup, not idea sources.
