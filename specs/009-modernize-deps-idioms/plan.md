# Implementation Plan: Modern Dependency Foundation for Maintained Fork

**Branch**: `009-modernize-deps-idioms` | **Date**: 2026-10-13 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/009-modernize-deps-idioms/spec.md` (assessment handoff:
`.specify/assessments/modernize-deps-idioms/`, rev 3, verdict `go`).

## Summary

Establish "current" as a checked, not-remembered property of the repo, then pay down the measured drift.
Two halves:

1. **The gate (P1)** — the only part that stops the problem recurring: MSRV declared once and enforced
   against a dynamically computed window of the three latest stable Rust releases; `cargo audit` in CI with a
   reason-carryed allowance list; a `cargo outdated` freshness report that distinguishes "behind" from
   "recorded hold". Pre-existing `fmt`/`clippy` redness on `main` is fixed first so the gate can mean
   anything (FR-009).
2. **The catch-up (P2/P3)** — apply the full-major Rust upgrade set that was already built and tested on a
   throwaway copy (`zip 2→9`, `ureq 2→3`, `ratatui 0.29→0.30`, `crossterm 0.28→0.29`, `rustyline 17→18`,
   `itertools 0.14→0.15`, `directories 5→6`, `toml 0.9→1`, `shlex 1→3`, `nanorand 0.7→0.8`,
   `fancy-regex 0.17→0.19`), with the one real decision being the zip-9 archive containment policy (refuse;
   validate raw entry names before normalization). Delete stdlib-overlapping deps (`num_cpus`, test-only
   `once_cell`), replace the two personal git forks (`xflags` → clap with a parity-tested CLI surface,
   `ansi_rgb` → ~10-line in-repo helper), reconcile the broken nested Tauri lockfile, clear the two live
   advisories (`anyhow ≥ 1.0.103`, `rustls ≥ 0.23.45`), retarget fork identity (FR-016), and — last, because
   it is the only **unmeasured** item — the frontend majors (`vite 6→8` Rolldown migration, `typescript
   5.6→7.0`, `eslint 9→10`).

Hard fence: nothing changes scoring, generation, corpus/data-version behavior (FR-017).

## Technical Context

**Language/Version**: Rust, single window policy = 3 latest stable releases (feed-derived floor; 1.97 at
planning time, local toolchain 1.99). Frontend: TypeScript 5.6 → 7.0, ESM/Vite.
**Primary Dependencies**: post-upgrade — zip 9.x, ureq 3, ratatui 0.30, crossterm 0.29, rustyline 18,
itertools 0.15, directories 6, toml 1, shlex 3, nanorand 0.8, anyhow ≥1.0.103, clap v4 (new, replaces xflags
fork); frontend vite 8 (Rolldown), eslint 10, tauri v2 stack.
**Storage**: files — committed root `Cargo.lock` (single, after nested-lock deletion), `audit.toml`,
`msrv-holds.toml`, `docs/dependency-policy.md`; no DB.
**Testing**: `cargo test --workspace`, `cargo test-all` alias, `cargo clippy --all-targets --all-features -D
warnings`, `cargo fmt --all -- --check`, the 12 tests in `oxeylyzer-resources/src/download.rs` (6 hostile-input, keep reject semantics; 39 across the crate),
new CLI-parity table tests, `vite build` + `tsc --noEmit` + tauri dev smoke.
**Target Platform**: Linux/macOS desktop (Tauri), terminal REPL/bench; CI ubuntu-latest.
**Project Type**: Rust workspace (5 crates: core, repl, resources, bench, tauri/src-tauri) + SolidJS frontend.
**Performance Goals**: unchanged — scoring hot path must stay byte-for-byte behavior-equivalent; `fxhash`
retention is a deliberate perf choice (benchmark-first if ever swapped; out of scope here).
**Constraints**: FR-017 behavior fence; fail-closed network degradation for the gate; MSRV window beats any
single dependency's recency (FR-002a).
**Scale/Scope**: 6 manifests, 2 lockfiles → 1, ≈12 major-version gaps, 2 advisories, 2 fork deps, 2
stdlib-overlap deps, 1 containment policy decision, 1 CI workflow extended.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

`.specify/memory/constitution.md` is the **unfilled template** (placeholder principles only) — no project
principles are ratified, so there are no named gates to fail. The effective constraints for this feature
come from `CLAUDE.md`'s design invariants, and the plan honors each:

| Invariant (CLAUDE.md) | Plan impact |
|---|---|
| All scoring is `i64`; weights baked ×100 | Untouched — FR-017 fence; no PR touches hot-path types. |
| `Box<[T]>` / `Arc<[T]>` preferences | Untouched. |
| `CharMapping` u8-only hot paths | Untouched. |
| `FastLayout::possible_swaps` bakes pins | Untouched. |
| Engine trait / algorithm set | `ratatui`/`crossterm`/`rustyline` bumps are UI/IO-layer only. |

**Gate result**: PASS (both initial and post-Phase-1 re-check — no design decision introduces a violation).
Note for the record: this feature is exactly what a filled constitution would codify (dependency policy +
CI gates); `docs/dependency-policy.md` is the repo-side artifact, per research R10.

## Project Structure

### Documentation (this feature)

```text
specs/009-modernize-deps-idioms/
├── spec.md                  # input, incl. clarifications
├── plan.md                  # this file
├── research.md              # Phase 0 — R1..R13, all clarifications resolved
├── data-model.md            # Phase 1 — policy artifacts E1..E7 (no runtime store)
├── quickstart.md            # Phase 1 — runnable validation per user story
├── contracts/
│   ├── ci-gate.md           # Phase 1 — CI step/pass/fail/degradation contract (US1)
│   └── cli-surface.md       # Phase 1 — xflags→clap accept/reject parity contract (FR-015)
├── checklists/requirements.md
└── tasks.md                 # Phase 2 (/speckit.tasks — NOT created by this command)
```

### Source Code (repository root)

```text
Cargo.toml                          # [workspace.package] rust-version = floor (E1); toml/once_cell/xflags
                                    #   workspace deps updated
.cargo/config.toml                  # unchanged (test-all alias)
msrv-holds.toml                     # NEW (E3), possibly empty
audit.toml                          # NEW (E2), allowance list with reason comments
docs/dependency-policy.md           # NEW (E4) — incl. lockfile, containment, retained-with-rationale
.github/workflows/ci.yml            # + steps: msrv floor/match/build-check, audit, allowance-list lint,
                                    #   freshness (contracts/ci-gate.md); fmt/clippy fixed first (FR-009)
scripts/ci/                         # NEW: rust-window.sh (floor computation) + gate README
.github/CODEOWNERS?                 # not touched
Cargo.lock                          # single workspace lock, refreshed
README.md                           # FR-016 identity retarget

oxeylyzer-core/
├── Cargo.toml                      # num_cpus removed; itertools/nanorand/fancy-regex bumped;
│                                   #   once_cell dev-dep dropped; rust-version.workspace
└── src/
    ├── data.rs                     # :431 num_cpus::get() → std::thread::available_parallelism()
    ├── trigram_patterns.rs         # test-only Lazy → std::sync::LazyLock  (:242)
    ├── fast_layout.rs              #   (:1072)
    └── generate.rs                 #   (:1122)

oxeylyzer-repl/
├── Cargo.toml                      # xflags/ansi_rgb git deps dropped; clap added; shlex 3; once_cell gone
└── src/
    ├── flags.rs                    # xflags! → clap; parity tests (contracts/cli-surface.md)
    ├── color.rs                    # NEW: in-repo ansi_rgb replacement (~10 lines)
    ├── display.rs                  # single import swap to color.rs
    └── repl.rs                     # test-only LazyLock (:1186); clippy fix @759 (FR-009)

oxeylyzer-resources/
├── Cargo.toml                      # zip 9, ureq 3, directories 6, toml → workspace
└── src/download.rs                 # E5: raw-name containment (FR-013), sentinel invariant (FR-014)

oxeylyzer-bench/
├── Cargo.toml                      # xflags → clap; ratatui 0.30, crossterm 0.29
└── src/flags.rs                    # clap port + parity tests

oxeylyzer-tauri/
├── package.json                    # vite 8 (Rolldown), typescript 7, eslint 10 — last, unmeasured (R11)
├── bun.lock                        # retained (frontend ecosystem)
├── src-tauri/
│   ├── Cargo.toml                  # rust-version.workspace; toml 0.8 → workspace toml 1; notify/glob check
│   └── Cargo.lock                  # DELETED (E7, FR-012) + .gitignore entry
└── (src/)                          # frontend only if Rolldown needs config changes
```

**Structure Decision**: existing workspace is kept exactly as-is — five crates + one frontend directory; this
feature adds only top-level policy files (`msrv-holds.toml`, `audit.toml`, `docs/dependency-policy.md`), a
small `scripts/ci/` helper directory (floor-computation script + gate README), one
tiny in-repo module (`oxeylyzer-repl/src/color.rs`), and CI steps. No new crates.

### Task ordering (guidance for /speckit.tasks)

1. FR-009 fmt/clippy cleanup → CI green baseline (R12; commits standalone).
2. The standard: `[workspace.package] rust-version` + 5 inherits; MSRV CI steps (FR-001, FR-002, FR-002a; R1, R2).
3. The signal: audit + allowance-list lint + freshness CI steps, `audit.toml`, `msrv-holds.toml`,
   `docs/dependency-policy.md` (FR-003, FR-004, FR-005, FR-012, FR-013; R3, R4, R10). Gate exists **before**
   the catch-up commits, so every upgrade's CI signal is attributable (SC-001..SC-003, SC-008).
4. Deletions & replacements: num_cpus, once_cell, nested lockfile (FR-010, FR-011, FR-012; R8, R9).
5. zip 9 + containment decision, one focused commit with its six hostile tests (FR-006, FR-008, FR-013,
   FR-014, FR-017; R5; SC-007).
6. Remaining Rust majors: ureq/ratatui/crossterm/rustyline/itertools/directories/toml/shlex/nanorand/
   fancy-regex; anyhow/rustls advisory clears (FR-006, FR-007, FR-008; R6; SC-004).
7. xflags → clap with parity contract + ansi_rgb → helper (FR-015, FR-017; R7; SC-005).
8. Tauri Rust member reconciliation (FR-006, FR-012 — compile fixes unmeasured; budget first tauri compile here).
9. Frontend majors, last, with recorded-hold fallback (FR-006, FR-018; R11 — principal estimating risk).
10. FR-016 identity text; final full-matrix validation per quickstart.md (FR-008, FR-017; SC-006).

## Post-Design Constitution Re-check

No contracts introduced a violation: the CI contract is additive; the CLI contract *binds* behavior (it
requires no surface change), matching FR-017's fence; the data model has no runtime footprint. Gate remains
PASS.

## Complexity Tracking

No violations to justify. (Constitution is unfilled; the only new artifacts are the 3 policy files +
1 color helper + CI steps, each traceable to a numbered FR.)
